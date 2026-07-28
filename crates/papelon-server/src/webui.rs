use axum::Router;
use axum::body::Body;
use axum::extract::{Json, Path, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use papelon_core::{Schema, TemplateName};
use rust_embed::Embed;
use serde::Serialize;

use crate::errors::ApiError;
use crate::server::AppState;

#[derive(Embed)]
#[folder = "../../webui/dist/"]
struct WebUiAssets;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/webui", get(webui_redirect))
        .route("/webui/", get(webui_index))
        .route("/webui/{*path}", get(webui_asset))
        .route("/_api/templates", get(list_templates))
        .route("/_api/templates/{template}", get(template_metadata))
}

async fn webui_redirect() -> Redirect {
    Redirect::permanent("/webui/")
}

async fn webui_index() -> Response {
    embedded_response("index.html", false)
}

async fn webui_asset(Path(path): Path<String>) -> Response {
    embedded_response(&path, true)
}

fn embedded_response(path: &str, cache_asset: bool) -> Response {
    let Some(asset) = WebUiAssets::get(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let mut response = Body::from(asset.data).into_response();
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let content_type = if matches!(
        std::path::Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str()),
        Some("html" | "css" | "js" | "json" | "svg" | "txt")
    ) {
        format!("{mime}; charset=utf-8")
    } else {
        mime.to_string()
    };
    if let Ok(content_type) = HeaderValue::from_str(&content_type) {
        response.headers_mut().insert(CONTENT_TYPE, content_type);
    }
    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static(if cache_asset {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        }),
    );
    response
}

async fn list_templates(
    State(state): State<AppState>,
) -> Result<Json<WebUiTemplatesResponse>, ApiError> {
    let templates = state
        .registry
        .names()
        .into_iter()
        .map(|name| {
            let has_schema = state
                .registry
                .get(&name)
                .is_some_and(|entry| entry.schema().is_some());
            WebUiTemplateSummary {
                name: name.into_inner(),
                has_schema,
            }
        })
        .collect();

    Ok(Json(WebUiTemplatesResponse { templates }))
}

async fn template_metadata(
    State(state): State<AppState>,
    Path(template): Path<String>,
) -> Result<Json<WebUiTemplateMetadata>, ApiError> {
    let name = TemplateName::new(template).map_err(|error| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_template_name",
            error.to_string(),
        )
    })?;
    let entry = state.registry.get(&name).ok_or_else(|| {
        ApiError::new(
            StatusCode::NOT_FOUND,
            "template_not_found",
            format!("Template '{name}' was not found"),
        )
    })?;

    Ok(Json(WebUiTemplateMetadata {
        name: name.into_inner(),
        schema: entry.schema().cloned(),
    }))
}

#[derive(Serialize)]
struct WebUiTemplatesResponse {
    templates: Vec<WebUiTemplateSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WebUiTemplateSummary {
    name: String,
    has_schema: bool,
}

#[derive(Serialize)]
struct WebUiTemplateMetadata {
    name: String,
    schema: Option<Schema>,
}
