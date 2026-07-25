use std::path::PathBuf;

use papelon_core::{RenderService, TemplateRegistry};

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
    pub registry: TemplateRegistry,
    pub render_service: RenderService,
}

impl AppState {
    pub fn new(registry: TemplateRegistry, render_service: RenderService) -> Self {
        Self {
            registry,
            render_service,
        }
    }
}
