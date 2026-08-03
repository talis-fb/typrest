# Typst Integration Reference

Curated reference for integrating with the Typst compiler through the Rust crate.
This file exists to give LLMs context about the Typst API; do not clone the entire repository.

Main source: https://typst.app/blog/2025/automated-generation/
Example repository: https://github.com/tfachmann/typst-as-library
Example repository 2: https://github.com/Relacibo/typst-as-lib

---

## How Typst Receives External Data

The official mechanism for injecting data into a Typst template is `sys.inputs`.
In the CLI this would be: `typst compile --input data='{"name":"John"}' main.typ`
Through the Rust crate, the same thing is done by passing a `HashMap<String, String>` of inputs.

In the template, the data is read like this:

```typst
#let data = json(bytes(sys.inputs.data))
```

`sys.inputs.data` is a string. `bytes()` converts it to bytes. `json()` parses the JSON.
This is the canonical pattern: **every template in this project must start with this line**.

---

## Dependencies in Cargo.toml

```toml
[dependencies]
typst = "0.13"        # or the latest version; check crates.io
typst-pdf = "0.13"    # PDF exporter; version must match typst
```

Current stable version: check https://crates.io/crates/typst

---

## The `World` Trait - Central Concept

The `typst` crate does not read files from the filesystem by itself.
The caller implements the `World` trait, which answers compiler questions:
- "what is the content of file X?"
- "what is font Y?"
- "what is the current date/time?"

This is the main extension point. For this project, the `World` receives
a `HashMap<PathBuf, Bytes>` with all template files already in memory
(coming from `TemplateSource`).

### Minimal `World` Trait Interface

```rust
use typst::World;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::foundations::Bytes;
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::Library;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct TyprestWorld {
    /// Root document source (main.typ)
    main: Source,
    /// All template files: path -> bytes
    files: HashMap<PathBuf, Bytes>,
    /// Typst standard library (built-in functions)
    library: LazyHash<Library>,
    /// Available font index
    book: LazyHash<FontBook>,
    /// Loaded fonts
    fonts: Vec<Font>,
}

impl World for TyprestWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        // returns the file source by FileId
        // main.typ is the primary case
        // imports from other .typ files also pass through here
        todo!()
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        // returns bytes for non-typ files (images, fonts, etc.)
        todo!()
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, offset: Option<i64>) -> Option<typst::foundations::Datetime> {
        // returning None is acceptable; templates using `datetime.today()` will fail
        // for the MVP, return the current system date
        None
    }
}
```

---

## Compilation and PDF Export

```rust
use typst::compile;
use typst_pdf::{pdf, PdfOptions};

// 1. Create the World with the template files
let world = TyprestWorld::new(files, input_json);

// 2. Compile: returns Result with the document or errors
let result = compile(&world);

match result.output {
    Ok(document) => {
        // 3. Export to PDF
        let options = PdfOptions::default();
        let pdf_bytes = pdf(&document, &options)?;
        // pdf_bytes: Vec<u8>, the PDF bytes
    }
    Err(errors) => {
        // errors: Vec<SourceDiagnostic>
        // each error has: message, span (location in .typ), severity
        for error in &errors {
            eprintln!("Typst error: {}", error.message);
        }
    }
}
```

**Note:** `compile()` returns a `SourceResult` that has both `output` (it may be Ok even with warnings)
and `warnings`. Always check `output`; warnings do not prevent PDF generation.

---

## Passing Inputs (sys.inputs) Through the Crate

Inputs are passed as part of the `Library` when building the World:

```rust
use typst::Library;
use typst::foundations::Dict;

// Build inputs as a Typst Dict
let mut inputs = Dict::new();
inputs.insert(
    "data".into(),
    // the JSON payload serialized as a string
    serde_json::to_string(&payload).unwrap().into()
);

// Create the library with the inputs
let library = Library::builder()
    .with_inputs(inputs)
    .build();
```

The template then accesses it through `sys.inputs.data`.

---

## Loading Fonts

Typst needs fonts to compile. Without fonts, text does not render.

```rust
use typst::text::{Font, FontBook};

// Option A: Embed default fonts (Linux Libertine, etc.)
// The `typst-assets` crate provides Typst's default fonts
// [dependencies]
// typst-assets = { version = "0.13", features = ["fonts"] }

use typst_assets::fonts;

fn load_default_fonts() -> (FontBook, Vec<Font>) {
    let mut book = FontBook::new();
    let mut fonts = Vec::new();
    
    for data in fonts() {
        let buffer = Bytes::from_static(data);
        for font in Font::iter(buffer) {
            book.push(font.info().clone());
            fonts.push(font);
        }
    }
    
    (book, fonts)
}

// Option B: Load custom fonts from the template (assets/)
// Font bytes come from template blobs; they are already in memory
fn load_font_from_bytes(data: Bytes, book: &mut FontBook, fonts: &mut Vec<Font>) {
    for font in Font::iter(data) {
        book.push(font.info().clone());
        fonts.push(font);
    }
}
```

**Recommendation for this project:**
- Always include Typst's default fonts (`typst-assets`)
- Custom template fonts (in `assets/`) are loaded from the `TemplateSource` blobs

---

## Resolving Imports Between Files

When a template has `#import "partials/header.typ"`, Typst calls `World::source()`
with the imported file's `FileId`. We need to map that FileId to the correct content.

```rust
use typst::syntax::{FileId, VirtualPath, Source};
use typst::diag::{FileError, FileResult};

impl World for TyprestWorld {
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            return Ok(self.main.clone());
        }
        
        // Extract the FileId path and look it up in the template files
        let path = id.vpath().as_rootless_path();
        
        match self.files.get(path) {
            Some(bytes) => {
                let text = std::str::from_utf8(bytes)
                    .map_err(|_| FileError::InvalidUtf8)?;
                Ok(Source::new(id, text.to_string()))
            }
            None => Err(FileError::NotFound(path.to_path_buf())),
        }
    }
    
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let path = id.vpath().as_rootless_path();
        self.files.get(path)
            .cloned()
            .ok_or_else(|| FileError::NotFound(path.to_path_buf()))
    }
}
```

---

## Creating the FileId for main.typ

```rust
use typst::syntax::{FileId, VirtualPath};

// main.typ is always the root: virtual path "/"
let main_id = FileId::new(None, VirtualPath::new("/main.typ"));

// Read main.typ content from the template files
let main_content = files
    .get(Path::new("main.typ"))
    .expect("main.typ must exist in the template");

let main_text = std::str::from_utf8(main_content).expect("main.typ must be UTF-8");
let main_source = Source::new(main_id, main_text.to_string());
```

---

## Extracting Compilation Errors Into Useful Messages

All access to `SourceDiagnostic` belongs in `compiler/world.rs`. The adapter must preserve:

- every error and warning, not only the first;
- the primary virtual path and full line/column range;
- detached and source-linked hints;
- call, import, include, and show trace frames;
- a location-less fallback when Typst cannot resolve a span.

Convert those values immediately into the Typrest-owned types documented in
`docs/diagnostics.md`. Never expose `SourceDiagnostic`, `DiagSpan`, or `Tracepoint` through a
public signature.

---

## Complete Minimal Example - Functional World

This is the example from the `typst-as-library` repository adapted to the project context:

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use typst::foundations::{Bytes, Dict};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};
use typst::diag::{FileError, FileResult};

pub struct TemplateWorld {
    main: Source,
    files: HashMap<PathBuf, Bytes>,
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
}

impl TemplateWorld {
    pub fn new(
        files: HashMap<PathBuf, Bytes>,
        input_json: &str,
    ) -> Self {
        // Inputs
        let mut inputs = Dict::new();
        inputs.insert("data".into(), input_json.into());
        
        let library = LazyHash::new(
            Library::builder().with_inputs(inputs).build()
        );
        
        // Fonts
        let mut book = FontBook::new();
        let mut fonts = Vec::new();
        for data in typst_assets::fonts() {
            let buffer = Bytes::from_static(data);
            for font in Font::iter(buffer) {
                book.push(font.info().clone());
                fonts.push(font);
            }
        }
        
        // main.typ
        let main_id = FileId::new(None, VirtualPath::new("/main.typ"));
        let main_bytes = files.get(Path::new("main.typ")).expect("main.typ required");
        let main_text = std::str::from_utf8(main_bytes).expect("main.typ must be UTF-8");
        let main = Source::new(main_id, main_text.to_string());
        
        Self {
            main,
            files,
            library,
            book: LazyHash::new(book),
            fonts,
        }
    }
}

impl World for TemplateWorld {
    fn library(&self) -> &LazyHash<Library> { &self.library }
    fn book(&self) -> &LazyHash<FontBook> { &self.book }
    fn main(&self) -> FileId { self.main.id() }
    
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() { return Ok(self.main.clone()); }
        let path = id.vpath().as_rootless_path();
        let bytes = self.files.get(path).ok_or_else(|| FileError::NotFound(path.into()))?;
        let text = std::str::from_utf8(bytes).map_err(|_| FileError::InvalidUtf8)?;
        Ok(Source::new(id, text.to_string()))
    }
    
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let path = id.vpath().as_rootless_path();
        self.files.get(path).cloned().ok_or_else(|| FileError::NotFound(path.into()))
    }
    
    fn font(&self, index: usize) -> Option<Font> { self.fonts.get(index).cloned() }
    
    fn today(&self, _offset: Option<i64>) -> Option<typst::foundations::Datetime> { None }
}

// Usage:
pub fn compile_template(
    files: HashMap<PathBuf, Bytes>,
    payload: &serde_json::Value,
) -> Result<Vec<u8>, String> {
    let input_json = serde_json::to_string(payload).unwrap();
    let world = TemplateWorld::new(files, &input_json);
    
    let result = typst::compile(&world);
    
    match result.output {
        Ok(document) => {
            let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
                .map_err(|e| format!("PDF export error: {:?}", e))?;
            Ok(pdf)
        }
        Err(errors) => {
            let messages: Vec<String> = errors.iter()
                .map(|e| e.message.to_string())
                .collect();
            Err(messages.join("; "))
        }
    }
}
```

---

## Additional Relevant Crates

```toml
[dependencies]
typst = "0.13"
typst-pdf = "0.13"
typst-assets = { version = "0.13", features = ["fonts"] }  # embedded default fonts
```

- `typst-assets`: provides default fonts (Linux Libertine, New Computer Modern, etc.)
  used by Typst when no font is specified in the template.
  **Without this, templates without explicit `#set text(font: "...")` do not compile.**

---

## Known Pitfalls

**Unstable API across versions:** The `typst` crate does not guarantee semantic stability.
There may be breaking changes between minor versions. Pin the exact version in `Cargo.toml`.

**`PdfOptions` changed between versions:** In older versions it was `pdf(&document, None, None)`.
In newer versions it is `pdf(&document, &PdfOptions::default())`. Check the signature
in the version being used.

**Thread-safe compilation:** The `World` must implement `Send + Sync` for use in async
servers. Use `Arc<RwLock<>>` if there is mutable state.

**No automatic filesystem access:** Typst through the crate does not read anything from disk by itself.
Every file the template needs (images, fonts, imports) must be in the `HashMap` passed
to the World. This property guarantees isolation between compilations.

**Typst Universe packages:** Templates that use `#import "@preview/..."` require
the package contents to be available through `World::file()`. For the MVP, do not support
external packages: return `FileError::NotFound` and document the limitation.
