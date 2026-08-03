use std::collections::BTreeMap;

use typst::{foundations::Bytes, syntax::Source};

use crate::{Schema, compiler::fonts::LoadedFonts, compiler::packages::PreparedPackages};

pub struct PreparedTemplate {
    pub(super) inner: Option<PreparedTemplateInner>,
}

impl PreparedTemplate {
    #[cfg(any(test, feature = "test-util"))]
    #[doc(hidden)]
    pub fn for_test() -> Self {
        Self { inner: None }
    }
}

pub(super) struct PreparedTemplateInner {
    pub(super) main: Source,
    pub(super) files: BTreeMap<String, Bytes>,
    pub(super) packages: PreparedPackages,
    pub(super) fonts: LoadedFonts,
}

pub struct PreparedTemplateBundle {
    pub schema: Option<Schema>,
    pub prepared: PreparedTemplate,
}

impl PreparedTemplateBundle {
    pub fn new(schema: Schema, prepared: PreparedTemplate) -> Self {
        Self {
            schema: Some(schema),
            prepared,
        }
    }

    pub fn without_schema(prepared: PreparedTemplate) -> Self {
        Self {
            schema: None,
            prepared,
        }
    }

    pub fn schema(&self) -> Option<&Schema> {
        self.schema.as_ref()
    }

    pub fn prepared(&self) -> &PreparedTemplate {
        &self.prepared
    }
}
