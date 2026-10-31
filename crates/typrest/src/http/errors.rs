use axum::http::header::RETRY_AFTER;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use serde::Serialize;
use thiserror::Error;
use typrest_core::{
    CompilationDiagnostic, CompilationError, DiagnosticLocation, DiagnosticSeverity,
    RelatedDiagnostic, RenderError, ValidationError,
};

pub const RENDER_RETRY_AFTER_SECONDS: u32 = 1;

#[derive(Debug, Error)]
pub enum ServeError {
    #[error("server I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("graceful shutdown timed out with requests still in flight")]
    ShutdownTimedOut,
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    body: ErrorBody,
    retry_after: Option<u32>,
}

impl ApiError {
    pub fn new(status: StatusCode, error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            body: ErrorBody {
                error: error.into(),
                message: message.into(),
                violations: Vec::new(),
                line: None,
                diagnostics: Vec::new(),
            },
            retry_after: None,
        }
    }

    pub fn from_render_error(error: RenderError) -> Self {
        match error {
            RenderError::TemplateNotFound(name) => Self::new(
                StatusCode::NOT_FOUND,
                "template_not_found",
                format!("Template '{name}' was not found"),
            ),
            RenderError::ValidationError(violations) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                body: ErrorBody {
                    error: "schema_validation_failed".to_owned(),
                    message: "Payload validation failed".to_owned(),
                    violations: violations.into_iter().map(Into::into).collect(),
                    line: None,
                    diagnostics: Vec::new(),
                },
                retry_after: None,
            },
            RenderError::CompilationError(error) => compilation_error_response(error),
            RenderError::Overloaded => {
                let mut error = Self::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "render_capacity_exceeded",
                    "The server is at render capacity; retry shortly",
                );
                error.retry_after = Some(RENDER_RETRY_AFTER_SECONDS);
                error
            }
            RenderError::ExecutionFailed { .. } => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "render_execution_failed",
                "Render execution failed",
            ),
        }
    }
}

fn compilation_error_response(error: CompilationError) -> ApiError {
    let message = error.primary_message().to_owned();
    let line = error.primary_line();
    let diagnostics = error
        .report()
        .diagnostics()
        .iter()
        .map(DiagnosticBody::from)
        .collect();

    ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        body: ErrorBody {
            error: "compilation_failed".to_owned(),
            message,
            violations: Vec::new(),
            line,
            diagnostics,
        },
        retry_after: None,
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = (self.status, Json(self.body)).into_response();
        if let Some(seconds) = self.retry_after {
            response
                .headers_mut()
                .insert(RETRY_AFTER, HeaderValue::from(seconds));
        }
        response
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub violations: Vec<ViolationBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DiagnosticBody>,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticBody {
    pub severity: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<DiagnosticLocationBody>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<RelatedDiagnosticBody>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trace: Vec<RelatedDiagnosticBody>,
}

impl From<&CompilationDiagnostic> for DiagnosticBody {
    fn from(diagnostic: &CompilationDiagnostic) -> Self {
        Self {
            severity: match diagnostic.severity {
                DiagnosticSeverity::Error => "error",
                DiagnosticSeverity::Warning => "warning",
            },
            message: diagnostic.message.clone(),
            location: diagnostic.location.as_ref().map(Into::into),
            hints: diagnostic.hints.iter().map(Into::into).collect(),
            trace: diagnostic.trace.iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RelatedDiagnosticBody {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<DiagnosticLocationBody>,
}

impl From<&RelatedDiagnostic> for RelatedDiagnosticBody {
    fn from(diagnostic: &RelatedDiagnostic) -> Self {
        Self {
            message: diagnostic.message.clone(),
            location: diagnostic.location.as_ref().map(Into::into),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DiagnosticLocationBody {
    pub path: String,
    pub range: DiagnosticRangeBody,
}

impl From<&DiagnosticLocation> for DiagnosticLocationBody {
    fn from(location: &DiagnosticLocation) -> Self {
        Self {
            path: location.path.clone(),
            range: DiagnosticRangeBody {
                start: DiagnosticPositionBody {
                    line: location.start.line,
                    column: location.start.column,
                },
                end: DiagnosticPositionBody {
                    line: location.end.line,
                    column: location.end.column,
                },
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DiagnosticRangeBody {
    pub start: DiagnosticPositionBody,
    pub end: DiagnosticPositionBody,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticPositionBody {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Serialize)]
pub struct ViolationBody {
    pub field: String,
    pub issue: String,
}

impl From<ValidationError> for ViolationBody {
    fn from(error: ValidationError) -> Self {
        Self {
            field: error.field,
            issue: error.issue,
        }
    }
}
