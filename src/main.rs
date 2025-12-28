mod routes;

use anyhow::Context;
use reqwest::Client;
use tracing::info;

use crate::routes::router;

#[derive(Clone)]
pub struct AppState {
    pub client: Client,
    pub ollama_base: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    info!("starting server...");

    let state = AppState {
        client: Client::new(),
        ollama_base: std::env::var("OLLAMA_BASE")
            .unwrap_or_else(|_| "http://localhost:11434".into()),
    };

    let app = router().with_state(state);

    let addr = "0.0.0.0:8080";

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind to address: {}", addr))?;

    info!(
        "listening on {}",
        listener
            .local_addr()
            .context("failed to get local address")?
    );

    axum::serve(listener, app)
        .await
        .context("failed to start server")?;

    Ok(())
}
