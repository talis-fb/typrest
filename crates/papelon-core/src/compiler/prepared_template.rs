use std::collections::BTreeMap;

use typst::{foundations::Bytes, syntax::Source};

use crate::{Schema, compiler::fonts::LoadedFonts};

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
    pub(super) fonts: LoadedFonts,
}

pub struct PreparedTemplateBundle {
    pub schema: Schema,
    pub prepared: PreparedTemplate,
}

impl PreparedTemplateBundle {
    pub fn new(schema: Schema, prepared: PreparedTemplate) -> Self {
        Self { schema, prepared }
    }

    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    pub fn prepared(&self) -> &PreparedTemplate {
        &self.prepared
    }
}
