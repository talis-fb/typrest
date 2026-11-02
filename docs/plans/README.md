# Implementation Plans

A plan describes intended work that is not finished. It is temporary: it separates the intended
change from current behavior, which stays in the [specifications](../specs/README.md) and
[architecture documents](../architecture/overview.md).

## When to write a plan

Write a plan for work that spans several changes, needs sequencing or coordination, or needs review
before implementation starts. A single focused change does not need one. A plan is not an ADR:
record a lasting architectural choice in an [ADR](../adr/README.md), after human approval.

## Rules

- One file per plan, named `short-topic.md`, based on [`template.md`](template.md).
- State current behavior and intended behavior separately. Link to the specification that describes
  the current behavior; do not restate it.
- Do not invent requirements. Record open questions explicitly.
- A plan never becomes the source of truth. When the work lands, move lasting rules into the
  specification, architecture document, or ADR.
- Delete the plan when its work is completed. Git history is the archive; there is no `completed/`
  directory.

## Active plans

None. Planned work is tracked in GitHub issues.
