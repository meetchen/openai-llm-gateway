use axum::{Router, routing::get};
use tracing::info;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/healthz", get(healthz))
}

pub async fn healthz() -> &'static str {
    info!("healthz check");
    "OK"
}
