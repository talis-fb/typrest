use std::path::PathBuf;
use std::sync::Arc;

use papelon_core::{RenderService, TemplateLoader};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub templates_dir: PathBuf,
    pub port: u16,
    pub host: [u8; 4],
    pub watch: bool,
}

impl ServerConfig {
    pub fn new(templates_dir: impl Into<PathBuf>, port: u16) -> Self {
        Self {
            templates_dir: templates_dir.into(),
            port,
            host: [0, 0, 0, 0],
            watch: false,
        }
    }

    pub fn with_watch(mut self, watch: bool) -> Self {
        self.watch = watch;
        self
    }
}

#[derive(Clone)]
pub struct AppState {
    pub loader: Arc<dyn TemplateLoader>,
    pub render_service: RenderService,
}

impl AppState {
    pub fn new(loader: Arc<dyn TemplateLoader>, render_service: RenderService) -> Self {
        Self {
            loader,
            render_service,
        }
    }
}
