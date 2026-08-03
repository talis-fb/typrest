# ADR-0002: Abstract Template Loading Behind a Trait

- **Date:** 2025-11
- **Status:** Accepted

## Context

Filesystem mode is the only current implementation, but a future Store mode must not require
storage-specific conditionals throughout the server and rendering domain.

## Decision

Represent template sources with the `TemplateLoader` trait. A future Store mode will use another
implementation of this boundary.

## Alternatives

- Cargo feature flags would create separate builds.
- A shared enum would make consumers aware of every storage mode.
- A trait keeps loading policy behind one interface.

## Consequences

- `typrest-core` owns the trait and test `MemoryLoader`.
- `typrest-server` owns `FilesystemLoader` because physical filesystem access is a server concern.
- Startup and watch mode receive a loader and build prepared registry entries.
- HTTP handlers and normal renders use `TemplateRegistry`, not `Arc<dyn TemplateLoader>`.
