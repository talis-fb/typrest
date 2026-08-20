use std::sync::Arc;

use tracing::info;
use typrest_core::{PdfCompiler, RenderService, TemplateRegistry, TypstCompiler};

use crate::http::config::ServerConfig;
use crate::http::state::AppState;
use crate::loader::filesystem::FilesystemLoader;
use crate::loader::watch::start_template_watcher;
use crate::output::{
    ColorChoice as ServerColorChoice, DiagnosticContext, DiagnosticReporter, DiagnosticStage,
    LifecycleReporter, OutputFormat,
};
use crate::package::universal_package::UniversePackageRepository;

use crate::config::load_config;

pub(crate) async fn run_serve(
    output_format: OutputFormat,
    color: ServerColorChoice,
    port: u16,
    hostname: std::net::Ipv4Addr,
    watch: bool,
) -> bool {
    let Some(config) = load_config(output_format, color, DiagnosticContext::Startup) else {
        return false;
    };
    let reporter = DiagnosticReporter::new(output_format, color, &config.templates_root);
    let lifecycle = LifecycleReporter::new(output_format, DiagnosticContext::Startup);
    let loader = Arc::new(FilesystemLoader::new(&config.templates_root));
    let packages = Arc::new(
        UniversePackageRepository::new(&config.package_cache_dir)
            .with_event_handler(lifecycle.package_event_handler()),
    );
    let compiler: Arc<dyn PdfCompiler> =
        Arc::new(TypstCompiler::new().with_package_repository(packages));

    let registry_observer = |event| lifecycle.report_registry_event(event);
    let registry_build = match TemplateRegistry::build_with_observer(
        loader.as_ref(),
        compiler.as_ref(),
        &registry_observer,
    )
    .await
    {
        Ok(build) => build,
        Err(error) => {
            reporter.report_build_error(DiagnosticContext::Startup, &error);
            return false;
        }
    };
    reporter.report_build(DiagnosticContext::Startup, &registry_build);
    let (registry, _) = registry_build.into_parts();
    lifecycle.report_ready(&registry);
    lifecycle.set_context(DiagnosticContext::Watch);

    let _watcher = if watch {
        let watch_reporter = reporter.clone();
        let watch_lifecycle = lifecycle.clone();
        let ready_lifecycle = lifecycle.clone();
        match start_template_watcher(
            loader,
            registry.clone(),
            compiler.clone(),
            Arc::new(move |event| watch_reporter.report_watch_event(event)),
            Arc::new(move |event| watch_lifecycle.report_registry_event(event)),
            Arc::new(move |registry| ready_lifecycle.report_ready(registry)),
        ) {
            Ok(watcher) => {
                info!(
                    templates_dir = %config.templates_root.display(),
                    "template watch mode enabled"
                );
                Some(watcher)
            }
            Err(error) => {
                reporter.report_simple_error(
                    DiagnosticContext::Startup,
                    DiagnosticStage::Load,
                    None,
                    &format!("template watcher failed: {error}"),
                );
                return false;
            }
        }
    } else {
        None
    };

    let render_service = RenderService::new(registry.clone(), compiler);
    let state = AppState::new(registry, render_service, reporter.clone());
    let server_config = ServerConfig::new(port)
        .with_host(hostname)
        .with_web_ui(config.web_ui);

    match crate::http::serve(server_config, state).await {
        Ok(()) => true,
        Err(error) => {
            reporter.report_simple_error(
                DiagnosticContext::Startup,
                DiagnosticStage::Load,
                None,
                &error.to_string(),
            );
            false
        }
    }
}
