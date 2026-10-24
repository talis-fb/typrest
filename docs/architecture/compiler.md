# Typst Integration Reference

- **Status:** Current
- **Scope:** Implementation reference for the private compiler anti-corruption layer.

## Boundary

Typrest links Typst as a library, so one application binary renders PDFs with no separate Typst
executable. See the [product vision](../product/vision.md) for why. The integration complexity is
isolated behind this anti-corruption layer, and [ADR-0003](../adr/0003-expose-typrest-owned-compiler-diagnostics.md)
explains why diagnostics cross it as Typrest-owned values.

The Typst compiler API can change between releases. Imports of `typst`, `typst-pdf`, and
`typst-assets` are therefore confined to `crates/typrest-core/src/compiler/`. Outside that
directory, compiler-facing code uses only Typrest-owned types such as `PdfCompiler`,
`PreparedTemplate`, `Document`, `CompilationDiagnostic`, `PackageCoordinate`, and
`PackageSource`. The application package resolver uses `typst-kit` and `typst-syntax` for the
official Universe package implementation.

The workspace `Cargo.toml` is the canonical dependency source. `typst`, `typst-pdf`,
`typst-assets`, `typst-kit`, and `typst-syntax` are pinned exactly to one matching version and
upgraded together. Compiler API-sensitive code lives in three private files:

- `world.rs`: the world implementation, compilation, export, spans, diagnostics, and inputs;
- `fonts.rs`: the font book and decoding APIs;
- `prepared_template.rs`: private Typst-backed prepared state behind public opaque types.

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
- custom lowercase `.ttf` and `.otf` fonts found anywhere in the selected template source;
- stable virtual paths required for imports and diagnostics.
- the complete direct and transitive `@preview` package source closure.

`PdfCompiler::compile_prepared()` borrows that state, builds request inputs, calls
`typst::compile`, maps warnings and errors, and exports with `typst-pdf`. It returns `Document` with
PDF bytes and non-fatal warnings.

Each template has its own prepared font book. Template-local fonts are available only to that
template, while fonts under the project-level `_shared/` directory are included in every template
source and therefore every template font book. Typrest does not discover operating-system fonts.

## World Responsibilities

The private `TemplateWorld` implements Typst's `World` interface:

- `main()` returns the `main.typ` virtual file identifier;
- `source()` returns UTF-8 Typst source from the in-memory file map;
- `file()` returns non-source bytes such as images and fonts;
- project and package virtual roots use separate prepared source maps;
- `book()` and `font()` expose the prepared font registry;
- `library()` exposes the request-specific standard library and inputs;
- `today()` returns the current UTC date by default and applies an explicit Typst offset to the
  current UTC instant before deriving the date.

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

Literal imports such as `#import "@preview/cetz:0.5.2"` are discovered during preparation. The
compiler passes Typrest-owned coordinates to `PackageRepository`. The application delegates cache
lookup and missing-package downloads to `typst-kit`'s official Universe implementation, then reads
the resolved package into a Typrest-owned source map. Direct and transitive package files are
retained in memory. Package-rooted diagnostics use paths such as
`@preview/cetz:0.5.2/src/lib.typ` and never expose the physical cache.

Only the `preview` namespace and fully versioned literal imports are supported. Request compilation
never reads the cache or downloads a package.

## Diagnostics and Export

Compilation may return warnings alongside either a document or errors. Preserve all warnings and
all errors in their original order. `world.rs` immediately maps Typst severities, spans, hints, and
tracepoints to the owned diagnostic model.

PDF export errors use `CompilationError::ExportFailed`; compilation errors use
`CompilationError::Failed`. See [Compiler diagnostics](../specs/diagnostics.md) for the public
contract.

## Upgrade Checklist

When upgrading Typst:

1. change all five exact Typst integration versions in one change;
2. adapt `world.rs`, `fonts.rs`, and private prepared state as required, and keep public signatures
   free of Typst types;
3. run compiler, diagnostics, font, import, payload, warning, and PDF export tests;
4. review diagnostic snapshots rather than accepting them blindly;
5. verify that no Typst import exists outside `compiler/` (no automated test enforces this today);
6. update this reference only when the implemented boundary changes.

Do not copy “latest version” snippets from external examples into project documentation. Inspect the
pinned crate source and current compiler tests when an upstream API differs.
