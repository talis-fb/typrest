use std::io::{IsTerminal, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice as TermColorChoice, StandardStream};
use codespan_reporting::term::{Styles, StylesWriter};
use tracing::{Level, error, event, info, warn};
use typrest_core::{CompilationDiagnostic, DiagnosticSeverity, TemplateId};

use crate::args::{ColorChoice, LogFormat};

static NEXT_REPORT_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn write(
    color: ColorChoice,
    templates_root: &Path,
    template: Option<&TemplateId>,
    diagnostic: &CompilationDiagnostic,
) {
    let stream = StandardStream::stdout(term_color(color));
    let level = match diagnostic.severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
    };
    let Some(location) = diagnostic.location.as_ref() else {
        let mut writer = stream.lock();
        let _ = writeln!(writer, "{level}: {}", diagnostic.message);
        write_plain_related(&mut writer, diagnostic);
        return;
    };
    let path = display_path(templates_root, template, &location.path);
    let Some(source) = location.source() else {
        let mut writer = stream.lock();
        let _ = writeln!(
            writer,
            "{level}: {}\n  --> {}:{}:{}",
            diagnostic.message,
            path.display(),
            location.start.line,
            location.start.column
        );
        write_plain_related(&mut writer, diagnostic);
        return;
    };
    let Some(range) = location.byte_range() else {
        return;
    };

    let mut files = SimpleFiles::new();
    let file_id = files.add(path.display().to_string(), source.to_owned());
    let rendered = match diagnostic.severity {
        DiagnosticSeverity::Error => Diagnostic::error(),
        DiagnosticSeverity::Warning => Diagnostic::warning(),
    }
    .with_message(diagnostic.message.clone())
    .with_labels(vec![Label::primary(
        file_id,
        visible_range(range, source.len()),
    )])
    .with_notes(diagnostic_notes(diagnostic));
    let lock = stream.lock();
    let styles = Styles::default();
    let mut writer = StylesWriter::new(lock, &styles);
    if let Err(error) =
        term::emit_to_write_style(&mut writer, &term::Config::default(), &files, &rendered)
    {
        drop(writer);
        let mut writer = stream.lock();
        let _ = writeln!(
            writer,
            "error: failed to render compiler diagnostic: {error}"
        );
    }
}

pub(crate) fn trace(
    format: LogFormat,
    context: &'static str,
    stage: &'static str,
    template: Option<&TemplateId>,
    diagnostic: &CompilationDiagnostic,
) {
    if format == LogFormat::Pretty {
        trace_pretty(context, stage, template, diagnostic);
    } else {
        let report_id = NEXT_REPORT_ID.fetch_add(1, Ordering::Relaxed);
        trace_json(report_id, context, stage, template, diagnostic);
    }
}

fn write_plain_related(writer: &mut impl Write, diagnostic: &CompilationDiagnostic) {
    for hint in &diagnostic.hints {
        let _ = writeln!(writer, "  = help: {}", format_related(hint));
    }
    for trace in &diagnostic.trace {
        let _ = writeln!(writer, "  = note: {}", format_related(trace));
    }
}

fn trace_pretty(
    context: &'static str,
    stage: &'static str,
    template: Option<&TemplateId>,
    diagnostic: &CompilationDiagnostic,
) {
    let message = format_diagnostic_message(template, diagnostic);
    match diagnostic.severity {
        DiagnosticSeverity::Error => error!(
            event = "compiler.diagnostic",
            context,
            stage,
            diagnostic_message = %diagnostic.message,
            "{message}"
        ),
        DiagnosticSeverity::Warning => warn!(
            event = "compiler.diagnostic",
            context,
            stage,
            diagnostic_message = %diagnostic.message,
            "{message}"
        ),
    }
    for hint in &diagnostic.hints {
        info!(
            event = "compiler.hint",
            context,
            stage,
            "help: {}",
            format_related(hint)
        );
    }
    for trace in &diagnostic.trace {
        info!(
            event = "compiler.trace",
            context,
            stage,
            "note: {}",
            format_related(trace)
        );
    }
}

fn trace_json(
    report_id: u64,
    context: &'static str,
    stage: &'static str,
    template: Option<&TemplateId>,
    diagnostic: &CompilationDiagnostic,
) {
    let template = template.map(ToString::to_string);
    let location = diagnostic.location.as_ref();
    match diagnostic.severity {
        DiagnosticSeverity::Error => event!(
            Level::ERROR,
            event = "compiler.diagnostic",
            report_id,
            context,
            stage,
            template,
            message = %diagnostic.message,
            path = location.map(|value| value.path.as_str()),
            line_start = location.map(|value| value.start.line),
            column_start = location.map(|value| value.start.column),
            line_end = location.map(|value| value.end.line),
            column_end = location.map(|value| value.end.column),
        ),
        DiagnosticSeverity::Warning => event!(
            Level::WARN,
            event = "compiler.diagnostic",
            report_id,
            context,
            stage,
            template,
            message = %diagnostic.message,
            path = location.map(|value| value.path.as_str()),
            line_start = location.map(|value| value.start.line),
            column_start = location.map(|value| value.start.column),
            line_end = location.map(|value| value.end.line),
            column_end = location.map(|value| value.end.column),
        ),
    }
    for (related_index, hint) in diagnostic.hints.iter().enumerate() {
        trace_json_related(
            "compiler.hint",
            report_id,
            related_index,
            context,
            stage,
            template.as_deref(),
            hint,
        );
    }
    for (related_index, trace) in diagnostic.trace.iter().enumerate() {
        trace_json_related(
            "compiler.trace",
            report_id,
            related_index,
            context,
            stage,
            template.as_deref(),
            trace,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn trace_json_related(
    event_name: &'static str,
    report_id: u64,
    related_index: usize,
    context: &'static str,
    stage: &'static str,
    template: Option<&str>,
    related: &typrest_core::RelatedDiagnostic,
) {
    let location = related.location.as_ref();
    event!(
        Level::INFO,
        event = event_name,
        report_id,
        related_index,
        context,
        stage,
        template,
        message = %related.message,
        path = location.map(|value| value.path.as_str()),
        line_start = location.map(|value| value.start.line),
        column_start = location.map(|value| value.start.column),
        line_end = location.map(|value| value.end.line),
        column_end = location.map(|value| value.end.column),
    );
}

fn display_path(templates_root: &Path, template: Option<&TemplateId>, path: &str) -> PathBuf {
    if path.starts_with("_shared/") {
        templates_root.join(path)
    } else if let Some(template) = template {
        template
            .segments()
            .fold(templates_root.to_owned(), |path, segment| {
                path.join(segment)
            })
            .join(path)
    } else {
        PathBuf::from(path)
    }
}

fn term_color(color: ColorChoice) -> TermColorChoice {
    match color {
        ColorChoice::Always => TermColorChoice::Always,
        ColorChoice::Never => TermColorChoice::Never,
        ColorChoice::Auto if std::io::stdout().is_terminal() => TermColorChoice::Auto,
        ColorChoice::Auto => TermColorChoice::Never,
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

fn diagnostic_notes(diagnostic: &CompilationDiagnostic) -> Vec<String> {
    diagnostic
        .hints
        .iter()
        .map(|hint| format!("help: {}", format_related(hint)))
        .chain(
            diagnostic
                .trace
                .iter()
                .map(|trace| format!("note: {}", format_related(trace))),
        )
        .collect()
}

fn visible_range(range: Range<usize>, source_len: usize) -> Range<usize> {
    if range.start == range.end && range.end < source_len {
        range.start..range.end.saturating_add(1)
    } else {
        range
    }
}

#[cfg(test)]
mod tests {
    use typrest_core::{CompilationDiagnostic, DiagnosticSeverity, RelatedDiagnostic};

    use super::diagnostic_notes;

    #[test]
    fn pretty_notes_keep_hints_before_traces() {
        let mut diagnostic = CompilationDiagnostic::new(DiagnosticSeverity::Error, "broken");
        diagnostic
            .hints
            .push(RelatedDiagnostic::new("try this", None));
        diagnostic
            .trace
            .push(RelatedDiagnostic::new("while calling helper", None));

        assert_eq!(
            diagnostic_notes(&diagnostic),
            ["help: try this", "note: while calling helper"]
        );
    }
}
