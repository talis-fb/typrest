//! Core domain and rendering abstractions for Typrest.

pub mod compiler;
mod diagnostic;
mod errors;
pub mod loader;
mod package;
mod registry;
pub mod render;
mod render_request;
pub mod schema;
mod template;
mod template_id;
mod warnings;

pub use compiler::{PdfCompiler, PreparedTemplate, PreparedTemplateBundle, TypstCompiler};
pub use diagnostic::{
    CompilationDiagnostic, CompilationReport, DiagnosticLocation, DiagnosticPosition,
    DiagnosticSeverity, RelatedDiagnostic,
};
pub use errors::{CompilationError, RenderError, ValidationError};
pub use loader::{LoaderError, TemplateLoader};
pub use package::{PackageCoordinate, PackageRepository, PackageResolutionError, PackageSource};
pub use registry::{TemplateRegistry, TemplateRegistryBuildFailure, TemplateRegistryError};
pub use render::RenderService;
pub use render_request::RenderRequest;
pub use schema::{
    Schema, SchemaError, SchemaField, SchemaType, SchemaVersionError, validate,
    validate_with_defaults,
};
pub use template::{Document, TemplateBundle, TemplateSource};
pub use template_id::{TemplateId, TemplateIdError};
pub use warnings::WithWarnings;
