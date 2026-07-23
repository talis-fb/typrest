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

pub(super) fn compile(
    source: &TemplateSource,
    payload: &serde_json::Value,
    fonts: &FontRegistry,
) -> Result<Vec<u8>, CompilationError> {
    let world = TemplateWorld::new(source, payload, fonts)?;
    let result = typst::compile(&world);

    let document = result.output.map_err(|errors| map_errors(errors, &world))?;

    typst_pdf::pdf(&document, &PdfOptions::default()).map_err(map_export_errors)
}

pub(super) struct TemplateWorld {
    main: Source,
    files: BTreeMap<String, Bytes>,
    library: LazyHash<Library>,
    fonts: LoadedFonts,
}

impl TemplateWorld {
    fn new(
        source: &TemplateSource,
        payload: &serde_json::Value,
        fonts: &FontRegistry,
    ) -> Result<Self, CompilationError> {
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

        Ok(Self {
            main,
            files,
            library: LazyHash::new(
                Library::builder()
                    .with_inputs(build_inputs(payload))
                    .build(),
            ),
            fonts: fonts.for_source(source),
        })
    }
}

impl World for TemplateWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.fonts.book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            return Ok(self.main.clone());
        }

        let path = id.vpath().get_without_slash();
        let bytes = self
            .files
            .get(path)
            .ok_or_else(|| FileError::NotFound(PathBuf::from(path)))?;
        let text = std::str::from_utf8(bytes).map_err(|_| FileError::InvalidUtf8)?;

        Ok(Source::new(id, text.to_owned()))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let path = id.vpath().get_without_slash();

        if id == self.main.id() {
            return Ok(Bytes::from_string(self.main.text().to_owned()));
        }

        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| FileError::NotFound(PathBuf::from(path)))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.fonts.get(index).cloned()
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

    use super::{FontRegistry, compile};

    fn source(main: &str) -> TemplateSource {
        TemplateSource::new(BTreeMap::from([(
            "main.typ".to_owned(),
            CoreBytes::from(main.to_owned()),
        )]))
    }

    #[test]
    fn compiles_minimal_template_to_pdf() {
        let fonts = FontRegistry::new();
        let pdf = compile(
            &source(
                r#"
#let data = json(bytes(sys.inputs.data))
Hello
"#,
            ),
            &json!({}),
            &fonts,
        )
        .unwrap();

        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn snapshots_rendered_pdf_text() {
        let fonts = FontRegistry::new();
        let pdf = compile(
            &source(
                r#"
#let data = json(bytes(sys.inputs.data))
Hello #data.name
"#,
            ),
            &json!({ "name": "Ada" }),
            &fonts,
        )
        .unwrap();

        assert!(pdf.starts_with(b"%PDF"));
        assert_snapshot!(normalize_text(&extract_text_from_pdf(&pdf)), @"Hello Ada");
    }

    #[test]
    fn maps_syntax_errors_to_compilation_error() {
        let fonts = FontRegistry::new();
        let error = compile(&source("#let broken ="), &json!({}), &fonts).unwrap_err();

        assert!(matches!(
            error,
            CompilationError::Failed {
                message,
                line: Some(_)
            } if !message.is_empty()
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
