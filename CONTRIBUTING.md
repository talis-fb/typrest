# Contributing to Typrest

Thank you for helping improve Typrest. Contributions of code, tests, documentation, examples, and
well-researched issue reports are welcome.

## Before You Start

- Search existing issues and pull requests before opening a new one.
- Use an issue to discuss substantial features, public contract changes, or architectural work
  before investing in an implementation.
- Small bug fixes, tests, and documentation corrections may go directly to a pull request when the
  intended behavior is already clear.
- Do not open a public issue for a suspected vulnerability. Report it privately through
  [GitHub private vulnerability reporting](https://github.com/talis-fb/typrest/security/advisories/new).

The current product scope is filesystem-backed templates. Store mode, template publishing,
authentication, SDKs, S3, compilation caches, other package namespaces, and custom registries are
not implemented. Proposals in these areas require an explicit scope decision before implementation.

## Repository Orientation

| Area | Responsibility |
|---|---|
| `crates/typrest-core` | Domain types, validation, prepared registry, rendering, and the Typst compiler boundary |
| `crates/typrest` | CLI, project loading, package resolution, watch mode, HTTP, and diagnostics |
| `webui` | Optional Svelte development UI and committed production assets |
| `docs` | Product vision, specifications, architecture, development guides, ADRs, and plans |
| `examples/templates` | Example templates, schemas, and payloads |

Start with the [documentation index](docs/README.md). Read the relevant document before
changing behavior:

- [Architecture overview](docs/architecture/overview.md) for domain or boundary changes;
- [Template layout](docs/specs/templates.md) and [Schema v1](docs/specs/schema-v1.md) for template
  discovery, packages, schema parsing, or validation;
- [CLI](docs/specs/cli.md) and [`typrest.toml`](docs/specs/typrest-toml.md) specifications for CLI
  or project-discovery changes;
- [HTTP API](docs/specs/http-api.md) for routes, responses, and errors;
- [Template lifecycle](docs/architecture/template-lifecycle.md) for registry, startup, or
  watch-mode changes;
- [Compiler architecture](docs/architecture/compiler.md) and
  [diagnostics](docs/specs/diagnostics.md) for compiler, fonts, packages, or error reporting;
- [Web UI development](docs/development/webui.md) and the [Web UI specification](docs/specs/webui.md)
  for frontend changes;
- [Testing](docs/development/testing.md) for test placement and validation guidance.

AI coding agents should also follow [AGENTS.md](AGENTS.md). Contributors remain responsible for
understanding their submissions, verifying generated or assisted work, and ensuring that all
submitted material may legally be contributed.

## Development Setup

Install a stable Rust toolchain that supports Rust edition 2024. The Web UI additionally requires
Node.js and npm. Normal Rust builds use the committed files under `webui/dist` and do not require
Node.js.

Clone and build the project:

```bash
git clone https://github.com/talis-fb/typrest.git
cd typrest
cargo build --locked
```

Create and validate a disposable Typrest project:

```bash
TYPREST_BIN="$PWD/target/debug/typrest"
"$TYPREST_BIN" init /tmp/typrest-contributor
cd /tmp/typrest-contributor
"$TYPREST_BIN" check
```

The repository root is not itself a Typrest project because it intentionally has no
`typrest.toml`. The current Docker files are being migrated and are not a supported development
entry point yet.

For Web UI development, install the locked dependencies:

```bash
cd webui
npm ci
```

See the [Web UI development guide](docs/development/webui.md) for the local Vite workflow.

## Making a Change

1. Create a focused branch from the current default branch.
2. Keep the change scoped; avoid unrelated cleanup in the same pull request.
3. Add or update tests for behavior changes and regressions.
4. Update the canonical specification, including its requirement IDs, when a public contract
   changes. Update the architecture document when structure changes.
5. Propose an [architectural decision record](docs/adr/README.md) with a maintainer before
   writing one. ADRs are reserved for costly-to-reverse choices with serious alternatives; they are
   not a change log. An accepted ADR is superseded by a new one, never edited to change its decision.
6. Update `ROADMAP.md` when delivered scope or project status changes.
7. For work spanning several changes, a [plan](docs/plans/README.md) may be agreed first; delete it
   when the work lands.

Treat current code and tests as executable truth when they disagree with prose, then update the
affected documentation in the same change. Do not silently choose between conflicting accepted
architectural decisions.

Production Rust code should use typed errors and must not add `unwrap()` or `expect()` unless an
invariant is locally proven and follows an existing compiler-boundary convention. Keep direct
filesystem and HTTP dependencies out of `typrest-core`. Imports of `typst`, `typst-pdf`, and
`typst-assets` belong only under `crates/typrest-core/src/compiler/`.

## Validation

Run focused tests while iterating, followed by the complete Rust validation pass:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For Web UI changes, also run:

```bash
cd webui
npm run check
npm test
npm run build
```

The production output under `webui/dist` is committed and embedded in the Rust binary. A frontend
source change must include the reviewed output of `npm run build`; do not edit generated assets by
hand.

Before submitting, also run:

```bash
git diff --check
python3 scripts/check-doc-links.py   # when documentation changed
```

If you could not run a relevant check, explain why in the pull request.

## Pull Requests

A useful pull request:

- explains the problem and why the chosen approach solves it;
- links the related issue or discussion when one exists;
- describes user-visible behavior and compatibility impact;
- lists the exact validation commands that were run;
- includes tests and documentation appropriate to the change;
- keeps generated files, dependency lockfiles, and source changes synchronized;
- contains no credentials, proprietary templates, personal data, or unrelated changes.

Prefer reviewable, focused pull requests over large batches of unrelated work. Maintainers may ask
for a large proposal to be split into independently useful changes.

Commit messages should be concise and describe the change in the imperative mood. Conventional
prefixes such as `feat:`, `fix:`, `docs:`, `test:`, and `refactor:` are encouraged but not required.

## Licensing

Typrest is licensed under the [Apache License, Version 2.0](LICENSE). By submitting a contribution
for inclusion in Typrest, you agree that your contribution is licensed under Apache-2.0 in
accordance with the license's contribution terms. You must have the right to submit all code,
documentation, generated output, and other material included in your contribution.
