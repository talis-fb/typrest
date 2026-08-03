pub mod memory;

use async_trait::async_trait;
use thiserror::Error;

use crate::schema::{SchemaError, SchemaVersionError};
use crate::{TemplateBundle, TemplateId, TemplateIdError};

#[async_trait]
pub trait TemplateLoader: Send + Sync {
    async fn load(&self, id: &TemplateId) -> Result<TemplateBundle, LoaderError>;
    async fn list(&self) -> Result<Vec<TemplateId>, LoaderError>;
}

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("template '{0}' was not found")]
    TemplateNotFound(TemplateId),

    #[error("invalid template ID: {0}")]
    InvalidTemplateId(#[from] TemplateIdError),

    #[error("{0}")]
    SchemaVersionError(#[from] SchemaVersionError),

    #[error("invalid schema: {0}")]
    SchemaError(#[from] SchemaError),

    #[error("template I/O error: {0}")]
    IoError(String),
}
