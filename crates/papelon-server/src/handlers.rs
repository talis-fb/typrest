use std::time::Instant;

use axum::body::Body;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Json, Path, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use papelon_core::{CompilationError, Document, RenderError, RenderRequest, TemplateName};
use serde::Serialize;
use serde_json::Value;
use tracing::{info, warn};

use crate::errors::ApiError;
use crate::output::{DiagnosticContext, DiagnosticStage};
use crate::server::AppState;

pub async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

pub async fn list_templates(
    State(state): State<AppState>,
) -> Result<Json<TemplatesResponse>, ApiError> {
    let templates = state
        .registry
        .names()
        .into_iter()
        .map(|name| TemplateResponse {
            name: name.into_inner(),
        })
        .collect();

    Ok(Json(TemplatesResponse { templates }))
}

pub async fn render_template(
    State(state): State<AppState>,
    Path(template): Path<String>,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let template = TemplateName::new(template).map_err(|error| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_template_name",
            error.to_string(),
        )
    })?;
    let template_for_log = template.clone();

    let Json(payload) = payload.map_err(|error| {
        ApiError::new(StatusCode::BAD_REQUEST, "invalid_json", error.body_text())
    })?;

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

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Serialize)]
pub struct TemplatesResponse {
    pub templates: Vec<TemplateResponse>,
}

#[derive(Serialize)]
pub struct TemplateResponse {
    pub name: String,
}
