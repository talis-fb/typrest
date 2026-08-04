mod args;
mod commands;
mod config;
mod http;
mod loader;
mod output;
mod package;

use std::process::ExitCode;

use clap::Parser;

use crate::args::{Args, ColorChoice, Command, LogFormat};
use crate::commands::check::run_check;
use crate::commands::init::run_init;
use crate::commands::serve::run_serve;
use crate::output::{ColorChoice as OutputColorChoice, OutputFormat, init_tracing};

pub async fn run() -> ExitCode {
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

fn color_choice(color: ColorChoice) -> OutputColorChoice {
    match color {
        ColorChoice::Auto => OutputColorChoice::Auto,
        ColorChoice::Always => OutputColorChoice::Always,
        ColorChoice::Never => OutputColorChoice::Never,
    }
}
