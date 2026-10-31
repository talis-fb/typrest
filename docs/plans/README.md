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

All plans are Draft and independent of one another except for the dependencies below. Each one
starts with, or contains, an approval gate for any ADR or scope change it proposes.

| Plan | Subject | Needs first |
|---|---|---|
| [`operational-limits.md`](operational-limits.md) | Body limit, graceful shutdown, render timeout | Steps 1-4 are independent; the timeout needs the plan above |
| [`render-metrics.md`](render-metrics.md) | Prometheus metrics and a core render observer | ADR approval |
| [`render-result-cache.md`](render-result-cache.md) | Opt-in in-memory cache of rendered PDFs | Scope change and ADR approval |
| [`async-render-jobs.md`](async-render-jobs.md) | Asynchronous render jobs with polling and idempotency | Scope change, ADR approval, and the concurrency limiter |
| [`pdf-output-destinations.md`](pdf-output-destinations.md) | Deliver PDFs to filesystem or S3 destinations | Scope change and ADR approval |

### Cross-plan rules

- Requirement IDs written in a plan are provisional. Several plans propose the same
  `REQ-HTTP-070` and `REQ-CFG-050` ranges. The plan implemented first claims the next free IDs in
  the specification; later plans renumber to the next free IDs at that time. Never reuse an ID.
- Configuration placement (`typrest.toml` versus `serve` flags) is decided across plans, not per
  plan; resolve it before the first plan that adds configuration is implemented.
- Overload status (`503` versus `429`) and its error code are shared by the concurrency limiter and
  async jobs; settle them once. The concurrency limiter is delivered with `503` and
  `render_capacity_exceeded` (see [HTTP API](../specs/http-api.md#render-capacity)).
