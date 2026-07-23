use axum::Router;
use axum::routing::{get, post};

use crate::handlers::{healthz, list_templates, render_template};
use crate::server::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/templates", get(list_templates))
        .route("/render/{template}", post(render_template))
        .with_state(state)
}
