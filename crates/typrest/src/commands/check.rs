use std::sync::Arc;

use tracing::{error, info};

use crate::args::{ColorChoice, LogFormat};
use crate::loader::filesystem::FilesystemLoader;
use crate::logging::compiler_diagnostics;
use crate::package::universal_package::UniversePackageRepository;
use typrest_core::{
    CompilationDiagnostic, TemplateId, TemplateRegistry, TemplateRegistryError,
    TemplateRegistryEvent, TypstCompiler,
};

use crate::config::load_config;

pub(crate) async fn run_check(log_format: LogFormat, color: ColorChoice) -> bool {
    let Some(config) = load_config() else {
        return false;
    };

    let loader = FilesystemLoader::new(&config.templates_root);
    let packages = Arc::new(UniversePackageRepository::new(
        &config.package_cache_dir,
        "check",
    ));
    let compiler = TypstCompiler::new().with_package_repository(packages);
    let diagnostic =
        |stage: &'static str, template: Option<&TemplateId>, value: &CompilationDiagnostic| {
            if log_format == LogFormat::Pretty {
                compiler_diagnostics::write(color, &config.templates_root, template, value);
            } else {
                compiler_diagnostics::trace(log_format, "check", stage, template, value);
            }
        };
    let registry_observer = |event| match event {
        TemplateRegistryEvent::CompilationStarted { template } => info!(
            event = "template.compilation.started",
            context = "check",
            template = %template,
            status = "started",
            "compiling template `{template}`"
        ),
        TemplateRegistryEvent::CompilationCompleted {
            template,
            warning_count,
        } => info!(
            event = "template.compilation.completed",
            context = "check",
            template = %template,
            warning_count,
            status = "ok",
            "compiled template `{template}`"
        ),
        TemplateRegistryEvent::CompilationFailed { template } => error!(
            event = "template.compilation.failed",
            context = "check",
            template = %template,
            status = "error",
            "failed to compile template `{template}`"
        ),
    };
    let registry_build_result =
        TemplateRegistry::build_with_observer(&loader, &compiler, &registry_observer).await;

    match registry_build_result {
        Ok(build) => {
            for warning in build.warnings() {
                diagnostic("compile", warning.template.as_ref(), warning);
            }

            let templates = build.value().ids();

            info!(
                event = "templates.ready",
                context = "check",
                template_count = templates.len(),
                status = "ok",
                "templates ready: {}",
                templates.len()
            );
            for template in templates {
                info!(
                    event = "template.ready",
                    context = "check",
                    template = %template,
                    status = "ok",
                    "template ready: `{template}`"
                );
            }

            true
        }
        Err(error) => {
            for warning in error.warnings() {
                diagnostic("compile", warning.template.as_ref(), warning);
            }
            for failure in error.value().failures() {
                match failure {
                    TemplateRegistryError::List { source } => error!(
                        event = "compiler.diagnostic",
                        context = "check",
                        stage = "load",
                        diagnostic_message = %source,
                        "{source}"
                    ),
                    TemplateRegistryError::Load { template, source } => error!(
                        event = "compiler.diagnostic",
                        context = "check",
                        stage = "load",
                        template = %template,
                        diagnostic_message = %source,
                        "template `{template}`: {source}"
                    ),
                    TemplateRegistryError::Prepare { template, source } => {
                        for value in source.report().diagnostics() {
                            diagnostic("prepare", Some(template), value);
                        }
                    }
                    TemplateRegistryError::SamplePayload {
                        template,
                        violations,
                    } => {
                        let message = violations
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join("; ");
                        error!(
                            event = "compiler.diagnostic",
                            context = "check",
                            stage = "prepare",
                            template = %template,
                            diagnostic_message = message,
                            "template `{template}`: {message}"
                        );
                    }
                    TemplateRegistryError::Warmup { template, source } => {
                        for value in source.report().diagnostics() {
                            diagnostic("compile", Some(template), value);
                        }
                    }
                }
            }
            error!(
                event = "templates.check.failed",
                context = "check",
                template_count = error.value().template_count(),
                failed_count = error.value().failures().len(),
                warning_count = error.warnings().len(),
                "template compilation failed"
            );
            false
        }
    }
}
