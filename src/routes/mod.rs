pub mod healthz;
pub mod v1;

use crate::appstate::AppState;
use axum::Router;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(healthz::router())
        .nest("/v1", v1::router())
}
