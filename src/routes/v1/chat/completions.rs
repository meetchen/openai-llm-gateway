use axum::{
    extract::{Json, State},
    http::{HeaderName, StatusCode, header},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

use crate::AppState;

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatReq {
    model: String,
    messages: Vec<Message>,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
pub struct HealthResp {
    ok: bool,
}

#[derive(Serialize)]
pub struct ErrResp {
    error: String,
}

pub async fn ask() -> &'static str {
    info!("ask check");
    "OK"
}

pub async fn completions_handler(
    State(state): State<AppState>,
    Json(body): Json<ChatReq>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrResp>)> {
    info!("completions check");
    info!(">>> HIT completions_handler <<<");

    info!("Request Body: {:?}", body);
    for message in &body.messages {
        info!(
            "Message - Role: {}, Content: {}",
            message.role, message.content
        );
    }

    let base = state.ollama_base.trim_end_matches('/');
    let url = format!("{}/v1/chat/completions", base);

    info!("Forwarding request to URL: {}", url);

    let resp = state
        .client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrResp {
                    error: format!("Failed to reach external service: {}", e),
                }),
            )
        })?;

    let status = resp.status();

    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    let bytes = resp.bytes().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrResp {
                error: format!("Failed to read response body: {}", e),
            }),
        )
    })?;

    let rid = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    Ok((
        status,
        [
            (header::CONTENT_TYPE, content_type),
            (HeaderName::from_static("x-rid"), rid.to_string()),
        ],
        bytes,
    ))
}
