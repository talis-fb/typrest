mod args;
mod config;
mod init;

use clap::Parser;
use papelon_server::server::ServerConfig;

use crate::args::{Args, Command};
use crate::config::load_project_config;
use crate::init::init_project;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    match args.command {
        Command::Serve { port, watch } => {
            let project_root = std::env::current_dir()?;
            let config = load_project_config(&project_root)?;
            papelon_server::serve(ServerConfig::new(config.templates_root, port).with_watch(watch))
                .await
        }
        Command::Init => {
            let project_root = std::env::current_dir()?;
            init_project(&project_root)?;
            Ok(())
        }
    }
}
