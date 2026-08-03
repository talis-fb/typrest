use std::path::Path;
use std::sync::Arc;

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use typrest_core::{PdfCompiler, TemplateLoader, TemplateName, TemplateRegistry};

use crate::loader::filesystem::FilesystemLoader;
use crate::output::{DiagnosticContext, DiagnosticReporter, DiagnosticStage};

pub struct TemplateWatcher {
    _watcher: RecommendedWatcher,
    worker: tokio::task::JoinHandle<()>,
}

impl Drop for TemplateWatcher {
    fn drop(&mut self) {
        self.worker.abort();
    }
}

pub fn start_template_watcher(
    loader: Arc<FilesystemLoader>,
    registry: TemplateRegistry,
    compiler: Arc<dyn PdfCompiler>,
    reporter: DiagnosticReporter,
) -> notify::Result<TemplateWatcher> {
    let root = loader.root().to_owned();
    let watch_root = root.clone();
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let mut watcher = RecommendedWatcher::new(
        move |result| {
            if sender.send(result).is_err() {
                warn!("template watch worker stopped");
            }
        },
        Config::default(),
    )?;

    watcher.watch(&watch_root, RecursiveMode::Recursive)?;
    let worker = tokio::spawn(async move {
        while let Some(result) = receiver.recv().await {
            handle_event(
                result,
                &root,
                loader.as_ref(),
                &registry,
                compiler.as_ref(),
                &reporter,
            )
            .await;
        }
    });
    info!(templates_dir = %watch_root.display(), "template watch mode enabled");

    Ok(TemplateWatcher {
        _watcher: watcher,
        worker,
    })
}

async fn handle_event(
    result: notify::Result<Event>,
    root: &Path,
    loader: &FilesystemLoader,
    registry: &TemplateRegistry,
    compiler: &dyn PdfCompiler,
    reporter: &DiagnosticReporter,
) {
    let event = match result {
        Ok(event) => event,
        Err(error) => {
            error!(error = %error, "template watch event failed");
            return;
        }
    };

    if !is_relevant_event(&event) {
        return;
    }

    let mut saw_shared = false;
    let mut templates = Vec::new();

    for path in event.paths {
        match changed_template(root, &path) {
            Some(ChangedTemplate::Shared) => saw_shared = true,
            Some(ChangedTemplate::Template(name)) if !templates.contains(&name) => {
                templates.push(name);
            }
            Some(ChangedTemplate::Template(_)) | None => {}
        }
    }

    if saw_shared {
        reload_shared(loader, registry, compiler, reporter).await;
        return;
    }

    for template in templates {
        reload_template(loader, registry, compiler, &template, reporter).await;
    }
}

fn is_relevant_event(event: &Event) -> bool {
    matches!(
        event.kind,
        notify::EventKind::Create(_) | notify::EventKind::Modify(_) | notify::EventKind::Remove(_)
    )
}

async fn reload_shared(
    loader: &FilesystemLoader,
    registry: &TemplateRegistry,
    compiler: &dyn PdfCompiler,
    reporter: &DiagnosticReporter,
) {
    match loader.list().await {
        Ok(templates) => {
            for template in templates {
                reload_template(loader, registry, compiler, &template, reporter).await;
            }
            info!("shared template files changed");
        }
        Err(error) => warn!(error = %error, "failed to list templates after shared file change"),
    }
}

async fn reload_template(
    loader: &FilesystemLoader,
    registry: &TemplateRegistry,
    compiler: &dyn PdfCompiler,
    template: &TemplateName,
    reporter: &DiagnosticReporter,
) {
    match registry.reload(loader, compiler, template).await {
        Ok(reload) => {
            reporter.report_diagnostics(
                DiagnosticContext::Watch,
                DiagnosticStage::Compile,
                reload.warnings(),
            );
            info!(
                event = "template.reload.completed",
                template = %template,
                warning_count = reload.warnings().len(),
                "template reloaded"
            );
        }
        Err(error) => {
            match &error {
                typrest_core::TemplateRegistryError::Prepare { source, .. } => {
                    reporter.report_compilation(
                        DiagnosticContext::Watch,
                        DiagnosticStage::Prepare,
                        template,
                        source,
                    );
                }
                typrest_core::TemplateRegistryError::Warmup { source, .. } => {
                    reporter.report_compilation(
                        DiagnosticContext::Watch,
                        DiagnosticStage::Compile,
                        template,
                        source,
                    );
                }
                _ => reporter.report_simple_error(
                    DiagnosticContext::Watch,
                    DiagnosticStage::Load,
                    Some(template),
                    &error.to_string(),
                ),
            }
            warn!(
                event = "template.reload.failed",
                template = %template,
                last_known_good = true,
                "template reload failed; keeping last known good version"
            );
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ChangedTemplate {
    Template(TemplateName),
    Shared,
}

pub(crate) fn changed_template(root: &Path, path: &Path) -> Option<ChangedTemplate> {
    let relative_path = path.strip_prefix(root).ok()?;
    let first_segment = relative_path.components().next()?.as_os_str().to_str()?;

    if first_segment == "_shared" {
        return Some(ChangedTemplate::Shared);
    }

    TemplateName::new(first_segment.to_owned())
        .ok()
        .map(ChangedTemplate::Template)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use async_trait::async_trait;
    use bytes::Bytes;
    use tempfile::tempdir;
    use typrest_core::{
        CompilationError, Document, PdfCompiler, PreparedTemplate, TemplateName, TemplateRegistry,
        TemplateSource,
    };

    use crate::loader::filesystem::FilesystemLoader;
    use crate::output::{ColorChoice, DiagnosticReporter, OutputFormat};

    use super::{ChangedTemplate, changed_template, reload_shared, reload_template};

    struct MockCompiler {
        fail: AtomicBool,
        prepares: AtomicUsize,
    }

    #[async_trait]
    impl PdfCompiler for MockCompiler {
        async fn prepare(&self, _: &TemplateSource) -> Result<PreparedTemplate, CompilationError> {
            self.prepares.fetch_add(1, Ordering::SeqCst);
            Ok(PreparedTemplate::for_test())
        }

        async fn compile_prepared(
            &self,
            _: &PreparedTemplate,
            _: &serde_json::Value,
        ) -> Result<Document, CompilationError> {
            if self.fail.load(Ordering::SeqCst) {
                Err(CompilationError::failed("broken reload", Some(2)))
            } else {
                Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock")))
            }
        }
    }

    fn write_template(root: &Path, name: &str, main: &str) {
        let template = root.join(name);
        fs::create_dir_all(&template).unwrap();
        fs::write(template.join("main.typ"), main).unwrap();
        fs::write(
            template.join("schema.yml"),
            "schemaVersion: v1\nrequest:\n  body:\n    type: object\n",
        )
        .unwrap();
    }

    #[test]
    fn resolves_template_path() {
        assert_eq!(
            changed_template(
                Path::new("/templates"),
                Path::new("/templates/invoice/main.typ")
            ),
            Some(ChangedTemplate::Template(
                TemplateName::new("invoice").unwrap()
            ))
        );
    }

    #[test]
    fn resolves_shared_path() {
        assert_eq!(
            changed_template(
                Path::new("/templates"),
                Path::new("/templates/_shared/brand.typ")
            ),
            Some(ChangedTemplate::Shared)
        );
    }

    #[test]
    fn ignores_paths_outside_root() {
        assert_eq!(
            changed_template(Path::new("/templates"), Path::new("/elsewhere/main.typ")),
            None
        );
    }

    #[test]
    fn ignores_invalid_template_names() {
        assert_eq!(
            changed_template(
                Path::new("/templates"),
                Path::new("/templates/not.valid/main.typ")
            ),
            None
        );
    }

    #[tokio::test]
    async fn successful_reload_replaces_entry_and_failure_keeps_last_good() {
        let root = tempdir().unwrap();
        write_template(root.path(), "invoice", "first");
        let loader = FilesystemLoader::new(root.path());
        let registry = TemplateRegistry::default();
        let compiler = MockCompiler {
            fail: AtomicBool::new(false),
            prepares: AtomicUsize::new(0),
        };
        let name = TemplateName::new("invoice").unwrap();
        let reporter =
            DiagnosticReporter::new(OutputFormat::Pretty, ColorChoice::Never, root.path());

        reload_template(&loader, &registry, &compiler, &name, &reporter).await;
        let first = registry.get(&name).unwrap();

        fs::write(root.path().join("invoice/main.typ"), "second").unwrap();
        reload_template(&loader, &registry, &compiler, &name, &reporter).await;
        let second = registry.get(&name).unwrap();
        assert!(!std::sync::Arc::ptr_eq(&first, &second));

        fs::write(
            root.path().join("invoice/schema.json"),
            r#"{"schemaVersion":"v1","request":{"body":{}}}"#,
        )
        .unwrap();
        reload_template(&loader, &registry, &compiler, &name, &reporter).await;
        let after_conflict = registry.get(&name).unwrap();
        assert!(std::sync::Arc::ptr_eq(&second, &after_conflict));
        fs::remove_file(root.path().join("invoice/schema.json")).unwrap();

        fs::write(root.path().join("invoice/schema.yml"), "schemaVersion: [").unwrap();
        reload_template(&loader, &registry, &compiler, &name, &reporter).await;
        let after_schema_error = registry.get(&name).unwrap();
        assert!(std::sync::Arc::ptr_eq(&second, &after_schema_error));
        fs::write(
            root.path().join("invoice/schema.yml"),
            "schemaVersion: v1\nrequest:\n  body:\n    type: object\n",
        )
        .unwrap();

        compiler.fail.store(true, Ordering::SeqCst);
        fs::write(root.path().join("invoice/main.typ"), "#let broken =").unwrap();
        reload_template(&loader, &registry, &compiler, &name, &reporter).await;
        let after_failure = registry.get(&name).unwrap();

        assert!(std::sync::Arc::ptr_eq(&second, &after_failure));
    }

    #[tokio::test]
    async fn shared_reload_rebuilds_every_template() {
        let root = tempdir().unwrap();
        write_template(root.path(), "invoice", "invoice");
        write_template(root.path(), "welcome", "welcome");
        let loader = FilesystemLoader::new(root.path());
        let registry = TemplateRegistry::default();
        let compiler = MockCompiler {
            fail: AtomicBool::new(false),
            prepares: AtomicUsize::new(0),
        };
        let reporter =
            DiagnosticReporter::new(OutputFormat::Pretty, ColorChoice::Never, root.path());

        reload_shared(&loader, &registry, &compiler, &reporter).await;
        let first_count = compiler.prepares.load(Ordering::SeqCst);
        reload_shared(&loader, &registry, &compiler, &reporter).await;

        assert_eq!(first_count, 2);
        assert_eq!(compiler.prepares.load(Ordering::SeqCst), 4);
    }
}
