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
    // Parse all multipart fields by name
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name = String::new();
    let mut game_title = "game".to_string();
    let mut target = "x86_64-linux".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                file_name = field.file_name().unwrap_or("").to_string();
                let bytes = field.bytes().await.map_err(|e| {
                    (
                        axum::http::StatusCode::BAD_REQUEST,
                        format!("Failed to read uploaded file: {}", e),
                    )
                })?;
                file_data = Some(bytes.to_vec());
            }
            "game_title" => {
                let text = field.text().await.unwrap_or_default();
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    game_title = trimmed.to_string();
                }
            }
            "target" => {
                let text = field.text().await.unwrap_or_default();
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    target = trimmed.to_string();
                }
            }
            _ => {
                // Skip unknown fields
            }
        }
    }

    let Some(data) = file_data else {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "No file provided. Please select a .dol or .zip file.".to_string(),
        ));
    };

    if data.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Uploaded file is empty.".to_string(),
        ));
    }

    if data.len() > security::MAX_UPLOAD_SIZE {
        return Err((
            axum::http::StatusCode::PAYLOAD_TOO_LARGE,
            "File too large (max 64 MB).".to_string(),
        ));
    }

    // If the uploaded file is a zip, extract the first .dol from it
    let data = if file_name.to_lowercase().ends_with(".zip") {
        extract_dol_from_zip(&data)?
    } else {
        data
    };

    if !security::validate_dol_magic(&data) {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid DOL file. The file does not appear to be a valid GameCube DOL binary."
                .to_string(),
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
            "A recompilation is already in progress. Please wait for it to finish.".to_string(),
        ));
    }

    // Broadcast initial status
    let _ = state.status_tx.send(StatusEvent {
        state: "running".into(),
        stage: "upload".into(),
        message: "File uploaded, starting recompilation...".into(),
        stats: None,
        error: None,
        binary_path: None,
    });

    let size = data.len();

    // Spawn recompile + compile task
    let status_tx_lua = state.status_tx.clone();
    let status_tx_compile = state.status_tx.clone();
    let state_clone = state.clone();
    let target_for_compile = target.clone();
    let game_title_for_compile = game_title.clone();

    tokio::task::spawn(async move {
        // Phase 1: Lua recompilation pipeline
        let lua_result =
            tokio::task::spawn_blocking(move || run_lua_recompile(status_tx_lua, &target)).await;

        let stats = match lua_result {
            Ok(Ok(stats)) => stats,
            Ok(Err(e)) => {
                let _ = state_clone.status_tx.send(StatusEvent {
                    state: "error".into(),
                    stage: "".into(),
                    message: e.to_string(),
                    stats: None,
                    error: Some(e.to_string()),
                    binary_path: None,
                });
                state_clone.recompiling.store(false, Ordering::SeqCst);
                return;
            }
            Err(e) => {
                let _ = state_clone.status_tx.send(StatusEvent {
                    state: "error".into(),
                    stage: "".into(),
                    message: format!("Task panicked: {}", e),
                    stats: None,
                    error: Some(format!("Internal error: {}", e)),
                    binary_path: None,
                });
                state_clone.recompiling.store(false, Ordering::SeqCst);
                return;
            }
        };

        // Phase 2: Compile game executable
        let _ = status_tx_compile.send(StatusEvent {
            state: "running".into(),
            stage: "compile".into(),
            message: "Compiling game executable...".into(),
            stats: None,
            error: None,
            binary_path: None,
        });

        let target = target_for_compile;
        let game_title = game_title_for_compile;
        let compile_result =
            tokio::task::spawn_blocking(move || compile_game(&game_title, &target)).await;

        match compile_result {
            Ok(Ok(binary_path)) => {
                let _ = state_clone.status_tx.send(StatusEvent {
                    state: "complete".into(),
                    stage: "done".into(),
                    message: format!("Build complete: {}", binary_path),
                    stats: Some(stats),
                    error: None,
                    binary_path: Some(binary_path),
                });
            }
            Ok(Err(e)) => {
                let _ = state_clone.status_tx.send(StatusEvent {
                    state: "error".into(),
                    stage: "compile".into(),
                    message: format!("Compilation failed: {}", e),
                    stats: Some(stats),
                    error: Some(e.to_string()),
                    binary_path: None,
                });
            }
            Err(e) => {
                let _ = state_clone.status_tx.send(StatusEvent {
                    state: "error".into(),
                    stage: "compile".into(),
                    message: format!("Compile task panicked: {}", e),
                    stats: Some(stats),
                    error: Some(format!("Internal error: {}", e)),
                    binary_path: None,
                });
            }
        }

        state_clone.recompiling.store(false, Ordering::SeqCst);
    });

    Ok(Json(serde_json::json!({
        "status": "started",
        "size": size,
    })))
}

fn run_lua_recompile(
    status_tx: tokio::sync::broadcast::Sender<StatusEvent>,
    target: &str,
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
                binary_path: None,
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
    params
        .set("target", target)
        .map_err(|e| anyhow::anyhow!("Failed to set target: {}", e))?;

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

/// Compile the game crate and produce a named executable.
fn compile_game(game_title: &str, target: &str) -> anyhow::Result<String> {
    // Copy recompiled code into game crate
    std::fs::create_dir_all("output")?;
    if Path::new("output/recompiled.rs").exists() {
        std::fs::copy("output/recompiled.rs", "game/src/recompiled.rs")?;
    }

    // Determine the Rust target triple
    let target_triple = match target {
        "x86_64-linux" => "x86_64-unknown-linux-gnu",
        "x86_64-windows" => "x86_64-pc-windows-gnu",
        "aarch64-linux" => "aarch64-unknown-linux-gnu",
        "aarch64-macos" => "aarch64-apple-darwin",
        _ => "x86_64-unknown-linux-gnu",
    };

    // Build the game crate
    let output = std::process::Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "game",
            "--target",
            target_triple,
        ])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to invoke cargo: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Extract the most useful part of the error
        let short_err = stderr
            .lines()
            .filter(|l| l.contains("error") || l.contains("Error"))
            .take(10)
            .collect::<Vec<_>>()
            .join("\n");
        let msg = if short_err.is_empty() {
            stderr.to_string()
        } else {
            short_err
        };
        return Err(anyhow::anyhow!("Compilation failed:\n{}", msg));
    }

    // Determine binary extension and source path
    let ext = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };
    let src_binary = format!("target/{}/release/game{}", target_triple, ext);

    // Sanitize game title for use as filename
    let safe_title: String = game_title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let dst_name = format!("{}{}", safe_title, ext);
    let dst_path = format!("output/{}", dst_name);

    std::fs::copy(&src_binary, &dst_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to copy binary from {} to {}: {}",
            src_binary,
            dst_path,
            e
        )
    })?;

    log::info!("Game binary written to {}", dst_path);
    Ok(dst_name)
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
            format!("Invalid zip file: {}", e),
        )
    })?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                format!("Zip read error: {}", e),
            )
        })?;

        let name = file.name().to_lowercase();
        if name.ends_with(".dol") {
            if file.size() > security::MAX_UPLOAD_SIZE as u64 {
                return Err((
                    axum::http::StatusCode::PAYLOAD_TOO_LARGE,
                    "DOL file inside zip is too large.".to_string(),
                ));
            }
            let mut buf = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut buf).map_err(|e| {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    format!("Failed to extract DOL from zip: {}", e),
                )
            })?;
            return Ok(buf);
        }
    }

    Err((
        axum::http::StatusCode::BAD_REQUEST,
        "No .dol file found inside the zip archive.".to_string(),
    ))
}
