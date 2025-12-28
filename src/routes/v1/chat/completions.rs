use crate::AppState;
use axum::{
    body::Body,
    extract::{Json, State},
    http::{HeaderMap, HeaderName, HeaderValue, Response, StatusCode, header},
};
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tracing::{debug, info};

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
) -> Result<Response<Body>, (StatusCode, Json<ErrResp>)> {
    let base = state.ollama_base.trim_end_matches('/');
    let url = format!("{}/v1/chat/completions", base);

    debug!("Forwarding request to URL: {}", url);

    let start = Instant::now();
    let first_token_received = Arc::new(AtomicBool::new(false));

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

    let rid = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let mut header = HeaderMap::new();
    header.insert(
        HeaderName::from_static("x-rid"),
        rid.to_string().parse().unwrap(),
    );
    header.insert(
        HeaderName::from_static("x-gateway"),
        HeaderValue::from_static("openai-llm-gateway"),
    );
    header.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&content_type).unwrap(),
    );
    header.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));

    if body.stream {
        header.insert(header::CONNECTION, HeaderValue::from_static("keep-alive"));
        header.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/event-stream"),
        );

        let stream = resp
            .bytes_stream()
            .inspect_ok(move |chunk| {
                if !first_token_received.swap(true, Ordering::Relaxed) {
                    let ttft_ms = start.elapsed().as_millis();
                    info!(
                        rid = %rid,
                        ttft_ms = ttft_ms,
                        first_chunk_bytes = chunk.len(),
                        "ttft"
                    );
                }
            })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));

        let mut builder = Response::builder().status(status);
        for (key, value) in header.iter() {
            builder = builder.header(key, value);
        }
        let setup_duration = start.elapsed();
        info!(
            rid = %rid,
            setup_ms = setup_duration.as_millis(),
            "streaming response initialized"
        );
        return Ok(builder.body(Body::from_stream(stream)).unwrap());
    }
    let bytes = resp.bytes().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrResp {
                error: format!("Failed to read response body: {}", e),
            }),
        )
    })?;
    let mut builder = Response::builder().status(status);
    for (key, value) in header.iter() {
        builder = builder.header(key, value);
    }
    Ok(builder.body(Body::from(bytes)).unwrap())
}
