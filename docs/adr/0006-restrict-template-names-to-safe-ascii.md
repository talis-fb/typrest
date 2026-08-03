# ADR-0006: Restrict Template Names to Safe ASCII Identifiers

- **Date:** 2026-06
- **Status:** Superseded by [ADR-0010](0010-derive-template-ids-from-folder-routes.md)

## Context

The same identifier appears in HTTP routes and filesystem directory lookup. Separators, relative
segments, whitespace, and non-ASCII names create ambiguity or traversal risk.

## Decision

`TemplateName` must be non-empty and contain only ASCII alphanumeric characters, hyphen (`-`), or
underscore (`_`).

## Alternatives

Accept general Unicode names or path-like identifiers and sanitize them at each filesystem and HTTP
boundary.

## Consequences

Names such as `invoice`, `invoice-v2`, and `invoice_v2` are valid. Names such as `../invoice`,
`invoice/v2`, names with spaces, and non-ASCII names are rejected when constructing the type.
