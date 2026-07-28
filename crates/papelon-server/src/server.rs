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
    pub web_ui: bool,
}

impl ServerConfig {
    pub fn new(templates_dir: impl Into<PathBuf>, port: u16) -> Self {
        Self {
            templates_dir: templates_dir.into(),
            port,
            host: [0, 0, 0, 0],
            watch: false,
            web_ui: false,
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

    pub fn with_web_ui(mut self, web_ui: bool) -> Self {
        self.web_ui = web_ui;
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::ServerConfig;

    #[test]
    fn web_ui_is_disabled_by_default_and_can_be_enabled() {
        let disabled = ServerConfig::new(PathBuf::from("templates"), 8080);
        assert!(!disabled.web_ui);

        let enabled = disabled.with_web_ui(true);
        assert!(enabled.web_ui);
    }
}
