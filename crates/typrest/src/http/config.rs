use std::net::Ipv4Addr;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub host: [u8; 4],
    pub web_ui: bool,
}

impl ServerConfig {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            host: [0, 0, 0, 0],
            web_ui: false,
        }
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
