use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use papelon_core::{RenderService, TypstCompiler};
use papelon_server::loader::filesystem::FilesystemLoader;
use papelon_server::{AppState, router};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "papelon-server")]
struct Args {
    #[arg(long)]
    templates: PathBuf,

    #[arg(default_value_t = 8080)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let args = Args::parse();
    let loader = Arc::new(FilesystemLoader::new(args.templates));
    let compiler = Arc::new(TypstCompiler::new());
    let render_service = RenderService::new(loader.clone(), compiler);
    let app = router(AppState::new(loader, render_service));
    let address = SocketAddr::from(([0, 0, 0, 0], args.port));
    let listener = tokio::net::TcpListener::bind(address).await?;

    info!(%address, "papelon server listening");
    axum::serve(listener, app).await?;

    Ok(())
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("papelon_server=info"));

    tracing_subscriber::fmt().with_env_filter(filter).init();
}
