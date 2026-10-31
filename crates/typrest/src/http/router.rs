use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::get;

use crate::http::config::ServerConfig;
use crate::http::handlers::{api, healthz, render_template, webui};
use crate::http::state::AppState;

pub fn router(state: AppState, config: &ServerConfig) -> Router {
    let router = Router::new()
        .route("/_healthz", get(healthz::healthz))
        .route("/_api/templates", get(api::list_templates))
        .route(
            "/_api/templates/{*template_id}",
            get(api::template_metadata),
        )
        .route(
            "/{*template_id}",
            get(render_template::render_template).post(render_template::render_template),
        );

    let router = if config.web_ui {
        router
            .route("/", get(webui::webui_redirect))
            .route("/_webui", get(webui::webui_redirect))
            .route("/_webui/", get(webui::webui_index))
            .route("/_webui/{*path}", get(webui::webui_asset))
    } else {
        router
            .route("/_webui", get(webui::webui_not_found))
            .route("/_webui/", get(webui::webui_not_found))
            .route("/_webui/{*path}", get(webui::webui_not_found))
    };

    router
        .layer(DefaultBodyLimit::max(config.max_body_bytes))
        .with_state(state)
}
