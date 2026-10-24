# ADR-0004: Embed the Tracked Development Web UI

- **Date:** 2026-07
- **Status:** Accepted

*Rewritten during the 2026 documentation migration to state the current decision and combine the
route-gating choice that was first recorded separately.*

## Context

Runtime asset directories would make the binary location-dependent. Invoking npm from Cargo would
add Node and network requirements to normal Rust and container builds. Separately, the bundled UI
depends on prepared-registry metadata routes, while serving its static assets is an independent
configuration choice.

## Decision

Build the Vite Web UI into a small tracked `webui/dist` directory and embed those files in the
Typrest binary.

`web_ui` controls only the static routes under `/_webui` (and the redirect from `/`). The
prepared-registry metadata routes under `/_api/templates` remain mounted independently of that flag.

## Alternatives

- Load frontend assets from a runtime directory.
- Invoke npm from Cargo and build frontend assets during every Rust build.
- Gate metadata and static routes together, making the metadata endpoints unavailable while the UI
  is disabled.

## Consequences

- Normal Rust builds do not require Node.
- Frontend changes require `npm run build` and committed `webui/dist` output.
- The Web UI remains optional at runtime and development-only.
- Disabling `web_ui` does not disable metadata exposure. Metadata handlers read only
  `TemplateRegistry`, and the routes are internal and unversioned.
- Deployments requiring confidentiality must protect the server with network controls. The
  requirement is stated in the [HTTP API specification](../specs/http-api.md) (`REQ-HTTP-051`).
