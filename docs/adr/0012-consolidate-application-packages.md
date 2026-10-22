# ADR-0012: Consolidate Application Packages Around the Core Boundary

- **Date:** 2026-08
- **Status:** Accepted

## Context

The original `typrest-core`, `typrest-server`, and `typrest-cli` split anticipated independent
layers before their responsibilities were established. The CLI imported filesystem loading,
package resolution, and diagnostics from the server package, while the server package mixed HTTP,
project I/O, watch supervision, and startup orchestration. These packages exposed internal APIs
without providing independent release or reuse boundaries.

## Decision

Keep `typrest-core` as the enforced HTTP-free and filesystem-free engine package. Consolidate CLI,
project/filesystem infrastructure, package transport, watch mode, HTTP, and output into one
`typrest` application package. Separate application responsibilities with private Rust modules.

The CLI command layer is the composition root. It prepares templates and optional watch state
before giving an in-memory registry and render service to the HTTP module. HTTP handlers never
receive a loader or package repository.

## Alternatives

- Keep the three packages. This preserves churn-free boundaries but leaves project infrastructure
  owned by an HTTP-named package and forces application internals to be public.
- Move filesystem and network implementations into `typrest-core`. This violates the core I/O
  boundary and couples the engine to one project mode.
- Add runtime and filesystem packages. This gives stricter layering but creates speculative public
  APIs before another runtime consumer or storage mode exists.

## Consequences

- The workspace has two packages: `typrest-core` and `typrest`.
- `typrest-core` remains reusable and cannot access HTTP or the physical filesystem.
- Application modules can evolve through crate-private APIs while the product is a prototype.
- A new package should be extracted only after an independent consumer, backend, release, or
  measured dependency/compile boundary exists.
- The `TemplateLoader` abstraction and the in-memory test loader live in `typrest-core`; the
  filesystem implementation lives in the `typrest` application package, because physical
  filesystem access is an application concern.
