use axum::{
    extract::{Multipart, State},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post, put},
    Json, Router,
};
use std::convert::Infallible;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

use gcrecomp_lua::convert::{json_to_lua_value, lua_table_to_json};
use gcrecomp_lua::engine::LuaEngine;

use crate::security;
use crate::server::{AppState, StatusEvent};

pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/upload", post(upload_dol))
        .route("/status", get(sse_status))
        .route("/config", get(get_config))
        .route("/config", put(update_config))
        .route("/targets", get(list_targets))
}

async fn upload_dol(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    let Some(field) = field else {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "No file provided".to_string(),
        ));
    };

    let file_name = field.file_name().unwrap_or("").to_string();

    let data = field
        .bytes()
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    if data.len() > security::MAX_UPLOAD_SIZE {
        return Err((
            axum::http::StatusCode::PAYLOAD_TOO_LARGE,
            "File too large".to_string(),
        ));
    }

    // If the uploaded file is a zip, extract the first .dol from it
    let data = if file_name.ends_with(".zip") {
        extract_dol_from_zip(&data)?
    } else {
        data.to_vec()
    };

    if !security::validate_dol_magic(&data) {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid DOL file".to_string(),
        ));
    }

    // Save to disk
    let upload_dir = Path::new("uploads");
    std::fs::create_dir_all(upload_dir)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let dol_path = upload_dir.join("uploaded.dol");
    std::fs::write(&dol_path, &data)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Check recompile lock
    if state
        .recompiling
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err((
            axum::http::StatusCode::CONFLICT,
            "Recompile already in progress".to_string(),
        ));
    }

    // Broadcast initial status
    let _ = state.status_tx.send(StatusEvent {
        state: "running".into(),
        stage: "upload".into(),
        message: "File uploaded, starting recompilation...".into(),
        stats: None,
        error: None,
    });

    // Spawn recompile task
    let status_tx = state.status_tx.clone();
    let state_clone = state.clone();
    tokio::task::spawn(async move {
        let result = tokio::task::spawn_blocking(move || run_lua_recompile(status_tx)).await;

        match result {
            Ok(Ok(stats)) => {
                let _ = state_clone.status_tx.send(StatusEvent {
                    state: "complete".into(),
                    stage: "done".into(),
                    message: "Recompilation complete".into(),
                    stats: Some(stats),
                    error: None,
                });
            }
            Ok(Err(e)) => {
                let _ = state_clone.status_tx.send(StatusEvent {
                    state: "error".into(),
                    stage: "".into(),
                    message: e.to_string(),
                    stats: None,
                    error: Some(e.to_string()),
                });
            }
            Err(e) => {
                let _ = state_clone.status_tx.send(StatusEvent {
                    state: "error".into(),
                    stage: "".into(),
                    message: format!("Task panicked: {}", e),
                    stats: None,
                    error: Some(format!("Internal error: {}", e)),
                });
            }
        }

        state_clone.recompiling.store(false, Ordering::SeqCst);
    });

    let size = data.len();
    Ok(Json(serde_json::json!({
        "status": "started",
        "size": size,
    })))
}

fn run_lua_recompile(
    status_tx: tokio::sync::broadcast::Sender<StatusEvent>,
) -> anyhow::Result<serde_json::Value> {
    let engine = LuaEngine::new()?;
    engine.set_package_path("lua/?.lua;lua/?/init.lua")?;

    let lua = engine.lua();

    // Inject gcrecomp.web.update_status
    let gcrecomp: mlua::Table = lua
        .globals()
        .get("gcrecomp")
        .map_err(|e| anyhow::anyhow!("Failed to get gcrecomp global: {}", e))?;
    let web_table = lua
        .create_table()
        .map_err(|e| anyhow::anyhow!("Failed to create web table: {}", e))?;

    let tx = status_tx;
    let update_fn = lua
        .create_function(move |_, (stage, message): (String, String)| {
            let _ = tx.send(StatusEvent {
                state: "running".into(),
                stage,
                message,
                stats: None,
                error: None,
            });
            Ok(())
        })
        .map_err(|e| anyhow::anyhow!("Failed to create update_status function: {}", e))?;

    web_table
        .set("update_status", update_fn)
        .map_err(|e| anyhow::anyhow!("Failed to set update_status: {}", e))?;
    gcrecomp
        .set("web", web_table)
        .map_err(|e| anyhow::anyhow!("Failed to set web table: {}", e))?;

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

    // Call handle_recompile
    let handle_fn: mlua::Function = routes
        .get("handle_recompile")
        .map_err(|e| anyhow::anyhow!("Failed to get handle_recompile: {}", e))?;
    let result: mlua::Table = handle_fn
        .call(params)
        .map_err(|e| anyhow::anyhow!("Recompilation failed: {}", e))?;

    // Convert result to JSON
    let stats_json = lua_table_to_json(&result)
        .map_err(|e| anyhow::anyhow!("Failed to convert stats: {}", e))?;

    Ok(stats_json)
}

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

async fn get_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let engine = state.lua_engine.lock().await;
    let lua = engine.lua();

    let web_routes: mlua::Table = lua.globals().get("web_routes").map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Routes not loaded: {}", e),
        )
    })?;
    let handle_fn: mlua::Function = web_routes
        .get("handle_config_get")
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let result: mlua::Table = handle_fn
        .call(())
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let json = lua_table_to_json(&result)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json))
}

async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(value): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let engine = state.lua_engine.lock().await;
    let lua = engine.lua();

    let web_routes: mlua::Table = lua.globals().get("web_routes").map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Routes not loaded: {}", e),
        )
    })?;
    let handle_fn: mlua::Function = web_routes
        .get("handle_config_set")
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let lua_value = json_to_lua_value(lua, &value)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let result: mlua::Table = handle_fn
        .call(lua_value)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let json = lua_table_to_json(&result)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json))
}

async fn list_targets(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let engine = state.lua_engine.lock().await;
    let lua = engine.lua();

    let web_routes: mlua::Table = lua.globals().get("web_routes").map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Routes not loaded: {}", e),
        )
    })?;
    let handle_fn: mlua::Function = web_routes
        .get("handle_list_targets")
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let result: mlua::Table = handle_fn
        .call(())
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let json = lua_table_to_json(&result)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json))
}

/// Extract the first .dol file found inside a zip archive.
fn extract_dol_from_zip(zip_data: &[u8]) -> Result<Vec<u8>, (axum::http::StatusCode, String)> {
    let cursor = std::io::Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            format!("Invalid zip file: {e}"),
        )
    })?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                format!("Zip read error: {e}"),
            )
        })?;

        let name = file.name().to_lowercase();
        if name.ends_with(".dol") {
            if file.size() > security::MAX_UPLOAD_SIZE as u64 {
                return Err((
                    axum::http::StatusCode::PAYLOAD_TOO_LARGE,
                    "DOL file inside zip is too large".to_string(),
                ));
            }
            let mut buf = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut buf).map_err(|e| {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    format!("Failed to extract DOL from zip: {e}"),
                )
            })?;
            return Ok(buf);
        }
    }

    Err((
        axum::http::StatusCode::BAD_REQUEST,
        "No .dol file found inside the zip archive".to_string(),
    ))
}
