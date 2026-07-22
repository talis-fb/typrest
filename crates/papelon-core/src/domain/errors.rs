use thiserror::Error;

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
#[error("{message}")]
pub struct CompilationError {
    pub message: String,
}

impl CompilationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
