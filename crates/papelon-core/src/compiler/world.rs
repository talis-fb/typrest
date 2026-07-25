use std::collections::BTreeMap;
use std::path::PathBuf;

use typst::diag::{FileError, FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Dict, Duration, IntoValue, Str};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World, WorldExt};
use typst_pdf::PdfOptions;

use crate::{CompilationError, TemplateSource};

use super::fonts::{FontRegistry, LoadedFonts};

const MAIN_TYP: &str = "main.typ";

pub(super) fn prepare(
    source: &TemplateSource,
    fonts: &FontRegistry,
) -> Result<PreparedTemplateState, CompilationError> {
    let files = source
        .files()
        .iter()
        .map(|(path, bytes)| (path.clone(), Bytes::new(bytes.to_vec())))
        .collect::<BTreeMap<_, _>>();

    let main_bytes = files
        .get(MAIN_TYP)
        .ok_or_else(|| CompilationError::Failed {
            message: "main.typ is missing from template source".to_owned(),
            line: None,
        })?;
    let main_text = std::str::from_utf8(main_bytes).map_err(|_| CompilationError::Failed {
        message: "main.typ must be valid UTF-8".to_owned(),
        line: None,
    })?;
    let main = Source::new(file_id(MAIN_TYP), main_text.to_owned());

    Ok(PreparedTemplateState {
        main,
        files,
        fonts: fonts.for_source(source),
    })
}

pub(super) fn compile_prepared(
    template: &PreparedTemplateState,
    payload: &serde_json::Value,
) -> Result<Vec<u8>, CompilationError> {
    let world = TemplateWorld::new(template, payload);
    let result = typst::compile(&world);

    let document = result.output.map_err(|errors| map_errors(errors, &world))?;

    typst_pdf::pdf(&document, &PdfOptions::default()).map_err(map_export_errors)
}

pub(super) struct PreparedTemplateState {
    main: Source,
    files: BTreeMap<String, Bytes>,
    fonts: LoadedFonts,
}

struct TemplateWorld<'a> {
    template: &'a PreparedTemplateState,
    library: LazyHash<Library>,
}

impl<'a> TemplateWorld<'a> {
    fn new(template: &'a PreparedTemplateState, payload: &serde_json::Value) -> Self {
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

fn map_errors(
    errors: typst::diag::EcoVec<SourceDiagnostic>,
    world: &TemplateWorld,
) -> CompilationError {
    let Some(error) = errors.into_iter().next() else {
        return CompilationError::Failed {
            message: "unknown Typst compilation error".to_owned(),
            line: None,
        };
    };

    let line = world
        .range(error.span)
        .and_then(|range| {
            world
                .source(error.span.id()?)
                .ok()?
                .lines()
                .byte_to_line(range.start)
        })
        .map(|line| line + 1);

    CompilationError::Failed {
        message: error.message.to_string(),
        line: line.and_then(|line| u32::try_from(line).ok()),
    }
}

fn map_export_errors(errors: typst::diag::EcoVec<SourceDiagnostic>) -> CompilationError {
    CompilationError::ExportFailed(
        errors
            .into_iter()
            .map(|error| error.message.to_string())
            .collect::<Vec<_>>()
            .join("; "),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bytes::Bytes as CoreBytes;
    use insta::assert_snapshot;
    use serde_json::json;

    use crate::{CompilationError, TemplateSource};

    use super::{FontRegistry, compile_prepared, prepare};

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

        assert!(pdf.starts_with(b"%PDF"));
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

        assert!(pdf.starts_with(b"%PDF"));
        assert_snapshot!(normalize_text(&extract_text_from_pdf(&pdf)), @"Hello Ada");
    }

    #[test]
    fn maps_syntax_errors_to_compilation_error() {
        let fonts = FontRegistry::new();
        let template = prepare(&source("#let broken ="), &fonts).unwrap();
        let error = compile_prepared(&template, &json!({})).unwrap_err();

        assert!(matches!(
            error,
            CompilationError::Failed {
                message,
                line: Some(_)
            } if !message.is_empty()
        ));
    }

    #[test]
    fn maps_missing_imports_to_compilation_error() {
        let fonts = FontRegistry::new();
        let template = prepare(&source(r#"#import "missing.typ""#), &fonts).unwrap();
        let error = compile_prepared(&template, &json!({})).unwrap_err();

        assert!(matches!(
            error,
            CompilationError::Failed {
                message,
                line: Some(1)
            } if message.contains("file not found")
        ));
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

        assert_snapshot!(normalize_text(&extract_text_from_pdf(&ada)), @"Hello Ada");
        assert_snapshot!(normalize_text(&extract_text_from_pdf(&grace)), @"Hello Grace");
    }

    #[test]
    fn prepare_rejects_missing_main() {
        let fonts = FontRegistry::new();
        let error = match prepare(&TemplateSource::default(), &fonts) {
            Ok(_) => panic!("missing main.typ should fail"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            CompilationError::Failed { message, line: None }
                if message.contains("main.typ")
        ));
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
