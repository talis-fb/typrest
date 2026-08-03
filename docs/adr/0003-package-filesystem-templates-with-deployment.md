# ADR-0003: Package Filesystem Templates with the Deployment

- **Date:** 2025-11
- **Status:** Accepted

## Context

The MVP must avoid publishing, versioning, rollback, and remote-storage complexity while validating
the core HTTP-to-PDF product.

## Decision

In filesystem mode, templates are deployment artifacts. Updating a template requires rebuilding or
redeploying the container/image or updating the mounted filesystem under watch mode.

## Alternatives

- Implement Store mode, publishing, and versioning in the MVP.
- Manage templates independently through a remote backend.

## Consequences

- Filesystem mode has no tags or independent template release workflow.
- Store mode becomes relevant only when real usage requires independent template deployment.
- The current Docker files still need migration to the project-config CLI; see
  [`ROADMAP.md`](../../ROADMAP.md).
