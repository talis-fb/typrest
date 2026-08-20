use std::path::Path;
use std::sync::Arc;

use notify::event::ModifyKind;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use typrest_core::{
    CompilationDiagnostic, PdfCompiler, TemplateId, TemplateRegistry, TemplateRegistryBuildFailure,
    TemplateRegistryError, TemplateRegistryEvent, WithWarnings,
};

use crate::loader::filesystem::FilesystemLoader;

pub(crate) type WatchEventHandler = Arc<dyn Fn(WatchEvent) + Send + Sync>;
pub(crate) type RegistryEventHandler = Arc<dyn Fn(TemplateRegistryEvent) + Send + Sync>;
pub(crate) type RegistryReadyHandler = Arc<dyn Fn(&TemplateRegistry) + Send + Sync>;

struct WatchHandlers {
    event: WatchEventHandler,
    registry_event: RegistryEventHandler,
    registry_ready: RegistryReadyHandler,
}

#[derive(Debug)]
pub(crate) enum WatchEvent {
    BackendFailed(notify::Error),
    RegistryRebuilt {
        template_count: usize,
        warnings: Vec<CompilationDiagnostic>,
    },
    RegistryRebuildFailed(WithWarnings<TemplateRegistryBuildFailure>),
    TemplateReloaded {
        template: TemplateId,
        warnings: Vec<CompilationDiagnostic>,
    },
    TemplateReloadFailed(TemplateRegistryError),
}

pub struct TemplateWatcher {
    _watcher: RecommendedWatcher,
    worker: tokio::task::JoinHandle<()>,
}

impl Drop for TemplateWatcher {
    fn drop(&mut self) {
        self.worker.abort();
    }
}

pub(crate) fn start_template_watcher(
    loader: Arc<FilesystemLoader>,
    registry: TemplateRegistry,
    compiler: Arc<dyn PdfCompiler>,
    on_event: WatchEventHandler,
    on_registry_event: RegistryEventHandler,
    on_registry_ready: RegistryReadyHandler,
) -> notify::Result<TemplateWatcher> {
    let root = loader.root().to_owned();
    let watch_root = root.clone();
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let mut watcher = RecommendedWatcher::new(
        move |result| {
            let _ = sender.send(result);
        },
        Config::default(),
    )?;

    watcher.watch(&watch_root, RecursiveMode::Recursive)?;
    let handlers = WatchHandlers {
        event: on_event,
        registry_event: on_registry_event,
        registry_ready: on_registry_ready,
    };
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
                &handlers,
            )
            .await;
        }
    });

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
    handlers: &WatchHandlers,
) {
    let mut rebuild_all = false;
    let mut templates = Vec::new();

    for result in results {
        let event = match result {
            Ok(event) if is_relevant_event(&event) => event,
            Ok(_) => continue,
            Err(error) => {
                (handlers.event)(WatchEvent::BackendFailed(error));
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
        reload_all_observed(
            loader,
            registry,
            compiler,
            &handlers.event,
            handlers.registry_event.as_ref(),
            handlers.registry_ready.as_ref(),
        )
        .await;
        return;
    }

    for template in templates {
        reload_template_observed(
            loader,
            registry,
            compiler,
            &template,
            &handlers.event,
            handlers.registry_event.as_ref(),
        )
        .await;
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

#[cfg(test)]
async fn reload_all(
    loader: &FilesystemLoader,
    registry: &TemplateRegistry,
    compiler: &dyn PdfCompiler,
    on_event: &WatchEventHandler,
) {
    reload_all_observed(loader, registry, compiler, on_event, &|_| {}, &|_| {}).await;
}

async fn reload_all_observed(
    loader: &FilesystemLoader,
    registry: &TemplateRegistry,
    compiler: &dyn PdfCompiler,
    on_event: &WatchEventHandler,
    on_registry_event: &(dyn Fn(TemplateRegistryEvent) + Send + Sync),
    on_registry_ready: &(dyn Fn(&TemplateRegistry) + Send + Sync),
) {
    match TemplateRegistry::build_with_observer(loader, compiler, on_registry_event).await {
        Ok(build) => {
            let template_count = build.value().ids().len();
            let warnings = build.warnings().to_vec();
            registry.replace_all(build.value());
            on_event(WatchEvent::RegistryRebuilt {
                template_count,
                warnings,
            });
            on_registry_ready(registry);
        }
        Err(build_error) => {
            on_event(WatchEvent::RegistryRebuildFailed(build_error));
        }
    }
}

#[cfg(test)]
async fn reload_template(
    loader: &FilesystemLoader,
    registry: &TemplateRegistry,
    compiler: &dyn PdfCompiler,
    template: &TemplateId,
    on_event: &WatchEventHandler,
) {
    reload_template_observed(loader, registry, compiler, template, on_event, &|_| {}).await;
}

async fn reload_template_observed(
    loader: &FilesystemLoader,
    registry: &TemplateRegistry,
    compiler: &dyn PdfCompiler,
    template: &TemplateId,
    on_event: &WatchEventHandler,
    on_registry_event: &(dyn Fn(TemplateRegistryEvent) + Send + Sync),
) {
    match registry
        .reload_with_observer(loader, compiler, template, on_registry_event)
        .await
    {
        Ok(reload) => {
            on_event(WatchEvent::TemplateReloaded {
                template: template.clone(),
                warnings: reload.warnings().to_vec(),
            });
        }
        Err(error) => {
            on_event(WatchEvent::TemplateReloadFailed(error));
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
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use bytes::Bytes;
    use tempfile::tempdir;
    use typrest_core::{
        CompilationError, Document, PdfCompiler, PreparedTemplate, TemplateId, TemplateRegistry,
        TemplateSource,
    };

    use crate::loader::filesystem::FilesystemLoader;

    use super::{
        ChangedTemplate, WatchEventHandler, changed_template, reload_all, reload_template,
        reload_template_observed,
    };

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

    fn ignore_events() -> WatchEventHandler {
        Arc::new(|_| {})
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
        let on_event = ignore_events();

        reload_template(&loader, &registry, &compiler, &name, &on_event).await;
        let first = registry.get(&name).unwrap();

        fs::write(root.path().join("invoice/main.typ"), "second").unwrap();
        reload_template(&loader, &registry, &compiler, &name, &on_event).await;
        let second = registry.get(&name).unwrap();
        assert!(!std::sync::Arc::ptr_eq(&first, &second));

        fs::write(
            root.path().join("invoice/schema.json"),
            r#"{"schemaVersion":"v1","request":{"body":{}}}"#,
        )
        .unwrap();
        reload_template(&loader, &registry, &compiler, &name, &on_event).await;
        let after_conflict = registry.get(&name).unwrap();
        assert!(std::sync::Arc::ptr_eq(&second, &after_conflict));
        fs::remove_file(root.path().join("invoice/schema.json")).unwrap();

        fs::write(root.path().join("invoice/schema.yml"), "schemaVersion: [").unwrap();
        reload_template(&loader, &registry, &compiler, &name, &on_event).await;
        let after_schema_error = registry.get(&name).unwrap();
        assert!(std::sync::Arc::ptr_eq(&second, &after_schema_error));
        fs::write(
            root.path().join("invoice/schema.yml"),
            "schemaVersion: v1\nrequest:\n  body:\n    type: object\n",
        )
        .unwrap();

        compiler.fail.store(true, Ordering::SeqCst);
        fs::write(root.path().join("invoice/main.typ"), "#let broken =").unwrap();
        reload_template(&loader, &registry, &compiler, &name, &on_event).await;
        let after_failure = registry.get(&name).unwrap();

        assert!(std::sync::Arc::ptr_eq(&second, &after_failure));
    }

    #[tokio::test]
    async fn observed_reload_reports_compilation_progress() {
        let root = tempdir().unwrap();
        write_template(root.path(), "invoice", "invoice");
        let loader = FilesystemLoader::new(root.path());
        let registry = TemplateRegistry::default();
        let compiler = MockCompiler {
            fail: AtomicBool::new(false),
            prepares: AtomicUsize::new(0),
        };
        let name = TemplateId::new("invoice").unwrap();
        let on_event = ignore_events();
        let events = Mutex::new(Vec::new());

        reload_template_observed(&loader, &registry, &compiler, &name, &on_event, &|event| {
            events.lock().unwrap().push(event)
        })
        .await;

        assert_eq!(
            *events.lock().unwrap(),
            [
                typrest_core::TemplateRegistryEvent::CompilationStarted {
                    template: name.clone(),
                },
                typrest_core::TemplateRegistryEvent::CompilationCompleted {
                    template: name,
                    warning_count: 0,
                },
            ]
        );
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
        let on_event = ignore_events();

        reload_all(&loader, &registry, &compiler, &on_event).await;
        let first_count = compiler.prepares.load(Ordering::SeqCst);
        reload_all(&loader, &registry, &compiler, &on_event).await;

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
        let on_event = ignore_events();
        reload_all(&loader, &registry, &compiler, &on_event).await;
        let previous_names = registry.ids();

        fs::write(root.path().join("financial/main.typ"), "financial").unwrap();
        compiler.fail.store(true, Ordering::SeqCst);
        reload_all(&loader, &registry, &compiler, &on_event).await;

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
        let on_event = ignore_events();

        reload_all(&loader, &registry, &compiler, &on_event).await;
        assert_eq!(
            registry
                .ids()
                .into_iter()
                .map(TemplateId::into_inner)
                .collect::<Vec<_>>(),
            ["financial/invoice", "financial/report"]
        );

        fs::write(root.path().join("financial/main.typ"), "financial").unwrap();
        reload_all(&loader, &registry, &compiler, &on_event).await;
        assert_eq!(
            registry
                .ids()
                .into_iter()
                .map(TemplateId::into_inner)
                .collect::<Vec<_>>(),
            ["financial"]
        );

        fs::remove_file(root.path().join("financial/main.typ")).unwrap();
        reload_all(&loader, &registry, &compiler, &on_event).await;
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
