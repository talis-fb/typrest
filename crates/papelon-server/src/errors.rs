use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use papelon_core::{CompilationError, LoaderError, RenderError, ValidationError};
use serde::Serialize;

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    body: ErrorBody,
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
            },
        }
    }

    pub fn from_loader_error(error: LoaderError) -> Self {
        match error {
            LoaderError::TemplateNotFound(name) => Self::new(
                StatusCode::NOT_FOUND,
                "template_not_found",
                format!("Template '{name}' was not found"),
            ),
            error => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "template_loader_failed",
                error.to_string(),
            ),
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
                },
            },
            RenderError::CompilationError(error) => compilation_error_response(error),
        }
    }
}

fn compilation_error_response(error: CompilationError) -> ApiError {
    match error {
        CompilationError::Failed { message, line } => ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: ErrorBody {
                error: "compilation_failed".to_owned(),
                message,
                violations: Vec::new(),
                line,
            },
        },
        CompilationError::ExportFailed(message) => ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "compilation_failed",
            message,
        ),
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
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
