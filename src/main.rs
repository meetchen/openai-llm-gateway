mod routes;

use crate::routes::healthz::healthz;
use anyhow::Context;
use axum::{Router, routing::get};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    info!("starting server...");

    let app = Router::new().route("/healthz", get(healthz));

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
