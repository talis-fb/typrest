# Typrest Agent Guide

This file is the repository-wide entry point for AI coding agents. Keep it concise and operational;
detailed knowledge lives in the canonical documents under [`docs/`](docs/README.md).

## Project at a Glance

Typrest is a Rust HTTP service that validates JSON payloads, renders Typst templates, and returns
PDF bytes. The current product supports filesystem-backed templates only. Templates are loaded,
prepared, and warm-compiled into an in-memory registry before the server listens. See the
[product vision](docs/product/vision.md).

- `crates/typrest-core`: domain types, schema validation, prepared registry, render service, and
  the Typst anti-corruption layer;
- `crates/typrest`: the application package, with private modules for CLI commands, project
  loading, package resolution, watch mode, HTTP, and diagnostics;
- `webui`: optional Svelte development UI and committed production assets;
- `examples/templates`: example Typst templates and payloads.

## Documentation Navigation

[`docs/README.md`](docs/README.md) is the index and holds the authoring conventions. Read the
relevant document before changing code:

| Task | Read first |
|---|---|
| Architecture or domain changes | [`docs/architecture/overview.md`](docs/architecture/overview.md), [`docs/adr/`](docs/adr/README.md) |
| Schema parsing or validation | [`docs/specs/schema-v1.md`](docs/specs/schema-v1.md) |
| CLI or project discovery | [`docs/specs/cli.md`](docs/specs/cli.md), [`docs/specs/typrest-toml.md`](docs/specs/typrest-toml.md) |
| Template discovery, bundles, fonts, packages | [`docs/specs/templates.md`](docs/specs/templates.md) |
| Registry, startup, or watch mode | [`docs/architecture/template-lifecycle.md`](docs/architecture/template-lifecycle.md) |
| Typst compiler or fonts | [`docs/architecture/compiler.md`](docs/architecture/compiler.md), [`docs/specs/diagnostics.md`](docs/specs/diagnostics.md) |
| HTTP routes, handlers, or errors | [`docs/specs/http-api.md`](docs/specs/http-api.md), [`docs/specs/diagnostics.md`](docs/specs/diagnostics.md) |
| Logging or process streams | [`docs/specs/logging.md`](docs/specs/logging.md) |
| Web UI | [`docs/specs/webui.md`](docs/specs/webui.md), [`docs/development/webui.md`](docs/development/webui.md) |
| Tests | [`docs/development/testing.md`](docs/development/testing.md) |

Before a non-trivial change, read the specification and architecture document for the area, and
check [`ROADMAP.md`](ROADMAP.md) for scope.

## Source of Truth

- Code and tests are the executable truth for implemented behavior; specifications are the contract.
- When code, tests, and prose disagree, treat the code and tests as truth, then update the affected
  documentation in the same change.
- Do not silently choose between conflicting accepted decisions. Propose a superseding ADR.
- Cite requirement IDs (`HTTP-020`, `SCHEMA-090`) instead of quoting requirements.

## Documentation Updates

- A change to observable behavior updates its specification, including requirement IDs, in the
  same change. IDs are stable: never renumber or reuse one; mark a removed one `(removed)`.
- A structural change updates the architecture document. A scope or status change updates
  [`ROADMAP.md`](ROADMAP.md). Update the [`docs/README.md`](docs/README.md) index when adding or
  moving a document.
- Do not add duplicate sources of truth. Link to the canonical document.
- Check links with `python3 scripts/check-doc-links.py`.

### ADRs and plans

- Do not create an ADR on your own. When a change involves a costly-to-reverse choice with serious
  alternatives, propose the ADR to the user and wait for approval. The criteria are in
  [`docs/adr/README.md`](docs/adr/README.md). Never log routine changes as ADRs, and never edit an
  accepted ADR to change its decision; supersede it.
- Write a plan under [`docs/plans/`](docs/plans/README.md) only for work that spans several
  changes or needs review before implementation. Delete the plan when the work is complete.

## Critical Architecture Rules

- Filesystem mode is the only implemented mode. Literal, fully versioned `@preview` packages resolve
  during preparation. Do not implement Store mode, tags, publishing, authentication, SDKs, S3, a
  compilation cache, other package namespaces, or custom registries without an explicit decision
  and scope change.
- HTTP handlers never read template files. Startup and watch mode use `TemplateLoader`; requests
  use `TemplateRegistry` through `RenderService`.
- Load-time schema errors do not become render errors. Startup compilation failures occur before
  socket binding. Request-time validation and payload-dependent compilation failures are render
  errors.
- Only files under `crates/typrest-core/src/compiler/` may import `typst`, `typst-pdf`, or
  `typst-assets`. Public compiler types must not expose Typst types. The application package uses
  `typst-kit` and `typst-syntax` only for Universe package resolution.
- `typst`, `typst-pdf`, `typst-assets`, `typst-kit`, and `typst-syntax` stay exactly pinned to the
  same version and are upgraded together.
- Use typed `thiserror` errors in production code. Do not add `unwrap()` or `expect()` outside tests
  unless an invariant is locally proven and the existing compiler boundary already permits it.
- Keep `typrest-core` free of HTTP dependencies and direct filesystem access.

Template identity, discovery, `_shared/`, and schema file rules are in the
[template layout specification](docs/specs/templates.md); routes are in the
[HTTP API specification](docs/specs/http-api.md).

## Code Conventions

- Use canonical domain terms from [`docs/architecture/overview.md`](docs/architecture/overview.md#canonical-domain-terms).
- Prefer edition-2018-style modules: `foo.rs` plus `foo/`; do not add new `mod.rs` files.
- Do not add `pub use` solely as an internal import shortcut. Public API re-exports must be
  intentional.
- HTTP errors use the structured JSON envelope defined by the HTTP specification; never return a
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

`webui/dist` is committed and embedded in the application. Any frontend source change is incomplete
until the production build is regenerated and reviewed. Do not hand-edit generated files.

For documentation changes: `python3 scripts/check-doc-links.py` and `git diff --check`.

See [`docs/development/testing.md`](docs/development/testing.md) for targeted commands and test placement.

## Change Completion

Before finishing:

1. Verify the behavior with focused tests and the relevant full suite.
2. Update the canonical specification when a contract changes.
3. Update `ROADMAP.md` for scope or status changes; propose an ADR only as described above.
4. Check local Markdown links and remove stale commands or duplicate sources of truth.
5. Report checks that were run and any known limitation that remains.
