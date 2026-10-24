# ADR-0001: Resolve `_shared/` During Loading

- **Date:** 2025-11
- **Status:** Accepted

## Context

The compiler must receive a self-contained virtual source tree without knowing the physical project
layout or active loading mode.

## Decision

`FilesystemLoader` includes `_shared/` files in each template's `TemplateSource` during load and
reload, not during compilation.

## Alternatives

Resolve `_shared/` during compilation, which would make the compiler aware of the physical project
layout and active loading mode.

## Consequences

- Every `TemplateBundle` is self-contained.
- The compiler does not know the physical project layout.
- A `_shared/` change causes watch mode to rebuild every template.
