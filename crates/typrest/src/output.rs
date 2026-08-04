use std::io::IsTerminal;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice as TermColorChoice, StandardStream};
use codespan_reporting::term::{Styles, StylesWriter};
use tracing::{Level, error, event, info, warn};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::util::SubscriberInitExt;
use typrest_core::{
    CompilationDiagnostic, CompilationError, DiagnosticSeverity, TemplateId, TemplateRegistry,
    TemplateRegistryBuildFailure, TemplateRegistryError, WithWarnings,
};

use crate::loader::watch::WatchEvent;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    Pretty,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

#[derive(Clone)]
pub struct DiagnosticReporter {
    format: OutputFormat,
    color: ColorChoice,
    templates_root: Arc<PathBuf>,
    next_report_id: Arc<AtomicU64>,
}

impl DiagnosticReporter {
    pub fn new(
        format: OutputFormat,
        color: ColorChoice,
        templates_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            format,
            color,
            templates_root: Arc::new(templates_root.into()),
            next_report_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn report_build(&self, context: DiagnosticContext, build: &WithWarnings<TemplateRegistry>) {
        self.report_diagnostics(context, DiagnosticStage::Compile, build.warnings());
    }

    pub fn report_build_error(
        &self,
        context: DiagnosticContext,
        error: &WithWarnings<TemplateRegistryBuildFailure>,
    ) {
        self.report_diagnostics(context, DiagnosticStage::Compile, error.warnings());

        for failure in error.value().failures() {
            match failure {
                TemplateRegistryError::List { source } => self.report_simple_error(
                    context,
                    DiagnosticStage::Load,
                    None,
                    &source.to_string(),
                ),
                TemplateRegistryError::Load { template, source } => self.report_simple_error(
                    context,
                    DiagnosticStage::Load,
                    Some(template),
                    &source.to_string(),
                ),
                TemplateRegistryError::Prepare { template, source } => {
                    self.report_compilation(context, DiagnosticStage::Prepare, template, source)
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
                    self.report_simple_error(
                        context,
                        DiagnosticStage::Prepare,
                        Some(template),
                        &message,
                    );
                }
                TemplateRegistryError::Warmup { template, source } => {
                    self.report_compilation(context, DiagnosticStage::Compile, template, source)
                }
            }
        }

        error!(
            event = "templates.check.failed",
            context = context.as_str(),
            template_count = error.value().template_count(),
            failed_count = error.value().failures().len(),
            warning_count = error.warnings().len(),
            "template compilation failed"
        );
    }

    pub(crate) fn report_watch_event(&self, event: WatchEvent) {
        match event {
            WatchEvent::BackendFailed(source) => {
                error!(error = %source, "template watch event failed");
            }
            WatchEvent::RegistryRebuilt {
                template_count,
                warnings,
            } => {
                self.report_diagnostics(
                    DiagnosticContext::Watch,
                    DiagnosticStage::Compile,
                    &warnings,
                );
                info!(
                    event = "template.registry.rebuilt",
                    template_count,
                    warning_count = warnings.len(),
                    "template registry rebuilt"
                );
            }
            WatchEvent::RegistryRebuildFailed(source) => {
                self.report_build_error(DiagnosticContext::Watch, &source);
                warn!(
                    event = "template.registry.rebuild.failed",
                    last_known_good = true,
                    "template registry rebuild failed; keeping last known good version"
                );
            }
            WatchEvent::TemplateReloaded { template, warnings } => {
                self.report_diagnostics(
                    DiagnosticContext::Watch,
                    DiagnosticStage::Compile,
                    &warnings,
                );
                info!(
                    event = "template.reload.completed",
                    template = %template,
                    warning_count = warnings.len(),
                    "template reloaded"
                );
            }
            WatchEvent::TemplateReloadFailed(source) => {
                match &source {
                    TemplateRegistryError::Prepare { template, source } => self.report_compilation(
                        DiagnosticContext::Watch,
                        DiagnosticStage::Prepare,
                        template,
                        source,
                    ),
                    TemplateRegistryError::Warmup { template, source } => self.report_compilation(
                        DiagnosticContext::Watch,
                        DiagnosticStage::Compile,
                        template,
                        source,
                    ),
                    _ => self.report_simple_error(
                        DiagnosticContext::Watch,
                        DiagnosticStage::Load,
                        watch_error_template(&source),
                        &source.to_string(),
                    ),
                }
                warn!(
                    event = "template.reload.failed",
                    template = watch_error_template(&source).map(ToString::to_string),
                    last_known_good = true,
                    "template reload failed; keeping last known good version"
                );
            }
        }
    }

    pub fn report_compilation(
        &self,
        context: DiagnosticContext,
        stage: DiagnosticStage,
        template: &TemplateId,
        error: &CompilationError,
    ) {
        for diagnostic in error.report().diagnostics() {
            self.report_diagnostic(context, stage, Some(template), diagnostic);
        }
    }

    pub fn report_warnings(
        &self,
        context: DiagnosticContext,
        stage: DiagnosticStage,
        template: &TemplateId,
        warnings: &[CompilationDiagnostic],
    ) {
        for diagnostic in warnings {
            self.report_diagnostic(context, stage, Some(template), diagnostic);
        }
    }

    pub fn report_simple_error(
        &self,
        context: DiagnosticContext,
        stage: DiagnosticStage,
        template: Option<&TemplateId>,
        message: &str,
    ) {
        match self.format {
            OutputFormat::Pretty => {
                if let Some(template) = template {
                    eprintln!("error: template `{template}`: {message}");
                } else {
                    eprintln!("error: {message}");
                }
            }
            OutputFormat::Json => {
                if let Some(template) = template {
                    error!(
                        event = "compiler.diagnostic",
                        context = context.as_str(),
                        stage = stage.as_str(),
                        template = %template,
                        message,
                    );
                } else {
                    error!(
                        event = "compiler.diagnostic",
                        context = context.as_str(),
                        stage = stage.as_str(),
                        message,
                    );
                }
            }
        }
    }

    pub fn report_diagnostics(
        &self,
        context: DiagnosticContext,
        stage: DiagnosticStage,
        diagnostics: &[CompilationDiagnostic],
    ) {
        for diagnostic in diagnostics {
            self.report_diagnostic(context, stage, diagnostic.template.as_ref(), diagnostic);
        }
    }

    fn report_diagnostic(
        &self,
        context: DiagnosticContext,
        stage: DiagnosticStage,
        template: Option<&TemplateId>,
        diagnostic: &CompilationDiagnostic,
    ) {
        let report_id = self.next_report_id.fetch_add(1, Ordering::Relaxed);
        match self.format {
            OutputFormat::Pretty => self.report_pretty(template, diagnostic),
            OutputFormat::Json => self.report_json(report_id, context, stage, template, diagnostic),
        }
    }

    fn report_pretty(&self, template: Option<&TemplateId>, diagnostic: &CompilationDiagnostic) {
        let Some(location) = diagnostic.location.as_ref() else {
            let level = match diagnostic.severity {
                DiagnosticSeverity::Error => "error",
                DiagnosticSeverity::Warning => "warning",
            };
            eprintln!("{level}: {}", diagnostic.message);
            for hint in &diagnostic.hints {
                eprintln!("  = help: {}", format_related(hint));
            }
            for trace in &diagnostic.trace {
                eprintln!("  = note: {}", format_related(trace));
            }
            return;
        };
        let Some(source) = location.source() else {
            let level = match diagnostic.severity {
                DiagnosticSeverity::Error => "error",
                DiagnosticSeverity::Warning => "warning",
            };
            eprintln!(
                "{level}: {}\n  --> {}:{}:{}",
                diagnostic.message,
                self.display_path(template, &location.path).display(),
                location.start.line,
                location.start.column
            );
            return;
        };
        let Some(range) = location.byte_range() else {
            return;
        };

        let mut files = SimpleFiles::new();
        let file_id = files.add(
            self.display_path(template, &location.path)
                .display()
                .to_string(),
            source.to_owned(),
        );
        let mut rendered = match diagnostic.severity {
            DiagnosticSeverity::Error => Diagnostic::error(),
            DiagnosticSeverity::Warning => Diagnostic::warning(),
        }
        .with_message(diagnostic.message.clone())
        .with_labels(vec![Label::primary(
            file_id,
            visible_range(range, source.len()),
        )]);

        let mut notes = Vec::new();
        notes.extend(
            diagnostic
                .hints
                .iter()
                .map(|hint| format!("help: {}", format_related(hint))),
        );
        notes.extend(
            diagnostic
                .trace
                .iter()
                .map(|trace| format!("note: {}", format_related(trace))),
        );
        rendered = rendered.with_notes(notes);

        let writer = StandardStream::stderr(self.term_color());
        let config = term::Config::default();
        let styles = Styles::default();
        let lock = writer.lock();
        let mut styled = StylesWriter::new(lock, &styles);
        if let Err(error) = term::emit_to_write_style(&mut styled, &config, &files, &rendered) {
            eprintln!("error: failed to render compiler diagnostic: {error}");
        }
    }

    fn report_json(
        &self,
        report_id: u64,
        context: DiagnosticContext,
        stage: DiagnosticStage,
        template: Option<&TemplateId>,
        diagnostic: &CompilationDiagnostic,
    ) {
        emit_json_diagnostic(report_id, context, stage, template, diagnostic);

        for (index, hint) in diagnostic.hints.iter().enumerate() {
            emit_json_related(
                "compiler.hint",
                report_id,
                index,
                context,
                stage,
                template,
                hint,
            );
        }
        for (index, trace) in diagnostic.trace.iter().enumerate() {
            emit_json_related(
                "compiler.trace",
                report_id,
                index,
                context,
                stage,
                template,
                trace,
            );
        }
    }

    fn display_path(&self, template: Option<&TemplateId>, virtual_path: &str) -> PathBuf {
        if virtual_path.starts_with("_shared/") {
            self.templates_root.join(virtual_path)
        } else if let Some(template) = template {
            template
                .segments()
                .fold(self.templates_root.as_ref().clone(), |path, segment| {
                    path.join(segment)
                })
                .join(virtual_path)
        } else {
            PathBuf::from(virtual_path)
        }
    }

    fn term_color(&self) -> TermColorChoice {
        match self.color {
            ColorChoice::Always => TermColorChoice::Always,
            ColorChoice::Never => TermColorChoice::Never,
            ColorChoice::Auto if std::io::stderr().is_terminal() => TermColorChoice::Auto,
            ColorChoice::Auto => TermColorChoice::Never,
        }
    }
}

fn watch_error_template(error: &TemplateRegistryError) -> Option<&TemplateId> {
    match error {
        TemplateRegistryError::List { .. } => None,
        TemplateRegistryError::Load { template, .. }
        | TemplateRegistryError::Prepare { template, .. }
        | TemplateRegistryError::SamplePayload { template, .. }
        | TemplateRegistryError::Warmup { template, .. } => Some(template),
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DiagnosticContext {
    Check,
    Startup,
    Watch,
    Render,
}

impl DiagnosticContext {
    fn as_str(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::Startup => "startup",
            Self::Watch => "watch",
            Self::Render => "render",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DiagnosticStage {
    Load,
    Prepare,
    Compile,
    Export,
}

impl DiagnosticStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Load => "load",
            Self::Prepare => "prepare",
            Self::Compile => "compile",
            Self::Export => "export",
        }
    }
}

pub fn init_tracing(format: OutputFormat, color: ColorChoice) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("typrest=info"));
    let ansi = match color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => std::io::stderr().is_terminal(),
    };

    let result = match format {
        OutputFormat::Pretty => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .with_target(false)
            .with_ansi(ansi)
            .compact()
            .finish()
            .try_init(),
        OutputFormat::Json => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .with_ansi(false)
            .json()
            .flatten_event(true)
            .finish()
            .try_init(),
    };

    if let Err(error) = result {
        eprintln!("warning: logging subscriber was not installed: {error}");
    }
}

pub fn log_check_success(format: OutputFormat, build: &WithWarnings<TemplateRegistry>) {
    match format {
        OutputFormat::Pretty => eprintln!("templates compiled successfully"),
        OutputFormat::Json => {
            info!(
                target: "typrest",
                event = "templates.check.completed",
                template_count = build.value().ids().len(),
                warning_count = build.warnings().len(),
                status = "ok",
                "templates compiled successfully"
            );
        }
    }
}

fn emit_json_diagnostic(
    report_id: u64,
    context: DiagnosticContext,
    stage: DiagnosticStage,
    template: Option<&TemplateId>,
    diagnostic: &CompilationDiagnostic,
) {
    match diagnostic.severity {
        DiagnosticSeverity::Error => {
            emit_json_primary::<JsonError>(report_id, context, stage, template, diagnostic)
        }
        DiagnosticSeverity::Warning => {
            emit_json_primary::<JsonWarning>(report_id, context, stage, template, diagnostic)
        }
    }
}

trait JsonDiagnosticLevel {
    const LEVEL: Level;
}

struct JsonError;
struct JsonWarning;

impl JsonDiagnosticLevel for JsonError {
    const LEVEL: Level = Level::ERROR;
}

impl JsonDiagnosticLevel for JsonWarning {
    const LEVEL: Level = Level::WARN;
}

fn emit_json_primary<L: JsonDiagnosticLevel>(
    report_id: u64,
    context: DiagnosticContext,
    stage: DiagnosticStage,
    template: Option<&TemplateId>,
    diagnostic: &CompilationDiagnostic,
) {
    if let Some(location) = &diagnostic.location {
        if let Some(template) = template {
            match L::LEVEL {
                Level::ERROR => event!(
                    Level::ERROR,
                    event = "compiler.diagnostic",
                    report_id,
                    context = context.as_str(),
                    stage = stage.as_str(),
                    template = %template,
                    message = %diagnostic.message,
                    path = %location.path,
                    line_start = location.start.line,
                    column_start = location.start.column,
                    line_end = location.end.line,
                    column_end = location.end.column,
                ),
                _ => event!(
                    Level::WARN,
                    event = "compiler.diagnostic",
                    report_id,
                    context = context.as_str(),
                    stage = stage.as_str(),
                    template = %template,
                    message = %diagnostic.message,
                    path = %location.path,
                    line_start = location.start.line,
                    column_start = location.start.column,
                    line_end = location.end.line,
                    column_end = location.end.column,
                ),
            }
        } else {
            match L::LEVEL {
                Level::ERROR => event!(
                    Level::ERROR,
                    event = "compiler.diagnostic",
                    report_id,
                    context = context.as_str(),
                    stage = stage.as_str(),
                    message = %diagnostic.message,
                    path = %location.path,
                    line_start = location.start.line,
                    column_start = location.start.column,
                    line_end = location.end.line,
                    column_end = location.end.column,
                ),
                _ => event!(
                    Level::WARN,
                    event = "compiler.diagnostic",
                    report_id,
                    context = context.as_str(),
                    stage = stage.as_str(),
                    message = %diagnostic.message,
                    path = %location.path,
                    line_start = location.start.line,
                    column_start = location.start.column,
                    line_end = location.end.line,
                    column_end = location.end.column,
                ),
            }
        }
    } else {
        if let Some(template) = template {
            match L::LEVEL {
                Level::ERROR => event!(
                    Level::ERROR,
                    event = "compiler.diagnostic",
                    report_id,
                    context = context.as_str(),
                    stage = stage.as_str(),
                    template = %template,
                    message = %diagnostic.message,
                ),
                _ => event!(
                    Level::WARN,
                    event = "compiler.diagnostic",
                    report_id,
                    context = context.as_str(),
                    stage = stage.as_str(),
                    template = %template,
                    message = %diagnostic.message,
                ),
            }
        } else {
            match L::LEVEL {
                Level::ERROR => event!(
                    Level::ERROR,
                    event = "compiler.diagnostic",
                    report_id,
                    context = context.as_str(),
                    stage = stage.as_str(),
                    message = %diagnostic.message,
                ),
                _ => event!(
                    Level::WARN,
                    event = "compiler.diagnostic",
                    report_id,
                    context = context.as_str(),
                    stage = stage.as_str(),
                    message = %diagnostic.message,
                ),
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_json_related(
    event_name: &'static str,
    report_id: u64,
    related_index: usize,
    context: DiagnosticContext,
    stage: DiagnosticStage,
    template: Option<&TemplateId>,
    related: &typrest_core::RelatedDiagnostic,
) {
    if let Some(location) = &related.location {
        if let Some(template) = template {
            event!(
                Level::INFO,
                event = event_name,
                report_id,
                related_index,
                context = context.as_str(),
                stage = stage.as_str(),
                template = %template,
                message = %related.message,
                path = %location.path,
                line_start = location.start.line,
                column_start = location.start.column,
                line_end = location.end.line,
                column_end = location.end.column,
            );
        } else {
            event!(
                Level::INFO,
                event = event_name,
                report_id,
                related_index,
                context = context.as_str(),
                stage = stage.as_str(),
                message = %related.message,
                path = %location.path,
                line_start = location.start.line,
                column_start = location.start.column,
                line_end = location.end.line,
                column_end = location.end.column,
            );
        }
    } else if let Some(template) = template {
        event!(
            Level::INFO,
            event = event_name,
            report_id,
            related_index,
            context = context.as_str(),
            stage = stage.as_str(),
            template = %template,
            message = %related.message,
        );
    } else {
        event!(
            Level::INFO,
            event = event_name,
            report_id,
            related_index,
            context = context.as_str(),
            stage = stage.as_str(),
            message = %related.message,
        );
    }
}

fn format_related(related: &typrest_core::RelatedDiagnostic) -> String {
    match &related.location {
        Some(location) => format!(
            "{} at {}:{}:{}",
            related.message, location.path, location.start.line, location.start.column
        ),
        None => related.message.clone(),
    }
}

fn visible_range(range: Range<usize>, source_len: usize) -> Range<usize> {
    if range.start == range.end && range.end < source_len {
        range.start..range.end.saturating_add(1)
    } else {
        range
    }
}
