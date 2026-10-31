use std::path::Path;
use std::sync::Arc;

use notify::event::ModifyKind;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use typrest_core::{
    PdfCompiler, RenderExecutor, TemplateId, TemplateRegistry, TemplateRegistryError,
    TemplateRegistryEvent,
};

use crate::args::LogFormat;
use crate::loader::filesystem::FilesystemLoader;
use crate::logging::compiler_diagnostics;

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
    executor: Arc<dyn RenderExecutor>,
    log_format: LogFormat,
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
                &compiler,
                executor.as_ref(),
                log_format,
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
    compiler: &Arc<dyn PdfCompiler>,
    executor: &dyn RenderExecutor,
    log_format: LogFormat,
) {
    let mut rebuild_all = false;
    let mut templates = Vec::new();

    for result in results {
        let event = match result {
            Ok(event) if is_relevant_event(&event) => event,
            Ok(_) => continue,
            Err(error) => {
                error!(
                    event = "template.watch.backend.failed",
                    error = %error,
                    "template watch event failed: {error}"
                );
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
        reload_all_observed(loader, registry, compiler, executor, log_format).await;
        return;
    }

    for template in templates {
        reload_template_observed(loader, registry, compiler, executor, &template, log_format).await;
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
    compiler: &Arc<dyn PdfCompiler>,
    executor: &dyn RenderExecutor,
) {
    reload_all_observed(loader, registry, compiler, executor, LogFormat::Pretty).await;
}

async fn reload_all_observed(
    loader: &FilesystemLoader,
    registry: &TemplateRegistry,
    compiler: &Arc<dyn PdfCompiler>,
    executor: &dyn RenderExecutor,
    log_format: LogFormat,
) {
    let on_registry_event = |event| log_registry_event(event);
    match TemplateRegistry::build_with_observer(loader, compiler, executor, &on_registry_event)
        .await
    {
        Ok(build) => {
            let template_count = build.value().ids().len();
            for warning in build.warnings() {
                compiler_diagnostics::trace(
                    log_format,
                    "watch",
                    "compile",
                    warning.template.as_ref(),
                    warning,
                );
            }
            registry.replace_all(build.value());
            info!(
                event = "template.registry.rebuilt",
                template_count,
                warning_count = build.warnings().len(),
                "template registry rebuilt"
            );
            info!(
                event = "templates.ready",
                context = "watch",
                template_count,
                status = "ok",
                "templates ready: {template_count}"
            );
            for template in registry.ids() {
                info!(
                    event = "template.ready",
                    context = "watch",
                    template = %template,
                    status = "ok",
                    "template ready: `{template}`"
                );
            }
        }
        Err(build_error) => {
            for warning in build_error.warnings() {
                compiler_diagnostics::trace(
                    log_format,
                    "watch",
                    "compile",
                    warning.template.as_ref(),
                    warning,
                );
            }
            for failure in build_error.value().failures() {
                match failure {
                    TemplateRegistryError::List { source } => error!(
                        event = "compiler.diagnostic",
                        context = "watch",
                        stage = "load",
                        diagnostic_message = %source,
                        "{source}"
                    ),
                    TemplateRegistryError::Load { template, source } => error!(
                        event = "compiler.diagnostic",
                        context = "watch",
                        stage = "load",
                        template = %template,
                        diagnostic_message = %source,
                        "template `{template}`: {source}"
                    ),
                    TemplateRegistryError::Prepare { template, source } => {
                        for value in source.report().diagnostics() {
                            compiler_diagnostics::trace(
                                log_format,
                                "watch",
                                "prepare",
                                Some(template),
                                value,
                            );
                        }
                    }
                    TemplateRegistryError::SamplePayload {
                        template,
                        violations,
                    } => {
                        let message = violations
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join("; ");
                        error!(
                            event = "compiler.diagnostic",
                            context = "watch",
                            stage = "prepare",
                            template = %template,
                            diagnostic_message = message,
                            "template `{template}`: {message}"
                        );
                    }
                    TemplateRegistryError::Warmup { template, source } => {
                        for value in source.report().diagnostics() {
                            compiler_diagnostics::trace(
                                log_format,
                                "watch",
                                "compile",
                                Some(template),
                                value,
                            );
                        }
                    }
                    TemplateRegistryError::WarmupAborted { template, message } => error!(
                        event = "compiler.diagnostic",
                        context = "watch",
                        stage = "compile",
                        template = %template,
                        diagnostic_message = %message,
                        "template `{template}`: {message}"
                    ),
                }
            }
            error!(
                event = "templates.check.failed",
                context = "watch",
                template_count = build_error.value().template_count(),
                failed_count = build_error.value().failures().len(),
                warning_count = build_error.warnings().len(),
                "template compilation failed"
            );
            warn!(
                event = "template.registry.rebuild.failed",
                last_known_good = true,
                "template registry rebuild failed; keeping last known good version"
            );
        }
    }
}

#[cfg(test)]
async fn reload_template(
    loader: &FilesystemLoader,
    registry: &TemplateRegistry,
    compiler: &Arc<dyn PdfCompiler>,
    executor: &dyn RenderExecutor,
    template: &TemplateId,
) {
    reload_template_observed(
        loader,
        registry,
        compiler,
        executor,
        template,
        LogFormat::Pretty,
    )
    .await;
}

async fn reload_template_observed(
    loader: &FilesystemLoader,
    registry: &TemplateRegistry,
    compiler: &Arc<dyn PdfCompiler>,
    executor: &dyn RenderExecutor,
    template: &TemplateId,
    log_format: LogFormat,
) {
    let on_registry_event = |event| log_registry_event(event);
    match registry
        .reload_with_observer(loader, compiler, executor, template, &on_registry_event)
        .await
    {
        Ok(reload) => {
            for warning in reload.warnings() {
                compiler_diagnostics::trace(
                    log_format,
                    "watch",
                    "compile",
                    warning.template.as_ref(),
                    warning,
                );
            }
            info!(
                event = "template.reload.completed",
                template = %template,
                warning_count = reload.warnings().len(),
                "template reloaded"
            );
        }
        Err(error) => {
            match &error {
                TemplateRegistryError::List { source } => error!(
                    event = "compiler.diagnostic",
                    context = "watch",
                    stage = "load",
                    diagnostic_message = %source,
                    "{source}"
                ),
                TemplateRegistryError::Load { template, source } => error!(
                    event = "compiler.diagnostic",
                    context = "watch",
                    stage = "load",
                    template = %template,
                    diagnostic_message = %source,
                    "template `{template}`: {source}"
                ),
                TemplateRegistryError::Prepare { template, source } => {
                    for value in source.report().diagnostics() {
                        compiler_diagnostics::trace(
                            log_format,
                            "watch",
                            "prepare",
                            Some(template),
                            value,
                        );
                    }
                }
                TemplateRegistryError::SamplePayload {
                    template,
                    violations,
                } => {
                    let message = violations
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join("; ");
                    error!(
                        event = "compiler.diagnostic",
                        context = "watch",
                        stage = "prepare",
                        template = %template,
                        diagnostic_message = message,
                        "template `{template}`: {message}"
                    );
                }
                TemplateRegistryError::Warmup { template, source } => {
                    for value in source.report().diagnostics() {
                        compiler_diagnostics::trace(
                            log_format,
                            "watch",
                            "compile",
                            Some(template),
                            value,
                        );
                    }
                }
                TemplateRegistryError::WarmupAborted { template, message } => error!(
                    event = "compiler.diagnostic",
                    context = "watch",
                    stage = "compile",
                    template = %template,
                    diagnostic_message = %message,
                    "template `{template}`: {message}"
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

fn log_registry_event(event: TemplateRegistryEvent) {
    match event {
        TemplateRegistryEvent::CompilationStarted { template } => info!(
            event = "template.compilation.started",
            context = "watch",
            template = %template,
            status = "started",
            "compiling template `{template}`"
        ),
        TemplateRegistryEvent::CompilationCompleted {
            template,
            warning_count,
        } => info!(
            event = "template.compilation.completed",
            context = "watch",
            template = %template,
            warning_count,
            status = "ok",
            "compiled template `{template}`"
        ),
        TemplateRegistryEvent::CompilationFailed { template } => error!(
            event = "template.compilation.failed",
            context = "watch",
            template = %template,
            status = "error",
            "failed to compile template `{template}`"
        ),
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
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use async_trait::async_trait;
    use bytes::Bytes;
    use tempfile::tempdir;
    use typrest_core::{
        CompilationError, Document, InlineExecutor, PdfCompiler, PreparedTemplate, TemplateId,
        TemplateRegistry, TemplateSource,
    };

    use crate::loader::filesystem::FilesystemLoader;

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

        fn compile_prepared(
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
        let compiler = Arc::new(MockCompiler {
            fail: AtomicBool::new(false),
            prepares: AtomicUsize::new(0),
        });
        let dyn_compiler: Arc<dyn PdfCompiler> = compiler.clone();
        let name = TemplateId::new("invoice").unwrap();
        reload_template(&loader, &registry, &dyn_compiler, &InlineExecutor, &name).await;
        let first = registry.get(&name).unwrap();

        fs::write(root.path().join("invoice/main.typ"), "second").unwrap();
        reload_template(&loader, &registry, &dyn_compiler, &InlineExecutor, &name).await;
        let second = registry.get(&name).unwrap();
        assert!(!std::sync::Arc::ptr_eq(&first, &second));

        fs::write(
            root.path().join("invoice/schema.json"),
            r#"{"schemaVersion":"v1","request":{"body":{}}}"#,
        )
        .unwrap();
        reload_template(&loader, &registry, &dyn_compiler, &InlineExecutor, &name).await;
        let after_conflict = registry.get(&name).unwrap();
        assert!(std::sync::Arc::ptr_eq(&second, &after_conflict));
        fs::remove_file(root.path().join("invoice/schema.json")).unwrap();

        fs::write(root.path().join("invoice/schema.yml"), "schemaVersion: [").unwrap();
        reload_template(&loader, &registry, &dyn_compiler, &InlineExecutor, &name).await;
        let after_schema_error = registry.get(&name).unwrap();
        assert!(std::sync::Arc::ptr_eq(&second, &after_schema_error));
        fs::write(
            root.path().join("invoice/schema.yml"),
            "schemaVersion: v1\nrequest:\n  body:\n    type: object\n",
        )
        .unwrap();

        compiler.fail.store(true, Ordering::SeqCst);
        fs::write(root.path().join("invoice/main.typ"), "#let broken =").unwrap();
        reload_template(&loader, &registry, &dyn_compiler, &InlineExecutor, &name).await;
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
        let compiler = Arc::new(MockCompiler {
            fail: AtomicBool::new(false),
            prepares: AtomicUsize::new(0),
        });
        let dyn_compiler: Arc<dyn PdfCompiler> = compiler.clone();
        reload_all(&loader, &registry, &dyn_compiler, &InlineExecutor).await;
        let first_count = compiler.prepares.load(Ordering::SeqCst);
        reload_all(&loader, &registry, &dyn_compiler, &InlineExecutor).await;

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
        let compiler = Arc::new(MockCompiler {
            fail: AtomicBool::new(false),
            prepares: AtomicUsize::new(0),
        });
        let dyn_compiler: Arc<dyn PdfCompiler> = compiler.clone();
        reload_all(&loader, &registry, &dyn_compiler, &InlineExecutor).await;
        let previous_names = registry.ids();

        fs::write(root.path().join("financial/main.typ"), "financial").unwrap();
        compiler.fail.store(true, Ordering::SeqCst);
        reload_all(&loader, &registry, &dyn_compiler, &InlineExecutor).await;

        assert_eq!(registry.ids(), previous_names);
    }

    #[tokio::test]
    async fn full_reload_reconciles_ancestor_entrypoint_topology() {
        let root = tempdir().unwrap();
        write_template(root.path(), "financial/invoice", "invoice");
        write_template(root.path(), "financial/report", "report");
        let loader = FilesystemLoader::new(root.path());
        let registry = TemplateRegistry::default();
        let compiler = Arc::new(MockCompiler {
            fail: AtomicBool::new(false),
            prepares: AtomicUsize::new(0),
        });
        let dyn_compiler: Arc<dyn PdfCompiler> = compiler.clone();
        reload_all(&loader, &registry, &dyn_compiler, &InlineExecutor).await;
        assert_eq!(
            registry
                .ids()
                .into_iter()
                .map(TemplateId::into_inner)
                .collect::<Vec<_>>(),
            ["financial/invoice", "financial/report"]
        );

        fs::write(root.path().join("financial/main.typ"), "financial").unwrap();
        reload_all(&loader, &registry, &dyn_compiler, &InlineExecutor).await;
        assert_eq!(
            registry
                .ids()
                .into_iter()
                .map(TemplateId::into_inner)
                .collect::<Vec<_>>(),
            ["financial"]
        );

        fs::remove_file(root.path().join("financial/main.typ")).unwrap();
        reload_all(&loader, &registry, &dyn_compiler, &InlineExecutor).await;
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
