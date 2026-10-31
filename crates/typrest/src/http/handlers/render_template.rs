use std::time::Instant;

use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::Value;
use tracing::{info, warn};
use typrest_core::{CompilationError, Document, RenderError, RenderRequest, TemplateId};

use crate::http::errors::ApiError;
use crate::http::state::AppState;
use crate::logging::compiler_diagnostics;

pub async fn render_template(
    State(state): State<AppState>,
    Path(template_id): Path<String>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let template = TemplateId::new(template_id).map_err(|error| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_template_id",
            error.to_string(),
        )
    })?;
    let template_for_log = template.clone();

    if method == Method::POST && !has_json_content_type(&headers) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_json",
            "request body must have content-type application/json",
        ));
    }

    let payload = if body.is_empty() {
        Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_slice(&body).map_err(|error| {
            ApiError::new(StatusCode::BAD_REQUEST, "invalid_json", error.to_string())
        })?
    };

    let started_at = Instant::now();

    let result = state
        .render_service
        .render(RenderRequest::new(template, payload))
        .await;
    let elapsed_ms = started_at.elapsed().as_millis();

    match result {
        Ok(document) => {
            let pdf_size = document.bytes().len();
            for warning in document.warnings() {
                compiler_diagnostics::trace(
                    state.log_format,
                    "render",
                    "compile",
                    Some(&template_for_log),
                    warning,
                );
            }
            info!(
                event = "render.completed",
                template = %template_for_log,
                latency_ms = elapsed_ms,
                pdf_size,
                warning_count = document.warnings().len(),
                status = "ok",
                "rendered template `{template_for_log}` in {elapsed_ms} ms"
            );
            Ok(pdf_response(document))
        }
        Err(error) => {
            if let RenderError::CompilationError(compilation_error) = &error {
                let stage = match compilation_error {
                    CompilationError::Failed(_) => "compile",
                    CompilationError::ExportFailed(_) => "export",
                };
                for value in compilation_error.report().diagnostics() {
                    compiler_diagnostics::trace(
                        state.log_format,
                        "render",
                        stage,
                        Some(&template_for_log),
                        value,
                    );
                }
            }
            warn!(
                event = "render.failed",
                template = %template_for_log,
                latency_ms = elapsed_ms,
                status = "error",
                error_kind = render_error_kind(&error),
                execution_error = execution_error_detail(&error),
                "failed to render template `{template_for_log}` after {elapsed_ms} ms: {}",
                render_error_kind(&error)
            );
            Err(ApiError::from_render_error(error))
        }
    }
}

fn has_json_content_type(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|mime| mime.trim() == "application/json")
        })
}

/// The raw failure detail, for logs only. It may contain source text or paths, so it never reaches
/// the response body.
fn execution_error_detail(error: &RenderError) -> Option<&str> {
    match error {
        RenderError::ExecutionFailed { message } => Some(message),
        _ => None,
    }
}

fn render_error_kind(error: &RenderError) -> &'static str {
    match error {
        RenderError::TemplateNotFound(_) => "template_not_found",
        RenderError::ValidationError(_) => "schema_validation_failed",
        RenderError::CompilationError(CompilationError::Failed(_)) => "compilation_failed",
        RenderError::CompilationError(CompilationError::ExportFailed(_)) => "pdf_export_failed",
        RenderError::Overloaded => "render_capacity_exceeded",
        RenderError::ExecutionFailed { .. } => "render_execution_failed",
    }
}

fn pdf_response(document: Document) -> Response {
    let mut response = Body::from(document.into_bytes()).into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/pdf"));
    response
}
