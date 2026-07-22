mod memory;

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{ResolvedTemplate, TemplateName, TemplateNameError};
use crate::schema::SchemaError;

pub use memory::MemoryResolver;

#[async_trait]
pub trait TemplateResolver: Send + Sync {
    async fn resolve(&self, name: &TemplateName) -> Result<ResolvedTemplate, ResolverError>;
    async fn list(&self) -> Result<Vec<TemplateName>, ResolverError>;
}

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("template '{0}' was not found")]
    TemplateNotFound(TemplateName),

    #[error("invalid template name: {0}")]
    InvalidTemplateName(#[from] TemplateNameError),

    #[error("invalid schema: {0}")]
    InvalidSchema(#[from] SchemaError),

    #[error("failed to read template source: {0}")]
    Source(String),
}
