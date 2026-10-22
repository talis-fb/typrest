# Architectural Decision Records

This directory records why Typrest made a small number of costly-to-reverse architectural choices.
An ADR explains a decision; it is never the only place that describes current behavior. Current
behavior lives in [specifications](../specs/README.md) and [architecture documents](../architecture/overview.md).

## When an ADR is warranted

Write an ADR only when all of the following hold:

1. At least two viable alternatives were seriously considered.
2. The choice is costly to reverse, or it constrains a boundary, public contract, security
   property, or operational behavior.
3. The rationale cannot be derived from the product vision, a specification, or an obvious
   engineering default.

Do not write an ADR for:

- restating the product premise;
- describing the current structure, module layout, or behavior;
- product scope and MVP boundaries, which belong in the [product vision](../product/vision.md) and
  [`ROADMAP.md`](../../ROADMAP.md);
- routine implementation choices, refactors, or a decision whose only alternative is a strawman.

AI agents must propose an ADR and obtain human approval before creating one. They must not create
ADRs to log routine changes.

## Status values

- **Proposed**: under discussion and not approved for implementation.
- **Accepted**: approved and active unless superseded.
- **Superseded**: replaced by a later ADR. Link to the superseding record.
- **Rejected**: considered and not selected.

## Immutability

ADRs are append-only. Clarify wording when necessary, but do not delete or rewrite an accepted
decision. Change or reverse it with a new ADR, then mark the earlier record **Superseded** and link
both records. Numbers are permanent: never renumber or reuse one. Gaps are expected.

The set below was pruned once, before the first release, to remove records that restated the
product premise or described intermediate prototype decisions. Git history is the archive for the
deleted records.

## Decision index

| ID | Decision | Date | Status |
|---|---|---|---|
| [ADR-0004](0004-resolve-shared-during-loading.md) | Resolve `_shared/` during loading | 2025-11 | Accepted |
| [ADR-0005](0005-inject-request-data-through-sys-inputs.md) | Inject request data through `sys.inputs` | 2025-11 | Accepted |
| [ADR-0007](0007-expose-typrest-owned-compiler-diagnostics.md) | Expose Typrest-owned compiler diagnostics | 2026-07 | Accepted |
| [ADR-0008](0008-embed-tracked-development-web-ui.md) | Embed the tracked development Web UI | 2026-07 | Accepted |
| [ADR-0010](0010-derive-template-ids-from-folder-routes.md) | Derive template IDs and direct routes from folders | 2026-08 | Accepted |
| [ADR-0011](0011-resolve-preview-packages-during-preparation.md) | Resolve preview packages during preparation | 2026-08 | Accepted |
| [ADR-0012](0012-consolidate-application-packages.md) | Consolidate application packages around the core boundary | 2026-08 | Accepted |
| [ADR-0014](0014-use-one-stdout-tracing-stream.md) | Use one stdout tracing stream | 2026-08 | Accepted |

## Adding a decision

1. Get human approval for the proposal.
2. Copy [`template.md`](template.md) to the next unused four-digit number, such as
   `0015-short-decision-title.md`, with status **Proposed**.
3. Describe the context, decision, alternatives, and consequences.
4. Add the record to the index above.
5. When the decision is approved, set the status to **Accepted**. When replacing an earlier decision,
   link both records and mark the earlier one **Superseded**.
6. Update affected specifications, architecture documents, tests, and `ROADMAP.md` in the same change.
