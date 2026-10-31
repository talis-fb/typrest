mod args;
mod commands;
mod config;
mod http;
mod loader;
mod logging;
mod package;
mod render_executor;
mod shutdown;

use std::process::ExitCode;

use clap::Parser;
use tracing::error;

use crate::args::{Args, Command};
use crate::commands::check::run_check;
use crate::commands::init::run_init;
use crate::commands::serve::run_serve;
use crate::logging::subscriber;

pub fn run() -> ExitCode {
    let args = Args::parse();
    subscriber::init(args.log_format, args.color);

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            error!(
                event = "runtime.start.failed",
                error = %error,
                "failed to start the async runtime: {error}"
            );
            return ExitCode::FAILURE;
        }
    };

    let success = runtime.block_on(async {
        match args.command {
            Command::Serve {
                port,
                hostname,
                watch,
            } => run_serve(args.log_format, args.color, port, hostname, watch).await,
            Command::Check => run_check(args.log_format, args.color).await,
            Command::Init { path } => run_init(path).await,
        }
    });
    // Detached blocking work, such as an abandoned compilation, must not delay process exit.
    runtime.shutdown_background();

    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
