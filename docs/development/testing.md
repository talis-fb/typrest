# Testing Guide

- **Status:** Current. `.github/workflows/ci.yml` is authoritative for required CI jobs; it runs the Rust checks only.

## Standard Validation

Run the same Rust checks as CI:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Run focused suites while iterating:

```bash
cargo test -p typrest-core compiler
cargo test -p typrest-core schema
cargo test -p typrest project
cargo test -p typrest http
cargo test -p typrest --test check
```

For Web UI changes:

```bash
cd webui
npm run check
npm test
npm run build
```

The production output under `webui/dist` is committed and embedded in the Rust binary. Frontend
source changes must include the rebuilt output; never edit it by hand.

## Test Placement

| Area | Preferred coverage |
|---|---|
| Core domain | Direct unit assertions using `MemoryLoader` and mock compilers |
| Schema | Parsing, structural consistency, validation order, defaults, and paths |
| Compiler ACL | Real Typst compilation, fonts, imports, payload injection, diagnostics, export |
| Packages | In-memory fixtures, transitive imports, cache loading, and no request-time I/O |
| Registry | Startup aggregation, warmup samples, prepared reuse, replacement behavior |
| Application loading and packages | Filesystem loading, package cache/download, startup, and watch mode |
| HTTP | Router, status/error mapping, metadata, and render responses |
| CLI | Clap parsing, project discovery, configuration, scaffolding, check behavior |
| Web UI | Payload composition, mode conversion, custom fields, TypeScript/Svelte checks |
| End to end | Random local port, HTTP status/content type, and meaningful PDF content |

Normal core and handler tests should not invoke real Typst unless integration is the behavior under
test. Filesystem tests use temporary directories and must not depend on global machine state.
Package tests use mocked downloads or temporary cached fixtures and must not require live Universe
access. No automated test currently rejects `typst`, `typst-pdf`, and `typst-assets` imports outside the
compiler ACL; check with
`rg -n '\btypst(_pdf|_assets)?::' crates --glob '!crates/typrest-core/src/compiler/**'`. `typst-kit` and `typst-syntax` are used by the application package resolver.

## Snapshot Policy

Use snapshots only for stable user-visible output that is awkward to assert directly. Compiler
diagnostic snapshots are appropriate when they verify formatted ranges, hints, and traces.

For rendered documents, prefer text extracted from PDFs. Do not snapshot raw PDF bytes: object
ordering, compression, identifiers, metadata, and font subsets may change without a behavior
regression. Broad visual snapshots remain out of scope until their value exceeds their maintenance
cost.

## Manual Validation

The repository root is not itself a Typrest project because it has no `typrest.toml`. Build a binary
and create a disposable project before exercising the CLI manually:

```bash
cargo build
TYPREST_BIN="$PWD/target/debug/typrest"
"$TYPREST_BIN" init /tmp/typrest-manual
cd /tmp/typrest-manual
"$TYPREST_BIN" check
"$TYPREST_BIN" serve --hostname 127.0.0.1
```

Then render the generated template:

```bash
curl -X POST \
  http://127.0.0.1:8080/hello-world \
  -H 'Content-Type: application/json' \
  --data '{"name":"Ada"}' \
  --output hello-world.pdf
```

When investigating PDF content:

```bash
pdffonts hello-world.pdf
pdftotext hello-world.pdf -
```

The current Docker files use removed CLI syntax and are not a valid manual test path. Their
migration is tracked in [`ROADMAP.md`](../../ROADMAP.md).

## Documentation Validation

For documentation-only changes:

- verify commands against CLI `--help` and `webui/package.json`;
- verify routes and response shapes against router tests;
- check relative links and heading anchors;
- run `git diff --check`;
- ensure no stale contract appears elsewhere with `rg`.

Tests must not require Docker, Podman, or persistent generated artifacts.
