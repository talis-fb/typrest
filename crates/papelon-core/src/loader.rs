pub mod memory;

use async_trait::async_trait;
use thiserror::Error;

use crate::schema::{SchemaError, SchemaVersionError};
use crate::{TemplateBundle, TemplateName, TemplateNameError};

#[async_trait]
pub trait TemplateLoader: Send + Sync {
    async fn load(&self, name: &TemplateName) -> Result<TemplateBundle, LoaderError>;
    async fn list(&self) -> Result<Vec<TemplateName>, LoaderError>;
}

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("template '{0}' was not found")]
    TemplateNotFound(TemplateName),

    #[error("invalid template name: {0}")]
    InvalidTemplateName(#[from] TemplateNameError),

    #[error("{0}")]
    SchemaVersionError(#[from] SchemaVersionError),

    #[error("invalid schema: {0}")]
    SchemaError(#[from] SchemaError),

    #[error("template I/O error: {0}")]
    IoError(String),
}
