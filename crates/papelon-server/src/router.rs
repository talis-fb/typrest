use axum::Router;
use axum::routing::{get, post};

use crate::handlers::{healthz, list_templates, render_template};
use crate::server::AppState;
use crate::webui;

pub fn router(state: AppState) -> Router {
    router_with_web_ui(state, false)
}

pub fn router_with_web_ui(state: AppState, web_ui_enabled: bool) -> Router {
    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/templates", get(list_templates))
        .route("/render/{template}", post(render_template));

    let router = if web_ui_enabled {
        router.merge(webui::router())
    } else {
        router
    };

    router.with_state(state)
}
