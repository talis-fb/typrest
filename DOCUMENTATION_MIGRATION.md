# Documentation Standard Migration

Status: Draft  
Type: Repository-wide documentation refactor

This file is temporary. It'll be deleted when the migration is complete. Any rule in it that must
outlive the migration is moved into the permanent documentation first (see Phase 2).

## 1. Objective

Establish a consistent, maintainable, AI-agent-friendly documentation system for Typrest.

The migration must establish clear boundaries between:

- Product vision and high-level requirements
- Current normative specifications
- Current architecture documentation
- Historical architectural decisions (ADRs)
- Development and testing guides
- Temporary implementation plans
- AI agent instructions

The resulting documentation must provide a reliable source of truth for Codex, Claude Code, human contributors, and future development.

This is a documentation refactor. It must not introduce unrelated application changes or alter intended product behavior.

## 2. Non-Negotiable Principles

1. Preserve existing functionality and intended behavior.
2. Do not invent requirements, decisions, or implementation details.
3. Treat the existing code, tests, documentation, and ADRs as evidence to reconcile, not as individually infallible sources.
4. Explicitly record contradictions, ambiguities, and unresolved questions.
5. Give every requirement a single canonical home.
6. Keep specifications focused on current expected behavior.
7. Keep architecture documentation focused on the current implementation and design.
8. Keep ADRs only for genuine decisions; an ADR records why a choice was made, never the current state of the system (see [ADR Policy and Pruning](#4-adr-policy-and-pruning)).
9. Keep active plans separate from permanent documentation.
10. Keep `AGENTS.md` concise and operational.
11. Prefer links and references over duplicated explanations.
12. Avoid creating unnecessary documents or splitting files without a clear benefit.
13. Preserve useful examples, edge cases, constraints, and rationale from existing documentation.
14. Do not silently discard information simply because it does not fit the new structure.
15. Make all documentation changes traceable and reviewable.

## 3. Target Structure

Adapt this structure to the actual project after the audit. Do not create empty or unnecessary files merely to match the example.

- `AGENTS.md`: concise, operational agent instructions; links to canonical documents.
- `README.md`: user-facing introduction and quick start; links to specifications instead of
  restating contracts.
- `CONTRIBUTING.md`: human contributor workflow (setup, change process, pull requests, licensing);
  links to the same canonical rules as `AGENTS.md` instead of duplicating them.
- `ROADMAP.md`: delivered scope, open work, and deferred scope; not a specification.
- `docs/README.md`
- `docs/product/vision.md`: product vision, scope boundaries, and high-level requirements.
- `docs/specs/`
- `docs/architecture/`
- `docs/development/`
- `docs/adr/`
- `docs/plans/README.md`
- `docs/plans/template.md`
- `docs/plans/active/`
- `docs/plans/completed/` (only if D4 keeps completed plans)

Potential specifications include CLI behavior, configuration, schema versions, template discovery and loading, compilation, diagnostics, logging, and Web UI behavior.

Potential architecture documents include the system overview, template subsystem, compiler integration, and Web UI internals.

The final taxonomy must reflect actual conceptual boundaries discovered in the repository.

## 4. ADR Policy and Pruning

Many existing ADRs were written by AI agents to log every change. Several restate the product
premise, describe the current architecture, or record intermediate prototype decisions that were
reversed weeks later. This migration performs a one-time pruning of the ADR set.

### 4.1 ADR Admission Criteria

An ADR is warranted only when all of the following hold:

1. At least two viable alternatives were seriously considered.
2. The choice is costly to reverse, or it constrains a boundary, public contract, security
   property, or operational behavior.
3. The rationale cannot be derived from the product vision, a specification, or an obvious
   engineering default.

An ADR is not warranted for:

- Restating the product premise (for example, "use Typst" in a Typst rendering service).
- Describing the current structure, module layout, or behavior; that belongs in architecture
  documentation or specifications.
- Product scope and MVP boundaries; that belongs in the product vision and `ROADMAP.md`.
- Routine implementation choices, refactors, or a decision whose only alternative is a strawman.

### 4.2 One-Time Pruning Rules

Typrest is a pre-release prototype, and Git history is the archive. During this migration:

1. Delete ADRs that fail the admission criteria after moving any still-true content (constraints,
   consequences, rationale worth keeping) into its canonical specification, architecture document,
   or the product vision.
2. Delete ADRs fully superseded during the prototype phase. Rewrite the surviving ADR's context so
   it stands alone, without depending on the deleted record.
3. Merge an ADR that was only partly superseded into its successor, or rewrite it to state the
   current decision, when the split adds no historical value.
4. Remove `Supersedes`/`Superseded by` links that point to deleted ADRs.
5. Keep the original numbers of surviving ADRs; gaps are acceptable. Do not reuse deleted numbers.
6. Record every disposition (kept, rewritten, merged, deleted) and the destination of any moved
   content in the migration report.

After this migration, ADRs become append-only again: accepted decisions are superseded by new
records, not deleted or rewritten.

### 4.3 Proposed Disposition

This table is a proposal to confirm during the audit. Content marked "move" must be verified
against code and existing docs before the ADR is deleted.

| ADR | Title | Proposed disposition | Reason / destination of content |
|---|---|---|---|
| 0001 | Integrate Typst as a Rust library | Delete | Product premise. Move "single binary, compiler ACL, joint Typst pinning" to the compiler architecture document. |
| 0002 | Abstract template loading behind a trait | Delete | Architecture state (standard dependency inversion). Move the loader/registry boundary to architecture; already partly superseded by 0012. |
| 0003 | Package filesystem templates with the deployment | Delete | Product scope. Move "templates are deployment artifacts; no Store mode, tags, or publishing" to the product vision and `ROADMAP.md`. Drop the stale Docker note after checking `ROADMAP.md`. |
| 0004 | Resolve `_shared/` during loading | Keep | Real placement choice with a watch-mode consequence. |
| 0005 | Inject request data through `sys.inputs` | Keep | Real choice affecting the template authoring contract. The `sys.inputs.data` snippet already lives in `docs/typst-integration.md`; keep it in whichever spec or architecture document absorbs that file. |
| 0006 | Restrict template names to safe ASCII | Delete | Fully superseded by 0010 during the prototype. |
| 0007 | Expose Typrest-owned compiler diagnostics | Keep | Real boundary decision. |
| 0008 | Embed the tracked development Web UI | Keep, merge 0009 | Rewrite as the single Web UI decision: committed `webui/dist`, embedded assets, `web_ui` gates only static routes, metadata routes always mounted. |
| 0009 | Gate static Web UI routes, not metadata routes | Merge into 0008, delete | Its route paths are already superseded by 0010; the remaining gating choice fits 0008. Preserve its security consequence in the HTTP specification: metadata routes are internal and unversioned, and deployments requiring confidentiality must protect the server with network controls. |
| 0010 | Derive template IDs and routes from folders | Keep, rewrite links | Remove references to 0006 and 0009; keep context standalone. |
| 0011 | Resolve preview packages during preparation | Keep | Real decision with rejected alternatives. |
| 0012 | Consolidate application packages | Keep, rewrite links | Remove the ADR-0002 reference. |
| 0013 | Separate pre-build feedback from backend logs | Delete | Fully superseded by 0014 during the prototype. |
| 0014 | Use one stdout tracing stream | Keep, rewrite context | Describe the previous two-stream design without linking 0013. |

Result: 14 ADRs reduced to 8 (0004, 0005, 0007, 0008, 0010, 0011, 0012, 0014).

## 5. Phase 1 — Audit

Inspect:

- Root documentation: `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, and `ROADMAP.md`
- All Markdown files under `docs/`
- All existing ADRs, including their statuses and relationships, against the admission criteria in section 4.1
- Source code relevant to documented behavior
- Unit, integration, and end-to-end tests
- Build, lint, and test commands
- Existing scripts and CI checks related to documentation

Produce an audit report, appended to this file as an "Audit Report" section, containing:

- Existing file inventory
- Proposed classification and destination for each file
- Confirmed ADR disposition table (section 4.3), with the destination of any content moved out of a deleted ADR
- Overlapping and duplicated content
- Contradictory requirements or decisions
- Outdated or unsupported claims
- Important behavior that lacks documentation
- Requirements that can be inferred with confidence
- Questions requiring human decisions
- Proposed requirement ID namespaces
- Proposed final documentation tree
- Proposed answers to the [Decisions Needed](#14-decisions-needed)

Do not modify existing documentation or application code during this phase, apart from the approved migration plan and audit report.

Do not treat missing evidence as proof that a behavior does not exist.

## 6. Phase 2 — Establish the Standard

Create or update:

- `docs/README.md`
- `docs/adr/README.md`
- `docs/adr/template.md`
- `docs/plans/README.md`
- `docs/plans/template.md`
- `docs/specs/README.md`, if useful
- `docs/architecture/README.md`, if useful
- `docs/development/README.md`, if useful
- A concise documentation authoring guide

Define conventions for the following document types, and templates for those selected in D3:

- Product vision
- High-level requirements
- Specifications
- Architecture documentation
- ADRs
- Development guides
- Implementation plans

Define:

- Requirement ID format and lifecycle
- Normative requirement language
- Document status conventions
- Cross-reference conventions
- Source-of-truth rules
- Rules for resolving contradictions
- Rules for superseding ADRs
- Documentation update requirements for code changes

Move the rules from this plan that must outlive it into `docs/adr/README.md`:

- The ADR admission criteria (section 4.1).
- The append-only rule that applies after the migration (end of section 4.2).
- The rule that agents propose an ADR and obtain human approval before creating one.
- The approved ADR status set (see [Decisions Needed](#14-decisions-needed)).

Put the documentation authoring conventions in `docs/README.md`, extending its existing
"Documentation Conventions" section, unless the audit shows a separate guide is clearly needed.

Do not migrate all existing documents until the standard and audit report have been reviewed.

## 7. Phase 3 — Migrate Existing Documentation

For each existing document:

1. Read the complete relevant content.
2. Identify its purpose and canonical destination.
3. Preserve meaningful requirements, examples, edge cases, constraints, and rationale.
4. Separate normative behavior from implementation details and historical context.
5. Assign stable requirement IDs where appropriate.
6. Link related specifications, architecture documents, and ADRs.
7. Update every reference after moving or renaming files, including links and anchors in
   `AGENTS.md`, `CONTRIBUTING.md`, `README.md`, and `ROADMAP.md`, in the same commit as the move.
   Agents rely on `AGENTS.md` throughout the migration, so its links must never stay broken
   until Phase 5.
8. Remove duplication only after confirming that the canonical destination preserves the information.
9. Record unresolved conflicts rather than choosing arbitrarily.
10. Include the original file in the migration report.

Apply the approved ADR dispositions from section 4.3 using the pruning rules in section 4.2. Move
still-true content into its canonical destination before deleting an ADR. Do not renumber
surviving ADRs.

Keep the history reviewable:

- Move files with `git mv`.
- Commit pure moves and renames separately from content edits, so Git tracks the rename and the
  content diff stays readable.
- Use at least one commit per phase.

## 8. Phase 4 — Validate Consistency

Check that:

- Every normative requirement within the scope chosen in D1 has a unique, stable ID.
- Requirement IDs are not reused after removal.
- Requirements have one canonical definition.
- Specifications describe current expected behavior.
- Architecture documentation describes the current design.
- ADRs explain decisions rather than serving as the sole source of current behavior.
- Plans clearly distinguish current behavior from intended changes.
- Examples agree with their associated requirements.
- Cross-references resolve to existing files and sections.
- No important content was accidentally lost during migration.
- Contradictions and unknowns are documented.
- The documentation does not claim tests or implementation behavior that has not been verified.
- The document templates are followed wherever applicable.
- The documentation index matches the actual repository.

Run these checks:

- Verify links and anchors with the method chosen in [Decisions Needed](#14-decisions-needed).
- Search the repository for references to deleted ADRs (`ADR-0001`, `ADR-0002`, `ADR-0003`,
  `ADR-0006`, `ADR-0009`, `ADR-0013`, and their file names); none may remain.
- Search for references to `DOCUMENTATION_MIGRATION`; none may remain outside this file.
- Confirm that `git diff --stat main -- ':!*.md'` is empty. No source code references `docs/`, so
  Rust and Web UI test suites cannot detect documentation regressions; this diff check is what
  proves the migration stayed documentation-only.
- Run `git diff --check`.

## 9. Phase 5 — Update Agent Instructions

Rewrite `AGENTS.md` to describe:

- Project identity and purpose
- Documentation navigation
- Source-of-truth rules
- Required preparation before non-trivial changes
- Documentation update requirements
- ADR creation criteria, linking to `docs/adr/README.md` rather than restating them. Agents must
  propose an ADR and get human approval before creating one; they must not create ADRs to log
  routine changes.
- Plan creation criteria
- Project-specific coding conventions
- Canonical build, test, formatting, and lint commands

Keep detailed requirements, architectural explanations, and subsystem instructions in their canonical documents.

Add nested agent instruction files only when a subsystem has genuinely distinct rules that need to apply within that directory.

Update `CONTRIBUTING.md` in the same phase so its change process and ADR guidance match
`AGENTS.md` and link to the same canonical documents.

## 10. Phase 6 — Final Review

Produce a final report, appended to this file as a "Final Report" section and copied into the
pull request description before this file is deleted, containing:

- Final documentation tree
- Files created, moved, merged, renamed, or removed
- Mapping from every original document to its final destination
- Requirement ID namespaces and counts
- Validation commands and their results
- Unresolved questions and known documentation gaps
- Contradictions that require product-owner decisions
- Any information that could not be confidently migrated
- Recommended follow-up work

Review the final Git diff for accidental source-code changes, lost content, unnecessary duplication, and unrelated modifications.

## 11. Acceptance Criteria

The migration is complete only when:

- [ ] The documentation hierarchy has a clearly defined purpose for each document category.
- [ ] All existing documentation has been inventoried and accounted for.
- [ ] Every original ADR has a recorded disposition, and still-true content from deleted ADRs lives in its canonical document.
- [ ] Every surviving ADR meets the admission criteria and stands alone without links to deleted ADRs.
- [ ] The new templates and conventions are documented.
- [ ] Current behavior is documented without silently inventing requirements.
- [ ] Known contradictions and ambiguities are explicitly recorded.
- [ ] Normative requirements use stable IDs and consistent language.
- [ ] Specifications, architecture, ADRs, and plans have distinct responsibilities.
- [ ] Internal links and documentation indexes have been validated.
- [ ] `AGENTS.md` is concise, accurate, and actionable.
- [ ] Existing application behavior and source code remain unchanged unless a separate, explicitly approved change is required.
- [ ] The final migration report has been reviewed.
- [ ] All blocking decisions have been resolved or explicitly left for human approval.
- [ ] The lasting rules from this plan live in `docs/adr/README.md` and `docs/README.md`.
- [ ] No file references `DOCUMENTATION_MIGRATION.md` and it can be deleted when user decide.
## 12. Out of Scope

- Implementing new product features
- Refactoring application code
- Changing APIs or runtime behavior
- Making architectural decisions that have not been approved
- Adopting a new specification framework or external tool without explicit justification
- Creating documentation for hypothetical future features

## 13. Working Agreement

Execute the migration in phases.

At the end of each phase, summarize the work and identify the decisions needed before proceeding.

Do not interpret permission to migrate documentation as permission to invent product requirements or modify application behavior.

## 14. Decisions Needed

Resolve these before Phase 2. The audit report proposes an answer for each; record the approved
answer here.

| # | Decision | Recommendation |
|---|---|---|
| D1 | Requirement ID scope | Assign IDs only to externally observable contracts: schema v1, CLI, `typrest.toml`, and the HTTP surface. Architecture documents and guides have no IDs. |
| D2 | Requirement ID registry | Keep IDs unique per namespace within each specification. Mark removed IDs as removed in place instead of keeping a separate registry. |
| D3 | Document templates | Provide templates only for ADRs, plans, and specifications. Vision, architecture, and development documents follow the conventions in `docs/README.md` without a template. |
| D4 | Completed plans | Delete plans when they are completed; Git history is the archive, consistent with the ADR pruning policy. Drop `docs/plans/completed/`. |
| D5 | Normative language | Keep lowercase `must`/`must not`/`should`/`may`, as `docs/README.md` already does, instead of uppercase RFC 2119 keywords. |
| D6 | ADR status set | Use Proposed, Accepted, Superseded, and Rejected. Drop "In discussion" (replaced by Proposed) and "Superseded in part" (removed by the merges in section 4.3). |
| D7 | Link validation method | Use a small, committed shell or script check for relative links and anchors, run manually during the migration; decide separately whether to add it to CI. Adopting an external tool such as `lychee` requires justification under section 12. |