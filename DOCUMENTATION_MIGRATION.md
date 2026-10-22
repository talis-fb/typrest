# Documentation Standard Migration

Status: Executed; awaiting review  
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
- `docs/plans/`: active implementation plans, one file each, deleted when completed (D4).

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
- The approved ADR status set (D6 in [Approved Decisions](#14-approved-decisions)).

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

- Verify links and anchors with the link check script from D7.
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

- [x] The documentation hierarchy has a clearly defined purpose for each document category.
- [x] All existing documentation has been inventoried and accounted for.
- [x] Every original ADR has a recorded disposition, and still-true content from deleted ADRs lives in its canonical document.
- [x] Every surviving ADR meets the admission criteria and stands alone without links to deleted ADRs.
- [x] The new templates and conventions are documented.
- [x] Current behavior is documented without silently inventing requirements.
- [x] Known contradictions and ambiguities are explicitly recorded.
- [x] Normative requirements use stable IDs and consistent language.
- [x] Specifications, architecture, ADRs, and plans have distinct responsibilities.
- [x] Internal links and documentation indexes have been validated.
- [x] `AGENTS.md` is concise, accurate, and actionable.
- [x] Existing application behavior and source code remain unchanged unless a separate, explicitly approved change is required.
- [ ] The final migration report has been reviewed.
- [x] All blocking decisions have been resolved or explicitly left for human approval.
- [x] The lasting rules from this plan live in `docs/adr/README.md` and `docs/README.md`.
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

## 14. Approved Decisions

These decisions are approved. Phase 2 writes the lasting ones into the permanent documentation.

- **D1 — Requirement ID scope.** Assign requirement IDs only to externally observable contracts:
  schema v1, the CLI, `typrest.toml`, and the HTTP surface. Architecture documents and guides have
  no IDs.
- **D2 — Removed IDs.** IDs are unique within their namespace. When a requirement is removed, keep
  its ID in place marked as removed (for example, `SCHEMA-012: (removed)`); there is no separate
  registry. Removed IDs are never reused.
- **D3 — Document templates.** Provide templates only for ADRs, plans, and specifications. Vision,
  architecture, and development documents follow the conventions in `docs/README.md`.
- **D4 — Completed plans.** Delete a plan when it is completed; Git history is the archive. There
  is no `docs/plans/completed/` directory.
- **D5 — Normative language.** Use lowercase `must`, `must not`, `should`, and `may`, not
  uppercase RFC 2119 keywords.
- **D6 — ADR statuses.** Use Proposed, Accepted, Superseded, and Rejected. "In discussion" becomes
  Proposed; "Superseded in part" is no longer used.
- **D7 — Link validation.** Commit a small script that checks relative links and anchors, and run
  it manually during the migration. Whether to add it to CI is a separate decision.

## 15. Audit Report

Phase 1 output. Nothing outside this file was modified. Evidence: every Markdown file in the
repository, `crates/typrest/src/http/{router,errors}.rs`, `.github/workflows/ci.yml`, and the file
listing of the repository root.

### 15.1 Existing file inventory and proposed destinations

| Current file | Lines | Kind today | Proposed destination | Action |
|---|---|---|---|---|
| `README.md` | 200 | User intro + quickstart + restated contracts | `README.md` | Keep. Replace the HTTP API section with a short summary linking the HTTP spec. |
| `AGENTS.md` | 123 | Agent entry point | `AGENTS.md` | Rewrite in Phase 5. |
| `CONTRIBUTING.md` | 158 | Contributor workflow | `CONTRIBUTING.md` | Keep. Align with `AGENTS.md` in Phase 5. |
| `ROADMAP.md` | 92 | Status | `ROADMAP.md` | Keep. Fix links; add the Store-mode and packaging scope moved out of ADR-0003. |
| `docs/README.md` | 48 | Index + conventions | `docs/README.md` | Rewrite: new index and authoring conventions. |
| `docs/architecture.md` | 160 | Architecture, domain terms, HTTP route table | `docs/architecture/overview.md` | `git mv`, then move the HTTP route table to the HTTP spec. |
| `docs/startup-template-compilation.md` | 108 | Architecture with some observable behavior | `docs/architecture/template-lifecycle.md` | `git mv`. Replace the duplicated warmup-payload rules with a link to the schema spec. |
| `docs/typst-integration.md` | 111 | Implementation reference | `docs/architecture/compiler.md` | `git mv`. Absorbs the ADR-0001 residue and the single Typst-upgrade checklist. |
| `docs/cli-spec.md` | 174 | Spec | `docs/specs/cli.md` | `git mv`, add IDs. |
| `docs/typrest-toml-spec.md` | 112 | Spec | `docs/specs/typrest-toml.md` | `git mv`, add IDs. |
| `docs/schema-v1.md` | 212 | Spec | `docs/specs/schema-v1.md` | `git mv`, add IDs. |
| `docs/diagnostics.md` | 104 | Spec plus duplicated Typst-upgrade steps | `docs/specs/diagnostics.md` | `git mv`. Move the upgrade checklist to `compiler.md`. The HTTP body example becomes a link to the HTTP spec. |
| `docs/logging.md` | 75 | Spec | `docs/specs/logging.md` | `git mv`. |
| `docs/webui-spec.md` | 209 | Spec mixing HTTP routes and client behavior | `docs/specs/webui.md` | `git mv`. The route and security contract moves to the HTTP spec. Client behavior stays. |
| `docs/webui-development.md` | 194 | Guide | `docs/development/webui.md` | `git mv`. Trim the "Use the Workspace" section that restates the spec. |
| `docs/testing.md` | 112 | Guide | `docs/development/testing.md` | `git mv`. |
| `docs/adr/README.md`, `template.md` | 46, 23 | ADR process | same paths | Rewrite for D6 and the lasting rules from section 4. |
| `docs/adr/0001`–`0014` | — | ADRs | see 15.2 | 14 → 8. |
| `.github/workflows/ci.yml` | — | CI | unchanged | No documentation job exists. D7 keeps the link check manual. |
| `DOCUMENTATION_MIGRATION.md` | — | Plan | deleted at the end | — |
| `opencode.json` (untracked) | — | Unrelated tool config | untouched | Not part of the migration. Do not stage it. |
| `Dockerfile`, `docker-compose.yml` | — | Obsolete, non-Markdown | untouched | Out of scope. Docs already flag them as obsolete, and ROADMAP tracks the fix. |

There is no existing documentation script. CI runs only `cargo fmt`, `clippy` and `test`; it does not
run the Web UI checks.

### 15.2 Confirmed ADR dispositions

The proposal in 4.3 holds. The notes below add what the audit found.

| ADR | Disposition | Content move before deletion / rewrite notes |
|---|---|---|
| 0001 | Delete | Single binary, compiler ACL and joint pinning already live in `architecture.md` and `typst-integration.md`. Add one short "why in-process Typst" paragraph to the vision. Reasons: browser automation is heavy, language-specific PDF libraries couple callers to one ecosystem, a child process adds a binary dependency and per-render overhead. |
| 0002 | Delete | Loader/registry boundary is already in `architecture.md`. The statement "handlers never receive a loader" survives in ADR-0012 and `AGENTS.md`. |
| 0003 | Delete | Vision gets "templates are deployment artifacts". `ROADMAP.md` already defers Store mode. The Docker note is already an open item in `ROADMAP.md`. |
| 0004 | Keep | No change. |
| 0005 | Keep | The plan says keep. Its only alternative is a temp file, which is close to the "strawman" exclusion in 4.1. See question Q6. |
| 0006 | Delete | Fully superseded. The safe-ASCII rule and its examples (`invoice-v2` valid, `../invoice` invalid) are already carried by ADR-0010 and `architecture.md`. Put the examples in the template-layout spec. |
| 0007 | Keep | Replace "three Typst crates" with the five pinned crates. |
| 0008 | Rewrite, absorb 0009 | One Web UI decision. Context: no runtime asset directory, no npm inside Cargo. Decision: committed `webui/dist`, embedded assets, `web_ui` gates only the static routes, metadata routes always mounted. Use the current `/_webui` paths, not `/webui`. |
| 0009 | Delete | Its security consequence moves to the HTTP spec: metadata routes are internal, unversioned, and unauthenticated, so confidential deployments need network controls. This text already exists in `webui-spec.md`, `typrest-toml-spec.md` and `webui-development.md`. |
| 0010 | Rewrite header | Remove both `Supersedes` lines and any 0006/0009 wording. Keep the reference to ADR-0004. |
| 0011 | Keep | No change. |
| 0012 | Rewrite last bullet | Replace the ADR-0002 bullet with a standalone statement: the `TemplateLoader` abstraction lives in `typrest-core`, and the filesystem implementation lives in the application package. |
| 0013 | Delete | Fully superseded. |
| 0014 | Rewrite context | Describe the earlier design directly: pre-build feedback on stderr, running-server logs on stdout. Drop the `Supersedes` line and the ADR-0013 link. |

References to deleted ADRs found by the audit:

- `docs/README.md`: no direct 0001/0002/0003/0006/0009/0013 links. It links 0011 and 0012 as the "documentation" for package resolution and workspace boundaries (see 15.4).
- `docs/adr/0003`, `0010`, `0012`, `0014`: contain links to deleted records.
- `docs/adr/0013`: links `logging.md`. It is deleted anyway.
- `docs/adr/README.md`: index rows and the "Superseded in part" statuses.

No source or non-Markdown file references `docs/` or any ADR.

### 15.3 Overlap and duplication

| Topic | Currently stated in | Canonical home |
|---|---|---|
| Route table, `web_ui` gating, metadata routes always mounted | `architecture.md`, `typrest-toml-spec.md`, `webui-spec.md`, `webui-development.md`, `README.md`, `AGENTS.md`, ADR-0009 | New HTTP spec. Others link. |
| "No auth, bind to loopback" | `typrest-toml-spec.md`, `webui-spec.md`, `webui-development.md`, `README.md`, ADR-0009 | New HTTP spec, Web UI spec for the UI-specific warning. |
| Warmup sample payload precedence | `schema-v1.md` and `startup-template-compilation.md` (identical lists) | `schema-v1.md`. |
| Typst upgrade checklist | `diagnostics.md` and `typst-integration.md` | `compiler.md`. |
| Exact version pin of Typst crates | `architecture.md`, `typst-integration.md`, `diagnostics.md`, ADR-0007, `AGENTS.md` | `compiler.md` states it. `AGENTS.md` keeps a one-line rule. |
| `sys.inputs.data` snippet | `README.md`, `cli-spec.md`, `typst-integration.md`, ADR-0005 | `compiler.md` for the mechanism. `README.md` and `cli-spec.md` keep their examples because the scaffold output is part of the CLI spec. |
| Template discovery rules | `README.md`, `architecture.md`, `cli-spec.md`, ADR-0010, `AGENTS.md` | New template-layout spec. |
| Package resolution rules | `README.md`, `cli-spec.md`, `typst-integration.md`, ADR-0011 | Template-layout spec (author-visible rules). `compiler.md` keeps the mechanism. ADR-0011 keeps the rationale. |
| Validation commands | `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, `testing.md`, `webui-development.md` | `development/testing.md`. Others keep a short list or link. |
| Stream and event contract | `logging.md`, `diagnostics.md`, `cli-spec.md`, ADR-0014 | `logging.md`. |
| Registry/startup lifecycle | `architecture.md` and `startup-template-compilation.md` | `template-lifecycle.md`. `overview.md` keeps a diagram and a link. |
| Web UI use and behavior | `webui-spec.md` and `webui-development.md` | `webui.md` spec. The guide keeps only setup, local dev, troubleshooting. |

### 15.4 Contradictions and inconsistencies

1. **Pin count.** `diagnostics.md` says "update all three exact versions" and ADR-0007 says "three Typst crates", while both name five pinned crates (`typst`, `typst-pdf`, `typst-assets`, `typst-kit`, `typst-syntax`). `architecture.md`, `typst-integration.md` and `AGENTS.md` list five. Resolve to five.
2. **HTTP surface in `AGENTS.md`.** It lists the routes but omits `GET /{*template_id}`. The router (`router.rs`) mounts it, and `architecture.md`, `README.md` and `webui-spec.md` document it. The router is the truth.
3. **ADR-0009 route names.** It documents `/webui`, but the router mounts `/_webui` (changed by ADR-0010). It is being merged anyway.
4. **ADR README.** The "Adding a Decision" example uses `0010-...` as the next number, which is stale. The status set includes "In discussion" and "Superseded in part", which D6 removes.
5. **Docs index uses ADRs as the only reference** for package resolution (ADR-0011) and workspace boundaries (ADR-0012). That breaks principle 8. Current behavior needs a spec or architecture home.
6. **`docs/README.md` conventions** say "Use `must` for normative requirements" but do not define `should` or `may`. D5 needs the fuller convention.

No contradiction between code and docs on routes or error codes. I checked `router.rs` and `errors.rs`: `template_not_found` (404), `schema_validation_failed` (422), `compilation_failed` (500), and `invalid_template_id` / `invalid_json` (400) all match the docs.

### 15.5 Outdated or unsupported claims

- `webui-spec.md` and `webui-development.md` say "Rust and Docker builds do not invoke Node". The Docker files are obsolete, so the Docker half is unverified. Reword to Cargo builds only.
- `testing.md` says "CI configuration remains authoritative for required jobs". CI runs no Web UI or documentation checks, so the Web UI commands are conventions, not CI-enforced.
- `README.md` and `testing.md` say the Docker files are obsolete. Accurate today. The statement is removed once ROADMAP closes that item, and not before.
- `typst-integration.md` says the architecture test rejects imports outside `compiler/`. I did not verify the test exists (Phase 4 task).
- `webui-spec.md` "Acceptance Criteria" asserts test-like facts (blob revocation, content types). I have not verified each against the frontend tests. Phase 4 must either verify them or reword them as requirements.
- `testing.md` names focused commands such as `cargo test -p typrest --test check`. Verify each runs before keeping it.

### 15.6 Important behavior lacking documentation

- **No HTTP spec.** The render contract (`GET`/`POST /{*template_id}`, 200 with `application/pdf`, bodyless GET/POST behavior, 400 `invalid_json`, 400 `invalid_template_id`, 404, 422, 500 envelope) is described only in `README.md` and in fragments of `webui-spec.md` and `diagnostics.md`. The `/_healthz` response body appears only in `README.md`.
- **Template layout and discovery** has no spec. It is spread over `README.md`, `architecture.md`, `cli-spec.md` and ADR-0010: segment character rules, the `_` prefix, `templates/main.typ` being invalid, nested `main.typ`, `_shared/` precedence, `schema.json` vs `schema.yaml` handling, custom font discovery (lowercase `.ttf`/`.otf`).
- **Package support** has no spec: literal fully versioned `@preview` only, cache location, failure behavior in `check`, startup and watch.
- **Behavior I could not confirm from code in this phase** and will not document without checking: maximum payload size, request content-type enforcement, the response when a template is removed in watch mode, and HTTP response headers for PDFs other than `Content-Type`.

### 15.7 Requirements inferable with confidence

These are already stated normatively in current documents, so they can become IDs without invention:

- Schema v1 rules: file selection, document shape, field table, required/default/enum/type semantics, validation order, load-time errors, sample payload derivation.
- `typrest.toml` fields, `templates_root` resolution and rejection rules, `web_ui` effects.
- CLI syntax, `init` outputs and failure conditions, `check`/`serve` exit status and options, discovery from ancestors.
- HTTP routes, availability by `web_ui`, error codes and statuses, safe-diagnostic rule (no source text or physical paths).

### 15.8 Questions requiring a human decision

| # | Question | Recommendation |
|---|---|---|
| Q1 | D1 limits IDs to schema, CLI, `typrest.toml` and HTTP. Template layout/discovery and package imports are also externally observable contracts. Add a `TEMPLATE` namespace and a `docs/specs/templates.md`? | Yes. It is the largest uncovered contract, and ADR-0010 and ADR-0011 currently carry it. |
| Q2 | Create `docs/specs/http-api.md` as the single home for routes, errors, health and metadata? | Yes (see 15.6). |
| Q3 | Web UI client behavior (form controls, mode conversion, Blob lifecycle) falls outside D1. Keep it as an ID-free spec? | Yes. Its static routes and caching go to the HTTP spec with IDs. |
| Q4 | Language and location of the link-check script (D7). | A single-file Python 3, standard-library-only script at `scripts/check-doc-links.py`. The repo has no Python today, so the alternative is a small Node script under `webui/`, which ties documentation to the frontend toolchain. |
| Q5 | Rewritten ADRs 0008 and 0014: keep the original date or use the rewrite date? | Keep the original decision date and add a one-line "Rewritten during the 2026 documentation migration" note. |
| Q6 | ADR-0005 and ADR-0007 are borderline against 4.1 (weak alternatives). Keep both as the plan says? | Keep. Both constrain a public contract (template authoring and diagnostic boundary). Revisit only if you want a stricter set. |
| Q7 | Fix the "three vs five crates" wording as part of migration even though it changes a stated number? | Yes. Code and the other documents agree on five. |

### 15.9 Proposed requirement ID namespaces

Format `NAMESPACE-NNN`, zero-padded, per D2. Counts are estimates from the current prose, to be fixed in Phase 3.

| Namespace | Spec | Est. count |
|---|---|---|
| `SCHEMA` | `docs/specs/schema-v1.md` | ~45 |
| `CONFIG` | `docs/specs/typrest-toml.md` | ~15 |
| `CLI` | `docs/specs/cli.md` | ~30 |
| `HTTP` | `docs/specs/http-api.md` | ~35 |
| `TEMPLATE` (pending Q1) | `docs/specs/templates.md` | ~25 |

`logging.md`, `diagnostics.md` and `webui.md` get no IDs under D1 unless you extend it.

### 15.10 Proposed final documentation tree

```text
AGENTS.md
README.md
CONTRIBUTING.md
ROADMAP.md
scripts/check-doc-links.py            # D7, location pending Q4
docs/
  README.md                           # index + authoring conventions
  product/
    vision.md
  specs/
    README.md
    template.md                       # spec template (D3)
    cli.md
    typrest-toml.md
    schema-v1.md
    templates.md                      # pending Q1
    http-api.md                       # pending Q2
    diagnostics.md
    logging.md
    webui.md
  architecture/
    README.md
    overview.md
    template-lifecycle.md
    compiler.md
  development/
    README.md
    testing.md
    webui.md
  adr/
    README.md
    template.md
    0004-… 0005-… 0007-… 0008-… 0010-… 0011-… 0012-… 0014-…
  plans/
    README.md
    template.md
```

`docs/specs/README.md` and `docs/development/README.md` are short indexes. `docs/architecture/README.md` is
optional and may be folded into `docs/README.md` if it ends up with fewer than ten lines.

### 15.11 Decisions needed before Phase 2

Answer Q1–Q7, or reply "accept recommendations". Phase 2 then creates the conventions and templates,
and Phase 3 migrates the files.

## 16. Final Report

All approved recommendations (Q1–Q7) were applied.

### Final documentation tree

```text
AGENTS.md  README.md  CONTRIBUTING.md  ROADMAP.md
scripts/check-doc-links.py
docs/
  README.md
  product/vision.md
  specs/      README.md template.md cli.md typrest-toml.md templates.md schema-v1.md
              http-api.md diagnostics.md logging.md webui.md
  architecture/  overview.md template-lifecycle.md compiler.md
  development/   README.md testing.md webui.md
  adr/        README.md template.md 0004 0005 0007 0008 0010 0011 0012 0014
  plans/      README.md template.md
```

`docs/architecture/README.md` was not created; the index in `docs/README.md` covers it.

### Original document → destination

| Original | Destination |
|---|---|
| `docs/architecture.md` | `docs/architecture/overview.md` (HTTP route table → `specs/http-api.md`) |
| `docs/startup-template-compilation.md` | `docs/architecture/template-lifecycle.md` (warmup rules → `SCHEMA-100`) |
| `docs/typst-integration.md` | `docs/architecture/compiler.md` (absorbs ADR-0001 residue and the upgrade checklist) |
| `docs/cli-spec.md` | `docs/specs/cli.md` (`CLI-*`; discovery → `specs/templates.md`) |
| `docs/typrest-toml-spec.md` | `docs/specs/typrest-toml.md` (`CONFIG-*`) |
| `docs/schema-v1.md` | `docs/specs/schema-v1.md` (`SCHEMA-*`) |
| `docs/diagnostics.md` | `docs/specs/diagnostics.md` (HTTP body → `HTTP-041..044`; upgrade steps → `compiler.md`) |
| `docs/logging.md` | `docs/specs/logging.md` |
| `docs/webui-spec.md` | `docs/specs/webui.md` (routes, caching, security → `HTTP-050..064`) |
| `docs/webui-development.md` | `docs/development/webui.md` |
| `docs/testing.md` | `docs/development/testing.md` |
| ADR 0001, 0002, 0003, 0006, 0013 | deleted; content per 15.2 (vision, `ROADMAP.md`, `compiler.md`, `templates.md`) |
| ADR 0008 + 0009 | merged into rewritten ADR-0008 |
| ADR 0010, 0012, 0014 | rewritten to stand alone |
| ADR 0004, 0005, 0007, 0011 | kept (0007: five crates) |
| `docs/adr/README.md`, `template.md`, `docs/README.md` | rewritten |
| `README.md`, `ROADMAP.md`, `CONTRIBUTING.md`, `AGENTS.md` | links repaired, duplicated contracts replaced by links |

New: `product/vision.md`, `specs/http-api.md`, `specs/templates.md`, `specs/README.md`, `specs/template.md`,
`development/README.md`, `plans/README.md`, `plans/template.md`, `scripts/check-doc-links.py`.

### Requirement IDs

| Namespace | Count |
|---|---|
| `SCHEMA` | 42 |
| `CONFIG` | 14 |
| `CLI` | 23 |
| `TEMPLATE` | 26 |
| `HTTP` | 28 |

No removed IDs exist yet. Counts differ from the estimates in 15.9 because requirements were grouped
by observable behavior.

### Validation results

| Check | Result |
|---|---|
| `python3 -I scripts/check-doc-links.py` | 0 broken links |
| `git diff --check` | clean |
| References to deleted ADRs (numbers and file names) | none outside this file |
| References to `DOCUMENTATION_MIGRATION` | only this file |
| `git diff --stat main -- ':!*.md'` | only `scripts/check-doc-links.py` (the D7 script, added on purpose); no source code changed |
| `cargo test --workspace` | all pass |
| `cargo test -p typrest-core compiler`, `-p typrest-core schema`, `-p typrest project`, `-p typrest http`, `-p typrest --test check` (the commands in `testing.md`) | all run and pass |
| `typrest --help`, `typrest serve --help` | match `CLI-001`, `CLI-002`, and the `serve` options table |
| Typst imports outside `compiler/` | none (`rg` check) |

Not run: `cargo fmt`/`clippy` (no Rust file changed) and the Web UI commands (no frontend change).

### Contradictions and corrections made

1. `README.md` claimed that a bodyless request to a schema-backed template returns the invalid-JSON
   error. The handler uses `{}` for any empty body and then validates it; `POST` without
   `Content-Type: application/json` returns `400 invalid_json`. The code was treated as truth
   (`HTTP-021`, `HTTP-022`). The behavior is covered by the `post_render_requires_json_content_type`
   test, but no test covers an empty-body request to a schema with required properties.
2. "Three" vs five pinned Typst crates: corrected to five (ADR-0007, `compiler.md`).
3. `AGENTS.md` omitted `GET /{*template_id}`; the section now links to the HTTP spec.
4. Docs and `testing.md` claimed an "architecture test" rejecting Typst imports outside `compiler/`.
   None exists. The docs now say so and give an `rg` check.
5. `TEMPLATE-014` records that any visited non-underscore directory with an invalid segment fails
   discovery, even without `main.typ`. The prior prose implied only template folders were checked.
   This is current code behavior, not verified by a dedicated test.

### Open questions and known gaps

- Not verified: maximum payload size; response when a watched template is removed;
  `HTTP-064` and `CLI-004` are taken from router code and earlier prose, without new tests.
- `webui.md` acceptance criteria were reduced to behaviors the spec itself describes; frontend tests
  under `webui/` were not audited against them.
- Whether to run the link check in CI is deferred (tracked in `ROADMAP.md`).
- Docker files remain obsolete and out of scope; `ROADMAP.md` still tracks them.
- `opencode.json` is untracked, unrelated, and was not touched.

### Recommended follow-up

- Add an automated test that rejects Typst imports outside `compiler/`, or drop the rule's claim of enforcement.
- Add an end-to-end test for the empty-body, required-property case.
- Decide on CI for the link check.

### Delete this file

Delete `DOCUMENTATION_MIGRATION.md` once the report is copied into the pull request description.
The permanent rules now live in `docs/README.md` and `docs/adr/README.md`.
