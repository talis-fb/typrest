use std::path::Path;
use std::sync::Arc;

use notify::event::ModifyKind;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use typrest_core::{PdfCompiler, TemplateId, TemplateRegistry};

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
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let mut results = vec![result];
            while let Ok(result) = receiver.try_recv() {
                results.push(result);
            }
            handle_events(
                results,
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

async fn handle_events(
    results: Vec<notify::Result<Event>>,
    root: &Path,
    loader: &FilesystemLoader,
    registry: &TemplateRegistry,
    compiler: &dyn PdfCompiler,
    reporter: &DiagnosticReporter,
) {
    let mut rebuild_all = false;
    let mut templates = Vec::new();

    for result in results {
        let event = match result {
            Ok(event) if is_relevant_event(&event) => event,
            Ok(_) => continue,
            Err(error) => {
                error!(error = %error, "template watch event failed");
                continue;
            }
        };
        let topology_change = is_topology_event(&event);

        for path in event.paths {
            match changed_template(root, &path) {
                Some(ChangedTemplate::Shared) => rebuild_all = true,
                Some(ChangedTemplate::Template(id)) if topology_change => {
                    rebuild_all = true;
                    if !templates.contains(&id) {
                        templates.push(id);
                    }
                }
                Some(ChangedTemplate::Template(id)) if !templates.contains(&id) => {
                    templates.push(id);
                }
                Some(ChangedTemplate::Template(_)) => {}
                None if topology_change && !is_private_discovery_path(root, &path) => {
                    rebuild_all = true;
                }
                None => {}
            }
        }
    }

    if rebuild_all {
        reload_all(loader, registry, compiler, reporter).await;
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

fn is_topology_event(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(_))
    ) || event.paths.iter().any(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "main.typ")
    })
}

async fn reload_all(
    loader: &FilesystemLoader,
    registry: &TemplateRegistry,
    compiler: &dyn PdfCompiler,
    reporter: &DiagnosticReporter,
) {
    match TemplateRegistry::build(loader, compiler).await {
        Ok(build) => {
            reporter.report_build(DiagnosticContext::Watch, &build);
            let template_count = build.value().ids().len();
            registry.replace_all(build.value());
            info!(
                event = "template.registry.rebuilt",
                template_count,
                warning_count = build.warnings().len(),
                "template registry rebuilt"
            );
        }
        Err(build_error) => {
            reporter.report_build_error(DiagnosticContext::Watch, &build_error);
            warn!(
                event = "template.registry.rebuild.failed",
                last_known_good = true,
                "template registry rebuild failed; keeping last known good version"
            );
        }
    }
}

async fn reload_template(
    loader: &FilesystemLoader,
    registry: &TemplateRegistry,
    compiler: &dyn PdfCompiler,
    template: &TemplateId,
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
    Template(TemplateId),
    Shared,
}

pub(crate) fn changed_template(root: &Path, path: &Path) -> Option<ChangedTemplate> {
    let relative_path = path.strip_prefix(root).ok()?;
    let mut template_path = root.to_owned();
    let mut segments = Vec::new();

    for component in relative_path.components() {
        let segment = component.as_os_str().to_str()?;
        if segments.is_empty() && segment == "_shared" {
            return Some(ChangedTemplate::Shared);
        }
        if segment.starts_with('_') {
            return None;
        }

        template_path.push(segment);
        segments.push(segment);
        if template_path.join("main.typ").is_file() {
            return TemplateId::new(segments.join("/"))
                .ok()
                .map(ChangedTemplate::Template);
        }
    }

    None
}

fn is_private_discovery_path(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root).ok().is_some_and(|relative_path| {
        relative_path.components().any(|component| {
            component
                .as_os_str()
                .to_str()
                .is_some_and(|segment| segment.starts_with('_'))
        })
    })
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
        CompilationError, Document, PdfCompiler, PreparedTemplate, TemplateId, TemplateRegistry,
        TemplateSource,
    };

    use crate::loader::filesystem::FilesystemLoader;
    use crate::output::{ColorChoice, DiagnosticReporter, OutputFormat};

    use super::{ChangedTemplate, changed_template, reload_all, reload_template};

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
        let root = tempdir().unwrap();
        write_template(root.path(), "financial/invoice", "invoice");
        assert_eq!(
            changed_template(root.path(), &root.path().join("financial/invoice/main.typ")),
            Some(ChangedTemplate::Template(
                TemplateId::new("financial/invoice").unwrap()
            ))
        );
    }

    #[test]
    fn resolves_shared_path() {
        let root = tempdir().unwrap();
        assert_eq!(
            changed_template(root.path(), &root.path().join("_shared/brand.typ")),
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
    fn ignores_private_discovery_paths() {
        let root = tempdir().unwrap();
        write_template(root.path(), "_draft/invoice", "draft");
        assert_eq!(
            changed_template(root.path(), &root.path().join("_draft/invoice/main.typ")),
            None
        );
    }

    #[test]
    fn resolves_private_source_below_active_template() {
        let root = tempdir().unwrap();
        write_template(root.path(), "invoice", "invoice");

        assert_eq!(
            changed_template(
                root.path(),
                &root.path().join("invoice/_components/header.typ")
            ),
            Some(ChangedTemplate::Template(
                TemplateId::new("invoice").unwrap()
            ))
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
        let name = TemplateId::new("invoice").unwrap();
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
    async fn full_reload_rebuilds_every_template_atomically() {
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

        reload_all(&loader, &registry, &compiler, &reporter).await;
        let first_count = compiler.prepares.load(Ordering::SeqCst);
        reload_all(&loader, &registry, &compiler, &reporter).await;

        assert_eq!(first_count, 2);
        assert_eq!(compiler.prepares.load(Ordering::SeqCst), 4);
        assert_eq!(registry.ids().len(), 2);
    }

    #[tokio::test]
    async fn failed_full_reload_keeps_complete_previous_registry() {
        let root = tempdir().unwrap();
        write_template(root.path(), "financial/invoice", "invoice");
        write_template(root.path(), "financial/report", "report");
        let loader = FilesystemLoader::new(root.path());
        let registry = TemplateRegistry::default();
        let compiler = MockCompiler {
            fail: AtomicBool::new(false),
            prepares: AtomicUsize::new(0),
        };
        let reporter =
            DiagnosticReporter::new(OutputFormat::Pretty, ColorChoice::Never, root.path());
        reload_all(&loader, &registry, &compiler, &reporter).await;
        let previous_names = registry.ids();

        fs::write(root.path().join("financial/main.typ"), "financial").unwrap();
        compiler.fail.store(true, Ordering::SeqCst);
        reload_all(&loader, &registry, &compiler, &reporter).await;

        assert_eq!(registry.ids(), previous_names);
    }

    #[tokio::test]
    async fn full_reload_reconciles_ancestor_entrypoint_topology() {
        let root = tempdir().unwrap();
        write_template(root.path(), "financial/invoice", "invoice");
        write_template(root.path(), "financial/report", "report");
        let loader = FilesystemLoader::new(root.path());
        let registry = TemplateRegistry::default();
        let compiler = MockCompiler {
            fail: AtomicBool::new(false),
            prepares: AtomicUsize::new(0),
        };
        let reporter =
            DiagnosticReporter::new(OutputFormat::Pretty, ColorChoice::Never, root.path());

        reload_all(&loader, &registry, &compiler, &reporter).await;
        assert_eq!(
            registry
                .ids()
                .into_iter()
                .map(TemplateId::into_inner)
                .collect::<Vec<_>>(),
            ["financial/invoice", "financial/report"]
        );

        fs::write(root.path().join("financial/main.typ"), "financial").unwrap();
        reload_all(&loader, &registry, &compiler, &reporter).await;
        assert_eq!(
            registry
                .ids()
                .into_iter()
                .map(TemplateId::into_inner)
                .collect::<Vec<_>>(),
            ["financial"]
        );

        fs::remove_file(root.path().join("financial/main.typ")).unwrap();
        reload_all(&loader, &registry, &compiler, &reporter).await;
        assert_eq!(
            registry
                .ids()
                .into_iter()
                .map(TemplateId::into_inner)
                .collect::<Vec<_>>(),
            ["financial/invoice", "financial/report"]
        );
    }
}
