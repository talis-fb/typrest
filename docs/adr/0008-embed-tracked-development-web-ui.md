# ADR-0008: Embed the Tracked Development Web UI

- **Date:** 2026-07
- **Status:** Superseded in part by [ADR-0009](0009-gate-static-web-ui-routes.md)

## Context

Runtime asset directories would make the binary location-dependent. Invoking npm from Cargo would
add Node and network requirements to normal Rust and container builds.

## Decision

Build the Vite Web UI into a small tracked `webui/dist` directory and embed those files in the
Typrest binary. The original route-gating consequence is superseded by
[ADR-0009](0009-gate-static-web-ui-routes.md).

## Alternatives

- Load frontend assets from a runtime directory.
- Invoke npm from Cargo and build frontend assets during every Rust build.

## Consequences

- Normal Rust builds do not require Node.
- Frontend changes require `npm run build` and committed `webui/dist` output.
- The Web UI remains optional at runtime and development-only.
