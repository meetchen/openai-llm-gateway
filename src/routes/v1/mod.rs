pub mod chat;

use axum::Router;

use crate::appstate::AppState;

pub fn router() -> Router<AppState> {
    Router::new().nest("/chat", chat::router())
}
