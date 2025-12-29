use crate::types::ChatReq;
use axum::{
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue, Response, header},
};
use futures_util::TryStreamExt;
use reqwest::Client;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tracing::{debug, info};

#[derive(Debug)]
pub enum InferenceCommand {
    Chat {
        req: ChatReq,
        response_tx: tokio::sync::oneshot::Sender<anyhow::Result<Response<Body>>>,
    },
    Shutdown,
}

pub struct InferenceWorker {
    cmd_rx: tokio::sync::mpsc::Receiver<InferenceCommand>,
    client: Client,
    ollama_base: String,
}

impl InferenceWorker {
    pub fn new(
        cmd_rx: tokio::sync::mpsc::Receiver<InferenceCommand>,
        client: Client,
        ollama_base: String,
    ) -> Self {
        Self {
            cmd_rx,
            client,
            ollama_base,
        }
    }
    pub async fn run(mut self) {
        info!("Inference worker started");
        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                InferenceCommand::Chat { req, response_tx } => {
                    let res = self.handle_chat(req).await;
                    if response_tx.send(res).is_err() {
                        tracing::error!("Failed to send response");
                    };
                }
                InferenceCommand::Shutdown => {
                    break;
                }
            }
        }
        info!("Inference worker stopped");
    }

    async fn handle_chat(&self, body: ChatReq) -> anyhow::Result<Response<Body>> {
        // Implement chat handling logic here
        tracing::info!("Handling chat request: {:?}", body);
        let base = self.ollama_base.trim_end_matches('/');
        let url = format!("{}/v1/chat/completions", base);

        debug!("Forwarding request to URL: {}", url);

        let start = Instant::now();
        let first_token_received = Arc::new(AtomicBool::new(false));

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Upstream request failed: {}", e))?;

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
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response bytes: {}", e))?;
        let mut builder = Response::builder().status(status);
        for (key, value) in header.iter() {
            builder = builder.header(key, value);
        }
        Ok(builder.body(Body::from(bytes))?)
    }
}
