mod args;
mod commands;
mod config;

use std::process::ExitCode;

use clap::Parser;
use papelon_server::output::{ColorChoice as ServerColorChoice, OutputFormat, init_tracing};

use crate::args::{Args, ColorChoice, Command, LogFormat};
use crate::commands::check::run_check;
use crate::commands::init::run_init;
use crate::commands::serve::run_serve;

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
        Command::Init { path } => run_init(path, output_format, color).await,
    };

    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
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
