mod fonts;
mod world;

use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    CompilationDiagnostic, CompilationError, CompilationReport, DiagnosticSeverity, Document,
    TemplateSource,
};

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
            .ok_or_else(|| compilation_error("test prepared template cannot be compiled"))?;
        let compiled = world::compile_prepared(state, payload)?;
        Ok(Document::with_warnings(
            compiled.bytes.into(),
            compiled.warnings,
        ))
    }
}

fn compilation_error(message: impl Into<String>) -> CompilationError {
    CompilationError::Failed(CompilationReport::new(vec![CompilationDiagnostic::new(
        DiagnosticSeverity::Error,
        message,
    )]))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    #[test]
    fn typst_imports_are_confined_to_private_compiler_files() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let allowed = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/compiler");
        let mut violations = Vec::new();

        find_typst_imports(workspace, &allowed, &mut violations);

        assert!(
            violations.is_empty(),
            "Typst ACL violations: {}",
            violations
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    fn find_typst_imports(root: &Path, allowed: &Path, violations: &mut Vec<std::path::PathBuf>) {
        for entry in fs::read_dir(root).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            if path.is_dir() {
                find_typst_imports(&path, allowed, violations);
            } else if path.extension().is_some_and(|extension| extension == "rs")
                && !path.starts_with(allowed)
            {
                let source = fs::read_to_string(&path).unwrap();
                let typst_import = ["use typst", "::"].concat();
                let typst_pdf_import = ["use typst_pdf", "::"].concat();
                if source.contains(&typst_import) || source.contains(&typst_pdf_import) {
                    violations.push(path);
                }
            }
        }
    }
}
