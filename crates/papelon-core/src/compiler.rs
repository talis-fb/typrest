mod fonts;
mod world;

use std::sync::Arc;

use async_trait::async_trait;

use crate::{CompilationError, Document, TemplateSource};

use self::fonts::FontRegistry;

#[async_trait]
pub trait PdfCompiler: Send + Sync {
    async fn prepare(&self, source: &TemplateSource) -> Result<PreparedTemplate, CompilationError>;

    async fn compile_prepared(
        &self,
        template: &PreparedTemplate,
        payload: &serde_json::Value,
    ) -> Result<Document, CompilationError>;
}

pub struct PreparedTemplate {
    state: Option<world::PreparedTemplateState>,
}

impl PreparedTemplate {
    #[cfg(any(test, feature = "test-util"))]
    #[doc(hidden)]
    pub fn for_test() -> Self {
        Self { state: None }
    }
}

#[derive(Clone)]
pub struct TypstCompiler {
    fonts: Arc<FontRegistry>,
}

impl TypstCompiler {
    pub fn new() -> Self {
        Self {
            fonts: Arc::new(FontRegistry::new()),
        }
    }
}

impl Default for TypstCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PdfCompiler for TypstCompiler {
    async fn prepare(&self, source: &TemplateSource) -> Result<PreparedTemplate, CompilationError> {
        Ok(PreparedTemplate {
            state: Some(world::prepare(source, &self.fonts)?),
        })
    }

    async fn compile_prepared(
        &self,
        template: &PreparedTemplate,
        payload: &serde_json::Value,
    ) -> Result<Document, CompilationError> {
        let state = template
            .state
            .as_ref()
            .ok_or_else(|| CompilationError::Failed {
                message: "test prepared template cannot be compiled by TypstCompiler".to_owned(),
                line: None,
            })?;
        let bytes = world::compile_prepared(state, payload)?;
        Ok(Document::new(bytes.into()))
    }
}
