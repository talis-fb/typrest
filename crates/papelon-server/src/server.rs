use std::net::Ipv4Addr;
use std::path::PathBuf;

use papelon_core::{RenderService, TemplateRegistry};

use crate::output::DiagnosticReporter;

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

    pub fn with_host(mut self, host: Ipv4Addr) -> Self {
        self.host = host.octets();
        self
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
    pub reporter: DiagnosticReporter,
}

impl AppState {
    pub fn new(
        registry: TemplateRegistry,
        render_service: RenderService,
        reporter: DiagnosticReporter,
    ) -> Self {
        Self {
            registry,
            render_service,
            reporter,
        }
    }
}
