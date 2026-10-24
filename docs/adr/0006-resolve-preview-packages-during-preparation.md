# ADR-0006: Resolve Preview Packages During Preparation

- **Date:** 2026-08
- **Status:** Accepted

## Context

Templates need to import versioned packages from Typst Universe while Typrest must continue to
compile requests without network or filesystem access. Package files use a distinct Typst virtual
root and cannot be flattened safely into the template source map.

## Decision

Support literal, fully versioned `@preview` imports. `PdfCompiler::prepare()` discovers direct and
transitive imports and obtains them through a Typrest-owned `PackageRepository`. Filesystem mode
uses the official Typst package tooling to download missing packages into the project-local
`.typrest/packages` cache.

Prepared templates retain immutable in-memory package source maps keyed by their complete package
coordinate. The Typst world resolves project and package virtual roots independently. Normal render
requests use only this prepared state and never access the package repository.

The initial implementation does not support dynamic package import expressions, other namespaces,
custom registries, lockfiles, vendoring, or offline configuration.

## Alternatives

- Download packages from `World::source()` or `World::file()`. These synchronous callbacks also run
  during requests and would introduce blocking I/O and transient network failures into rendering.
- Resolve packages in `FilesystemLoader`. Package discovery depends on Typst syntax and is shared
  compiler preparation rather than template route discovery.
- Flatten package files into `TemplateSource`. This loses package-root identity and permits path
  collisions between packages and project files.

## Consequences

- `check`, startup, and watch preparation may perform package cache and network I/O.
- Missing or invalid packages fail preparation before startup; watch mode keeps the last-known-good
  prepared template.
- Package versions remain explicit in template imports, but archive hashes are not locked yet.
- `.typrest/` is generated project-local cache content and should not be committed.
