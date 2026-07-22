//! Core domain and rendering abstractions for Papelon.

mod errors;
pub mod loader;
mod render_request;
pub mod schema;
mod template;
mod template_name;

pub use errors::{CompilationError, ValidationError};
pub use loader::{LoaderError, TemplateLoader};
pub use render_request::RenderRequest;
pub use schema::{
    Schema, SchemaError, SchemaField, SchemaType, SchemaVersionError, validate,
    validate_with_defaults,
};
pub use template::{Document, TemplateBundle, TemplateSource};
pub use template_name::{TemplateName, TemplateNameError};
