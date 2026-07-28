use std::net::Ipv4Addr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub templates_dir: PathBuf,
    pub port: u16,
    pub host: [u8; 4],
    pub watch: bool,
    pub web_ui: bool,
    pub api_enabled: bool,
}

impl ServerConfig {
    pub fn new(templates_dir: impl Into<PathBuf>, port: u16) -> Self {
        Self {
            templates_dir: templates_dir.into(),
            port,
            host: [0, 0, 0, 0],
            watch: false,
            web_ui: false,
            api_enabled: true,
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

    pub fn with_api_enabled(mut self, api_enabled: bool) -> Self {
        self.api_enabled = api_enabled;
        self
    }
}
