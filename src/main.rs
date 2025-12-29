mod appstate;
mod inference;
mod routes;
mod types;

use crate::appstate::AppState;
use crate::inference::worker::InferenceWorker;
use crate::routes::router;

use anyhow::Context;
use reqwest::Client;
use tracing::info;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    info!("starting server...");

    let (tx, rx) = tokio::sync::mpsc::channel(100);

    tokio::spawn(async move {
        let mut worker = InferenceWorker::new(
            rx,
            Client::new(),
            std::env::var("OLLAMA_BASE").unwrap_or_else(|_| "http://localhost:11434".into()),
        );
        worker.run().await;
    });

    let state = AppState { worker_tx: tx };

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
