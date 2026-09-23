use crate::args::LogFormat;

use typrest_core::{RenderService, TemplateRegistry};

#[derive(Clone)]
pub struct AppState {
    pub registry: TemplateRegistry,
    pub render_service: RenderService,
    pub log_format: LogFormat,
}

impl AppState {
    pub fn new(
        registry: TemplateRegistry,
        render_service: RenderService,
        log_format: LogFormat,
    ) -> Self {
        Self {
            registry,
            render_service,
            log_format,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::http::config::ServerConfig;

    #[test]
    fn web_ui_is_disabled_by_default_and_can_be_enabled() {
        let disabled = ServerConfig::new(8080);
        assert!(!disabled.web_ui);

        let enabled = disabled.with_web_ui(true);
        assert!(enabled.web_ui);
    }
}
