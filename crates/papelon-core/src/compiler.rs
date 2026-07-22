mod fonts;
mod world;

use std::sync::Arc;

use async_trait::async_trait;

use crate::{CompilationError, Document, TemplateSource};

use self::fonts::FontRegistry;

#[async_trait]
pub trait PdfCompiler: Send + Sync {
    async fn compile(
        &self,
        source: &TemplateSource,
        payload: &serde_json::Value,
    ) -> Result<Document, CompilationError>;
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
    async fn compile(
        &self,
        source: &TemplateSource,
        payload: &serde_json::Value,
    ) -> Result<Document, CompilationError> {
        let bytes = world::compile(source, payload, &self.fonts)?;
        Ok(Document::new(bytes.into()))
    }
}
