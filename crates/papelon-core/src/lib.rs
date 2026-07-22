//! Core domain and rendering abstractions for Papelon.

pub mod domain;
pub mod resolver;
pub mod schema;

pub use domain::{
    CompilationError, Document, RenderRequest, ResolvedTemplate, TemplateName, TemplateNameError,
    TemplateSource, ValidationError,
};
pub use resolver::{MemoryResolver, ResolverError, TemplateResolver};
pub use schema::{Schema, SchemaError, SchemaField, SchemaType, validate, validate_with_defaults};
