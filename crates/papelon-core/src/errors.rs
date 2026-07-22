use thiserror::Error;

use crate::TemplateName;

#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("{field}: {issue}")]
pub struct ValidationError {
    pub field: String,
    pub issue: String,
}

impl ValidationError {
    pub fn new(field: impl Into<String>, issue: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            issue: issue.into(),
        }
    }
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum CompilationError {
    #[error("compilation failed: {message}")]
    Failed { message: String, line: Option<u32> },

    #[error("pdf export failed: {0}")]
    ExportFailed(String),
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum RenderError {
    #[error("template '{0}' was not found")]
    TemplateNotFound(TemplateName),

    #[error("payload validation failed")]
    ValidationError(Vec<ValidationError>),

    #[error("{0}")]
    CompilationError(#[from] CompilationError),
}
