use std::sync::Arc;

use typrest_core::{TemplateRegistry, TypstCompiler};
use typrest_server::{
    loader::filesystem::FilesystemLoader,
    output::{
        ColorChoice as ServerColorChoice, DiagnosticContext, DiagnosticReporter, OutputFormat,
        log_check_success,
    },
    package::UniversePackageRepository,
};

use crate::config::load_config;

pub(crate) async fn run_check(output_format: OutputFormat, color: ServerColorChoice) -> bool {
    let Some(config) = load_config(output_format, color, DiagnosticContext::Check) else {
        return false;
    };
    let reporter = DiagnosticReporter::new(output_format, color, &config.templates_root);

    let loader = FilesystemLoader::new(&config.templates_root);
    let packages = Arc::new(UniversePackageRepository::new(&config.package_cache_dir));
    let compiler = TypstCompiler::new().with_package_repository(packages);
    let registry_build_result = TemplateRegistry::build(&loader, &compiler).await;

    match registry_build_result {
        Ok(build) => {
            reporter.report_build(DiagnosticContext::Check, &build);
            log_check_success(output_format, &build);
            true
        }
        Err(error) => {
            reporter.report_build_error(DiagnosticContext::Check, &error);
            false
        }
    }
}
