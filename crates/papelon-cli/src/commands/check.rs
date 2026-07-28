use papelon_core::{TemplateRegistry, TypstCompiler};
use papelon_server::{
    loader::filesystem::FilesystemLoader,
    output::{
        ColorChoice as ServerColorChoice, DiagnosticContext, DiagnosticReporter, OutputFormat,
        log_check_success,
    },
};

use crate::config::load_config;

pub(crate) async fn run_check(output_format: OutputFormat, color: ServerColorChoice) -> bool {
    let Some(config) = load_config(output_format, color, DiagnosticContext::Check) else {
        return false;
    };
    let reporter = DiagnosticReporter::new(output_format, color, &config.templates_root);

    let loader = FilesystemLoader::new(&config.templates_root);
    let compiler = TypstCompiler::new();
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
