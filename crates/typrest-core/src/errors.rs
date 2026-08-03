use thiserror::Error;

use crate::{
    CompilationDiagnostic, CompilationReport, DiagnosticLocation, DiagnosticPosition,
    DiagnosticSeverity, TemplateId,
};

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
    #[error("compilation failed: {}", primary_message(.0))]
    Failed(CompilationReport),

    #[error("pdf export failed: {}", primary_message(.0))]
    ExportFailed(CompilationReport),
}

impl CompilationError {
    pub fn failed(message: impl Into<String>, line: Option<u32>) -> Self {
        let mut diagnostic = CompilationDiagnostic::new(DiagnosticSeverity::Error, message);
        diagnostic.location = line.map(|line| {
            DiagnosticLocation::new(
                "main.typ",
                DiagnosticPosition::new(line, 1),
                DiagnosticPosition::new(line, 1),
            )
        });
        Self::Failed(CompilationReport::new(vec![diagnostic]))
    }

    pub fn export_failed(message: impl Into<String>) -> Self {
        Self::ExportFailed(CompilationReport::new(vec![CompilationDiagnostic::new(
            DiagnosticSeverity::Error,
            message,
        )]))
    }

    pub fn report(&self) -> &CompilationReport {
        match self {
            Self::Failed(report) | Self::ExportFailed(report) => report,
        }
    }

    pub fn primary_message(&self) -> &str {
        primary_message(self.report())
    }

    pub fn primary_line(&self) -> Option<u32> {
        self.report()
            .primary()
            .and_then(|diagnostic| diagnostic.location.as_ref())
            .map(|location| location.start.line)
    }
}

fn primary_message(report: &CompilationReport) -> &str {
    report
        .primary()
        .map(|diagnostic| diagnostic.message.as_str())
        .unwrap_or("unknown compilation error")
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum RenderError {
    #[error("template '{0}' was not found")]
    TemplateNotFound(TemplateId),

    #[error("payload validation failed")]
    ValidationError(Vec<ValidationError>),

    #[error("{0}")]
    CompilationError(#[from] CompilationError),
}
