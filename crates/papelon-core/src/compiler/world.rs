use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use typst::diag::{FileError, FileResult, Severity, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Dict, Duration, IntoValue, Str};
use typst::syntax::{DiagSpan, FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World, WorldExt};
use typst_pdf::PdfOptions;

use crate::compiler::prepared_template::PreparedTemplateInner;
use crate::{
    CompilationDiagnostic, CompilationError, CompilationReport, DiagnosticLocation,
    DiagnosticPosition, DiagnosticSeverity, RelatedDiagnostic, TemplateSource,
};

use super::fonts::FontRegistry;

const MAIN_TYP: &str = "main.typ";

pub(super) fn prepare(
    source: &TemplateSource,
    fonts: &FontRegistry,
) -> Result<PreparedTemplateInner, CompilationError> {
    let files = source
        .files()
        .iter()
        .map(|(path, bytes)| (path.clone(), Bytes::new(bytes.to_vec())))
        .collect::<BTreeMap<_, _>>();

    let main_bytes = files
        .get(MAIN_TYP)
        .ok_or_else(|| failed("main.typ is missing from template source"))?;
    let main_text =
        std::str::from_utf8(main_bytes).map_err(|_| failed("main.typ must be valid UTF-8"))?;
    let main = Source::new(file_id(MAIN_TYP), main_text.to_owned());

    Ok(PreparedTemplateInner {
        main,
        files,
        fonts: fonts.for_source(source),
    })
}

pub(super) fn compile_prepared(
    template: &PreparedTemplateInner,
    payload: &serde_json::Value,
) -> Result<CompiledPdf, CompilationError> {
    let world = TemplateWorld::new(template, payload);
    let result = typst::compile(&world);

    let warnings = map_diagnostics(result.warnings, &world);
    let document = match result.output {
        Ok(document) => document,
        Err(errors) => {
            let mut diagnostics = map_diagnostics(errors, &world);
            diagnostics.extend(warnings);
            return Err(CompilationError::Failed(CompilationReport::new(
                diagnostics,
            )));
        }
    };

    match typst_pdf::pdf(&document, &PdfOptions::default()) {
        Ok(bytes) => Ok(CompiledPdf { bytes, warnings }),
        Err(errors) => {
            let mut diagnostics = map_diagnostics(errors, &world);
            diagnostics.extend(warnings);
            Err(CompilationError::ExportFailed(CompilationReport::new(
                diagnostics,
            )))
        }
    }
}

#[derive(Debug)]
pub(super) struct CompiledPdf {
    pub(super) bytes: Vec<u8>,
    pub(super) warnings: Vec<CompilationDiagnostic>,
}

struct TemplateWorld<'a> {
    template: &'a PreparedTemplateInner,
    library: LazyHash<Library>,
}

impl<'a> TemplateWorld<'a> {
    fn new(template: &'a PreparedTemplateInner, payload: &serde_json::Value) -> Self {
        Self {
            template,
            library: LazyHash::new(
                Library::builder()
                    .with_inputs(build_inputs(payload))
                    .build(),
            ),
        }
    }
}

impl World for TemplateWorld<'_> {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.template.fonts.book
    }

    fn main(&self) -> FileId {
        self.template.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.template.main.id() {
            return Ok(self.template.main.clone());
        }

        let path = id.vpath().get_without_slash();
        let bytes = self
            .template
            .files
            .get(path)
            .ok_or_else(|| FileError::NotFound(PathBuf::from(path)))?;
        let text = std::str::from_utf8(bytes).map_err(|_| FileError::InvalidUtf8)?;

        Ok(Source::new(id, text.to_owned()))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let path = id.vpath().get_without_slash();

        if id == self.template.main.id() {
            return Ok(Bytes::from_string(self.template.main.text().to_owned()));
        }

        self.template
            .files
            .get(path)
            .cloned()
            .ok_or_else(|| FileError::NotFound(PathBuf::from(path)))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.template.fonts.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

fn file_id(path: &str) -> FileId {
    let vpath = VirtualPath::new(format!("/{path}")).expect("template paths are virtual paths");
    FileId::new(RootedPath::new(VirtualRoot::Project, vpath))
}

fn build_inputs(payload: &serde_json::Value) -> Dict {
    let mut inputs = Dict::new();
    let data = serde_json::to_string(payload).expect("serde_json::Value should serialize");
    inputs.insert("data".into(), Str::from(data).into_value());
    inputs
}

fn map_diagnostics(
    errors: typst::diag::EcoVec<SourceDiagnostic>,
    world: &TemplateWorld,
) -> Vec<CompilationDiagnostic> {
    errors
        .into_iter()
        .map(|error| map_diagnostic(error, world))
        .collect()
}

fn map_diagnostic(error: SourceDiagnostic, world: &TemplateWorld) -> CompilationDiagnostic {
    CompilationDiagnostic {
        template: None,
        severity: match error.severity {
            Severity::Error => DiagnosticSeverity::Error,
            Severity::Warning => DiagnosticSeverity::Warning,
        },
        message: error.message.to_string(),
        location: map_location(error.span, world),
        hints: error
            .hints
            .into_iter()
            .map(|hint| RelatedDiagnostic::new(hint.v.to_string(), map_location(hint.span, world)))
            .collect(),
        trace: error
            .trace
            .into_iter()
            .map(|trace| {
                RelatedDiagnostic::new(trace.v.to_string(), map_location(trace.span.into(), world))
            })
            .collect(),
    }
}

fn map_location(span: DiagSpan, world: &TemplateWorld) -> Option<DiagnosticLocation> {
    let id = span.id()?;
    let range = world.range(span)?;
    let source = world.source(id).ok()?;
    let (start_line, start_column) = source.lines().byte_to_line_column(range.start)?;
    let (end_line, end_column) = source.lines().byte_to_line_column(range.end)?;

    Some(DiagnosticLocation::with_source(
        id.vpath().get_without_slash(),
        position(start_line, start_column)?,
        position(end_line, end_column)?,
        Arc::from(source.text()),
        range,
    ))
}

fn position(line: usize, column: usize) -> Option<DiagnosticPosition> {
    Some(DiagnosticPosition::new(
        u32::try_from(line).ok()?.checked_add(1)?,
        u32::try_from(column).ok()?.checked_add(1)?,
    ))
}

fn failed(message: impl Into<String>) -> CompilationError {
    CompilationError::Failed(CompilationReport::new(vec![CompilationDiagnostic::new(
        DiagnosticSeverity::Error,
        message,
    )]))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bytes::Bytes as CoreBytes;
    use insta::assert_snapshot;
    use serde_json::json;

    use typst::diag::{SourceDiagnostic, Tracepoint};

    use crate::{CompilationError, DiagnosticSeverity, TemplateSource};

    use super::{
        FontRegistry, TemplateWorld, compile_prepared, map_diagnostic, map_diagnostics, prepare,
    };

    fn source(main: &str) -> TemplateSource {
        TemplateSource::new(BTreeMap::from([(
            "main.typ".to_owned(),
            CoreBytes::from(main.to_owned()),
        )]))
    }

    #[test]
    fn compiles_minimal_template_to_pdf() {
        let fonts = FontRegistry::new();
        let template = prepare(
            &source(
                r#"
#let data = json(bytes(sys.inputs.data))
Hello
"#,
            ),
            &fonts,
        )
        .unwrap();
        let pdf = compile_prepared(&template, &json!({})).unwrap();

        assert!(pdf.bytes.starts_with(b"%PDF"));
        assert!(pdf.warnings.is_empty());
    }

    #[test]
    fn snapshots_rendered_pdf_text() {
        let fonts = FontRegistry::new();
        let template = prepare(
            &source(
                r#"
#let data = json(bytes(sys.inputs.data))
Hello #data.name
"#,
            ),
            &fonts,
        )
        .unwrap();
        let pdf = compile_prepared(&template, &json!({ "name": "Ada" })).unwrap();

        assert!(pdf.bytes.starts_with(b"%PDF"));
        assert_snapshot!(normalize_text(&extract_text_from_pdf(&pdf.bytes)), @"Hello Ada");
    }

    #[test]
    fn maps_syntax_errors_to_compilation_error() {
        let fonts = FontRegistry::new();
        let template = prepare(&source("#let broken ="), &fonts).unwrap();
        let error = compile_prepared(&template, &json!({})).unwrap_err();

        assert!(matches!(error, CompilationError::Failed(_)));
        assert!(!error.primary_message().is_empty());
        assert!(error.primary_line().is_some());
        assert_eq!(error.report().diagnostics().len(), 1);
    }

    #[test]
    fn preserves_all_typst_errors() {
        let fonts = FontRegistry::new();
        let template = prepare(&source("#first\n#second"), &fonts).unwrap();
        let world = TemplateWorld::new(&template, &json!({}));
        let span = template.main.root().span();
        let errors = [
            SourceDiagnostic::error(span, "first"),
            SourceDiagnostic::error(span, "second"),
        ]
        .into_iter()
        .collect();
        let diagnostics = map_diagnostics(errors, &world);

        assert_eq!(diagnostics.len(), 2);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.location.is_some())
        );
    }

    #[test]
    fn captures_successful_compilation_warnings() {
        let fonts = FontRegistry::new();
        let template = prepare(&source("<orphan>"), &fonts).unwrap();
        let compiled = compile_prepared(&template, &json!({})).unwrap();

        assert_eq!(compiled.warnings.len(), 1);
        assert_eq!(compiled.warnings[0].severity, DiagnosticSeverity::Warning);
        assert!(compiled.warnings[0].message.contains("not attached"));
    }

    #[test]
    fn maps_hints_traces_ranges_and_source_text() {
        let fonts = FontRegistry::new();
        let template = prepare(&source("#let value = 1"), &fonts).unwrap();
        let world = TemplateWorld::new(&template, &json!({}));
        let span = template.main.root().span();
        let diagnostic = SourceDiagnostic::error(span, "broken")
            .with_hint("try something else")
            .with_tracepoint(Tracepoint::Call(Some("helper".into())), span);

        let mapped = map_diagnostic(diagnostic, &world);
        let location = mapped.location.unwrap();

        assert_eq!(location.path, "main.typ");
        assert_eq!(location.start.line, 1);
        assert_eq!(location.start.column, 1);
        assert_eq!(location.source(), Some("#let value = 1"));
        assert!(location.byte_range().is_some());
        assert_eq!(mapped.hints[0].message, "try something else");
        assert!(mapped.hints[0].location.is_none());
        assert!(mapped.trace[0].message.contains("helper"));
        assert!(mapped.trace[0].location.is_some());
    }

    #[test]
    fn maps_missing_imports_to_compilation_error() {
        let fonts = FontRegistry::new();
        let template = prepare(&source(r#"#import "missing.typ""#), &fonts).unwrap();
        let error = compile_prepared(&template, &json!({})).unwrap_err();

        assert!(matches!(error, CompilationError::Failed(_)));
        assert_eq!(error.primary_line(), Some(1));
        assert!(error.primary_message().contains("file not found"));
    }

    #[test]
    fn prepared_template_uses_dynamic_payloads() {
        let fonts = FontRegistry::new();
        let template = prepare(
            &source(
                r#"
#let data = json(bytes(sys.inputs.data))
Hello #data.name
"#,
            ),
            &fonts,
        )
        .unwrap();

        let ada = compile_prepared(&template, &json!({ "name": "Ada" })).unwrap();
        let grace = compile_prepared(&template, &json!({ "name": "Grace" })).unwrap();

        assert_snapshot!(normalize_text(&extract_text_from_pdf(&ada.bytes)), @"Hello Ada");
        assert_snapshot!(normalize_text(&extract_text_from_pdf(&grace.bytes)), @"Hello Grace");
    }

    #[test]
    fn prepare_rejects_missing_main() {
        let fonts = FontRegistry::new();
        let error = match prepare(&TemplateSource::default(), &fonts) {
            Ok(_) => panic!("missing main.typ should fail"),
            Err(error) => error,
        };

        assert!(matches!(error, CompilationError::Failed(_)));
        assert!(error.primary_message().contains("main.typ"));
        assert_eq!(error.primary_line(), None);
    }

    fn extract_text_from_pdf(bytes: &[u8]) -> String {
        pdf_extract::extract_text_from_mem(bytes).unwrap()
    }

    fn normalize_text(text: &str) -> String {
        text.lines()
            .map(str::trim_end)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}
