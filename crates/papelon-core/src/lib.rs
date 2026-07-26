//! Core domain and rendering abstractions for Papelon.

pub mod compiler;
mod diagnostic;
mod errors;
pub mod loader;
mod registry;
pub mod render;
mod render_request;
pub mod schema;
mod template;
mod template_name;

pub use compiler::{PdfCompiler, PreparedTemplate, TypstCompiler};
pub use diagnostic::{
    CompilationDiagnostic, CompilationReport, DiagnosticLocation, DiagnosticPosition,
    DiagnosticSeverity, RelatedDiagnostic,
};
pub use errors::{CompilationError, RenderError, ValidationError};
pub use loader::{LoaderError, TemplateLoader};
pub use registry::{
    PreparedTemplateEntry, TemplateDiagnostic, TemplateRegistry, TemplateRegistryBuild,
    TemplateRegistryBuildError, TemplateRegistryError, TemplateRegistryReload,
};
pub use render::RenderService;
pub use render_request::RenderRequest;
pub use schema::{
    Schema, SchemaError, SchemaField, SchemaType, SchemaVersionError, validate,
    validate_with_defaults,
};
pub use template::{Document, TemplateBundle, TemplateSource};
pub use template_name::{TemplateName, TemplateNameError};
