use std::fmt;

use crate::CompilationDiagnostic;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithWarnings<T> {
    value: T,
    warnings: Vec<CompilationDiagnostic>,
}

impl<T> WithWarnings<T> {
    pub fn new(value: T, warnings: Vec<CompilationDiagnostic>) -> Self {
        Self { value, warnings }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn into_value(self) -> T {
        self.value
    }

    pub fn into_parts(self) -> (T, Vec<CompilationDiagnostic>) {
        (self.value, self.warnings)
    }

    pub fn warnings(&self) -> &[CompilationDiagnostic] {
        &self.warnings
    }
}

impl<T: fmt::Display> fmt::Display for WithWarnings<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(formatter)
    }
}

impl<T> std::error::Error for WithWarnings<T> where T: std::error::Error + fmt::Debug + 'static {}
