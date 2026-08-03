use std::ops::Range;
use std::sync::Arc;

use crate::TemplateId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticPosition {
    pub line: u32,
    pub column: u32,
}

impl DiagnosticPosition {
    pub fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticLocation {
    pub path: String,
    pub start: DiagnosticPosition,
    pub end: DiagnosticPosition,
    source: Option<Arc<str>>,
    byte_range: Option<Range<usize>>,
}

impl DiagnosticLocation {
    pub fn new(
        path: impl Into<String>,
        start: DiagnosticPosition,
        end: DiagnosticPosition,
    ) -> Self {
        Self {
            path: path.into(),
            start,
            end,
            source: None,
            byte_range: None,
        }
    }

    pub(crate) fn with_source(
        path: impl Into<String>,
        start: DiagnosticPosition,
        end: DiagnosticPosition,
        source: Arc<str>,
        byte_range: Range<usize>,
    ) -> Self {
        Self {
            path: path.into(),
            start,
            end,
            source: Some(source),
            byte_range: Some(byte_range),
        }
    }

    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    pub fn byte_range(&self) -> Option<Range<usize>> {
        self.byte_range.clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelatedDiagnostic {
    pub message: String,
    pub location: Option<DiagnosticLocation>,
}

impl RelatedDiagnostic {
    pub fn new(message: impl Into<String>, location: Option<DiagnosticLocation>) -> Self {
        Self {
            message: message.into(),
            location,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompilationDiagnostic {
    pub template: Option<TemplateId>,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub location: Option<DiagnosticLocation>,
    pub hints: Vec<RelatedDiagnostic>,
    pub trace: Vec<RelatedDiagnostic>,
}

impl CompilationDiagnostic {
    pub fn new(severity: DiagnosticSeverity, message: impl Into<String>) -> Self {
        Self {
            template: None,
            severity,
            message: message.into(),
            location: None,
            hints: Vec::new(),
            trace: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CompilationReport {
    diagnostics: Vec<CompilationDiagnostic>,
}

impl CompilationReport {
    pub fn new(diagnostics: Vec<CompilationDiagnostic>) -> Self {
        Self { diagnostics }
    }

    pub fn diagnostics(&self) -> &[CompilationDiagnostic] {
        &self.diagnostics
    }

    pub fn into_diagnostics(self) -> Vec<CompilationDiagnostic> {
        self.diagnostics
    }

    pub fn primary(&self) -> Option<&CompilationDiagnostic> {
        self.diagnostics
            .iter()
            .find(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
            .or_else(|| self.diagnostics.first())
    }
}
