use axum::Router;
use axum::routing::{get, post};

use crate::config::ServerConfig;
use crate::handlers::{api, healthz, render_template, webui};
use crate::server::AppState;

pub fn router(state: AppState, config: &ServerConfig) -> Router {
    let router = Router::new()
        .route("/_healthz", get(healthz::healthz))
        .route("/_api/templates", get(api::list_templates))
        .route("/_api/templates/{template}", get(api::template_metadata))
        .route("/render/{template}", post(render_template::render_template));

    let router = if config.web_ui {
        router
            .route("/webui", get(webui::webui_redirect))
            .route("/webui/", get(webui::webui_index))
            .route("/webui/{*path}", get(webui::webui_asset))
    } else {
        router
    };

    router.with_state(state)
}
