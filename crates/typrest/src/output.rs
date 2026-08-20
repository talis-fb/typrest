use std::cell::Cell;
use std::fmt;
use std::io::{IsTerminal, Write};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice as TermColorChoice, StandardStream};
use codespan_reporting::term::{Styles, StylesWriter};
use tracing::field::{Field, Visit};
use tracing::{Level, error, event, info, warn};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::field::{RecordFields, VisitOutput};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::fmt::format::{FormatFields, Writer};
use tracing_subscriber::util::SubscriberInitExt;
use typrest_core::{
    CompilationDiagnostic, CompilationError, DiagnosticSeverity, TemplateId, TemplateRegistry,
    TemplateRegistryBuildFailure, TemplateRegistryError, TemplateRegistryEvent, WithWarnings,
};

use crate::loader::watch::WatchEvent;
use crate::package::universal_package::{PackageEvent, PackageEventHandler};

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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum OutputDetail {
    Default,
    Verbose,
}

#[derive(Clone)]
pub struct LifecycleReporter {
    format: OutputFormat,
    context: Arc<AtomicU8>,
    max_detail: OutputDetail,
}

impl LifecycleReporter {
    pub fn new(format: OutputFormat, context: DiagnosticContext) -> Self {
        Self {
            format,
            context: Arc::new(AtomicU8::new(context as u8)),
            max_detail: OutputDetail::Default,
        }
    }

    pub fn set_context(&self, context: DiagnosticContext) {
        self.context.store(context as u8, Ordering::Relaxed);
    }

    pub fn package_event_handler(&self) -> PackageEventHandler {
        let reporter = self.clone();
        Arc::new(move |event| reporter.report_package_event(event))
    }

    pub fn report_registry_event(&self, event: TemplateRegistryEvent) {
        let context = self.context();
        match (self.format, event) {
            (OutputFormat::Pretty, TemplateRegistryEvent::CompilationStarted { template }) => {
                if context.is_prebuild() {
                    eprintln!("Compiling template `{template}`");
                } else {
                    info!(
                        event = "template.compilation.started",
                        context = context.as_str(),
                        template = %template,
                        status = "started",
                        "compiling template `{template}`"
                    );
                }
            }
            (
                OutputFormat::Pretty,
                TemplateRegistryEvent::CompilationCompleted { template, .. },
            ) => {
                if context.is_prebuild() {
                    eprintln!(" Compiled template `{template}`");
                } else {
                    info!(
                        event = "template.compilation.completed",
                        context = context.as_str(),
                        template = %template,
                        status = "ok",
                        "compiled template `{template}`"
                    );
                }
            }
            (OutputFormat::Pretty, TemplateRegistryEvent::CompilationFailed { template }) => {
                error!(
                    event = "template.compilation.failed",
                    context = context.as_str(),
                    template = %template,
                    status = "error",
                    "failed to compile template `{template}`"
                );
            }
            (OutputFormat::Json, TemplateRegistryEvent::CompilationStarted { template }) => {
                with_feedback_destination(context, || {
                    info!(
                        event = "template.compilation.started",
                        context = context.as_str(),
                        template = %template,
                        status = "started",
                        "template compilation started"
                    );
                });
            }
            (
                OutputFormat::Json,
                TemplateRegistryEvent::CompilationCompleted {
                    template,
                    warning_count,
                },
            ) => {
                with_feedback_destination(context, || {
                    info!(
                        event = "template.compilation.completed",
                        context = context.as_str(),
                        template = %template,
                        warning_count,
                        status = "ok",
                        "template compilation completed"
                    );
                });
            }
            (OutputFormat::Json, TemplateRegistryEvent::CompilationFailed { template }) => {
                error!(
                    event = "template.compilation.failed",
                    context = context.as_str(),
                    template = %template,
                    status = "error",
                    "template compilation failed"
                );
            }
        }
    }

    pub fn report_ready(&self, registry: &TemplateRegistry) {
        let ids = registry.ids();
        let context = self.context();
        match self.format {
            OutputFormat::Pretty => {
                if context.is_prebuild() {
                    if !ids.is_empty() {
                        eprintln!();
                    }
                    eprintln!("Templates ready: {}", ids.len());
                } else {
                    info!(
                        event = "templates.ready",
                        context = context.as_str(),
                        template_count = ids.len(),
                        status = "ok",
                        "templates ready: {}",
                        ids.len()
                    );
                }
            }
            OutputFormat::Json => {
                with_feedback_destination(context, || {
                    info!(
                        event = "templates.ready",
                        context = context.as_str(),
                        template_count = ids.len(),
                        status = "ok",
                        "templates ready"
                    );
                    for template in ids {
                        info!(
                            event = "template.ready",
                            context = context.as_str(),
                            template = %template,
                            status = "ok",
                            "template ready"
                        );
                    }
                });
            }
        }
    }

    fn report_package_event(&self, event: PackageEvent) {
        let detail = match event {
            PackageEvent::CacheHit { .. } => OutputDetail::Verbose,
            _ => OutputDetail::Default,
        };
        if detail > self.max_detail {
            return;
        }

        let context = self.context();
        match (self.format, event) {
            (OutputFormat::Pretty, PackageEvent::CacheHit { package }) => {
                if context.is_prebuild() {
                    eprintln!("Using cached package `{package}");
                } else {
                    info!(package = %package, "using cached package `{package}`");
                }
            }
            (OutputFormat::Pretty, PackageEvent::InstallationStarted { package }) => {
                if context.is_prebuild() {
                    eprintln!("Installing package `{package}`");
                } else {
                    info!(
                        event = "package.install.started",
                        context = context.as_str(),
                        package = %package,
                        status = "started",
                        "installing package `{package}`"
                    );
                }
            }
            (OutputFormat::Pretty, PackageEvent::InstallationCompleted { package }) => {
                if context.is_prebuild() {
                    eprintln!(" Installed package `{package}`");
                } else {
                    info!(
                        event = "package.install.completed",
                        context = context.as_str(),
                        package = %package,
                        status = "ok",
                        "installed package `{package}`"
                    );
                }
            }
            (OutputFormat::Pretty, PackageEvent::InstallationFailed { package }) => {
                error!(
                    event = "package.install.failed",
                    context = context.as_str(),
                    package = %package,
                    status = "error",
                    "failed to install package `{package}`"
                );
            }
            (OutputFormat::Json, PackageEvent::CacheHit { package }) => {
                with_feedback_destination(context, || {
                    info!(
                        event = "package.cache.hit",
                        context = context.as_str(),
                        package = %package,
                        status = "ok",
                        "package found in cache"
                    );
                });
            }
            (OutputFormat::Json, PackageEvent::InstallationStarted { package }) => {
                with_feedback_destination(context, || {
                    info!(
                        event = "package.install.started",
                        context = context.as_str(),
                        package = %package,
                        status = "started",
                        "package installation started"
                    );
                });
            }
            (OutputFormat::Json, PackageEvent::InstallationCompleted { package }) => {
                with_feedback_destination(context, || {
                    info!(
                        event = "package.install.completed",
                        context = context.as_str(),
                        package = %package,
                        status = "ok",
                        "package installation completed"
                    );
                });
            }
            (OutputFormat::Json, PackageEvent::InstallationFailed { package }) => {
                error!(
                    event = "package.install.failed",
                    context = context.as_str(),
                    package = %package,
                    status = "error",
                    "package installation failed"
                );
            }
        }
    }

    fn context(&self) -> DiagnosticContext {
        DiagnosticContext::from_u8(self.context.load(Ordering::Relaxed))
    }
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
                error!(
                    event = "template.watch.backend.failed",
                    error = %source,
                    "template watch event failed: {source}"
                );
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
                if self.format == OutputFormat::Json {
                    info!(
                        event = "template.registry.rebuilt",
                        template_count,
                        warning_count = warnings.len(),
                        "template registry rebuilt"
                    );
                }
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
                if self.format == OutputFormat::Json {
                    info!(
                        event = "template.reload.completed",
                        template = %template,
                        warning_count = warnings.len(),
                        "template reloaded"
                    );
                }
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
            OutputFormat::Pretty if context.is_prebuild() => {
                if let Some(template) = template {
                    println!("error: template `{template}`: {message}");
                } else {
                    println!("error: {message}");
                }
            }
            OutputFormat::Pretty => match template {
                Some(template) => error!(
                    event = "compiler.diagnostic",
                    context = context.as_str(),
                    stage = stage.as_str(),
                    template = %template,
                    diagnostic_message = message,
                    "template `{template}`: {message}"
                ),
                None => error!(
                    event = "compiler.diagnostic",
                    context = context.as_str(),
                    stage = stage.as_str(),
                    diagnostic_message = message,
                    "{message}"
                ),
            },
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
            OutputFormat::Pretty if context.is_prebuild() => {
                let destination = match diagnostic.severity {
                    DiagnosticSeverity::Error => OutputDestination::Stdout,
                    DiagnosticSeverity::Warning => OutputDestination::Stderr,
                };
                self.report_pretty(template, diagnostic, destination);
            }
            OutputFormat::Pretty => {
                self.report_runtime_pretty(context, stage, template, diagnostic)
            }
            OutputFormat::Json
                if context.is_prebuild() && diagnostic.severity == DiagnosticSeverity::Warning =>
            {
                with_stderr_logs(|| {
                    self.report_json(report_id, context, stage, template, diagnostic);
                });
            }
            OutputFormat::Json => self.report_json(report_id, context, stage, template, diagnostic),
        }
    }

    fn report_pretty(
        &self,
        template: Option<&TemplateId>,
        diagnostic: &CompilationDiagnostic,
        destination: OutputDestination,
    ) {
        let stream = destination.standard_stream(self.term_color(destination));
        let Some(location) = diagnostic.location.as_ref() else {
            let level = match diagnostic.severity {
                DiagnosticSeverity::Error => "error",
                DiagnosticSeverity::Warning => "warning",
            };
            let mut writer = stream.lock();
            let _ = writeln!(writer, "{level}: {}", diagnostic.message);
            for hint in &diagnostic.hints {
                let _ = writeln!(writer, "  = help: {}", format_related(hint));
            }
            for trace in &diagnostic.trace {
                let _ = writeln!(writer, "  = note: {}", format_related(trace));
            }
            return;
        };
        let Some(source) = location.source() else {
            let level = match diagnostic.severity {
                DiagnosticSeverity::Error => "error",
                DiagnosticSeverity::Warning => "warning",
            };
            let mut writer = stream.lock();
            let _ = writeln!(
                writer,
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

        let config = term::Config::default();
        let styles = Styles::default();
        let lock = stream.lock();
        let mut styled = StylesWriter::new(lock, &styles);
        if let Err(error) = term::emit_to_write_style(&mut styled, &config, &files, &rendered) {
            drop(styled);
            let mut writer = stream.lock();
            let _ = writeln!(
                writer,
                "error: failed to render compiler diagnostic: {error}"
            );
        }
    }

    fn report_runtime_pretty(
        &self,
        context: DiagnosticContext,
        stage: DiagnosticStage,
        template: Option<&TemplateId>,
        diagnostic: &CompilationDiagnostic,
    ) {
        let message = format_diagnostic_message(template, diagnostic);
        match diagnostic.severity {
            DiagnosticSeverity::Error => error!(
                event = "compiler.diagnostic",
                context = context.as_str(),
                stage = stage.as_str(),
                diagnostic_message = %diagnostic.message,
                "{message}"
            ),
            DiagnosticSeverity::Warning => warn!(
                event = "compiler.diagnostic",
                context = context.as_str(),
                stage = stage.as_str(),
                diagnostic_message = %diagnostic.message,
                "{message}"
            ),
        }

        for hint in &diagnostic.hints {
            info!(
                event = "compiler.hint",
                context = context.as_str(),
                stage = stage.as_str(),
                "help: {}",
                format_related(hint)
            );
        }
        for trace in &diagnostic.trace {
            info!(
                event = "compiler.trace",
                context = context.as_str(),
                stage = stage.as_str(),
                "note: {}",
                format_related(trace)
            );
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

    fn term_color(&self, destination: OutputDestination) -> TermColorChoice {
        match self.color {
            ColorChoice::Always => TermColorChoice::Always,
            ColorChoice::Never => TermColorChoice::Never,
            ColorChoice::Auto if destination.is_terminal() => TermColorChoice::Auto,
            ColorChoice::Auto => TermColorChoice::Never,
        }
    }
}

#[derive(Clone, Copy)]
enum OutputDestination {
    Stdout,
    Stderr,
}

impl OutputDestination {
    fn standard_stream(self, color: TermColorChoice) -> StandardStream {
        match self {
            Self::Stdout => StandardStream::stdout(color),
            Self::Stderr => StandardStream::stderr(color),
        }
    }

    fn is_terminal(self) -> bool {
        match self {
            Self::Stdout => std::io::stdout().is_terminal(),
            Self::Stderr => std::io::stderr().is_terminal(),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DiagnosticContext {
    Check,
    Startup,
    Watch,
    Render,
}

impl DiagnosticContext {
    fn from_u8(value: u8) -> Self {
        match value {
            value if value == Self::Check as u8 => Self::Check,
            value if value == Self::Startup as u8 => Self::Startup,
            value if value == Self::Watch as u8 => Self::Watch,
            _ => Self::Render,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::Startup => "startup",
            Self::Watch => "watch",
            Self::Render => "render",
        }
    }

    fn is_prebuild(self) -> bool {
        matches!(self, Self::Check | Self::Startup)
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
        ColorChoice::Auto => std::io::stdout().is_terminal(),
    };

    let result = match format {
        OutputFormat::Pretty => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(RoutedMakeWriter)
            .with_target(false)
            .with_ansi(ansi)
            .fmt_fields(MessageOnlyFields)
            .compact()
            .finish()
            .try_init(),
        OutputFormat::Json => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(RoutedMakeWriter)
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

thread_local! {
    static USE_STDERR_FOR_LOG: Cell<bool> = const { Cell::new(false) };
}

fn with_stderr_logs<T>(emit: impl FnOnce() -> T) -> T {
    USE_STDERR_FOR_LOG.with(|destination| {
        let previous = destination.replace(true);
        let _guard = StderrLogGuard { previous };
        emit()
    })
}

struct StderrLogGuard {
    previous: bool,
}

impl Drop for StderrLogGuard {
    fn drop(&mut self) {
        USE_STDERR_FOR_LOG.set(self.previous);
    }
}

fn with_feedback_destination<T>(context: DiagnosticContext, emit: impl FnOnce() -> T) -> T {
    if context.is_prebuild() {
        with_stderr_logs(emit)
    } else {
        emit()
    }
}

#[derive(Clone, Copy)]
struct RoutedMakeWriter;

enum RoutedWriter {
    Stdout(std::io::Stdout),
    Stderr(std::io::Stderr),
}

impl Write for RoutedWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Stdout(writer) => writer.write(buffer),
            Self::Stderr(writer) => writer.write(buffer),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Stdout(writer) => writer.flush(),
            Self::Stderr(writer) => writer.flush(),
        }
    }
}

impl<'writer> MakeWriter<'writer> for RoutedMakeWriter {
    type Writer = RoutedWriter;

    fn make_writer(&'writer self) -> Self::Writer {
        if USE_STDERR_FOR_LOG.get() {
            RoutedWriter::Stderr(std::io::stderr())
        } else {
            RoutedWriter::Stdout(std::io::stdout())
        }
    }
}

#[derive(Clone, Copy)]
struct MessageOnlyFields;

impl<'writer> FormatFields<'writer> for MessageOnlyFields {
    fn format_fields<R: RecordFields>(&self, writer: Writer<'writer>, fields: R) -> fmt::Result {
        let mut visitor = MessageVisitor {
            writer,
            result: Ok(()),
        };
        fields.record(&mut visitor);
        visitor.finish()
    }
}

struct MessageVisitor<'writer> {
    writer: Writer<'writer>,
    result: fmt::Result,
}

impl Visit for MessageVisitor<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" && self.result.is_ok() {
            self.result = write!(self.writer, "{value:?}");
        }
    }
}

impl VisitOutput<fmt::Result> for MessageVisitor<'_> {
    fn finish(self) -> fmt::Result {
        self.result
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

fn format_diagnostic_message(
    template: Option<&TemplateId>,
    diagnostic: &CompilationDiagnostic,
) -> String {
    let subject = template
        .map(|template| format!("template `{template}`: "))
        .unwrap_or_default();
    match diagnostic.location.as_ref() {
        Some(location) => format!(
            "{subject}{} at {}:{}:{}",
            diagnostic.message, location.path, location.start.line, location.start.column
        ),
        None => format!("{subject}{}", diagnostic.message),
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
