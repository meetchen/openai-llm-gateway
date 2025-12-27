use tracing::info;

pub async fn healthz() -> &'static str {
    info!("healthz check");
    "OK"
}
