use std::collections::BTreeMap;

use bytes::Bytes;

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
pub struct ResolvedTemplate {
    pub source: TemplateSource,
    pub schema: Schema,
}

impl ResolvedTemplate {
    pub fn new(source: TemplateSource, schema: Schema) -> Self {
        Self { source, schema }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    bytes: Bytes,
}

impl Document {
    pub fn new(bytes: Bytes) -> Self {
        Self { bytes }
    }

    pub fn bytes(&self) -> &Bytes {
        &self.bytes
    }

    pub fn into_bytes(self) -> Bytes {
        self.bytes
    }
}
