use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        Html,
    },
    routing::{get, post, put},
    Json, Router,
};
use std::convert::Infallible;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

use gcrecomp_lua::convert::{json_to_lua_value, lua_table_to_json};
use gcrecomp_lua::engine::LuaEngine;

use crate::security;
use crate::server::{AppState, StatusEvent};

/// All application routes (no nesting required).
pub fn app_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(handle_index))
        .route(
            "/api/upload",
            post(handle_upload).layer(DefaultBodyLimit::disable()),
        )
        .route("/api/status", get(sse_status))
        .route("/api/config", get(handle_config_get))
        .route("/api/config", put(handle_config_set))
        .route("/api/targets", get(handle_targets))
}

// ---------------------------------------------------------------------------
// GET / — Lua renders the full HTML page
// ---------------------------------------------------------------------------

async fn handle_index(
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, (StatusCode, String)> {
    let engine = state.lua_engine.lock().await;
    let lua = engine.lua();

    let web_routes: mlua::Table = lua.globals().get("web_routes").map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Routes not loaded: {}", e),
        )
    })?;
    let handle_fn: mlua::Function = web_routes
        .get("handle_index")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let result: mlua::Table = handle_fn
        .call(())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let body: String = result
        .get("body")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Html(body))
}

// ---------------------------------------------------------------------------
// POST /api/upload — body limit disabled, manual 5 GB check
// ---------------------------------------------------------------------------

async fn handle_upload(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Manual size enforcement (DefaultBodyLimit is disabled on this route)
    if body.len() > security::MAX_UPLOAD_SIZE {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            "File too large (max 5 GB).".to_string(),
        ));
    }

    // Check recompile lock early
    if state
        .recompiling
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err((
            StatusCode::CONFLICT,
            "A recompilation is already in progress. Please wait for it to finish.".to_string(),
        ));
    }

    // Build params table and call Lua
    let lua_result = {
        let engine = state.lua_engine.lock().await;
        let lua = engine.lua();

        let web_routes: mlua::Table = lua.globals().get("web_routes").map_err(|e| {
            state.recompiling.store(false, Ordering::SeqCst);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Routes not loaded: {}", e),
            )
        })?;
        let handle_fn: mlua::Function = web_routes.get("handle_upload").map_err(|e| {
            state.recompiling.store(false, Ordering::SeqCst);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

        let params = lua.create_table().map_err(|e| {
            state.recompiling.store(false, Ordering::SeqCst);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

        let file_name = headers
            .get("x-file-name")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let game_title = headers
            .get("x-game-title")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("game");
        let target = headers
            .get("x-target")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("x86_64-linux");

        let set = |k: &str, v: mlua::Value| -> Result<(), (StatusCode, String)> {
            params.set(k, v).map_err(|e| {
                state.recompiling.store(false, Ordering::SeqCst);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })
        };

        set("file_name", mlua::Value::String(lua.create_string(file_name).map_err(|e| {
            state.recompiling.store(false, Ordering::SeqCst);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?))?;
        set("game_title", mlua::Value::String(lua.create_string(game_title).map_err(|e| {
            state.recompiling.store(false, Ordering::SeqCst);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?))?;
        set("target", mlua::Value::String(lua.create_string(target).map_err(|e| {
            state.recompiling.store(false, Ordering::SeqCst);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?))?;
        // Pass raw body as Lua binary string
        set("body", mlua::Value::String(lua.create_string(body.as_ref()).map_err(|e| {
            state.recompiling.store(false, Ordering::SeqCst);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?))?;
        set("body_len", mlua::Value::Integer(body.len() as i64))?;

        let result: mlua::Table = handle_fn.call(params).map_err(|e| {
            state.recompiling.store(false, Ordering::SeqCst);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

        // Read status code from Lua result
        let status: i64 = result.get("status").unwrap_or(200);
        if status != 200 {
            let body_str: String = result.get("body").unwrap_or_default();
            state.recompiling.store(false, Ordering::SeqCst);
            let code = match status {
                400 => StatusCode::BAD_REQUEST,
                413 => StatusCode::PAYLOAD_TOO_LARGE,
                409 => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            return Err((code, body_str));
        }

        // Extract pipeline control fields
        let start_pipeline: bool = result.get("_start_pipeline").unwrap_or(false);
        let game_title: String = result.get("_game_title").unwrap_or_default();
        let target: String = result.get("_target").unwrap_or_default();

        // Extract response body (table)
        let body_val: mlua::Value = result.get("body").map_err(|e| {
            state.recompiling.store(false, Ordering::SeqCst);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;
        let response_json = match body_val {
            mlua::Value::Table(t) => lua_table_to_json(&t).map_err(|e| {
                state.recompiling.store(false, Ordering::SeqCst);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })?,
            _ => serde_json::json!({}),
        };

        Ok::<_, (StatusCode, String)>((response_json, start_pipeline, game_title, target))
    }?;

    let (response_json, start_pipeline, game_title, target) = lua_result;

    if start_pipeline {
        // Broadcast initial status
        let _ = state.status_tx.send(StatusEvent {
            state: "running".into(),
            stage: "upload".into(),
            message: "File uploaded, starting recompilation...".into(),
            stats: None,
            error: None,
            binary_path: None,
        });

        let state_clone = state.clone();
        tokio::task::spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                run_lua_pipeline(state_clone.status_tx.clone(), &target, &game_title)
            })
            .await;

            match result {
                Ok(Ok(stats)) => {
                    let binary_path = stats
                        .get("binary_path")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let _ = state.status_tx.send(StatusEvent {
                        state: "complete".into(),
                        stage: "done".into(),
                        message: binary_path
                            .as_ref()
                            .map(|p| format!("Build complete: {}", p))
                            .unwrap_or_else(|| "Build complete".into()),
                        stats: Some(stats),
                        error: None,
                        binary_path,
                    });
                }
                Ok(Err(e)) => {
                    let _ = state.status_tx.send(StatusEvent {
                        state: "error".into(),
                        stage: "".into(),
                        message: e.to_string(),
                        stats: None,
                        error: Some(e.to_string()),
                        binary_path: None,
                    });
                }
                Err(e) => {
                    let _ = state.status_tx.send(StatusEvent {
                        state: "error".into(),
                        stage: "".into(),
                        message: format!("Task panicked: {}", e),
                        stats: None,
                        error: Some(format!("Internal error: {}", e)),
                        binary_path: None,
                    });
                }
            }

            state.recompiling.store(false, Ordering::SeqCst);
        });
    } else {
        state.recompiling.store(false, Ordering::SeqCst);
    }

    Ok(Json(response_json))
}

// ---------------------------------------------------------------------------
// Pipeline — fresh LuaEngine, calls routes.handle_recompile()
// ---------------------------------------------------------------------------

fn run_lua_pipeline(
    status_tx: tokio::sync::broadcast::Sender<StatusEvent>,
    target: &str,
    game_title: &str,
) -> anyhow::Result<serde_json::Value> {
    let engine = LuaEngine::new()?;
    engine.set_package_path("lua/?.lua;lua/?/init.lua")?;

    let lua = engine.lua();

    // Override gcrecomp.web.update_status with a broadcasting version
    let gcrecomp: mlua::Table = lua
        .globals()
        .get("gcrecomp")
        .map_err(|e| anyhow::anyhow!("Failed to get gcrecomp global: {}", e))?;
    let web_table: mlua::Table = gcrecomp
        .get("web")
        .map_err(|e| anyhow::anyhow!("Failed to get web table: {}", e))?;

    let tx = status_tx;
    let update_fn = lua
        .create_function(move |_, (stage, message): (String, String)| {
            let _ = tx.send(StatusEvent {
                state: "running".into(),
                stage,
                message,
                stats: None,
                error: None,
                binary_path: None,
            });
            Ok(())
        })
        .map_err(|e| anyhow::anyhow!("Failed to create update_status function: {}", e))?;

    web_table
        .set("update_status", update_fn)
        .map_err(|e| anyhow::anyhow!("Failed to set update_status: {}", e))?;

    // Load routes
    let script = std::fs::read_to_string("lua/web/routes.lua")?;
    let routes: mlua::Table = lua
        .load(&script)
        .set_name("lua/web/routes.lua")
        .eval()
        .map_err(|e| anyhow::anyhow!("Failed to load routes: {}", e))?;

    // Create params table
    let params = lua
        .create_table()
        .map_err(|e| anyhow::anyhow!("Failed to create params table: {}", e))?;
    params
        .set("dol_path", "uploads/uploaded.dol")
        .map_err(|e| anyhow::anyhow!("Failed to set dol_path: {}", e))?;
    params
        .set("output_path", "output/recompiled.rs")
        .map_err(|e| anyhow::anyhow!("Failed to set output_path: {}", e))?;
    params
        .set("target", target)
        .map_err(|e| anyhow::anyhow!("Failed to set target: {}", e))?;
    params
        .set("game_title", game_title)
        .map_err(|e| anyhow::anyhow!("Failed to set game_title: {}", e))?;

    // Call handle_recompile
    let handle_fn: mlua::Function = routes
        .get("handle_recompile")
        .map_err(|e| anyhow::anyhow!("Failed to get handle_recompile: {}", e))?;
    let result: mlua::Table = handle_fn
        .call(params)
        .map_err(|e| anyhow::anyhow!("Recompilation failed: {}", e))?;

    let stats_json = lua_table_to_json(&result)
        .map_err(|e| anyhow::anyhow!("Failed to convert stats: {}", e))?;

    Ok(stats_json)
}

// ---------------------------------------------------------------------------
// GET /api/status — SSE stream (stays in Rust, async requirement)
// ---------------------------------------------------------------------------

async fn sse_status(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.status_tx.subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|result| result.ok())
        .map(|event| {
            let data = serde_json::to_string(&event).unwrap_or_default();
            Ok(Event::default().data(data))
        });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

// ---------------------------------------------------------------------------
// Generic Lua handler helper for simple request/response routes
// ---------------------------------------------------------------------------

async fn call_lua_handler(
    state: &AppState,
    method: &str,
    arg: Option<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let engine = state.lua_engine.lock().await;
    let lua = engine.lua();

    let web_routes: mlua::Table = lua.globals().get("web_routes").map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Routes not loaded: {}", e),
        )
    })?;
    let handle_fn: mlua::Function = web_routes
        .get(method)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let result: mlua::Table = if let Some(val) = arg {
        let lua_val =
            json_to_lua_value(lua, &val).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        handle_fn
            .call(lua_val)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        handle_fn
            .call(())
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };

    let json = lua_table_to_json(&result)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json))
}

async fn handle_config_get(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    call_lua_handler(&state, "handle_config_get", None).await
}

async fn handle_config_set(
    State(state): State<Arc<AppState>>,
    Json(value): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    call_lua_handler(&state, "handle_config_set", Some(value)).await
}

async fn handle_targets(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    call_lua_handler(&state, "handle_list_targets", None).await
}
