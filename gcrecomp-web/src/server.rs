use anyhow::Result;
use axum::extract::DefaultBodyLimit;
use axum::Router;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tower_http::services::ServeDir;

use gcrecomp_lua::engine::LuaEngine;

use crate::routes;
use crate::security;

pub struct AppState {
    pub lua_engine: Mutex<LuaEngine>,
    pub status_tx: broadcast::Sender<StatusEvent>,
    pub recompiling: AtomicBool,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct StatusEvent {
    pub state: String,
    pub stage: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub struct WebServer {
    state: Arc<AppState>,
}

impl WebServer {
    pub fn new() -> Result<Self> {
        let engine = LuaEngine::new()?;

        // Set Lua package.path so require() can find scripts
        engine.set_package_path("lua/?.lua;lua/?/init.lua")?;

        // Load web routes into a global
        let lua = engine.lua();
        let script = std::fs::read_to_string("lua/web/routes.lua").unwrap_or_default();
        if !script.is_empty() {
            let routes: mlua::Table = lua
                .load(&script)
                .set_name("lua/web/routes.lua")
                .eval()
                .map_err(|e| anyhow::anyhow!("Failed to load web routes: {}", e))?;
            lua.globals()
                .set("web_routes", routes)
                .map_err(|e| anyhow::anyhow!("Failed to set web_routes global: {}", e))?;
        }

        let (status_tx, _) = broadcast::channel(64);

        let state = Arc::new(AppState {
            lua_engine: Mutex::new(engine),
            status_tx,
            recompiling: AtomicBool::new(false),
        });

        Ok(Self { state })
    }

    pub async fn run(self) -> Result<()> {
        let app = Router::new()
            .nest("/api", routes::api_routes())
            .layer(DefaultBodyLimit::max(security::MAX_UPLOAD_SIZE))
            .fallback_service(ServeDir::new("web/static"))
            .with_state(self.state);

        let addr = security::bind_address();
        let listener = tokio::net::TcpListener::bind(addr).await?;
        log::info!("Web UI server running at http://{}", addr);
        axum::serve(listener, app).await?;
        Ok(())
    }
}
