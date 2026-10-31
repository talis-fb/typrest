use std::sync::Arc;

use tracing::{error, info};
use typrest_core::{
    CompilationDiagnostic, PdfCompiler, RenderExecutor, RenderService, TemplateId,
    TemplateRegistry, TemplateRegistryError, TemplateRegistryEvent, TypstCompiler,
};

use crate::args::{ColorChoice, LogFormat};
use crate::http::config::ServerConfig;
use crate::http::state::AppState;
use crate::loader::filesystem::FilesystemLoader;
use crate::loader::watch::start_template_watcher;
use crate::logging::compiler_diagnostics;
use crate::package::universal_package::UniversePackageRepository;
use crate::render_executor::BlockingRenderExecutor;

use crate::config::load_config;

pub(crate) async fn run_serve(
    log_format: LogFormat,
    color: ColorChoice,
    port: u16,
    hostname: std::net::Ipv4Addr,
    watch: bool,
) -> bool {
    let Some(config) = load_config() else {
        return false;
    };
    let loader = Arc::new(FilesystemLoader::new(&config.templates_root));
    let packages = Arc::new(UniversePackageRepository::new(
        &config.package_cache_dir,
        "startup",
    ));
    let compiler: Arc<dyn PdfCompiler> =
        Arc::new(TypstCompiler::new().with_package_repository(packages.clone()));
    let executor = Arc::new(BlockingRenderExecutor::new(config.render_limits));
    let diagnostic =
        |stage: &'static str, template: Option<&TemplateId>, value: &CompilationDiagnostic| {
            if log_format == LogFormat::Pretty {
                compiler_diagnostics::write(color, &config.templates_root, template, value);
            } else {
                compiler_diagnostics::trace(log_format, "startup", stage, template, value);
            }
        };

    let registry_observer = |event| match event {
        TemplateRegistryEvent::CompilationStarted { template } => info!(
            event = "template.compilation.started",
            context = "startup",
            template = %template,
            status = "started",
            "compiling template `{template}`"
        ),
        TemplateRegistryEvent::CompilationCompleted {
            template,
            warning_count,
        } => info!(
            event = "template.compilation.completed",
            context = "startup",
            template = %template,
            warning_count,
            status = "ok",
            "compiled template `{template}`"
        ),
        TemplateRegistryEvent::CompilationFailed { template } => error!(
            event = "template.compilation.failed",
            context = "startup",
            template = %template,
            status = "error",
            "failed to compile template `{template}`"
        ),
    };
    let registry_build = match TemplateRegistry::build_with_observer(
        loader.as_ref(),
        &compiler,
        executor.as_ref(),
        &registry_observer,
    )
    .await
    {
        Ok(build) => build,
        Err(error) => {
            for warning in error.warnings() {
                diagnostic("compile", warning.template.as_ref(), warning);
            }
            for failure in error.value().failures() {
                match failure {
                    TemplateRegistryError::List { source } => error!(
                        event = "compiler.diagnostic",
                        context = "startup",
                        stage = "load",
                        diagnostic_message = %source,
                        "{source}"
                    ),
                    TemplateRegistryError::Load { template, source } => error!(
                        event = "compiler.diagnostic",
                        context = "startup",
                        stage = "load",
                        template = %template,
                        diagnostic_message = %source,
                        "template `{template}`: {source}"
                    ),
                    TemplateRegistryError::Prepare { template, source } => {
                        for value in source.report().diagnostics() {
                            diagnostic("prepare", Some(template), value);
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
                            context = "startup",
                            stage = "prepare",
                            template = %template,
                            diagnostic_message = message,
                            "template `{template}`: {message}"
                        );
                    }
                    TemplateRegistryError::Warmup { template, source } => {
                        for value in source.report().diagnostics() {
                            diagnostic("compile", Some(template), value);
                        }
                    }
                    TemplateRegistryError::WarmupAborted { template, message } => error!(
                        event = "compiler.diagnostic",
                        context = "startup",
                        stage = "compile",
                        template = %template,
                        diagnostic_message = %message,
                        "template `{template}`: {message}"
                    ),
                }
            }
            error!(
                event = "templates.check.failed",
                context = "startup",
                template_count = error.value().template_count(),
                failed_count = error.value().failures().len(),
                warning_count = error.warnings().len(),
                "template compilation failed"
            );
            return false;
        }
    };
    for warning in registry_build.warnings() {
        diagnostic("compile", warning.template.as_ref(), warning);
    }
    let (registry, _) = registry_build.into_parts();
    let templates = registry.ids();
    info!(
        event = "templates.ready",
        context = "startup",
        template_count = templates.len(),
        status = "ok",
        "templates ready: {}",
        templates.len()
    );
    for template in templates {
        info!(
            event = "template.ready",
            context = "startup",
            template = %template,
            status = "ok",
            "template ready: `{template}`"
        );
    }

    packages.use_watch_context();

    let _watcher = if watch {
        let watch_executor: Arc<dyn RenderExecutor> = executor.clone();
        match start_template_watcher(
            loader,
            registry.clone(),
            compiler.clone(),
            watch_executor,
            log_format,
        ) {
            Ok(watcher) => Some(watcher),
            Err(error) => {
                error!(
                    event = "template.watch.start.failed",
                    error = %error,
                    "template watcher failed: {error}"
                );
                return false;
            }
        }
    } else {
        None
    };

    info!(
        event = "server.starting",
        max_concurrent_renders = config.render_limits.max_concurrent_renders,
        max_queued_renders = config.render_limits.max_queued_renders,
        "starting typrest server"
    );
    if _watcher.is_some() {
        info!(
            event = "template.watch.enabled",
            templates_dir = %config.templates_root.display(),
            "template watch mode enabled"
        );
    }

    let render_service = RenderService::new(registry.clone(), compiler, executor);
    let state = AppState::new(registry, render_service, log_format);
    let server_config = ServerConfig::new(port)
        .with_host(hostname)
        .with_web_ui(config.web_ui);

    match crate::http::serve(server_config, state).await {
        Ok(()) => true,
        Err(error) => {
            error!(
                event = "server.failed",
                error = %error,
                "typrest server failed: {error}"
            );
            false
        }
    }
}
