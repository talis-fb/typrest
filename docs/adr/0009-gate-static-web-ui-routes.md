# ADR-0009: Gate Static Web UI Routes, Not Metadata Routes

- **Date:** 2026-08
- **Status:** Accepted
- **Supersedes in part:** [ADR-0008](0008-embed-tracked-development-web-ui.md)

## Context

The current router and test contract always exposes prepared-registry metadata. The bundled UI uses
that API, but enabling its static assets is a separate configuration choice.

## Decision

`web_ui` controls only `/webui`, `/webui/`, and `/webui/*`. The prepared-registry metadata routes
`/_api/templates` and `/_api/templates/:template` remain mounted independently of that flag.

## Alternatives

Gate metadata and static routes together, making the metadata endpoints unavailable while the UI is
disabled.

## Consequences

- Disabling `web_ui` does not disable metadata exposure.
- Metadata handlers continue to read only `TemplateRegistry`.
- The routes remain internal and unversioned; external integrations should not assume stability.
- Deployments requiring confidentiality must protect the server with network controls.
