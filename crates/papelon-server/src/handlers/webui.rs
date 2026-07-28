use axum::body::Body;
use axum::extract::Path;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../webui/dist/"]
struct WebUiAssets;

pub async fn webui_redirect() -> Redirect {
    Redirect::permanent("/webui/")
}

pub async fn webui_index() -> Response {
    embedded_response("index.html", false)
}

pub async fn webui_asset(Path(path): Path<String>) -> Response {
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
