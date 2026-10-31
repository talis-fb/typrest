# Plan: Render and service metrics

- **Status:** Draft
- **Owner:** Talison Fabio

## Objective

Give operators a Prometheus-scrapable view of the service: render latency (total and per stage),
PDF size, outcomes by error kind, request counts by template and status, in-flight renders, watch
reload results, template count, and build info. Today the only runtime signal is the per-render log
line, which cannot be aggregated without a log pipeline.

The metric layer must be small, opt-in, free of per-request labels with unbounded cardinality, and
shaped so the sibling plans can add their metrics later (see
[Dependencies on other plans](#dependencies-on-other-plans)).

## Current behavior

- The render handler already measures wall-clock latency around `RenderService::render` and logs
  `render.completed` / `render.failed` with `latency_ms`, `pdf_size`, `warning_count`, and
  `error_kind` (`crates/typrest/src/http/handlers/render_template.rs:48-103`; error kinds at
  `:121-128`). Logging contract: [Logging specification](../specs/logging.md); its non-goals list
  "metrics" (`logging.md`, Non-goals).
- `RenderService::render` (`crates/typrest-core/src/render.rs:16-32`) does registry lookup,
  validation, and `compile_prepared` in one call and returns only `Result<Document, RenderError>`.
  It exposes no timings.
- `TypstCompiler::compile_prepared` (`crates/typrest-core/src/compiler.rs:65-79`) delegates to
  `world::compile_prepared`, which runs `typst::compile` (`world.rs:55`) and then `typst_pdf::pdf`
  (`world.rs:69`) back to back. Failures are already distinguished as
  `CompilationError::Failed` (compile) and `CompilationError::ExportFailed` (export)
  (`crates/typrest-core/src/errors.rs:207-214`). No stage durations are exposed.
- `Document` holds bytes and warnings only (`crates/typrest-core/src/template.rs:49-78`).
- Registry lifecycle already has a presentation-neutral observer pattern:
  `TemplateRegistryEvent` (`crates/typrest-core/src/registry.rs:167-179`), logged by the
  application. Core stays free of HTTP and logging dependencies
  ([architecture overview](../architecture/overview.md)).
- Watch mode logs reload outcomes: `template.reload.completed` / `template.reload.failed`
  (`crates/typrest/src/loader/watch.rs:314`, `:379`) and `template.registry.rebuilt` /
  `template.registry.rebuild.failed` (`:173`, `:273`). Details:
  [Template lifecycle](../architecture/template-lifecycle.md#watch-mode).
- Routes are fixed in `crates/typrest/src/http/router.rs:9-35`; `AppState` carries
  `registry`, `render_service`, `log_format` (`state.rs`); one listener is bound in
  `crates/typrest/src/http.rs:17-33`. `serve` composes everything in
  `crates/typrest/src/commands/serve.rs:192-198`.
- Paths whose first segment starts with `_` are reserved and never template IDs
  (`REQ-HTTP-004`, [ADR-0005](../adr/0005-derive-template-ids-from-folder-routes.md); test
  `render_has_no_legacy_prefix_alias_and_rejects_reserved_ids`, `http.rs:623-674`). An unmounted
  reserved path such as `/_metrics` currently reaches the catch-all and returns `400
  invalid_template_id`; with `web_ui = false`, `/_webui*` returns an explicit `404`
  (`REQ-HTTP-064`).
- `typrest.toml` has `schema_version`, `templates_root`, `web_ui`; unknown fields are ignored
  (`REQ-CFG-003`; `crates/typrest/src/config.rs:11-25`). Ports and hostnames are CLI options
  (`REQ-CLI-040` table), and the toml non-goals say the format does not define ports.
- There is no authentication on any route (`REQ-HTTP-050`, `REQ-HTTP-051`).
- No metrics dependency exists in the workspace.

## Intended change

1. A Prometheus/OpenMetrics text endpoint `GET /_metrics`, opt-in with `metrics = true` in
   `typrest.toml`, default off.
2. Metrics live in the **application package** (`crates/typrest/src/metrics.rs` plus submodules),
   backed by an instance-owned registry (no process-global recorder). Core gains only a small,
   dependency-free **render observer** that reports a per-render `RenderReport` with stage timings
   and outcome, mirroring the existing `TemplateRegistryEvent` observer pattern. Core imports no
   metrics crate.
3. The compiler ACL reports separate compile and export durations through Typrest-owned types
   (`Document` gains stage timings); no Typst type crosses the boundary.
4. HTTP middleware records request counts and durations by bounded route class; the render handler
   tracks in-flight renders; watch mode records reload outcomes.
5. Optionally, a second listener (`serve --metrics-port`) so metrics need not share the public port.
6. Out of scope: tracing spans, OpenTelemetry/OTLP, push gateways, authentication.

### Metric catalog

Naming follows Prometheus conventions: prefix `typrest_`, base units (seconds, bytes), counters end
in `_total`. With `prometheus-client`, register counters without `_total`; the encoder appends it.

Label value sets are closed enums except `template`, which is bounded by the registry (see
[Label cardinality](#label-cardinality)).

| Name | Type | Labels | Unit | Description |
|---|---|---|---|---|
| `typrest_build_info` | gauge (always 1) | `version` | none | Crate version of the running binary. |
| `typrest_process_start_time_seconds` | gauge | none | seconds | Unix time the server started. |
| `typrest_templates` | gauge | none | templates | Number of templates in the ready registry; set at scrape time from `TemplateRegistry::ids()`. |
| `typrest_http_requests_total` | counter | `route`, `method`, `status` | requests | All HTTP requests. `route` is one of `render`, `healthz`, `templates_api`, `webui`, `metrics`, `other`; `status` is the numeric status code. |
| `typrest_http_request_duration_seconds` | histogram | `route`, `method` | seconds | Time from request receipt to response headers for every route. Buckets: 0.001, 0.005, 0.025, 0.1, 0.5, 1, 5, 30. |
| `typrest_render_requests_total` | counter | `template`, `outcome` | renders | Renders that reached `RenderService`. `outcome` is `ok`, `template_not_found`, `schema_validation_failed`, `compilation_failed`, `pdf_export_failed`, or `cancelled` (client disconnected before completion). `template` is `_unknown` for `template_not_found`. |
| `typrest_render_duration_seconds` | histogram | `template`, `outcome` | seconds | Total `RenderService::render` time (equals the logged `latency_ms`). Buckets: 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10, 30. |
| `typrest_render_stage_duration_seconds` | histogram | `stage` | seconds | Per-stage time; `stage` is `validate`, `compile`, or `export`. No `template` label, to bound series count. Same buckets as above. |
| `typrest_render_pdf_size_bytes` | histogram | `template` | bytes | Size of successfully rendered PDFs. Buckets: 1 KiB, 10 KiB, 100 KiB, 1 MiB, 10 MiB, 50 MiB, 100 MiB. |
| `typrest_render_warnings_total` | counter | `template` | warnings | Compiler warnings on successful renders. |
| `typrest_renders_in_flight` | gauge | none | renders | Renders currently executing in handlers (incremented before `RenderService::render`, decremented on completion or drop). |
| `typrest_template_reloads_total` | counter | `scope`, `outcome` | reloads | Watch-mode reloads. `scope` is `template` or `registry`; `outcome` is `ok` or `error`. |
| `typrest_template_last_reload_success_timestamp_seconds` | gauge | none | seconds | Unix time of the last successful watch reload; absent value (0) before the first one. |

Validation failure counts and errors by kind are the `outcome` values of
`typrest_render_requests_total`; there is deliberately no separate validation counter. Requests
rejected before the service (`invalid_template_id`, `invalid_json`, unsupported content type) appear
only in `typrest_http_requests_total` with status `400`.

Startup compilation is not measured: it happens before the socket is bound, and `typrest check`
does not run a server.

### Label cardinality

- `template` is the canonical `TemplateId` of a template present in the registry when the report is
  produced. A request for an unknown or invalid ID never creates a new label value: it uses the
  constant `_unknown`, which cannot be a real ID (segments cannot start with `_`).
- Never label by payload, query string, raw URL path, error message, or diagnostic text.
- Histogram series scale with templates times buckets. Per-stage histograms therefore omit
  `template`. A removed template's series persist until restart; this is bounded by the number of
  distinct IDs seen during the process lifetime and is accepted (see [Risks](#risks)).
- `route` is derived from axum `MatchedPath` and mapped to a closed set; unmatched requests are
  `other`.

## Affected requirements

Existing requirements that change (IDs are stable; meaning changes keep the ID):

- `REQ-HTTP-001` ([HTTP API](../specs/http-api.md)): the always-mounted list gains nothing, but a
  new conditional route group is added by the proposed `REQ-HTTP-070`+ below; adjust wording to say
  the listed routes are mounted regardless of configuration, with conditional groups elsewhere.
- `REQ-HTTP-003`: add `/metrics` to the forbidden aliases.
- `REQ-HTTP-004`: unchanged; cite it in the metrics spec as the reason `/_metrics` cannot collide
  with a template.
- `REQ-HTTP-050`, `REQ-HTTP-051`: security note extended to the metrics endpoint (it exposes
  template IDs and traffic shape).
- `REQ-CFG`: new field rows in the [`typrest.toml`](../specs/typrest-toml.md) table and new
  requirements below; amend the Non-goals sentence only if a port field is added (it is not; see
  decisions).
- `REQ-CLI` ([CLI](../specs/cli.md)): the `serve` option table gains `--metrics-port` (step 5).
- [Logging](../specs/logging.md) Non-goals: replace "metrics" with a link to the metrics
  specification.
- `REQ-LOG-031`: no change; `render.completed` / `render.failed` fields stay as they are.

Proposed new requirements (*proposed*; not yet in any spec). Create
`docs/specs/metrics.md` from [`../specs/template.md`](../specs/template.md) with domain `MET`,
and add it to [`../specs/README.md`](../specs/README.md) and the domain list in
[`../README.md`](../README.md#requirement-ids).

*proposed* `docs/specs/metrics.md`:

- **REQ-MET-010:** With `metrics = true`, the server MUST serve `GET /_metrics` returning status
  `200`, `Content-Type: application/openmetrics-text; version=1.0.0; charset=utf-8`, and the metric
  families in the catalog.
- **REQ-MET-011:** Metric names, types, and label names in the specification are a public contract;
  removing or renaming one MUST be treated as a breaking change.
- **REQ-MET-020:** The `template` label MUST only take values that are IDs of prepared templates, or
  `_unknown`.
- **REQ-MET-021:** Metrics MUST NOT use payload data, raw paths, query strings, or message text as
  label values.
- **REQ-MET-030:** `typrest_render_requests_total` MUST record exactly one outcome for every render
  that reached `RenderService`, using the values in the catalog.
- **REQ-MET-031:** Render duration MUST cover `RenderService::render`; stage durations MUST
  separate validation, compilation, and PDF export.
- **REQ-MET-040:** Reload outcomes in watch mode MUST be counted per scope and outcome.
- **REQ-MET-050:** Collecting metrics MUST NOT read template files, package files, or the network
  (consistent with `REQ-HTTP-025`).
- **REQ-MET-060:** Metrics MUST NOT change render responses, status codes, or the log stream.

*proposed* additions to [HTTP API](../specs/http-api.md) (next free IDs; the highest existing group
is `REQ-HTTP-060` to `064`):

- **REQ-HTTP-070:** The server MUST mount `GET /_metrics` only when `metrics = true`
  (`REQ-CFG-050`); otherwise `GET /_metrics` MUST return `404` (like `REQ-HTTP-064`).
- **REQ-HTTP-071:** With `--metrics-port`, the metrics endpoint MUST be served on a separate
  listener bound to the same hostname, and `GET /_metrics` MUST NOT be mounted on the main
  listener (step 5).
- **REQ-HTTP-072:** The metrics endpoint MUST NOT be authenticated; the `REQ-HTTP-051` network
  guidance applies.

*proposed* additions to [`typrest.toml`](../specs/typrest-toml.md) (next free group after
`REQ-CFG-040`):

| Field | Type | Required | Default |
|---|---|---|---|
| `metrics` | boolean | no | `false` |

- **REQ-CFG-050:** `metrics` MUST default to `false`.
- **REQ-CFG-051:** `metrics = true` MUST enable the metrics endpoint as defined in
  the proposed `docs/specs/metrics.md`; all other routes are independent of the flag.

*proposed* addition to [CLI](../specs/cli.md) (next free group after `REQ-CLI-043`):

- **REQ-CLI-044:** `serve --metrics-port <port>` MUST require `metrics = true` and MUST fail before
  listening otherwise.

## Decisions and alternatives

Recommendations are mine; the owner decides.

1. **Library: `prometheus-client` (official OpenMetrics client) with an instance-owned
   `Registry`.** Alternatives: `metrics` + `metrics-exporter-prometheus` (global recorder; macro
   ergonomics, but global state complicates parallel tests and tests per `AppState`, and adds a
   facade we would otherwise not need); `prometheus` crate (mature, text format 0.0.4, but untyped
   string labels); OpenTelemetry/OTLP (heavy dependency tree, push model, out of scope for a
   single-binary service). Recommended: `prometheus-client`. It is added to the `typrest`
   package only, never to `typrest-core`.
2. **Where metrics live: application layer, fed by a core `RenderObserver`.** Alternatives: a
   `metrics` facade call inside core (acceptable dependency-wise, but couples core to a global
   recorder and makes tests assert on globals); wrapping `RenderService` in the handler and timing
   there (cannot split stages or see validation/compile boundaries). Recommended: a trait
   `RenderObserver` in `typrest-core` with one method receiving a `RenderReport` (template,
   outcome enum, `RenderTimings`, optional pdf size, warning count). Same pattern as
   `TemplateRegistryEvent`; `RenderService::with_observer(Arc<dyn RenderObserver>)`; default no
   observer so existing call sites and tests are untouched.
3. **Compile vs export timing without leaking Typst types.** `world::compile_prepared` wraps the two
   calls with `std::time::Instant` and returns durations; `Document` gains
   `timings() -> RenderTimings`-style accessor (compile and export `Duration`s, default zero for
   mock documents). On failure the error carries no timings, so `RenderService` attributes the
   elapsed time of the `compile_prepared` call to `compile` for `CompilationError::Failed` and to
   `export` for `ExportFailed` (slightly overstating export on failure; documented in the spec).
   Alternative: add timings to `CompilationError` (changes `PartialEq`/`Clone` surfaces and every
   constructor) or change the `PdfCompiler` trait (breaks all mocks); rejected for churn.
4. **Route and exposure: `GET /_metrics` on the main listener, opt-in, plus optional separate
   listener.** `_metrics` starts with `_`, so by `REQ-HTTP-004`/[ADR-0005](../adr/0005-derive-template-ids-from-folder-routes.md)
   it cannot collide with any template ID; no reserved-ID change needed. The conventional
   `/metrics` is rejected: it would be a template ID and breaks `REQ-HTTP-003`'s no-alias rule
   (Prometheus scrape config accepts `metrics_path`). Because there is no auth, a separate
   `--metrics-port` listener (step 5) lets deployments keep metrics off the public port and mirrors
   how `--port`/`--hostname` already live on the CLI. Alternative: a `metrics_port` field in
   `typrest.toml` (rejected: contradicts the "no ports in toml" non-goal and the container model
   where port is a runtime concern).
5. **Disabled behavior returns 404** via an explicit handler, like `/_webui` (`REQ-HTTP-064`),
   instead of falling through to `400 invalid_template_id`.
6. **Text format: OpenMetrics text only** (what `prometheus-client` emits); Prometheus servers
   accept it. Alternative: also serve classic text format; not needed now.
7. **Tracing/OpenTelemetry: out of scope.** Logs remain the single stdout stream
   ([ADR-0008](../adr/0008-use-one-stdout-tracing-stream.md)); metrics add no log lines other than
   one `metrics.enabled` info event at startup.
8. **Web UI interaction: none.** The UI does not read metrics; its requests are counted under
   `route="webui"`/`templates_api`/`render` like any other client. The embedded UI is not changed,
   so no `webui/dist` rebuild.

**ADR proposal — needs human approval.** Criteria in [`../adr/README.md`](../adr/README.md) are met:
at least two serious alternatives (decision 1 and 2), the choice constrains the core boundary
(observer instead of a metrics facade in core) and public contract (metric names, endpoint,
exposure/security posture). Suggested title: "Expose Prometheus metrics through an application-owned
registry fed by a core render observer" (next number `0009`). Do not create the ADR until the owner
approves; step 1 must not start before that decision. The route name and label rules are spec
material, not ADR material.

## Metric layer design (for sibling plans)

- `crates/typrest/src/metrics.rs` declares `pub(crate) struct Metrics` (cheap `Clone`, holds
  `Arc` families and the `Arc<Mutex<Registry>>` or an immutable built registry) and
  `Metrics::new(version)`; it also implements `RenderObserver`.
- Families are grouped in private submodules, one per concern: `metrics/http.rs`,
  `metrics/render.rs`, `metrics/watch.rs`. A sibling plan adds its own submodule (for example
  `metrics/cache.rs`) and registers it in `Metrics::new`; the catalog table above gets new rows in
  the same change as the spec.
- Label sets are typed `#[derive(EncodeLabelSet)]` structs with enum values, so a new label cannot
  carry free text by accident.
- Metric-name and label constants (`ROUTE_RENDER`, `TEMPLATE_UNKNOWN = "_unknown"`) are named
  constants, per the repository code conventions.
- `Metrics` is stored in `AppState` as `Option<Metrics>`; when metrics are disabled nothing is
  allocated or recorded (the observer is not attached).

## Out of scope

- Tracing spans, OpenTelemetry, OTLP export, exemplars.
- Authentication or TLS for the metrics endpoint.
- Alert rules or Grafana dashboards (could be added to `examples/` later; not part of this plan).
- Metrics for startup compilation and `typrest check`.
- Per-template stage histograms, payload-size metrics, process/runtime metrics (CPU, RSS).
- Metrics owned by the sibling plans (listed below).

## Risks

- **Series growth from removed templates in watch mode.** Mitigation: bounded by distinct IDs per
  process lifetime; stage histograms omit `template`. Open question 3 covers dropping template from
  the duration histogram.
- **Exposure of template IDs and traffic shape without auth.** Opt-in, default off, separate port
  option, documented in `REQ-HTTP-051` neighbors.
- **Failure-time stage attribution is approximate** (decision 3).
- **Client disconnects**: a dropped handler future never reaches the service report. The handler's
  in-flight guard records `outcome="cancelled"` from `Drop` so counters and gauge stay consistent.
- **Public contract risk**: metric names become hard to change (`REQ-MET-011`).
- **Hot-path cost**: one `Instant` pair per stage and a few atomic increments; no allocation of
  label strings beyond cloning a bounded `TemplateId`. A micro-benchmark is not required.
- **Dependency footprint** of `prometheus-client` is small, but a new third-party dependency in a
  service that currently has none for telemetry; pin like other workspace deps.
- **Older binaries ignore `metrics = true`** (`REQ-CFG-003`), silently serving no metrics.

## Open questions

1. Approve the ADR proposal and library choice (decisions 1 and 2)?
2. Is `outcome="cancelled"` worth the complexity, or should disconnects only decrement the gauge?
3. Should `typrest_render_duration_seconds` keep the `template` label (useful per-template SLOs,
   costs 13 series x outcomes x templates), or move to an unlabeled histogram plus a per-template
   summary counter pair (sum/count)?
4. Should `--metrics-port` ship in the first release or only the same-listener endpoint?
5. Should `GET /_metrics` also be reachable when `--metrics-port` is set (no; current proposal)?
6. Should the container image (`Dockerfile`, `docker-compose.yml`) expose a metrics port, or stay
   unchanged until the first operator asks?
7. `typrest_template_last_reload_success_timestamp_seconds` before the first reload: omit the
   sample or report `0`? Proposal: report `0` for simplicity.

## Dependencies on other plans

Siblings written in parallel (files may not exist yet):

- [Blocking render execution](../specs/http-api.md#render-capacity): adds queued/blocking-pool gauges and
  queue-wait time. It should add them under `metrics/render.rs` and report queue wait through
  `RenderReport` (new optional field) so stage timings stay consistent. `typrest_renders_in_flight`
  defined here is the handler-level count; the sibling defines "queued" and "executing in pool".
- [Operational limits](operational-limits.md): timeouts and rejections add outcomes
  (`timeout`, `rejected`) to `typrest_render_requests_total`; the outcome enum is `#[non_exhaustive]`
  in the app layer so adding values is not a breaking code change, but it is a spec change.
- [Render result cache](render-result-cache.md): hit/miss/eviction counters in a new
  `metrics/cache.rs`; a hit must still produce a `RenderReport` (with zero compile/export time) or
  explicitly bypass it; that plan decides, this plan requires exactly one outcome per render
  (`REQ-MET-030`).
- [Async render jobs](async-render-jobs.md): job queue depth, job duration, and state counters.
- [PDF output destinations](pdf-output-destinations.md): upload attempts, duration, and failures by
  destination kind (never by URL).

This plan should land first or in parallel with the others; none of them is a prerequisite for it.

## Steps

Each step is one focused commit. Before step 1, the ADR decision from open question 1 is resolved.
Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and
`cargo test --workspace` at the end of every step
([testing guide](../development/testing.md)).

1. **Core: render observer and stage timings.**
   - `crates/typrest-core/src/render.rs`: add `RenderObserver` trait (`Send + Sync`),
     `RenderOutcome` enum, `RenderTimings`, `RenderReport`; `RenderService::with_observer`; emit one
     report per `render` call, covering not-found, validation failure, compile failure, export
     failure, and success. Use `std::time::Instant` only; no new dependencies.
   - `crates/typrest-core/src/compiler/world.rs`: time `typst::compile` and `typst_pdf::pdf`
     separately; `CompiledPdf` carries the durations. `compiler.rs` passes them into `Document`
     (`crates/typrest-core/src/template.rs`: new field, default zero in `Document::new`, accessor
     `timings()`). Re-export intentionally from `lib.rs`.
   - Tests (core, mock compiler): one report per outcome; validation failure reports no compile
     time; observer absent changes nothing; one real-Typst test in the compiler module asserting
     both durations are non-zero for a minimal template.
   - Check: `rg -n '\btypst(_pdf|_assets)?::' crates --glob '!crates/typrest-core/src/compiler/**'`
     returns nothing; public signatures expose only `std::time::Duration`.

2. **Application: metrics module.**
   - Add `prometheus-client` to workspace dependencies and `crates/typrest/Cargo.toml` only.
   - Create `crates/typrest/src/metrics.rs` and `metrics/{http,render,watch}.rs` with the catalog,
     typed label sets, bucket constants, `Metrics::encode() -> String`, and the `RenderObserver`
     impl (mapping `RenderOutcome` and `_unknown`).
   - Tests (unit, no HTTP): feed reports and assert encoded output contains expected series;
     `template_not_found` yields `template="_unknown"`; output ends with `# EOF`.
   - Add `mod metrics;` in `crates/typrest/src/lib.rs`.

3. **HTTP endpoint, config, middleware.**
   - `crates/typrest/src/config.rs`: `metrics: bool` (`#[serde(default)]`) in `RawProjectConfig`
     and `ProjectConfig`; extend config tests including the default and wrong type.
   - `crates/typrest/src/http/config.rs`: `with_metrics`. `state.rs`: `metrics: Option<Metrics>`.
   - `crates/typrest/src/http/handlers/metrics.rs` (new): `GET /_metrics` handler and a `404`
     handler for the disabled case; wire in `router.rs`. Collection sets `typrest_templates` from
     the registry before encoding.
   - Middleware (`axum::middleware::from_fn_with_state`) records `typrest_http_requests_total` and
     duration with `MatchedPath` mapped to the closed `route` set.
   - `render_template.rs`: in-flight guard (increment before `render`, decrement and optional
     `cancelled` on `Drop`); no change to responses or log fields.
   - `serve.rs`: build `Metrics` when `config.metrics`, attach observer via
     `RenderService::with_observer`, log `metrics.enabled`.
   - Tests (`crates/typrest/src/http.rs`, `oneshot`): disabled returns `404` and not `400`;
     enabled returns the content type and families; render success, `422`, `404`, and `500` change
     the right series; unknown IDs never create new `template` values (send 20 random IDs); GET
     `/metrics` still returns `400 invalid_template_id`; in-flight gauge returns to zero.
   - Docs in this commit: create `docs/specs/metrics.md` (Status: Current),
     update `http-api.md`, `typrest-toml.md`, `specs/README.md`, `docs/README.md`.

4. **Watch mode, build info, process start.**
   - Pass `Option<Metrics>` into `start_template_watcher` (`watch.rs:27`) and the observed reload
     functions (`:153`, `:292`); count `scope`/`outcome` where the existing `info!`/`warn!` events
     fire (`:173`, `:273`, `:314`, `:379`) and set the last-success timestamp.
   - Register `typrest_build_info` and `typrest_process_start_time_seconds` in `Metrics::new`.
   - Tests (`watch.rs`, existing mock-compiler tests): successful reload and last-known-good failure
     increment `ok` and `error`; registry rebuild success and failure.
   - Update [Template lifecycle](../architecture/template-lifecycle.md#watch-mode) with one sentence.

5. **Optional: separate metrics listener.**
   - `crates/typrest/src/args.rs`: `serve --metrics-port <port>`; `lib.rs` and `serve.rs` pass it
     through; fail before listening when `metrics = false` (`REQ-CLI-044`).
   - `crates/typrest/src/http.rs`: when set, spawn a second `axum::serve` on the same hostname
     serving only `/_metrics`, and do not mount it on the main router; shut both down together.
   - Tests: argument parsing; config error; integration test on random local ports (existing end-to-end
     pattern at `http.rs:922`) asserting the main port returns `404` for `/_metrics`.
   - Update `cli.md` (`REQ-CLI-044`, option table) and `http-api.md` (`REQ-HTTP-071`).

6. **Documentation, examples, and closeout.**
   - `README.md` (HTTP API/Docker sections): enabling metrics, a Prometheus scrape snippet using
     `metrics_path: /_metrics`, and the loopback/network warning.
   - `docs/specs/logging.md` Non-goals; `docs/architecture/overview.md` (observer in core, metrics
     in the application layer, system boundary table); `docs/architecture/template-lifecycle.md`;
     `ROADMAP.md` (new delivered item under CLI, HTTP, and Diagnostics); `docs/README.md` index.
   - If approved, add the ADR (`0009`) and its index row in the same commit as step 1 or here.
   - Container check: optional `scripts/check-container.sh` extension is out of scope.
   - Delete this plan and remove it from [`README.md`](README.md) "Active plans".
   - Check: `python3 scripts/check-doc-links.py` and `git diff --check`.

## Documents to update on completion

- New: `docs/specs/metrics.md` (domain `REQ-MET`), indexed in
  [`../specs/README.md`](../specs/README.md) and [`../README.md`](../README.md).
- [`../specs/http-api.md`](../specs/http-api.md): `REQ-HTTP-003`, `REQ-HTTP-050`/`051` notes, new
  `REQ-HTTP-070` to `072`.
- [`../specs/typrest-toml.md`](../specs/typrest-toml.md): `metrics` field, `REQ-CFG-050`/`051`.
- [`../specs/cli.md`](../specs/cli.md): `--metrics-port`, `REQ-CLI-044` (if step 5 ships).
- [`../specs/logging.md`](../specs/logging.md): Non-goals.
- [`../architecture/overview.md`](../architecture/overview.md) and
  [`../architecture/template-lifecycle.md`](../architecture/template-lifecycle.md).
- [`../development/testing.md`](../development/testing.md): test placement row for metrics, if new
  test conventions were introduced.
- [`../adr/README.md`](../adr/README.md): index row, only if the ADR is approved and created.
- [`../../ROADMAP.md`](../../ROADMAP.md) and [`../../README.md`](../../README.md).
- Delete this plan and update [`README.md`](README.md) "Active plans".
