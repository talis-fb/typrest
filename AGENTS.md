# Typrest Agent Guide

This file is the repository-wide entry point for AI coding agents. Keep it concise and route
detailed knowledge to the canonical documents under [`docs/`](docs/README.md).

## Project at a Glance

Typrest is a Rust HTTP service that validates JSON payloads, renders Typst templates, and returns
PDF bytes. The current product supports filesystem-backed templates only. Templates are loaded,
prepared, and warm-compiled into an in-memory registry before the server listens.

The repository contains:

- `crates/typrest-core`: domain types, schema validation, prepared registry, render service, and
  the Typst anti-corruption layer;
- `crates/typrest-server`: filesystem loading, watch mode, Axum routes, diagnostics, and server
  startup;
- `crates/typrest-cli`: `init`, `check`, and `serve`, including project configuration discovery;
- `webui`: optional Svelte development UI and committed production assets;
- `examples/templates`: example Typst templates and payloads.

## Read Before Changing

| Task | Read first |
|---|---|
| Architecture or domain changes | [`docs/architecture.md`](docs/architecture.md), [`docs/adr/`](docs/adr/README.md) |
| Schema parsing or validation | [`docs/schema-v1.md`](docs/schema-v1.md) |
| CLI or project discovery | [`docs/cli-spec.md`](docs/cli-spec.md), [`docs/typrest-toml-spec.md`](docs/typrest-toml-spec.md) |
| Registry, startup, or watch mode | [`docs/startup-template-compilation.md`](docs/startup-template-compilation.md) |
| Typst compiler or fonts | [`docs/typst-integration.md`](docs/typst-integration.md), [`docs/diagnostics.md`](docs/diagnostics.md) |
| HTTP handlers or errors | [`docs/architecture.md`](docs/architecture.md#http-surface), [`docs/diagnostics.md`](docs/diagnostics.md) |
| Web UI | [`docs/webui-spec.md`](docs/webui-spec.md), [`docs/webui-development.md`](docs/webui-development.md) |
| Tests | [`docs/testing.md`](docs/testing.md) |

When code, tests, and prose disagree, treat the current code and tests as executable truth, then
update the affected documentation in the same change. Do not silently choose between conflicting
accepted decisions; record a superseding ADR under `docs/adr/`.

## Critical Architecture Rules

- Filesystem mode is the only implemented mode. Literal, fully versioned `@preview` packages resolve
  during preparation. Do not implement Store mode, tags, publishing, authentication, SDKs, S3, a
  compilation cache, other package namespaces, or custom registries without an explicit decision
  and scope change.
- HTTP handlers never read template files. Startup and watch mode use `TemplateLoader`; requests
  use `TemplateRegistry` through `RenderService`.
- A template requires `main.typ`. `schema.yml` is optional; legacy `schema.json` is accepted, but a
  template cannot contain both.
- Load-time schema errors do not become render errors. Startup compilation failures occur before
  socket binding. Request-time validation and payload-dependent compilation failures are render
  errors.
- Only files under `crates/typrest-core/src/compiler/` may import `typst::*` or `typst_pdf::*`.
  Public compiler types must not expose Typst types.
- `typst`, `typst-pdf`, `typst-assets`, `typst-kit`, and `typst-syntax` stay exactly pinned to the
  same version and are upgraded together.
- Filesystem template IDs are derived recursively from the first non-root folder containing
  `main.typ`. Route discovery skips folders beginning with `_`; selected template bundles do not.
- `_shared/` files are assembled into each `TemplateSource` by the filesystem loader. Relative
  paths and template subdirectories must be preserved.
- Use typed `thiserror` errors in production code. Do not add `unwrap()` or `expect()` outside tests
  unless an invariant is locally proven and the existing compiler boundary already permits it.
- Keep `typrest-core` free of HTTP dependencies and direct filesystem access.

## Current HTTP Surface

The implemented routes are:

- `GET /_healthz`;
- `GET /_api/templates`;
- `GET /_api/templates/{*template_id}`;
- `POST /{*template_id}`;
- `/`, `/_webui`, `/_webui/`, and `/_webui/*` only when `web_ui = true`.

The `/_api/*` metadata routes are currently mounted even when the Web UI is disabled. There are no
implemented `/healthz` or `/templates` aliases.

## Code Conventions

- Use canonical domain terms from [`docs/architecture.md`](docs/architecture.md#canonical-domain-terms).
- Prefer edition-2018-style modules: `foo.rs` plus `foo/`; do not add new `mod.rs` files.
- Do not add `pub use` solely as an internal import shortcut. Public API re-exports must be
  intentional.
- HTTP errors use the structured JSON envelope defined by handlers and diagnostics; never return a
  generic string error from a handler.
- Tests for core behavior should use `MemoryLoader` and mock compilers. Keep real Typst integration
  tests inside the compiler ACL or explicit end-to-end coverage.
- Define named constants for shared project-specific values instead of repeating magic strings.
- Preserve unrelated user changes in a dirty worktree and keep changes scoped to the task.

## Validation Commands

Run the smallest relevant checks while iterating, then the complete affected suite.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For Web UI changes:

```bash
cd webui
npm run check
npm test
npm run build
```

`webui/dist` is committed and embedded in the server. Any frontend source change is incomplete
until the production build is regenerated and reviewed. Do not hand-edit generated files.

See [`docs/testing.md`](docs/testing.md) for targeted commands and test placement.

## Change Completion

Before finishing:

1. Verify the behavior with focused tests and the relevant full suite.
2. Update the canonical specification when a contract changes.
3. Add or update an ADR for non-obvious architectural choices and `ROADMAP.md` for scope/status.
4. Check local Markdown links and remove stale commands or duplicate sources of truth.
5. Report checks that were run and any known limitation that remains.
