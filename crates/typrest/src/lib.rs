mod args;
mod commands;
mod config;
mod http;
mod loader;
mod logging;
mod package;
mod render_executor;

use std::process::ExitCode;

use clap::Parser;

use crate::args::{Args, Command};
use crate::commands::check::run_check;
use crate::commands::init::run_init;
use crate::commands::serve::run_serve;
use crate::logging::subscriber;

pub async fn run() -> ExitCode {
    let args = Args::parse();
    subscriber::init(args.log_format, args.color);

    let success = match args.command {
        Command::Serve {
            port,
            hostname,
            watch,
        } => run_serve(args.log_format, args.color, port, hostname, watch).await,
        Command::Check => run_check(args.log_format, args.color).await,
        Command::Init { path } => run_init(path).await,
    };

    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
