# ADR-0001: Integrate Typst as a Rust Library

- **Date:** 2025-11
- **Status:** Accepted

## Context

Browser-based rendering is operationally heavy, language-specific PDF libraries couple callers to
one ecosystem, and invoking Typst as a child process would add a binary dependency and per-render
process overhead. Typst provides native JSON inputs and fits the Rust service model.

## Decision

Use Typst through the `typst` and `typst-pdf` Rust crates rather than invoking an external binary.

## Alternatives

- Browser automation such as headless Chrome.
- LaTeX or abandoned HTML-to-PDF executables.
- A language-specific PDF library.
- Invoking the Typst CLI as a child process.

## Consequences

- Typrest ships one application binary without a separate Typst executable.
- Integration complexity is isolated behind the compiler anti-corruption layer.
- Typst API upgrades require deliberate compatibility work and exact joint dependency pinning.
