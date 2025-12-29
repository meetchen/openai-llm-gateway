pub mod completions;

use axum::{Router, routing::post};

use crate::appstate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ask", post(completions::ask))
        .route("/completions", post(completions::completions_handler))
}
