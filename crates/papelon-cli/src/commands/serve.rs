use papelon_server::config::ServerConfig;
use papelon_server::errors::ServeError;
use papelon_server::output::{
    ColorChoice as ServerColorChoice, DiagnosticContext, DiagnosticReporter, DiagnosticStage,
    OutputFormat,
};

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
    let server_config = ServerConfig::new(&config.templates_root, port)
        .with_host(hostname)
        .with_watch(watch)
        .with_web_ui(config.web_ui);

    match papelon_server::serve(server_config, reporter.clone()).await {
        Ok(()) => true,
        Err(ServeError::Registry(error)) => {
            reporter.report_build_error(DiagnosticContext::Startup, &error);
            false
        }
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
