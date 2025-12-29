use crate::appstate::AppState;
use crate::inference::worker::InferenceCommand;
use crate::types::ChatReq;
use crate::types::ErrResp;
use axum::{
    body::Body,
    extract::{Json, State},
    http::{Response, StatusCode},
};

use tracing::info;

pub async fn ask() -> &'static str {
    info!("ask check");
    "OK"
}

pub async fn completions_handler(
    State(state): State<AppState>,
    Json(body): Json<ChatReq>,
) -> Result<Response<Body>, (StatusCode, Json<ErrResp>)> {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel::<anyhow::Result<Response<Body>>>();

    let cmd = InferenceCommand::Chat {
        req: body,
        response_tx: resp_tx,
    };

    state.worker_tx.send(cmd).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrResp {
                error: format!("Failed to send command to worker: {}", e),
            }),
        )
    })?;

    let res = resp_rx.await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrResp {
                error: "Worker response channel closed".into(),
            }),
        )
    })?;

    match res {
        Ok(res) => Ok(res),
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrResp {
                    error: format!("Worker failed to handle request: {}", e),
                }),
            ));
        }
    }
}
