use std::net::Ipv4Addr;

use crate::config::DEFAULT_MAX_BODY_BYTES;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub host: [u8; 4],
    pub web_ui: bool,
    pub max_body_bytes: usize,
}

impl ServerConfig {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            host: [0, 0, 0, 0],
            web_ui: false,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
        }
    }

    pub fn with_max_body_bytes(mut self, max_body_bytes: usize) -> Self {
        self.max_body_bytes = max_body_bytes;
        self
    }

    pub fn with_host(mut self, host: Ipv4Addr) -> Self {
        self.host = host.octets();
        self
    }

    pub fn with_web_ui(mut self, web_ui: bool) -> Self {
        self.web_ui = web_ui;
        self
    }
}
