# Architectural Decision Records

This directory records non-obvious architectural decisions for Typrest. Each ADR is an immutable
historical record: clarify wording when necessary, but use a new ADR to change or reverse an
accepted decision.

## Status Values

- **Accepted**: active unless superseded.
- **In discussion**: proposed but not approved for implementation.
- **Rejected**: considered and not selected.
- **Superseded**: replaced wholly or partly by a later ADR.

## Decision Index

| ID | Decision | Date | Status |
|---|---|---|---|
| [ADR-0001](0001-integrate-typst-as-rust-library.md) | Integrate Typst as a Rust library | 2025-11 | Accepted |
| [ADR-0002](0002-abstract-template-loading.md) | Abstract template loading behind a trait | 2025-11 | Accepted |
| [ADR-0003](0003-package-filesystem-templates-with-deployment.md) | Package filesystem templates with the deployment | 2025-11 | Accepted |
| [ADR-0004](0004-resolve-shared-during-loading.md) | Resolve `_shared/` during loading | 2025-11 | Accepted |
| [ADR-0005](0005-inject-request-data-through-sys-inputs.md) | Inject request data through `sys.inputs` | 2025-11 | Accepted |
| [ADR-0006](0006-restrict-template-names-to-safe-ascii.md) | Restrict template names to safe ASCII identifiers | 2026-06 | Superseded by ADR-0010 |
| [ADR-0007](0007-expose-typrest-owned-compiler-diagnostics.md) | Expose Typrest-owned compiler diagnostics | 2026-07 | Accepted |
| [ADR-0008](0008-embed-tracked-development-web-ui.md) | Embed the tracked development Web UI | 2026-07 | Superseded in part by ADR-0009 |
| [ADR-0009](0009-gate-static-web-ui-routes.md) | Gate static Web UI routes, not metadata routes | 2026-08 | Superseded in part by ADR-0010 |
| [ADR-0010](0010-derive-template-ids-from-folder-routes.md) | Derive template IDs and direct routes from folders | 2026-08 | Accepted |

## Adding a Decision

1. Copy [`template.md`](template.md) to the next four-digit sequence, such as
   `0010-short-decision-title.md`.
2. Keep the numeric identifier forever; do not reorder or reuse numbers.
3. Describe the context, decision, considered alternatives, and consequences.
4. Add the ADR to the ordered table above.
5. When replacing an earlier decision, link both records and update the earlier status to
   **Superseded** or **Superseded in part**.
6. Update related specifications, architecture documentation, tests, and roadmap entries in the
   same change.

Use ADRs for decisions with meaningful architectural, compatibility, security, operational, or
product-scope consequences. Routine implementation details do not need an ADR.
