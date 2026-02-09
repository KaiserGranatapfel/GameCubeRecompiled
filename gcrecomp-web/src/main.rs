mod routes;
mod security;
mod server;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let server = server::WebServer::new()?;
    server.run().await
}
