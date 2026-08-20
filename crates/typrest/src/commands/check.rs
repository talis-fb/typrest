use std::sync::Arc;

use crate::loader::filesystem::FilesystemLoader;
use crate::output::{
    ColorChoice as ServerColorChoice, DiagnosticContext, DiagnosticReporter, LifecycleReporter,
    OutputFormat,
};
use crate::package::universal_package::UniversePackageRepository;
use typrest_core::{TemplateRegistry, TypstCompiler};

use crate::config::load_config;

pub(crate) async fn run_check(output_format: OutputFormat, color: ServerColorChoice) -> bool {
    let Some(config) = load_config(output_format, color, DiagnosticContext::Check) else {
        return false;
    };
    let reporter = DiagnosticReporter::new(output_format, color, &config.templates_root);
    let lifecycle = LifecycleReporter::new(output_format, DiagnosticContext::Check);

    let loader = FilesystemLoader::new(&config.templates_root);
    let packages = Arc::new(
        UniversePackageRepository::new(&config.package_cache_dir)
            .with_event_handler(lifecycle.package_event_handler()),
    );
    let compiler = TypstCompiler::new().with_package_repository(packages);
    let registry_observer = |event| lifecycle.report_registry_event(event);
    let registry_build_result =
        TemplateRegistry::build_with_observer(&loader, &compiler, &registry_observer).await;

    match registry_build_result {
        Ok(build) => {
            reporter.report_build(DiagnosticContext::Check, &build);
            lifecycle.report_ready(build.value());
            true
        }
        Err(error) => {
            reporter.report_build_error(DiagnosticContext::Check, &error);
            false
        }
    }
}
