mod fonts;
mod packages;
mod prepared_template;
mod world;

use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    CompilationDiagnostic, CompilationError, CompilationReport, DiagnosticSeverity, Document,
    PackageRepository, TemplateSource,
};

use self::fonts::FontRegistry;

pub use prepared_template::{PreparedTemplate, PreparedTemplateBundle};

#[async_trait]
pub trait PdfCompiler: Send + Sync {
    async fn prepare(&self, source: &TemplateSource) -> Result<PreparedTemplate, CompilationError>;

    async fn compile_prepared(
        &self,
        template: &PreparedTemplate,
        payload: &serde_json::Value,
    ) -> Result<Document, CompilationError>;
}

#[derive(Clone)]
pub struct TypstCompiler {
    fonts: Arc<FontRegistry>,
    packages: Option<Arc<dyn PackageRepository>>,
}

impl TypstCompiler {
    pub fn new() -> Self {
        Self {
            fonts: Arc::new(FontRegistry::new()),
            packages: None,
        }
    }

    pub fn with_package_repository(mut self, packages: Arc<dyn PackageRepository>) -> Self {
        self.packages = Some(packages);
        self
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
        let packages = packages::resolve(source, self.packages.as_deref()).await?;
        Ok(PreparedTemplate {
            inner: Some(world::prepare(source, packages, &self.fonts)?),
        })
    }

    async fn compile_prepared(
        &self,
        template: &PreparedTemplate,
        payload: &serde_json::Value,
    ) -> Result<Document, CompilationError> {
        let state = template
            .inner
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
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use bytes::Bytes;
    use serde_json::json;

    use crate::{
        PackageCoordinate, PackageRepository, PackageResolutionError, PackageSource, PdfCompiler,
        TemplateSource, TypstCompiler,
    };

    struct MemoryPackages {
        packages: BTreeMap<PackageCoordinate, Arc<PackageSource>>,
        resolutions: AtomicUsize,
    }

    #[async_trait]
    impl PackageRepository for MemoryPackages {
        async fn resolve(
            &self,
            package: &PackageCoordinate,
        ) -> Result<Arc<PackageSource>, PackageResolutionError> {
            self.resolutions.fetch_add(1, Ordering::SeqCst);
            self.packages
                .get(package)
                .cloned()
                .ok_or_else(|| PackageResolutionError::NotFound {
                    package: package.clone(),
                })
        }
    }

    fn coordinate(name: &str) -> PackageCoordinate {
        PackageCoordinate::new("preview", name, "1.0.0")
    }

    fn package(name: &str, library: &str) -> Arc<PackageSource> {
        Arc::new(PackageSource::new(BTreeMap::from([
            (
                "typst.toml".to_owned(),
                Bytes::from(format!(
                    "[package]\nname = \"{name}\"\nversion = \"1.0.0\"\nentrypoint = \"lib.typ\"\n"
                )),
            ),
            ("lib.typ".to_owned(), Bytes::from(library.to_owned())),
        ])))
    }

    fn source(main: &str) -> TemplateSource {
        TemplateSource::new(BTreeMap::from([(
            "main.typ".to_owned(),
            Bytes::from(main.to_owned()),
        )]))
    }

    #[test]
    fn typst_owned_imports_are_confined_to_compiler_and_package_resolver_boundaries() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let compiler = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/compiler");
        let package_resolver = workspace.join("crates/typrest/src/package/universal_package.rs");
        let mut violations = Vec::new();

        find_typst_imports(workspace, &compiler, &package_resolver, &mut violations);

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

    #[test]
    fn application_crates_only_depend_on_typst_package_tooling_in_the_package_resolver() {
        let crates_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let core_manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let package_resolver_manifest = crates_dir.join("typrest/Cargo.toml");
        let mut violations = Vec::new();

        for entry in fs::read_dir(crates_dir).unwrap() {
            let manifest = entry.unwrap().path().join("Cargo.toml");
            if !manifest.is_file() || manifest == core_manifest {
                continue;
            }
            let contents = fs::read_to_string(&manifest).unwrap();
            if contents.lines().any(|line| {
                let dependency = line
                    .trim_start()
                    .split('=')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .split('.')
                    .next()
                    .unwrap_or_default();
                let is_package_tooling = manifest == package_resolver_manifest
                    && matches!(dependency, "typst-kit" | "typst-syntax");
                (dependency == "typst" || dependency.starts_with("typst-")) && !is_package_tooling
            }) {
                violations.push(manifest);
            }
        }

        assert!(
            violations.is_empty(),
            "Typst dependency ACL violations: {}",
            violations
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    #[test]
    fn prepares_transitive_packages_and_never_resolves_during_render() {
        let packages = Arc::new(MemoryPackages {
            packages: BTreeMap::from([
                (
                    coordinate("fixture"),
                    package(
                        "fixture",
                        r#"#import "@preview/dependency:1.0.0": greeting
#let message = greeting"#,
                    ),
                ),
                (
                    coordinate("dependency"),
                    package("dependency", "#let greeting = [Hello from a package]"),
                ),
            ]),
            resolutions: AtomicUsize::new(0),
        });
        let compiler = TypstCompiler::new().with_package_repository(packages.clone());
        let prepared = futures_executor::block_on(compiler.prepare(&source(
            r#"#import "@preview/fixture:1.0.0": message
#message"#,
        )))
        .unwrap();

        assert_eq!(packages.resolutions.load(Ordering::SeqCst), 2);
        let first =
            futures_executor::block_on(compiler.compile_prepared(&prepared, &json!({}))).unwrap();
        let second =
            futures_executor::block_on(compiler.compile_prepared(&prepared, &json!({}))).unwrap();

        assert!(first.bytes().starts_with(b"%PDF"));
        assert!(second.bytes().starts_with(b"%PDF"));
        assert_eq!(packages.resolutions.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn package_diagnostics_include_the_coordinate() {
        let packages = Arc::new(MemoryPackages {
            packages: BTreeMap::from([(
                coordinate("broken"),
                package("broken", "#let value = missing-name"),
            )]),
            resolutions: AtomicUsize::new(0),
        });
        let compiler = TypstCompiler::new().with_package_repository(packages);
        let prepared = futures_executor::block_on(compiler.prepare(&source(
            r#"#import "@preview/broken:1.0.0": value
#value"#,
        )))
        .unwrap();
        let error = futures_executor::block_on(compiler.compile_prepared(&prepared, &json!({})))
            .unwrap_err();

        assert!(
            error
                .report()
                .diagnostics()
                .iter()
                .filter_map(|diagnostic| diagnostic.location.as_ref())
                .any(|location| location.path == "@preview/broken:1.0.0/lib.typ")
        );
    }

    #[test]
    fn rejects_unsupported_package_namespaces_during_prepare() {
        let packages = Arc::new(MemoryPackages {
            packages: BTreeMap::new(),
            resolutions: AtomicUsize::new(0),
        });
        let compiler = TypstCompiler::new().with_package_repository(packages.clone());
        let error = match futures_executor::block_on(
            compiler.prepare(&source(r#"#import "@local/fixture:1.0.0""#)),
        ) {
            Ok(_) => panic!("unsupported package namespace should fail preparation"),
            Err(error) => error,
        };

        assert!(error.primary_message().contains("only '@preview'"));
        assert_eq!(packages.resolutions.load(Ordering::SeqCst), 0);
    }

    fn find_typst_imports(
        root: &Path,
        compiler: &Path,
        package_resolver: &Path,
        violations: &mut Vec<std::path::PathBuf>,
    ) {
        for entry in fs::read_dir(root).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            if path.is_dir() {
                find_typst_imports(&path, compiler, package_resolver, violations);
            } else if path.extension().is_some_and(|extension| extension == "rs")
                && !path.starts_with(compiler)
                && path != package_resolver
            {
                let source = fs::read_to_string(&path).unwrap();
                let typst_owned_prefixes = [
                    ["typst", "::"].concat(),
                    ["typst_assets", "::"].concat(),
                    ["typst_kit", "::"].concat(),
                    ["typst_pdf", "::"].concat(),
                    ["typst_syntax", "::"].concat(),
                ];
                if typst_owned_prefixes
                    .iter()
                    .any(|prefix| source.contains(prefix))
                {
                    violations.push(path);
                }
            }
        }
    }
}
