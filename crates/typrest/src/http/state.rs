use crate::output::DiagnosticReporter;

use typrest_core::{RenderService, TemplateRegistry};

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
    use crate::http::config::ServerConfig;

    #[test]
    fn web_ui_is_disabled_by_default_and_can_be_enabled() {
        let disabled = ServerConfig::new(8080);
        assert!(!disabled.web_ui);

        let enabled = disabled.with_web_ui(true);
        assert!(enabled.web_ui);
    }
}
