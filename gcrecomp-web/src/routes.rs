use axum::{
    extract::{Multipart, State},
    routing::{get, post, put},
    Json, Router,
};
use gcrecomp_core::recompiler::pipeline::{PipelineContext, RecompilationPipeline};
use std::sync::Arc;

use crate::security;
use crate::server::{AppState, RecompileStatus};

pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/upload", post(upload_dol))
        .route("/recompile", post(start_recompile))
        .route("/status", get(get_status))
        .route("/config", get(get_config))
        .route("/config", put(update_config))
        .route("/targets", get(list_targets))
}

async fn upload_dol(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?
    {
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

        if !security::validate_dol_magic(&data) {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid DOL file".to_string(),
            ));
        }

        // Save to temp location
        let upload_dir = std::path::Path::new("uploads");
        std::fs::create_dir_all(upload_dir)
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let path = upload_dir.join("uploaded.dol");
        std::fs::write(&path, &data)
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        // Initialize pipeline context
        let mut ctx = PipelineContext::new();
        let dol = gcrecomp_core::recompiler::parser::DolFile::parse(
            &data,
            path.to_str().unwrap_or("uploaded.dol"),
        )
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;
        ctx.dol_file = Some(dol);

        *state.pipeline_ctx.lock().await = Some(ctx);

        return Ok(Json(serde_json::json!({
            "status": "uploaded",
            "size": data.len(),
        })));
    }

    Err((
        axum::http::StatusCode::BAD_REQUEST,
        "No file provided".to_string(),
    ))
}

async fn start_recompile(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    // Check that we have a pipeline context
    {
        let ctx = state.pipeline_ctx.lock().await;
        if ctx.is_none() {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "No DOL file uploaded".to_string(),
            ));
        }
    }

    // Update status
    {
        let mut status = state.current_status.lock().await;
        *status = RecompileStatus {
            state: "running".to_string(),
            stage: "analyze".to_string(),
            stats: None,
            error: None,
        };
    }

    // Run pipeline stages in a background task
    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        let stages: &[(&str, fn(&mut PipelineContext) -> anyhow::Result<()>)] = &[
            ("analyze", RecompilationPipeline::stage_analyze),
            ("decode", RecompilationPipeline::stage_decode),
            ("build_cfg", RecompilationPipeline::stage_build_cfg),
            ("data_flow", RecompilationPipeline::stage_analyze_data_flow),
            ("type_inference", RecompilationPipeline::stage_infer_types),
            ("codegen", RecompilationPipeline::stage_generate_code),
            ("validate", RecompilationPipeline::stage_validate),
        ];

        for (name, stage_fn) in stages {
            {
                let mut status = state_clone.current_status.lock().await;
                status.stage = name.to_string();
            }

            let result = {
                let mut ctx_guard = state_clone.pipeline_ctx.lock().await;
                if let Some(ref mut ctx) = *ctx_guard {
                    stage_fn(ctx)
                } else {
                    Err(anyhow::anyhow!("Pipeline context lost"))
                }
            };

            if let Err(e) = result {
                let mut status = state_clone.current_status.lock().await;
                status.state = "error".to_string();
                status.error = Some(e.to_string());
                return;
            }
        }

        // Write output
        {
            let mut ctx_guard = state_clone.pipeline_ctx.lock().await;
            if let Some(ref mut ctx) = *ctx_guard {
                std::fs::create_dir_all("output").ok();
                if let Err(e) = RecompilationPipeline::stage_write_output(ctx, "output/recompiled.rs") {
                    let mut status = state_clone.current_status.lock().await;
                    status.state = "error".to_string();
                    status.error = Some(e.to_string());
                    return;
                }

                let mut status = state_clone.current_status.lock().await;
                status.state = "complete".to_string();
                status.stage = "done".to_string();
                status.stats = Some(ctx.stats.clone());
            }
        }
    });

    Ok(Json(serde_json::json!({
        "status": "started",
    })))
}

async fn get_status(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let status = state.current_status.lock().await;
    Json(serde_json::json!({
        "state": status.state,
        "stage": status.stage,
        "stats": status.stats,
        "error": status.error,
    }))
}

async fn get_config() -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let config = gcrecomp_ui::config::GameConfig::load()
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let value = serde_json::to_value(&config)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(value))
}

async fn update_config(
    Json(value): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let config: gcrecomp_ui::config::GameConfig = serde_json::from_value(value)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;
    config
        .save()
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({"status": "saved"})))
}

async fn list_targets() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "targets": [
            {"id": "x86_64-linux", "name": "x86_64 Linux"},
            {"id": "x86_64-windows", "name": "x86_64 Windows"},
            {"id": "aarch64-linux", "name": "AArch64 Linux"},
            {"id": "aarch64-macos", "name": "AArch64 macOS"},
        ]
    }))
}
