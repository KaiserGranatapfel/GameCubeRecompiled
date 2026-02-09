use anyhow::Result;
use axum::Router;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

use gcrecomp_core::recompiler::pipeline::{PipelineContext, PipelineStats};
use gcrecomp_lua::engine::LuaEngine;

use crate::routes;
use crate::security;

pub struct AppState {
    #[allow(dead_code)]
    pub lua_engine: Mutex<LuaEngine>,
    pub pipeline_ctx: Mutex<Option<PipelineContext>>,
    pub current_status: Mutex<RecompileStatus>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecompileStatus {
    pub state: String,
    pub stage: String,
    pub stats: Option<PipelineStats>,
    pub error: Option<String>,
}

impl Default for RecompileStatus {
    fn default() -> Self {
        Self {
            state: "idle".to_string(),
            stage: "".to_string(),
            stats: None,
            error: None,
        }
    }
}

pub struct WebServer {
    state: Arc<AppState>,
}

impl WebServer {
    pub fn new() -> Result<Self> {
        let lua_engine = LuaEngine::new()?;
        let state = Arc::new(AppState {
            lua_engine: Mutex::new(lua_engine),
            pipeline_ctx: Mutex::new(None),
            current_status: Mutex::new(RecompileStatus::default()),
        });
        Ok(Self { state })
    }

    pub async fn run(self) -> Result<()> {
        let app = Router::new()
            .nest("/api", routes::api_routes())
            .fallback_service(ServeDir::new("web/static"))
            .with_state(self.state);

        let addr = security::bind_address();
        let listener = tokio::net::TcpListener::bind(addr).await?;
        log::info!("Web UI server running at http://{}", addr);
        axum::serve(listener, app).await?;
        Ok(())
    }
}
