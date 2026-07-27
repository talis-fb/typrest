use std::collections::BTreeMap;

use bytes::Bytes;

use crate::CompilationDiagnostic;
use crate::schema::Schema;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TemplateSource {
    files: BTreeMap<String, Bytes>,
}

impl TemplateSource {
    pub fn new(files: BTreeMap<String, Bytes>) -> Self {
        Self { files }
    }

    pub fn files(&self) -> &BTreeMap<String, Bytes> {
        &self.files
    }

    pub fn into_files(self) -> BTreeMap<String, Bytes> {
        self.files
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TemplateBundle {
    pub source: TemplateSource,
    pub schema: Option<Schema>,
}

impl TemplateBundle {
    pub fn new(source: TemplateSource, schema: Schema) -> Self {
        Self {
            source,
            schema: Some(schema),
        }
    }

    pub fn without_schema(source: TemplateSource) -> Self {
        Self {
            source,
            schema: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    bytes: Bytes,
    warnings: Vec<CompilationDiagnostic>,
}

impl Document {
    pub fn new(bytes: Bytes) -> Self {
        Self {
            bytes,
            warnings: Vec::new(),
        }
    }

    pub(crate) fn with_warnings(bytes: Bytes, warnings: Vec<CompilationDiagnostic>) -> Self {
        Self { bytes, warnings }
    }

    pub fn bytes(&self) -> &Bytes {
        &self.bytes
    }

    pub fn into_bytes(self) -> Bytes {
        self.bytes
    }

    pub fn warnings(&self) -> &[CompilationDiagnostic] {
        &self.warnings
    }
}
