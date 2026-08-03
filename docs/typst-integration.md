# Typst Integration Reference

> Status: Current implementation reference for the private compiler anti-corruption layer.

## Boundary

The Typst Rust API can change between releases. All imports of `typst::*` and `typst_pdf::*` are
therefore confined to `crates/typrest-core/src/compiler/`. Outside that directory, code uses only
Typrest-owned types such as `PdfCompiler`, `PreparedTemplate`, `Document`, and
`CompilationDiagnostic`.

The workspace `Cargo.toml` is the canonical dependency source. `typst`, `typst-pdf`,
`typst-assets`, `typst-kit`, and `typst-syntax` are pinned exactly to one matching version and
upgraded together.

## Data Injection

The compiler serializes the request `serde_json::Value` and places it in the Typst library inputs
under `data`. Templates that consume request data parse the string with:

```typst
#let data = json(bytes(sys.inputs.data))
```

No temporary JSON file is written. A fresh Typst `Library` is built for each render because inputs
are request-specific.

## Prepared State and Request State

`PdfCompiler::prepare()` converts `TemplateSource` into reusable state:

- `main.typ` as a Typst `Source`;
- all virtual files as Typst bytes;
- default fonts from `typst-assets`;
- custom `.ttf` and `.otf` fonts found under template `assets/`;
- stable virtual paths required for imports and diagnostics.
- the complete direct and transitive `@preview` package source closure.

`PdfCompiler::compile_prepared()` borrows that state, builds request inputs, calls
`typst::compile`, maps warnings and errors, and exports with `typst-pdf`. It returns `Document` with
PDF bytes and non-fatal warnings.

## World Responsibilities

The private `TemplateWorld` implements Typst's `World` interface:

- `main()` returns the `main.typ` virtual file identifier;
- `source()` returns UTF-8 Typst source from the in-memory file map;
- `file()` returns non-source bytes such as images and fonts;
- project and package virtual roots use separate prepared source maps;
- `book()` and `font()` expose the prepared font registry;
- `library()` exposes the request-specific standard library and inputs;
- `today()` currently returns no date.

Typst performs no automatic filesystem access during compilation. Every project and package source,
image, font, and imported file is already present in prepared state.

## Virtual Files and Imports

The filesystem loader treats the selected template directory—not the global templates root—as the
source root. It preserves paths relative to that directory. The compiler maps `main.typ` and
imported files to rooted virtual project paths, so an import such as:

```typst
#import "partials/header.typ"
```

resolves only against the prepared in-memory map. `_shared/` content is already included by the
filesystem loader before the compiler receives the source.

Literal imports such as `#import "@preview/cetz:0.5.2"` are discovered during preparation. Missing
packages are downloaded through `PackageRepository` into `<project>/.typrest/packages`; direct and
transitive package files are then retained in memory. Package-rooted diagnostics use paths such as
`@preview/cetz:0.5.2/src/lib.typ` and never expose the physical cache.

Only the `preview` namespace and fully versioned literal imports are supported. Request compilation
never reads the cache or downloads a package.

## Diagnostics and Export

Compilation may return warnings alongside either a document or errors. Preserve all warnings and
all errors in their original order. `world.rs` immediately maps Typst severities, spans, hints, and
tracepoints to the owned diagnostic model.

PDF export errors use `CompilationError::ExportFailed`; compilation errors use
`CompilationError::Failed`. See [Compiler Diagnostics and Logging](diagnostics.md) for the public
contract.

## Upgrade Checklist

When upgrading Typst:

1. change all exact Typst integration versions together;
2. adapt `world.rs`, `fonts.rs`, and private prepared state as required;
3. keep public signatures free of Typst types;
4. run compiler, diagnostics, font, import, payload, and PDF export tests;
5. review diagnostic snapshots rather than accepting them blindly;
6. verify the architecture test still rejects imports outside `compiler/`;
7. update this reference only when the implemented boundary changes.

Do not copy “latest version” snippets from external examples into project documentation. Inspect the
pinned crate source and current compiler tests when an upstream API differs.
