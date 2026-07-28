use axum::extract::Json;
use serde::Serialize;

pub async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}
