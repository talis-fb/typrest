mod args;
mod config;
mod init;

use std::process::ExitCode;

use clap::Parser;
use papelon_server::ServeError;
use papelon_server::output::{
    ColorChoice as ServerColorChoice, DiagnosticContext, DiagnosticReporter, DiagnosticStage,
    OutputFormat, init_tracing, log_check_success,
};
use papelon_server::server::ServerConfig;

use crate::args::{Args, ColorChoice, Command, LogFormat};
use crate::config::{find_project_root, load_project_config};
use crate::init::init_project;

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    let output_format = output_format(args.log_format);
    let color = color_choice(args.color);
    init_tracing(output_format, color);

    let success = match args.command {
        Command::Serve {
            port,
            hostname,
            watch,
        } => run_serve(output_format, color, port, hostname, watch).await,
        Command::Check => run_check(output_format, color).await,
        Command::Init { path } => {
            let current_dir = std::env::current_dir().unwrap_or_default();
            let reporter = DiagnosticReporter::new(output_format, color, &current_dir);
            match init_project(&current_dir, path.as_deref()) {
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
    };

    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

async fn run_serve(
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

async fn run_check(output_format: OutputFormat, color: ServerColorChoice) -> bool {
    let Some(config) = load_config(output_format, color, DiagnosticContext::Check) else {
        return false;
    };
    let reporter = DiagnosticReporter::new(output_format, color, &config.templates_root);

    match papelon_server::compile_templates(&config.templates_root).await {
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

fn load_config(
    output_format: OutputFormat,
    color: ServerColorChoice,
    context: DiagnosticContext,
) -> Option<config::ProjectConfig> {
    let current_dir = std::env::current_dir().unwrap_or_default();
    let reporter = DiagnosticReporter::new(output_format, color, &current_dir);
    let result = find_project_root(&current_dir).and_then(|root| load_project_config(&root));

    match result {
        Ok(config) => Some(config),
        Err(error) => {
            reporter.report_simple_error(context, DiagnosticStage::Load, None, &error.to_string());
            None
        }
    }
}

fn output_format(format: LogFormat) -> OutputFormat {
    match format {
        LogFormat::Pretty => OutputFormat::Pretty,
        LogFormat::Json => OutputFormat::Json,
    }
}

fn color_choice(color: ColorChoice) -> ServerColorChoice {
    match color {
        ColorChoice::Auto => ServerColorChoice::Auto,
        ColorChoice::Always => ServerColorChoice::Always,
        ColorChoice::Never => ServerColorChoice::Never,
    }
}
