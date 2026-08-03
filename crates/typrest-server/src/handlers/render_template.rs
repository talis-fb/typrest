use std::time::Instant;

use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::Value;
use tracing::{info, warn};
use typrest_core::{CompilationError, Document, RenderError, RenderRequest, TemplateId};

use crate::errors::ApiError;
use crate::output::{DiagnosticContext, DiagnosticStage};
use crate::server::AppState;

pub async fn render_template(
    State(state): State<AppState>,
    Path(template_id): Path<String>,
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

    let schema = state
        .registry
        .get(&template)
        .and_then(|entry| entry.schema().cloned());

    let payload = if body.is_empty() && schema.is_none() {
        Value::Object(serde_json::Map::new())
    } else {
        if !has_json_content_type(&headers) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "invalid_json",
                "request body must have content-type application/json",
            ));
        }
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
            state.reporter.report_warnings(
                DiagnosticContext::Render,
                DiagnosticStage::Compile,
                &template_for_log,
                document.warnings(),
            );
            info!(
                event = "render.completed",
                template = %template_for_log,
                latency_ms = elapsed_ms,
                pdf_size,
                warning_count = document.warnings().len(),
                status = "ok",
                "render completed"
            );
            Ok(pdf_response(document))
        }
        Err(error) => {
            if let RenderError::CompilationError(compilation_error) = &error {
                let stage = match compilation_error {
                    CompilationError::Failed(_) => DiagnosticStage::Compile,
                    CompilationError::ExportFailed(_) => DiagnosticStage::Export,
                };
                state.reporter.report_compilation(
                    DiagnosticContext::Render,
                    stage,
                    &template_for_log,
                    compilation_error,
                );
            }
            warn!(
                event = "render.failed",
                template = %template_for_log,
                latency_ms = elapsed_ms,
                status = "error",
                error_kind = render_error_kind(&error),
                "render failed"
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

fn render_error_kind(error: &RenderError) -> &'static str {
    match error {
        RenderError::TemplateNotFound(_) => "template_not_found",
        RenderError::ValidationError(_) => "schema_validation_failed",
        RenderError::CompilationError(CompilationError::Failed(_)) => "compilation_failed",
        RenderError::CompilationError(CompilationError::ExportFailed(_)) => "pdf_export_failed",
    }
}

fn pdf_response(document: Document) -> Response {
    let mut response = Body::from(document.into_bytes()).into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/pdf"));
    response
}
