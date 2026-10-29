# Plan: Asynchronous Render Jobs

- **Status:** Draft
- **Owner:** Talison Fabio

## Objective

Today a client sends `POST /{template_id}` and Typrest answers only when compilation and PDF export
finish. For a heavy document this can exceed a client, proxy, or load-balancer timeout, and if the
connection drops, the finished PDF is lost and the client must submit the whole request again.

This plan lets a client submit a render, get an immediate `202 Accepted` with a job handle, and
retrieve the result later by polling. It also adds idempotency keys so a retry after a dropped
connection reuses the original job instead of duplicating work. The synchronous render contract
stays the default and is unchanged unless a client opts in.

The MVP is deliberately small: in-memory jobs, polling, idempotency, bounded queue and retention.
Durable storage, webhooks, multi-instance sharing, and external result destinations are later
phases that each need an explicit decision first.

## Current behavior

Contracts live in the [HTTP API specification](../specs/http-api.md); this section records only the
facts the plan depends on.

- **One synchronous path.** `GET` and `POST /{*template_id}` are served by one handler
  ([`render_template.rs:16`](../../crates/typrest/src/http/handlers/render_template.rs)). It parses the
  ID, checks the content type (line 32), parses the body (line 41), awaits
  `render_service.render(...)` (line 51), and either returns the PDF or maps the error. Success and
  failure are logged as `render.completed` (line 69) and `render.failed` (line 96).
  Requirements: `REQ-HTTP-020` to `REQ-HTTP-025`.
- **Lookup, validation, and compile are one call.** `RenderService::render`
  ([`render.rs:16-32`](../../crates/typrest-core/src/render.rs)) gets the `Arc<PreparedTemplateBundle>`
  from the registry (`TemplateNotFound`), validates and applies defaults (`ValidationError`), then
  calls `PdfCompiler::compile_prepared`. There is no way to validate without compiling.
- **The compile runs inline.** `TypstCompiler::compile_prepared`
  ([`compiler.rs:65-79`](../../crates/typrest-core/src/compiler.rs)) calls the synchronous
  `world::compile_prepared` ([`world.rs:50`](../../crates/typrest-core/src/compiler/world.rs)) inside
  an `async fn`. There is no spawn, queue, concurrency limit, or timeout today. These are the subject
  of the sibling plans listed under [Dependencies on other plans](#dependencies-on-other-plans).
- **Registry snapshots are `Arc` entries.** `TemplateRegistry::get`
  ([`registry.rs:124-130`](../../crates/typrest-core/src/registry.rs)) returns a cloned `Arc`; watch
  mode swaps entries or the whole map ([`registry.rs:141-151`](../../crates/typrest-core/src/registry.rs),
  [`watch.rs:172`](../../crates/typrest/src/loader/watch.rs)). A captured `Arc` is unaffected by later
  swaps.
- **Routes.** [`router.rs:9-19`](../../crates/typrest/src/http/router.rs) mounts `/_healthz`,
  `/_api/templates`, `/_api/templates/{*template_id}` and the catch-all `/{*template_id}`;
  `/_webui*` is mounted or answered with `404` depending on `web_ui` (lines 21-32). A path whose first
  segment begins with `_` is reserved and never a template (`REQ-HTTP-004`, `REQ-HTTP-003`,
  [ADR-0005](../adr/0005-derive-template-ids-from-folder-routes.md)); a reserved path that matches no
  route currently returns `400 invalid_template_id` (test
  `render_has_no_legacy_prefix_alias_and_rejects_reserved_ids`, `http.rs:623`).
- **State and server.** `AppState` holds the registry, `RenderService`, and `LogFormat`
  ([`state.rs:5-9`](../../crates/typrest/src/http/state.rs)). `serve` calls plain `axum::serve` with no
  graceful shutdown ([`http.rs:17-31`](../../crates/typrest/src/http.rs), line 28). Workspace `tokio`
  enables `macros`, `rt-multi-thread`, `net`, `signal`, `sync` only (no `time`).
- **Errors.** `ApiError` and `ErrorBody` (`error`, `message`, `violations`, `line`, `diagnostics`) in
  [`errors.rs:17,36,87`](../../crates/typrest/src/http/errors.rs) implement `REQ-HTTP-040` to
  `REQ-HTTP-044` and the safe-diagnostics rule.
- **Config.** `typrest.toml` has `schema_version`, `templates_root`, `web_ui`
  ([`config.rs:11-24`](../../crates/typrest/src/config.rs); `REQ-CFG-001` to `REQ-CFG-040`); unknown
  top-level fields are ignored (`REQ-CFG-003`); the spec says to update it before adding fields.
- **Security posture.** No authentication (`REQ-HTTP-050`, `REQ-HTTP-051`).
- **Scope boundaries.** The [product vision](../product/vision.md) excludes S3 and remote backends and
  states "Rendering requests must not read files or access the network"; `AGENTS.md` forbids direct
  filesystem access and HTTP dependencies in `typrest-core`.
- **Web UI** renders only through `POST /{*template_id}` and aborts the previous request
  (`REQ-WEB-070`, `REQ-WEB-074`; [`webui/src/api.ts:85`](../../webui/src/api.ts)).

## Intended change

### Overview

1. A client opts in per request with `Prefer: respond-async` on `POST /{*template_id}`. The server
   resolves the template, validates the payload (applying defaults) synchronously, and only then
   accepts the job. Unknown template, invalid ID, bad JSON, and schema violations return the same
   `400`/`404`/`422` as a synchronous render, immediately.
2. A valid request returns `202 Accepted` with `Location: /_jobs/{job_id}`. The work is queued behind
   the shared render concurrency limit.
3. `GET /_jobs/{job_id}` reports state; `GET /_jobs/{job_id}/result` returns the PDF (or the stored
   error envelope); `DELETE /_jobs/{job_id}` cancels or deletes.
4. An `Idempotency-Key` header makes a retried submission return the original job.
5. Jobs, results, and keys live in memory with a bounded queue, a result TTL, and a byte budget. They
   are lost on restart; this is documented, not hidden.
6. The feature is opt-in through `[jobs]` in `typrest.toml`. When disabled, behavior is exactly
   today's (the `Prefer` header is ignored, which RFC 7240 permits).

### API sketch

All error bodies use the existing envelope (`error`, `message`, optional detail fields). Status
codes and codes below are *proposed*.

**Submit**

```http
POST /financial/invoice HTTP/1.1
Content-Type: application/json
Prefer: respond-async
Idempotency-Key: 5d0c7b0e-2f0e-4c53-9a3a-1d1f0c4e8a11

{"customer":"Ada"}
```

```http
HTTP/1.1 202 Accepted
Location: /_jobs/0f8e3c1a-7d1b-4b6e-9a47-6c7d0b2e51aa
Preference-Applied: respond-async
Retry-After: 1
Content-Type: application/json

{
  "id": "0f8e3c1a-7d1b-4b6e-9a47-6c7d0b2e51aa",
  "template": "financial/invoice",
  "state": "queued",
  "createdAt": "2026-10-29T12:00:00Z",
  "startedAt": null,
  "finishedAt": null,
  "expiresAt": null,
  "result": null,
  "error": null
}
```

Re-submitting the same `Idempotency-Key` with the same template and payload returns `202` with the
same `Location` and the job's current body (never a second job). Same key with a different template
or payload returns `422 idempotency_key_mismatch`.

**Status**: `GET /_jobs/{job_id}` returns `200` with the body above. States are `queued`, `running`,
`succeeded`, `failed`, `cancelled`. `expired` is not a state: an expired job is removed and answers
`404 job_not_found` (see open question 6).

```json
{
  "id": "0f8e…",
  "template": "financial/invoice",
  "state": "succeeded",
  "createdAt": "2026-10-29T12:00:00Z",
  "startedAt": "2026-10-29T12:00:01Z",
  "finishedAt": "2026-10-29T12:00:09Z",
  "expiresAt": "2026-10-29T12:15:09Z",
  "result": { "href": "/_jobs/0f8e…/result", "contentType": "application/pdf", "size": 183204 },
  "error": null
}
```

For `failed`, `result` is `null` and `error` is exactly the object a synchronous render would have
returned as its body (for example `compilation_failed` with `diagnostics`), plus `status` (the HTTP
status the synchronous render would have used). No new diagnostic shape; the safe-diagnostics rule
(`REQ-HTTP-044`) applies unchanged.

**Result**: `GET /_jobs/{job_id}/result`

| Job state | Response |
|---|---|
| `succeeded` | `200`, `Content-Type: application/pdf`, PDF bytes, `Cache-Control: no-store`. Repeatable until expiry, so a dropped download can be retried. |
| `failed` | The stored error status and envelope, identical to the synchronous failure. |
| `queued`, `running` | `409 job_not_ready` with `Retry-After`. |
| `cancelled` | `409 job_cancelled`. |
| unknown or expired | `404 job_not_found`. |

**Cancel or delete**: `DELETE /_jobs/{job_id}` returns `204`. A `queued` job becomes `cancelled`
and never runs. A `running` job is marked `cancelled` and its output is discarded when the compile
returns (a Typst compile cannot be interrupted mid-run; the concurrency permit is held until it
ends). A finished job is removed with its result and idempotency key. Deleting an unknown job is
`404`.

**Other new errors (proposed)**

| Condition | Status | `error` |
|---|---|---|
| Job queue is full | `503` + `Retry-After` | `job_queue_full` |
| Retained-result byte budget exhausted | `503` + `Retry-After` | `job_storage_full` |
| Malformed `Idempotency-Key` (empty, over 255 bytes, non-printable ASCII) | `400` | `invalid_idempotency_key` |
| Idempotency key reused with different input | `422` | `idempotency_key_mismatch` |

There is no `GET /_jobs` listing route by design (see security below).

### Job lifecycle and concurrency

```text
POST + Prefer: respond-async
  -> parse ID/body (as today)
  -> RenderService::prepare: registry lookup + validate/defaults   (404/422 here, synchronously)
  -> idempotency lookup / queue admission                          (503 here if full)
  -> job `queued` holding Arc<PreparedTemplateBundle> + validated payload
  -> worker acquires the shared render permit  -> `running`
  -> compile_prepared                          -> `succeeded` | `failed` | `cancelled`
  -> result retained until TTL                 -> removed (`expired`)
```

- **Snapshot at accept time.** The job keeps the `Arc` returned at accept time, so a watch reload
  while queued does not change what is rendered, and the validation outcome returned to the client
  stays truthful. The cost is that a replaced bundle stays in memory until the job finishes.
- **Shared limiter.** Workers draw from the same render concurrency limit as synchronous renders
  (provided by `blocking-render-execution`), so asynchronous work cannot starve or exceed the
  configured CPU budget. A configurable `jobs.max_concurrent` caps the share jobs may take.
- **Bounded memory.** Queue capacity bounds queued payloads; `max_retained_bytes` bounds stored
  PDFs. Unfetched results are never evicted early; new submissions are rejected instead.
- **Timeouts and shutdown.** A per-render timeout is defined by `operational-limits` and a timed-out
  job ends `failed` with that plan's error code. On shutdown the server stops accepting jobs, lets
  running jobs finish within the drain deadline while still serving `/_jobs/*`, and drops queued
  jobs (logging how many). Persisting the queue is a later phase.
- **Sync render on client disconnect.** Unchanged in the MVP: synchronous results are not retained.
  A render that is still queued for a permit when the client disconnects SHOULD be cancelled by
  `blocking-render-execution`; a running one cannot be interrupted. Resilience to dropped
  connections is delivered by the asynchronous path plus idempotency keys.

### Domain and module layout

Core stays free of HTTP, tokio, and filesystem access:

- `crates/typrest-core/src/render.rs`: split `RenderService::render` into
  `prepare(&self, RenderRequest) -> Result<PreparedRender, RenderError>` (lookup, validate, defaults;
  holds the `Arc` and validated payload) and `execute(&self, PreparedRender) -> Result<Document,
  RenderError>`; `render` becomes `prepare` then `execute`, behavior unchanged.
- New `crates/typrest-core/src/job.rs` and `job/store.rs`: `JobId`, `JobState`, `Job`,
  `IdempotencyKey`, `JobStore` trait and an in-memory implementation (std `Mutex`, injected `Clock`
  trait for tests). Pure state machine: `submit`, `claim_next`, `complete`, `fail`, `cancel`,
  `delete`, `sweep_expired`. No spawning.
- New `crates/typrest/src/jobs.rs` (private module, edition-2018 style): the tokio runtime side -
  worker tasks, permit acquisition, TTL sweeper, shutdown drain. The only place that needs tokio
  `time`.
- `crates/typrest/src/http/handlers/jobs.rs`: status, result, delete handlers.
  `render_template.rs` gains the `Prefer` branch; the shared parsing code is factored out so the
  sync and async paths cannot diverge.
- `AppState` gains `jobs: Option<JobService>`.

### Output boundary (coupling with `pdf-output-destinations`)

A succeeded job must not assume the PDF is a `Bytes` in memory. Define one narrow port, owned by
whichever of the two plans lands first (this plan proposes the shape; see
[`pdf-output-destinations.md`](pdf-output-destinations.md)):

```rust
/// Where a finished document lives. Core owns the trait; adapters live in the application.
pub enum RenderOutputRef { Inline(Bytes), External(OutputLocation) }

#[async_trait]
pub trait RenderOutputStore: Send + Sync {
    async fn put(&self, key: &OutputKey, document: Document)
        -> Result<RenderOutputRef, OutputError>;
    async fn open(&self, reference: &RenderOutputRef) -> Result<OutputRead, OutputError>;
    async fn delete(&self, reference: &RenderOutputRef) -> Result<(), OutputError>;
}
```

The MVP ships only an in-memory `Inline` implementation. A filesystem or S3 adapter would live in
the application package, which keeps `typrest-core` free of I/O. The job record stores a
`RenderOutputRef`, the status body's `result` object stays the same (`href`, `contentType`,
`size`), and an external destination later may add a `location` field or a redirect without
changing the state machine.

### Security

- **Job IDs are bearer capabilities.** With no authentication (`REQ-HTTP-050`), anyone holding an ID
  can read the PDF, which can be an invoice or a contract. IDs MUST come from a CSPRNG with at least
  122 bits of randomness (UUIDv4), so guessing is infeasible. The job routes are deliberately
  limited to ID-addressed operations with no listing and no ID-revealing error, and results are
  `Cache-Control: no-store`. IDs appear in `Location` headers and may reach proxy logs; the
  deployment guidance in `REQ-HTTP-051` (trusted network boundary) applies with more weight, and the
  spec must say so. Typrest's own logs SHOULD record only a short ID prefix.
- Anyone who can reach the server can already render and list templates, so jobs do not widen
  *who* may read output, only *how long* output remains retrievable. This is why the feature is
  opt-in and the TTL is short.
- **Denial of service.** The bounded queue, the byte budget, and the body-size limit (verify the axum
  default extractor limit and coordinate with `operational-limits`) bound resource use; there is no
  per-client fairness without identity.
- **Webhooks are not in the MVP.** They need outbound network access (SSRF, DNS rebinding, redirects,
  internal-address blocking, retries, HMAC signing, secret storage) and contradict the vision rule
  that render requests do not access the network.

## Affected requirements

Existing requirements that change when the plan lands:

| ID | Change |
|---|---|
| `REQ-HTTP-001` | Add the job routes, mounted only when `jobs.enabled = true` (like `REQ-HTTP-002`). |
| `REQ-HTTP-003`, `REQ-HTTP-004` | State that `/_jobs` is a reserved Typrest prefix; with jobs disabled, `/_jobs/*` returns `404` like `/_webui/*` (`REQ-HTTP-064`) instead of `400 invalid_template_id`. |
| `REQ-HTTP-020` to `REQ-HTTP-025` | Clarify they describe the synchronous path; `Prefer: respond-async` with jobs enabled returns `202` after the same parsing, lookup, and validation. |
| `REQ-HTTP-041` | Add the new codes to the table (or link to the job specification). |
| `REQ-HTTP-050`, `REQ-HTTP-051` | Add the job-ID capability warning. |
| `REQ-CFG-003` | No change; new `[jobs]` table is additive. Update the `typrest-toml` non-goals sentence that lists related fields. |
| `REQ-LOG-031`, `REQ-LOG-033` | Job lifecycle events join the event set; payloads and PDF bytes stay out of logs. |
| `REQ-WEB-070` | No change in the MVP (the UI stays synchronous). |
| Vision, ROADMAP, architecture overview and template lifecycle | See [Documents to update on completion](#documents-to-update-on-completion). |

Proposed new requirements (*proposed*; final IDs allocated when the spec is written; a new
specification `docs/specs/async-jobs.md` with domain `JOB` keeps the HTTP spec small and avoids ID
collisions with sibling plans that also edit `http-api.md` and `typrest-toml.md`):

- *proposed* **REQ-JOB-010** to **REQ-JOB-019**, opt-in: jobs are disabled by default; `Prefer` is
  ignored when disabled; `Preference-Applied: respond-async` is returned when honored.
- *proposed* **REQ-JOB-020** to **REQ-JOB-029**, acceptance: ID parsing, body parsing, template
  lookup, and schema validation (with defaults) happen before the `202`; they fail exactly as
  `REQ-HTTP-033`, `REQ-HTTP-041`, `REQ-HTTP-042`; the job renders against the registry entry
  captured at accept time.
- *proposed* **REQ-JOB-030** to **REQ-JOB-039**, state machine: the five states and permitted
  transitions; terminal states are immutable; `finishedAt` set once.
- *proposed* **REQ-JOB-040** to **REQ-JOB-049**, status, result, and delete responses (table above).
- *proposed* **REQ-JOB-050** to **REQ-JOB-059**, IDs: CSPRNG, at least 122 bits, UUIDv4 text form;
  no listing route; `Cache-Control: no-store`.
- *proposed* **REQ-JOB-060** to **REQ-JOB-069**, idempotency: header grammar, scope, fingerprint
  comparison, retention equals job retention, concurrent duplicates map to one job.
- *proposed* **REQ-JOB-070** to **REQ-JOB-079**, capacity: queue limit, retained-byte budget,
  `503` plus `Retry-After`, shared concurrency limit, TTL and sweeping, no early eviction of
  unfetched results.
- *proposed* **REQ-JOB-080** to **REQ-JOB-089**, lifecycle: shutdown drain, loss on restart is
  documented behavior, no persistence guarantee.
- *proposed* **REQ-CFG-050** to **REQ-CFG-059** in `typrest-toml.md` for the `[jobs]` table.
- *proposed* **REQ-LOG-0xx** for the job events listed below.

New `typrest.toml` fields (all optional; names and defaults are *proposed* and need owner sign-off,
see open question 4):

```toml
[jobs]
enabled = false            # boolean, default false
queue_capacity = 64        # integer >= 1, default 64: queued (not yet running) jobs
max_concurrent = 0         # integer >= 0, default 0 = share the global render limit
result_ttl_seconds = 900   # integer >= 1, default 900: retention after a job reaches a terminal state
max_retained_bytes = 268435456   # integer >= 1, default 256 MiB: total retained PDF bytes
```

Log events (*proposed*): `job.accepted`, `job.started`, `job.completed`, `job.failed`,
`job.cancelled`, `job.expired`, `job.rejected` (queue or storage full), `job.idempotent_replay`.
Fields: `template`, short `job` prefix, `state`, `latency_ms`, `queue_wait_ms`, `pdf_size`,
`warning_count`, `error_kind`. Compiler diagnostics for failed jobs reuse `compiler_diagnostics::trace`
with context `render` as the synchronous path does.

## Decisions and alternatives

Each row has a recommendation. Rows marked "ADR proposal" are costly to reverse and need human
approval first; this plan does not create ADR files.

**D1. API shape (ADR proposal - needs human approval).**

| Option | Notes |
|---|---|
| A. `Prefer: respond-async` on the existing route plus `/_jobs/{id}` for status and result | One URL and one body contract per template; the standards-based negotiation (RFC 7240) lets old clients and the Web UI keep working; `/_` prefix is already reserved (ADR-0005). |
| B. Separate submit route, for example `POST /_jobs/{*template_id}` | Explicit and easy to document; but `{*template_id}` collides with `/_jobs/{id}/result` unless the ID segment is fixed (for example `/_jobs/render/{*template_id}`), and duplicates the handler contract. |
| C. A query flag such as `?async=1` | Pollutes the template URL space and caching semantics. |

Recommendation: A. If review prefers discoverability, B can be added later as an alias without
breaking A. Status/result live under `/_jobs`, not `/_api`, because `REQ-HTTP-034` declares `/_api`
unversioned and internal, while job routes are a stable integration contract.

**D2. Job ID format.** Recommend UUIDv4 (`uuid` crate with `v4`; or generate 128 random bits from
the OS RNG already in the lockfile via `getrandom`). ULID is time-sortable, which leaks creation
time and has fewer random bits, and sortability has no use without a listing route. IDs are never
reused.

**D3. Validation before acceptance.** Recommended and not optional: it matches the rule that
request-time validation errors are render errors, gives instant `422` feedback, and keeps queued
payloads known-valid. The cost is a `prepare`/`execute` split in core.

**D4. Registry snapshot at accept time versus run time.** Recommend accept time (above). Run-time
lookup would let a template vanish or change schema after a `202`, invalidating the earlier
validation.

**D5. Persistence model (ADR proposal - needs human approval).**

| Option | Notes |
|---|---|
| A. In-memory only (MVP) | No new dependency or operational surface; honest about limits; fits the "single-process service, no stores" posture. Jobs vanish on restart or crash. |
| B. Local filesystem spool (application adapter behind `JobStore`/`RenderOutputStore`) | Survives restart on one host; needs a data directory (`.typrest/` exists), fsync, cleanup, and crash recovery; core stays I/O-free via the port. |
| C. External store (Redis, database, S3) | Required for multi-instance; collides with "S3 or other remote backends" exclusion in the vision and adds operational scope. |

Recommendation: A for the MVP, with the `JobStore` and `RenderOutputStore` ports designed so B can
be a later adapter. B and C each require their own scope change and ADR before work starts. The
ADR to propose now records: in-memory first, ports in core, durable adapters in the application,
jobs are best-effort across restarts.

**D6. Multi-instance deployments.** In the MVP a job exists only on the instance that accepted it;
polling another instance returns `404 job_not_found`. Operators needing more must use sticky
routing (route by client or connection, since the ID carries no instance hint) or wait for a shared
store (C above). Do not embed an instance hint in the ID without a decision. Document this in the
spec as a limitation, not a guarantee.

**D7. Completion notification (ADR proposal for webhooks - needs human approval before any work).**
MVP is polling with `Retry-After` hints and an `expiresAt` timestamp. Webhooks, SSE, and WebSockets
are out of scope. Webhooks would need: a destination allowlist in `typrest.toml` (never trusting
per-request URLs), private-address and redirect blocking, bounded retries with backoff, HMAC
signing, timeouts, and an amendment of the vision rule about network access. Recommend deferring
until real demand exists.

**D8. Idempotency (included in the ADR for D1).** `Idempotency-Key` header, scoped to the whole
service (no identity exists to scope by). Fingerprint is a SHA-256 over canonical
`(template id, validated-or-raw payload)`; recommend hashing the raw parsed JSON with sorted object
keys so key order does not matter, and sharing the fingerprint function with
[`render-result-cache.md`](render-result-cache.md). Retention equals the job's retention, so after
expiry the same key creates a new job. Concurrent duplicates must resolve to one job under one
lock. Idempotency for synchronous renders would need response caching and is out of scope here (see
Later phases).

**D9. Queue overload status.** Recommend `503` with `Retry-After` for both `job_queue_full` and
`job_storage_full`, since the cause is server capacity, not client request rate. `429` is the
alternative if `operational-limits` standardizes it for the synchronous path; align with that plan.

**D10. Cancellation of running jobs.** Mark cancelled and discard output; do not claim to stop
CPU work. A cooperative cancel hook is a future compiler-ACL change and is not planned.

**D11. Web UI.** No MVP change; the UI keeps the synchronous path. A later phase may add an
"async" toggle with polling. Any frontend change requires `npm run check`, `npm test`,
`npm run build` and committed `webui/dist`.

## Out of scope

- Authentication, per-client quotas, job ownership, or a job listing route.
- Webhooks, SSE, WebSockets, email, or any outbound notification.
- Durable or shared job storage, S3, and other remote backends (separate decisions).
- Retaining synchronous render results, or idempotency for synchronous renders.
- Interrupting a running Typst compilation.
- Store mode, tags, publishing, compilation cache (see `render-result-cache.md`), SDKs.
- A job priority system, scheduling, or batch (multi-document) submissions.
- Job metrics implementation itself (owned by `render-metrics.md`; this plan only emits the data).

## Risks

- **Memory growth.** Retained PDFs and queued payloads can be large. Mitigated by capacity limits,
  TTL, and rejection rather than eviction; defaults need review (open question 4).
- **Silent job loss on restart.** Clients can be surprised. Mitigated by documentation, `404`
  semantics, idempotent resubmission, and a drain on graceful shutdown.
- **Capability-token leakage** through logs, proxies, or referrers. Mitigated by short log prefixes,
  `no-store`, and deployment guidance; not eliminated without authentication.
- **Starvation.** Jobs and sync renders share one limiter; heavy jobs can delay interactive requests.
  `jobs.max_concurrent` mitigates; fairness policy is an open question.
- **Prerequisite churn.** The `prepare`/`execute` split and limiter integration touch code that
  `blocking-render-execution` also changes; sequence after it to avoid conflicts.
- **Time and clocks.** TTL tests need an injected clock; `tokio` lacks the `time` feature today.
- **Idempotency edge cases.** Race between duplicate submissions and cancellation or expiry must be
  tested; mismatch detection depends on a stable fingerprint.

## Dependencies on other plans

This plan is written in parallel with its siblings. Links may not resolve until those files land.

- [`blocking-render-execution.md`](blocking-render-execution.md) - **prerequisite.** *Needed:* a
  shared render concurrency limiter (permit acquisition API usable from a worker task), compilation
  moved off the async executor, and cancellation of work still waiting for a permit. *Provided:* the
  `prepare`/`execute` split of `RenderService` (step 1) which that plan may also need; agree on one
  owner to avoid duplicate edits to `render.rs`.
- [`operational-limits.md`](operational-limits.md) - *Needed:* per-render timeout and its error
  code, graceful-shutdown signal plus drain deadline in `http.rs`/`serve.rs`, request body limit,
  and the configuration naming convention. *Provided:* queue and retention limits that should follow
  the same naming and validation style; the shutdown hook the job runtime subscribes to.
- [`render-metrics.md`](render-metrics.md) - *Needed:* the metrics facade or registry. *Provided:*
  the instrumentation points: queue depth, running jobs, retained bytes, jobs by terminal state,
  queue wait and run duration, idempotent replays, rejections.
- [`render-result-cache.md`](render-result-cache.md) - *Needed:* nothing hard. *Provided and shared:*
  the render fingerprint (template identity, template version, canonical payload) used for
  idempotency comparison. If the cache lands, a job `execute` SHOULD go through it; a cache hit
  makes a job complete without taking a render permit. Decide who owns the fingerprint type; this
  plan proposes the cache plan owns it and jobs consume it.
- [`pdf-output-destinations.md`](pdf-output-destinations.md) - strong coupling. *Shared:* the
  `RenderOutputStore` boundary described above. If destinations land first, jobs adopt their port;
  if jobs land first, they ship the in-memory `Inline` adapter and destinations add adapters behind
  the same trait. Delivery to an external destination should be an attribute of the job result
  (`RenderOutputRef::External`), never a reason to change the state machine.

## Prerequisites (scope and ADR approval)

Step 0 gates everything. Human approvals needed:

1. **Scope change.** Asynchronous jobs are not in the vision or ROADMAP. Approve adding them as an
   opt-in filesystem-mode feature (in-memory only), and approve the new vocabulary
   (`Job`, `JobId`, `JobState`, `IdempotencyKey`) for the canonical terms table.
2. **ADR proposal - API shape and idempotency** (D1, D2, D8). Costly-to-reverse public contract.
   *Needs human approval.*
3. **ADR proposal - job persistence model** (D5, D6). Constrains a boundary and operations.
   *Needs human approval.*
4. **ADR proposal - completion notification** (D7). Only needed before any webhook work; polling
   needs no ADR. *Needs human approval.* Webhooks also need a vision amendment about network access.
5. Confirm the open questions below that block the MVP (at least 1, 4, 5).

Do not create ADR files or start step 1 before the first three are resolved.

## Steps

Each step is sized for one session and one commit, ends with the listed checks, and updates its own
documents in the same change. Run `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` after each Rust
step, and `python3 scripts/check-doc-links.py` plus `git diff --check` after documentation edits.

0. **Approval gate.** Obtain the approvals under [Prerequisites](#prerequisites-scope-and-adr-approval),
   and confirm `blocking-render-execution` is merged. Done when the owner has recorded the decisions
   and, for each approved ADR proposal, the ADR is written by the human-approved process in
   [`docs/adr/README.md`](../adr/README.md).

1. **Split `RenderService` into `prepare` and `execute`.** Files:
   `crates/typrest-core/src/render.rs`, `lib.rs` (export `PreparedRender`). `render` keeps its
   signature and behavior. Tests (core, `MemoryLoader`/mock compiler as in the existing module):
   `prepare` returns `TemplateNotFound` and `ValidationError` without calling the compiler; defaults
   applied in the prepared payload; `execute` after `prepare` equals `render`; a registry swap
   between `prepare` and `execute` does not change the output. No docs change except the
   architecture note on the request lifecycle (done in step 8).

2. **Core job model and in-memory store.** New `job.rs`, `job/store.rs` in `typrest-core`; `JobId`
   generation behind a small `IdGenerator` trait (UUIDv4; add `uuid` with `v4`, or `getrandom`,
   to workspace dependencies; add `sha2` for fingerprints if not shared with the cache plan); a
   `Clock` trait. Implement states, transitions, bounded queue, byte budget, TTL sweep, idempotency
   index, and the `RenderOutputStore` port with the in-memory inline adapter. Typed `thiserror`
   errors; no `unwrap`/`expect` outside tests. Tests: every legal and illegal transition; terminal
   immutability; queue-full and storage-full rejection; no early eviction of unfetched results; TTL
   with a fake clock; idempotent replay, mismatch, and concurrent duplicate submissions (threads);
   cancel of queued versus running; ID format and uniqueness. Core must not gain HTTP, tokio, or
   filesystem dependencies.

3. **Configuration.** `crates/typrest/src/config.rs`: parse `[jobs]` (defaults above, validation of
   ranges, `deny` negative or zero where required), expose a `JobsConfig` in `ProjectConfig`;
   update `docs/specs/typrest-toml.md` (new `REQ-CFG` group, remove the stale non-goal). Tests
   alongside the existing config tests: defaults, valid overrides, wrong types, out-of-range values,
   unknown keys inside `[jobs]` (decide: reject; open question 4), and that omitting `[jobs]` leaves
   behavior unchanged.

4. **Job runtime.** New `crates/typrest/src/jobs.rs`; enable tokio `time`. Worker tasks pull queued
   jobs, acquire the render permit from the limiter, call `execute`, record the result through the
   store. TTL sweeper task; shutdown subscription (drain running, drop queued, log counts). Tests
   (`#[tokio::test]` with a mock compiler and gate/barrier to control timing): FIFO order; limiter
   shared with sync renders; `max_concurrent`; cancel while queued and while running; timeout maps to
   `failed`; shutdown drain; sweeper removes expired. No real Typst.

5. **HTTP surface.** Files: `http/router.rs`, `http/handlers/jobs.rs`, `render_template.rs`,
   `http/state.rs`, `http/errors.rs`. Factor request parsing out of the render handler; add the
   `Prefer` branch returning `202` with `Location`, `Preference-Applied`, `Retry-After`; add the
   status/result/delete routes (mounted only when enabled; answer `404` when disabled, mirroring the
   Web UI handlers); add the new error codes to `ApiError`. Update `docs/specs/http-api.md`
   (`REQ-HTTP-001/003/004/020/041/050/051`) and create `docs/specs/async-jobs.md` from
   `docs/specs/template.md`, indexed in `docs/specs/README.md` and `docs/README.md`. Tests (router
   with `tower::ServiceExt::oneshot`, as in `http.rs`): `202` plus `Location`; `404`/`422`/`400`
   returned synchronously and no job created; polling through every state; result `200` repeated
   twice; failed job returns the sync envelope and status with safe diagnostics (no paths or source,
   reuse the existing assertions); `409` when not ready; cancel and delete; `Prefer` ignored and
   `/_jobs/x` is `404` when disabled; sync behavior byte-identical when `Prefer` is absent;
   `GET` with `Prefer` is ignored (open question 8).

6. **Idempotency.** Wire `Idempotency-Key` through the handler: header validation
   (`invalid_idempotency_key`), fingerprint, replay `202` with the same `Location`, mismatch `422`,
   retention tied to the job. Tests: replay returns the same ID with no second render (count mock
   compiler calls); changed payload mismatch; key order of JSON objects does not matter; replay
   after expiry creates a new job; replay of a failed or cancelled job returns that job; concurrent
   duplicates produce one job. Update `async-jobs.md`.

7. **Logging and metrics hooks.** Emit the `job.*` events (short ID prefix only) through the existing
   tracing setup; compiler diagnostics for failed jobs go through `compiler_diagnostics::trace`.
   Add instrumentation points that `render-metrics` consumes (or stubs if it has not landed). Update
   `docs/specs/logging.md`. Tests: `crates/typrest/tests/logging.rs` style assertions that events
   exist with the right fields and that JSON output contains no payload, PDF bytes, or full ID.

8. **End-to-end test and documentation close-out.** Add one end-to-end test with real Typst on a
   random port (submit, poll, fetch, verify extracted PDF text per
   [`testing.md`](../development/testing.md)); update README HTTP table, architecture overview and
   template lifecycle (request lifecycle section, new canonical terms, restart-loss invariant),
   ROADMAP, vision, and the examples README only if an example is added (a short curl walkthrough
   belongs in the spec, not a new example). Run the doc-link check and `git diff --check`. Delete
   this plan or reduce it to the remaining later phases.

### Later phases (each needs its own approval before starting)

- **Phase B - local durable spool.** `JobStore`/`RenderOutputStore` filesystem adapters in the
  application package; crash recovery; startup rehydration of `queued` jobs (decide whether to
  re-validate against the then-current registry).
- **Phase C - shared store / multi-instance.** Needs the remote-backend scope decision and an ADR.
- **Phase D - webhooks.** Needs the D7 ADR, a vision amendment, and a dedicated security review.
- **Phase E - external result destinations.** Adopt `pdf-output-destinations` adapters for job
  results; add `location` to the status body.
- **Phase F - idempotency for synchronous renders**, built on `render-result-cache`.
- **Phase G - Web UI async mode** (`REQ-WEB` updates, rebuilt `webui/dist`).

## Open questions

1. Is an opt-in, in-memory job feature acceptable now, or must the durable option be decided first?
2. Should the status/result routes be `/_jobs/...` (recommended) or live under `/_api`?
3. Should `GET` with `Prefer: respond-async` be honored? Recommended: no, `POST` only.
4. Defaults for `queue_capacity`, `result_ttl_seconds`, `max_retained_bytes`, `max_concurrent`, and
   whether unknown keys inside `[jobs]` are rejected. All values here are placeholders, not product
   decisions.
5. `503` or `429` for overload (align with `operational-limits`).
6. Should an expired job answer `404 job_not_found` (simple, recommended) or `410 job_expired` via a
   tombstone (needs extra memory and a retention rule)?
7. Should jobs have their own timeout longer than the synchronous render timeout, or share it?
8. Fairness: should synchronous renders have reserved capacity so large job backlogs do not delay
   them?
9. Should `failed` job results return the original HTTP status on the result route (recommended) or
   always `200` with the envelope?
10. Is the full job ID acceptable in logs at `debug`, or never?
11. Should the shared fingerprint and `RenderOutputStore` be owned by the cache plan, the
    destinations plan, or this one?
12. Does the operator-facing limitation (jobs lost on restart, one instance only) meet the intent of
    the original request about dropped connections, or is durability expected in the first release?

## Documents to update on completion

- `docs/specs/async-jobs.md` - new specification (this file will not exist until
  step 5; link it from other documents only then).
- [`docs/specs/http-api.md`](../specs/http-api.md) - routes, reserved prefix, rendering note, error
  table, security note.
- [`docs/specs/typrest-toml.md`](../specs/typrest-toml.md) - `[jobs]` table and requirement IDs.
- [`docs/specs/logging.md`](../specs/logging.md) - job events and fields.
- [`docs/specs/README.md`](../specs/README.md) and [`docs/README.md`](../README.md) - index the new
  specification.
- [`docs/architecture/overview.md`](../architecture/overview.md) and
  [`docs/architecture/template-lifecycle.md`](../architecture/template-lifecycle.md) - canonical
  terms, request lifecycle, snapshot-at-accept invariant, in-memory limitation.
- [`docs/product/vision.md`](../product/vision.md) - include asynchronous jobs in scope; keep the
  network-access rule unless webhooks are approved.
- [`docs/development/testing.md`](../development/testing.md) - job test placement, fake clock note.
- [`ROADMAP.md`](../../ROADMAP.md) - status and deferred phases.
- [`README.md`](../../README.md) - HTTP API table and a short opt-in note.
- [`docs/adr/README.md`](../adr/README.md) - index entries for ADRs once humans approve and write
  them.
- [`docs/plans/README.md`](README.md) - active plans list; delete this plan when complete.
