# Testing Guide

This project uses a small set of focused test styles. Most tests should be direct assertions over domain behavior. Snapshot tests are allowed, but they must stay minimal and remain a minority of the suite.

## Standard Commands

Run the full local validation pass:

```bash
cargo fmt --check
cargo check --workspace
cargo test --workspace
```

Run targeted suites:

```bash
cargo test -p papelon-core compiler
cargo test -p papelon-server repository_examples_load_and_render_invoice_over_http
cargo test -p papelon-cli
```

## Test Categories

Core unit tests cover schema parsing, schema validation, template names, in-memory loading, and `RenderService` behavior. These tests should use direct assertions and avoid real Typst unless they are specifically testing compiler integration.

Compiler tests live under `crates/papelon-core/src/compiler/`. They are allowed to use real Typst because the compiler module is the anti-corruption layer around Typst APIs. These tests validate PDF compilation, payload injection through `sys.inputs.data`, syntax error mapping, and default font loading.

Server tests cover HTTP handlers, error mapping, template listing, and render responses. Most server tests use `MemoryLoader` and mock compiler output. Tests that intentionally validate filesystem or real Typst integration can use `FilesystemLoader` and `TypstCompiler`.

CLI tests cover command parsing and filesystem scaffolding for `papelon serve`, `--watch`, and `papelon init`. They should use temporary directories and must not rely on global local state.

E2E-style tests may start the server on a random local port and issue an HTTP request. They should verify status code, `application/pdf`, and enough PDF content behavior to catch blank output regressions.

## Snapshot Policy

Snapshot tests are reserved for user-visible rendered output where normal assertions are too weak. They should be minimal and rare.

Use text snapshots extracted from rendered PDFs when validating rendered content. This catches blank PDFs and missing text without committing unstable binary PDFs.

Do not snapshot raw PDF bytes. PDF binaries include object ordering, compression streams, document identifiers, metadata, and font subset names that can change without a meaningful behavior change.

Avoid broad visual snapshots for now. They can catch layout regressions, but they add heavier tooling and more maintenance. Consider them only after the rendering surface is stable and the value outweighs the cost.

## Manual Validation

Run the server without containers:

```bash
cargo run -- serve ./examples/templates 8080 --watch
```

Render a sample PDF:

```bash
curl -X POST \
  http://localhost:8080/render/invoice \
  -H 'Content-Type: application/json' \
  --data-binary @examples/templates/invoice/payload.json \
  --output invoice.pdf
```

Inspect the PDF when debugging rendering issues:

```bash
pdffonts invoice.pdf
pdftotext invoice.pdf -
```

Container validation is manual or CI-specific. With Podman:

```bash
podman compose config
podman compose build
podman compose up
podman compose down
```

Tests should not require Docker, Podman, or generated local artifacts such as `invoice.pdf`.
