# Product Vision

- **Status:** Current
- **Scope:** Why Typrest exists, what it includes today, and what it deliberately excludes.

## Purpose

Typrest is an HTTP service for server-side PDF generation with [Typst](https://typst.app/).
Applications in any language send JSON over HTTP; Typrest validates the payload when a schema is
present, compiles the selected template, and returns PDF bytes.

The MVP goal is to validate Typst as a fast, language-agnostic server-side PDF engine exposed
through HTTP. That goal is met: a filesystem project can be checked, started, and rendered over
HTTP with payload validation and structured diagnostics.

## Why Typst, in process

Typrest uses Typst as a Rust library, not as a separate program:

- browser-based rendering is operationally heavy;
- language-specific PDF libraries couple callers to one ecosystem;
- invoking the Typst CLI as a child process adds a binary dependency and per-render process
  overhead;
- Typst accepts JSON inputs natively and fits a Rust service.

The result is a single application binary with no separate Typst executable. The integration
boundary is described in the [compiler architecture](../architecture/compiler.md).

## Scope

Filesystem mode is the only implemented mode. Templates are deployment artifacts: they live in a
project directory, are loaded and compiled before the server listens, and change by redeploying or,
with watch mode, by updating the mounted files. Filesystem mode has no tags or independent template
release workflow.

Included:

- the `typrest` CLI (`init`, `check`, `serve`) described in the [CLI specification](../specs/cli.md);
- project configuration in `typrest.toml` ([specification](../specs/typrest-toml.md));
- folder-derived templates, optional payload schemas, and literal, fully versioned `@preview`
  packages ([template specification](../specs/templates.md), [schema](../specs/schema-v1.md));
- an HTTP API that renders PDFs, reports health, and lists prepared templates
  ([HTTP specification](../specs/http-api.md));
- structured diagnostics and logs ([diagnostics](../specs/diagnostics.md),
  [logging](../specs/logging.md));
- an optional development Web UI ([specification](../specs/webui.md)).

Excluded until real usage justifies them and the decision is recorded:

- Store mode, template publishing, tags, and rollback;
- authentication and authorization;
- SDKs;
- S3 or other remote backends;
- a compilation cache;
- package namespaces other than `preview`, custom registries, and package lockfiles.

[`ROADMAP.md`](../../ROADMAP.md) tracks delivered and deferred work.

## High-level requirements

- A broken template must be found before the server accepts traffic.
- Rendering requests must not read files or access the network.
- Failures must be reported with structured, safe diagnostics that do not expose physical paths or
  source excerpts over HTTP.
- The compiler dependency must stay behind a Typrest-owned boundary so Typst upgrades stay local.

Detailed, testable requirements live in the [specifications](../specs/README.md).
