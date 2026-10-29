# Plan: Operational Limits — Body Limit, Graceful Shutdown, Render Timeout

- **Status:** Draft
- **Owner:** Talison Fabio

## Objective

Make `typrest serve` safe to run unattended behind Docker, Compose, or Kubernetes:

1. an **explicit, configurable request body limit** whose rejection uses the structured JSON
   envelope;
2. **graceful shutdown** on `SIGTERM` and `SIGINT`, with a bounded drain of in-flight requests;
3. a **per-render timeout** so a slow or pathological template cannot hold an HTTP response open
   indefinitely.

The three parts are independent in value. Body limit and graceful shutdown can ship before the
render timeout. The render timeout depends on
[`blocking-render-execution.md`](blocking-render-execution.md).

## Current behavior

Contracts: [HTTP API](../specs/http-api.md), [CLI](../specs/cli.md),
[`typrest.toml`](../specs/typrest-toml.md), [Logging](../specs/logging.md),
[Template lifecycle](../architecture/template-lifecycle.md#request-lifecycle),
[Compiler](../architecture/compiler.md). Only what matters here:

### Server loop and process lifecycle

- `crates/typrest/src/http.rs:17-31`: `serve()` binds the listener and runs
  `axum::serve(listener, app_router)` at line 28 with no `with_graceful_shutdown`. The future only
  finishes on an I/O error. No signal handler is installed anywhere in the codebase.
- `crates/typrest/src/main.rs:3`: `#[tokio::main]`. When the runtime is dropped, tokio's
  `BlockingPool` drop calls `shutdown(None)` and **waits without limit** for blocking tasks
  (tokio 1.53.1, `src/runtime/blocking/pool.rs:282-286`). Multi-thread worker threads are themselves
  blocking-pool threads, so even today a compile stuck on a worker thread would block process exit.
- `crates/typrest/src/commands/serve.rs:167-181`: the optional `TemplateWatcher` lives in
  `_watcher` until `run_serve` returns; `crates/typrest/src/loader/watch.rs:21-25` aborts its worker
  task on drop. Nothing stops watch mode before the server stops.
- `crates/typrest/src/commands/serve.rs:198-208`: `run_serve` returns `true` only if
  `crate::http::serve` returns `Ok`, which today never happens in normal operation.
- Workspace `Cargo.toml:29` already enables tokio's `signal` feature.
- `Dockerfile:33` uses `CMD ["typrest", "serve"]` (exec form), so Typrest is **PID 1** in the
  container. Linux ignores `SIGTERM` and `SIGINT` for PID 1 unless a handler is installed, so
  `docker stop` and `docker compose down` currently wait the full stop timeout (10 s by default) and
  then `SIGKILL` the process. `docker-compose.yml:16` has the same issue and sets no
  `stop_grace_period`. Kubernetes behaves the same with its 30 s default grace period.

### Health

- `crates/typrest/src/http/handlers/healthz.rs:4`: `GET /_healthz` always returns
  `{"status":"ok"}` (`REQ-HTTP-010`). There is no readiness concept.

### Request body

- `crates/typrest/src/http/handlers/render_template.rs:21`: the render handler extracts
  `body: Bytes`. axum applies its implicit default limit of 2 097 152 bytes
  (axum-core 0.5.6, `src/ext_traits/request.rs:319`). Nothing in Typrest configures it, and no
  specification mentions it.
- An oversized body is rejected by axum **before the handler runs** with `413` and the plain-text
  body `Failed to buffer the request body: …` (axum-core `src/extract/rejection.rs:41-47`). This
  already violates `REQ-HTTP-040` (every error is a JSON envelope).
- The handler checks the content type (line 32) and parses JSON (line 43) after extraction, so an
  oversized body with an invalid template ID currently returns `413`, not `400`.
- `crates/typrest/src/http/router.rs:8`: no layers are applied to the router.

### Render execution and timing

- `crates/typrest/src/http/handlers/render_template.rs:48-53`: the handler records `started_at`
  after JSON parsing and awaits `RenderService::render()` with no deadline.
- `crates/typrest-core/src/render.rs:16`: `RenderService::render()` validates, then awaits
  `PdfCompiler::compile_prepared()`. `crates/typrest-core/src/compiler.rs:65-79` calls the
  synchronous `world::compile_prepared()` (`compiler/world.rs:50-79`, `typst::compile` at line 55)
  directly inside the async function, so compilation currently blocks a tokio worker thread.
  Moving it to a blocking thread is the subject of
  [`blocking-render-execution.md`](blocking-render-execution.md).
- `typrest-core` has no tokio dependency and must stay free of HTTP concerns, so any deadline
  belongs in the application package.
- Render outcomes are logged as `render.completed` / `render.failed` with `error_kind`
  (`render_template.rs:121-128`). These event names are not specified anywhere yet.

### Typst cancellation (investigated, typst 0.15.1)

Typst has **no cancellation, interruption, or deadline API**:

- `typst::compile(world: &dyn World)` (`typst-0.15.1/src/lib.rs:74`) runs to completion.
- The `World` trait (`typst-library-0.15.1/src/lib.rs`) exposes only `library`, `book`, `main`,
  `source`, `file`, `font`, and `today`. They are called on demand, not periodically, so a pure
  computation loop never reaches them. Their results are memoized by `comemo`; making `file()` or
  `source()` deadline-dependent would break comemo's purity assumption and could poison the global
  memoization cache across requests.
- Built-in engine limits exist but are narrow: `while` loops stop after 10 000 iterations
  (`typst-eval-0.15.1/src/flow.rs:10`), and call depth is limited to 80
  (`typst-library-0.15.1/src/engine.rs:351`). `for` loops over large ranges, heavy layout, and
  large documents are unbounded.

Consequence: once compilation runs on a blocking thread, a tokio timeout can only stop *waiting*.
The thread keeps computing until Typst returns.

### Web UI

`webui/src/api.ts:122-131` turns any non-OK response into an `ApiRequestError` using the
envelope's `message` and shows `HTTP <status> · <error>` (`PdfPreview.svelte:33-34`). New error
codes are displayed without frontend changes. Aborting a render from the UI (`REQ-WEB-074`) drops
the server-side handler future, which has the same detach semantics as a timeout.

## Intended change

1. **Body limit.** `typrest serve` accepts a configured maximum request body size, default
   2 097 152 bytes (unchanged behavior). Oversized bodies return `413` with
   `{"error":"payload_too_large","message":…}`. Error precedence is defined.
2. **Graceful shutdown.** On the first `SIGTERM` or `SIGINT`, `serve` logs the signal, stops watch
   mode, stops accepting connections, lets in-flight requests finish for at most the shutdown
   timeout, then exits without waiting for detached blocking work. Exit status is `0` when the
   drain completes in time. A second signal forces an immediate exit with status `1`.
3. **Container integration.** Compose and documentation give the process a stop grace period larger
   than the shutdown timeout. The container check verifies that `docker stop` is prompt.
4. **Render timeout.** Every render has a deadline (default proposed: 20 s) that covers waiting for
   render capacity plus compilation and export. On expiry the client receives `504` with
   `{"error":"render_timeout",…}`. The detached compilation keeps its concurrency permit until
   Typst actually returns. Its result is discarded, and a log event records its real duration.

## Out of scope

- Preemptive cancellation of a running Typst compilation, or process-isolated render workers (see
  decision D1).
- Timeouts for startup warmup, `typrest check`, or watch-mode reloads. A template that loops
  "forever" during warmup still hangs startup. See open questions.
- Slow-client protection (header or body read timeouts, slowloris). Recommend a reverse proxy.
- Per-template limits or timeouts, rate limiting, authentication, and environment-variable
  configuration (a CLI non-goal).
- A readiness endpoint or "lame duck" delay (see decision D6 and open questions).
- JSON envelopes for other framework-generated errors (for example `405 Method Not Allowed` or
  unmatched methods). They also violate `REQ-HTTP-040` and deserve a separate fix.
- Web UI changes. The generic error path already displays the new codes.

## Affected requirements

### Existing requirements that change

| ID | Change |
|---|---|
| `REQ-HTTP-041` | Add rows: `413 payload_too_large` (step 1) and `504 render_timeout` (step 5). |
| `REQ-HTTP-040` | Unchanged text. Step 1 makes the oversized-body path comply. |
| `REQ-CLI-003` | Clarify that a signal-initiated shutdown that drains within the deadline is a successful completion of `serve` (exit `0`). |
| `REQ-CLI-042` | Unchanged. New shutdown events are specified separately (`REQ-CLI-047`). |
| `REQ-HTTP-010` | Unchanged under the recommended option D6. It changes only if a readiness option is approved. |
| CLI "Command summary" and `serve` option table | Add the new flags. |
| `typrest.toml` Non-goals | Under recommended option D3, add "request limits and timeouts" to the excluded fields. |

### Proposed new requirements (*proposed*, wording to be finalized in the spec change)

HTTP API ([`http-api.md`](../specs/http-api.md)), new group "Limits and timeouts". The next free
block after `REQ-HTTP-064` is `070`:

- **REQ-HTTP-070** *(proposed)*: A render request whose body exceeds the configured maximum body
  size MUST fail with `413` and `payload_too_large` in the JSON envelope. The body MUST NOT be
  parsed.
- **REQ-HTTP-071** *(proposed)*: A render request MUST report the first applicable error in this
  order: `invalid_template_id`, the content-type `invalid_json` check (`REQ-HTTP-021`),
  `payload_too_large`, JSON parse `invalid_json`, then render errors.
- **REQ-HTTP-072** *(proposed)*: A render that has not produced a document within the configured
  render timeout MUST fail with `504` and `render_timeout`. The deadline starts when the handler
  begins the render (after body parsing) and includes any wait for render capacity.
- **REQ-HTTP-073** *(proposed)*: A `render_timeout` response MUST NOT include `diagnostics`,
  `violations`, or `line`, and MUST NOT be reported as `compilation_failed`.
- **REQ-HTTP-074** *(proposed)*: After a timeout or client disconnect, a compilation already running
  MUST keep counting against render concurrency until it actually finishes. Its result MUST be
  discarded.
- **REQ-HTTP-075** *(proposed)*: A timed-out render MUST be logged as `render.failed` with
  `error_kind = "render_timeout"`. When its detached compilation later finishes, the server MUST log
  `render.abandoned` with the template and the total compilation duration.

CLI ([`cli.md`](../specs/cli.md)), `typrest serve`. The next free IDs after `REQ-CLI-043` are
`044`–`049`:

- **REQ-CLI-044** *(proposed)*: `serve` MUST accept the options below. Values MUST be positive
  integers. Zero or a non-integer is a usage error.

  | Option | Type | Default | Behavior |
  |---|---|---|---|
  | `--max-body-bytes <BYTES>` | integer ≥ 1 | `2097152` | Maximum render request body |
  | `--render-timeout <SECONDS>` | integer ≥ 1 | `20` | Per-render deadline (step 5) |
  | `--shutdown-timeout <SECONDS>` | integer ≥ 1 | `25` | Maximum drain time after a signal |

- **REQ-CLI-045** *(proposed)*: On the first `SIGTERM` or `SIGINT`, `serve` MUST stop watch mode,
  stop accepting new connections, close idle connections, and let in-flight requests complete for
  at most `--shutdown-timeout`.
- **REQ-CLI-046** *(proposed)*: `serve` MUST exit with status `0` when all in-flight requests finish
  within the shutdown timeout, and with status `1` when the deadline elapses first or a second
  signal arrives. A second `SIGTERM` or `SIGINT` MUST cause immediate exit.
- **REQ-CLI-047** *(proposed)*: Shutdown MUST log `server.shutdown.started` (field `signal`),
  then `server.shutdown.completed` (field `duration_ms`) or `server.shutdown.timed_out` (field
  `timeout_ms`), on stdout like every process event (`REQ-LOG-010`).
- **REQ-CLI-048** *(proposed)*: After the drain completes or its deadline elapses, the process MUST
  exit without waiting for detached compilations.

`typrest.toml`: **no new fields** under the recommended option D3. If the alternative is chosen,
see the field list in D3. It would use `REQ-CFG-050` onward.

Logging ([`logging.md`](../specs/logging.md)): no new requirement IDs. Add the shutdown sequence to
the examples.

## Decisions and alternatives

### D1. What a render timeout can do to a running compilation

**ADR proposal — needs human approval.** Two viable alternatives exist, the choice constrains
operational behavior and capacity guarantees, and the rationale does not follow from existing
documents.

| Option | Description | Trade-offs |
|---|---|---|
| A. Response deadline, non-preemptive (**recommended**) | `tokio::time::timeout` around the render. On expiry, return `504` and detach. The blocking thread finishes on its own and keeps its permit until then. | Simple and in-process. Matches the current single-binary design. Does **not** bound CPU. N pathological requests can occupy all N permits for as long as Typst runs, so it limits client waiting, not resource use. |
| B. Process-isolated render workers | Compile in a child process (pre-forked pool) that can be killed on deadline. | True preemption and memory isolation. Large change: IPC and serialization of prepared state, startup cost, a new failure model, and a possible conflict with the "one binary, in-process Typst" vision and the compiler ACL layout. |
| C. Cooperative cancellation inside Typst | Patch or fork Typst, or abuse `World` callbacks, to check a deadline. | No upstream API in 0.15.1. `World` callbacks are not reached in compute loops and are memoized by comemo (cache poisoning risk). A fork breaks the "exact pinned version" rule. Not viable now. |

Recommendation: A now, with an explicit risk entry. Revisit B if real usage shows runaway templates.
The ADR would record A versus B.

### D2. Status code and error code for timeouts

| Option | Trade-offs |
|---|---|
| `504 render_timeout` (**recommended**) | Conventional for deadline exceeded (gRPC `DEADLINE_EXCEEDED` maps to 504). Separates timeouts from capacity rejections, so `503` stays free for overload and shutdown. |
| `503 render_timeout` | Implies "retry later". Retrying a deterministic slow render does not help. It also conflicts with a possible `503` capacity code from [`blocking-render-execution.md`](blocking-render-execution.md). |
| `408` | Means the *client* was too slow to send the request. Semantically wrong. |
| `500 compilation_failed` | Mixes infrastructure limits with template errors. Breaks `REQ-HTTP-043`, because there are no diagnostics. |

A spec-level contract choice, not an ADR.

### D3. Where the operational settings live

| Option | Trade-offs |
|---|---|
| A. `serve` CLI flags (**recommended**) | Consistent with `--port`, `--hostname`, and `--watch`. The `typrest.toml` spec explicitly excludes deployment settings ("ports, hostnames, watch settings"). Easy to set per environment in Compose and Kubernetes args. No precedence rules. |
| B. `typrest.toml` `[server]` table | Values travel with the project, which helps because render cost depends on its templates. Reverses a stated non-goal. Needs a precedence rule if flags are added later. Fields: `max_body_bytes` (integer, default `2097152`), `render_timeout_secs` (integer, default `20`), `shutdown_timeout_secs` (integer, default `25`). Wrong types fail loading (`REQ-CFG-002`). |
| C. Environment variables | Explicitly a CLI non-goal ("environment-variable configuration overrides"). |
| D. Both A and B with precedence | Most flexible. Most specification and test surface. Premature. |

Recommendation: A. A spec change, not an ADR, unless the human wants to change the project and
deployment configuration split, which would justify an ADR.

### D4. Default values

- `--max-body-bytes 2097152`: preserves current behavior. No client breaks.
- `--render-timeout 20` and `--shutdown-timeout 25`. Rationale: shutdown timeout larger than render
  timeout means every in-flight request gets a real response (a PDF or `504`) before forced exit,
  and 25 s fits inside Kubernetes' default 30 s grace period. Docker's default stop timeout is 10 s,
  so the Compose file and README must raise it (step 4). A lower `--shutdown-timeout` is allowed;
  the documentation SHOULD recommend `shutdown ≥ render timeout`. No hard validation is proposed.
- These are proposals. See the open questions.

### D5. Where the timeout is enforced

| Option | Trade-offs |
|---|---|
| A. Application package, around the render call (**recommended**) | `typrest-core` stays runtime-agnostic (no tokio). Implemented in the handler or in the app-level render executor that [`blocking-render-execution.md`](blocking-render-execution.md) introduces. Async jobs can reuse the same helper. |
| B. `tower` timeout layer (tower-http `TimeoutLayer`) | Also counts body upload time, returns `408` with a non-JSON body, and needs a new dependency. Rejected. |
| C. Inside `RenderService` in core | Adds a timer dependency to core. Rejected by the "core has no I/O or runtime coupling" rule. |

### D6. Health and readiness during shutdown

| Option | Trade-offs |
|---|---|
| A. No change (**recommended for this plan**) | After the signal, the listener closes, so probes get "connection refused" and the pod becomes unready. Kubernetes users add a `preStop` `sleep` to cover endpoint propagation. Documented, not coded. |
| B. "Lame duck" delay (`--shutdown-delay`): keep serving and make `/_healthz` return `503` before closing the listener | Best practice for Kubernetes rolling updates without a `preStop` hook. Changes `REQ-HTTP-010` and conflates liveness with readiness. |
| C. New `/_readyz` route | Clean separation. Adds a route to `REQ-HTTP-001` and the route set. |

Product decision. See the open questions. B or C would become their own follow-up.

### D7. Body-limit rejection handling

Recommended: set `axum::extract::DefaultBodyLimit::max(config.max_body_bytes)` as a router layer,
and change the handler to extract `Result<Bytes, BytesRejection>`. This lets the handler apply the
`REQ-HTTP-071` order and map `FailedToBufferBody::LengthLimitError` to `413 payload_too_large`.
Other buffering failures (client aborted mid-body) map to `400 invalid_json` with the message
"failed to read request body", so no extra code is added. Alternative: a global middleware that
rewrites every non-JSON error body. It is broader, and better addressed in the separate
`REQ-HTTP-040` cleanup noted in Out of scope.

### D8. Shutdown coordination primitive

Recommended: a small `shutdown` module in the application package that owns signal listening and
exposes a cloneable `ShutdownSignal` handle built on `tokio::sync::watch`, with no new dependency.
Consumers are the axum graceful-shutdown future, watch mode, and later async job workers and
metrics flushing. `tokio_util::sync::CancellationToken` is an alternative, but it adds a
dependency for little gain.

### D9. Runtime exit without waiting for detached work

Recommended: replace `#[tokio::main]` with an explicitly built multi-thread runtime in
`main.rs`/`lib.rs`. After `serve` returns, call `Runtime::shutdown_background()` (or
`shutdown_timeout(Duration::ZERO)`). Alternative: `std::process::exit` from inside `run_serve`. It
works, but skips destructors and makes testing awkward. Runtime build failure must not `unwrap()`:
log through the already-initialized subscriber and exit `1`. This requires parsing args and
initializing the subscriber before the runtime is built (stdout only, `REQ-LOG-011`).

## Risks

- **Timeouts do not bound CPU (D1-A).** A template with an unbounded `for` loop or huge layout pins
  a permit and a core until Typst returns. Repeated requests can saturate every permit; later
  requests then time out while waiting for capacity. Mitigations are operational: concurrency limit,
  watch for `render.abandoned` logs and metrics, and fix templates.
- **Memory of abandoned work.** Detached compilations keep their prepared state `Arc`s, payload,
  and partially built document alive until they finish.
- **Watch reload during shutdown.** A reload in progress is aborted only at an `.await` point. If
  its compile runs synchronously, shutdown waits for it unless it runs on a blocking thread too.
  This needs confirmation once `blocking-render-execution.md` decides where watch compiles run.
- **PID 1 behavior.** Signal handling fixes the 10 s `docker stop` delay only if handlers are
  installed before any slow startup work. Warmup happens before binding: a `SIGTERM` during warmup
  must still terminate promptly (see open questions).
- **Kubernetes endpoint propagation.** Without a `preStop` delay (D6-A), a few requests can hit a
  closed listener during rollouts.
- **Test flakiness.** Shutdown and timeout tests involve timing. Use explicit synchronization
  (`tokio::sync::Notify`, barriers, oneshot channels) and generous upper bounds, never sleeps as
  assertions.
- **Signal tests are Unix-only.** Gate them with `#[cfg(unix)]`. `tokio::signal::ctrl_c` remains
  cross-platform; `SIGTERM` uses `tokio::signal::unix`.

## Steps

Steps 1–4 do not depend on any sibling plan. Step 5 requires
[`blocking-render-execution.md`](blocking-render-execution.md) to have landed. Each step updates its
own specifications in the same commit (AGENTS.md "Documentation Updates").

1. **Explicit body limit with JSON `413`.**
   - `crates/typrest/src/args.rs`: add `--max-body-bytes` to `Command::Serve`
     (`clap::value_parser!(u64).range(1..)` or the equivalent for `usize`). Extend the existing
     parse tests (default, custom value, `0` rejected).
   - `crates/typrest/src/lib.rs` and `commands/serve.rs`: pass the value through.
   - `crates/typrest/src/http/config.rs`: add `max_body_bytes` to `ServerConfig`, with a named
     constant `DEFAULT_MAX_BODY_BYTES: usize = 2 * 1024 * 1024` and a `with_max_body_bytes`
     builder.
   - `crates/typrest/src/http/router.rs`: apply `DefaultBodyLimit::max(config.max_body_bytes)`.
   - `crates/typrest/src/http/handlers/render_template.rs`: extract
     `Result<Bytes, BytesRejection>` and apply the `REQ-HTTP-071` order (D7).
   - `crates/typrest/src/http/errors.rs`: add a named constructor for `payload_too_large`.
   - Tests in the `http.rs` test module: an oversized body returns `413` with a JSON envelope and
     `error == "payload_too_large"`; a body of exactly the limit is accepted; a custom small limit
     via `ServerConfig` is honored; an invalid template ID with an oversized body returns `400
     invalid_template_id`; a wrong content type with an oversized body returns `400 invalid_json`.
   - Specs: `http-api.md` (`REQ-HTTP-041` row, `REQ-HTTP-070`, `REQ-HTTP-071`), `cli.md`
     (summary, option table, the `--max-body-bytes` part of `REQ-CLI-044`), `typrest-toml.md`
     Non-goals, README "CLI" and "HTTP API".
   - Done when: `cargo test -p typrest http`, `cargo test -p typrest args`, and the full validation
     commands in AGENTS.md pass.

2. **Graceful shutdown with bounded drain.**
   - New `crates/typrest/src/shutdown.rs`: `ShutdownSignal` (D8). A listener task waits for
     `tokio::signal::ctrl_c()` or `SIGTERM` (Unix), records which signal fired, and notifies
     subscribers. A second signal triggers immediate exit with status `1`
     (`std::process::exit(1)` after logging is acceptable here). Install it at the start of
     `run_serve`, before config loading and warmup, so `SIGTERM` during warmup aborts startup
     with exit `1` (pending the open question).
   - `crates/typrest/src/http.rs`: change `serve` to take a shutdown future, or a `ShutdownSignal`
     and a `Duration`. Use `axum::serve(..).with_graceful_shutdown(..)`, then race the drain
     against `tokio::time::sleep(shutdown_timeout)` started when the signal fires (axum's own
     drain has no deadline). Return a distinct `ServeError::ShutdownTimedOut` on expiry.
   - `crates/typrest/src/commands/serve.rs`: on signal, drop the `TemplateWatcher` before draining;
     log `server.shutdown.started`, `server.shutdown.completed`, or `server.shutdown.timed_out`
     (`REQ-CLI-047`); map the result to the exit status (`REQ-CLI-046`). Add
     `--shutdown-timeout` to `args.rs` with tests.
   - Tests:
     - In-process (`http.rs` tests), with a oneshot as the shutdown trigger, a server on
       `127.0.0.1:0`, and a mock compiler gated by `tokio::sync::Notify`: an idle server returns
       `Ok` promptly; an in-flight request started before the signal completes with `200` while a
       new connection attempt fails; a gated request that never completes makes `serve` return
       `ShutdownTimedOut` after a short test deadline.
     - New integration test `crates/typrest/tests/shutdown.rs` (`#[cfg(unix)]`), following
       `tests/logging.rs`: spawn the binary in JSON log mode, wait for the port, send `SIGTERM`
       through `std::process::Command::new("kill").args(["-TERM", pid])`, and assert exit status
       `0` within a bound and that stdout contains `server.shutdown.started` and
       `server.shutdown.completed`. Repeat for `SIGINT`. Also test that `--watch` shuts down
       cleanly.
   - Specs: `cli.md` (`REQ-CLI-003` clarification, `REQ-CLI-044` shutdown option, `REQ-CLI-045` to
     `REQ-CLI-047`), `logging.md` example of the shutdown sequence, README "CLI".
   - Done when: the new tests pass, the full suite passes, and a manual
     `typrest serve` + Ctrl+C exits immediately with the documented events.

3. **Runtime exit that never waits for detached blocking work (D9).**
   - `crates/typrest/src/main.rs` and `crates/typrest/src/lib.rs`: build the runtime explicitly,
     run the command, then call `shutdown_background()`. Keep `init` and `check` behavior
     identical. Parse args and initialize the subscriber before the runtime so a runtime build
     error is logged to stdout.
   - Spec: `REQ-CLI-048`.
   - Test: extend `tests/shutdown.rs` with a template whose warm compile is cheap but whose
     render loops for several seconds (payload-controlled loop bound). Start a render, send
     `SIGTERM` with `--shutdown-timeout 1`, and assert that the process exits within about 2 s with
     status `1` and logs `server.shutdown.timed_out`. Before step 5 and the blocking plan, the
     render runs on a worker thread, so this test also proves that runtime drop no longer waits
     for it.
   - Done when: that test passes reliably (run it 20 times locally) and the full suite passes.

4. **Container and deployment integration.**
   - `docker-compose.yml`: add `stop_grace_period: 30s` and a comment linking it to
     `--shutdown-timeout`.
   - `Dockerfile`: add `STOPSIGNAL SIGTERM` (explicit, documents intent) and a comment noting that
     Typrest runs as PID 1 and handles signals itself.
   - `scripts/check-container.sh`: time a `stop` of the minimal container and fail if it exceeds a
     bound well below the engine's 10 s default (for example 5 s), or if the container exit code is
     not `0`.
   - README "Quickstart with Docker": mention `docker stop -t`/`--stop-timeout` and the Kubernetes
     `terminationGracePeriodSeconds` / `preStop` guidance (D6-A).
   - `docs/development/testing.md` "Container Check": describe the new stop assertion.
   - Done when: `scripts/check-container.sh` passes locally with Docker or Podman, plus
     `python3 scripts/check-doc-links.py` and `git diff --check`.

5. **Render timeout (requires [`blocking-render-execution.md`](blocking-render-execution.md)).**
   - `args.rs`: `--render-timeout`, with tests. `ServerConfig` or `AppState`: carry a
     `render_timeout: Duration` with a named default constant.
   - Apply the deadline in the application package (D5): wrap the render executor call introduced
     by the blocking plan, or `state.render_service.render(..)` at
     `render_template.rs:50-53`, in `tokio::time::timeout`. On `Elapsed`, return
     `ApiError::render_timeout()` (`504`, `render_timeout`, no diagnostics) and log `render.failed`
     with `error_kind = "render_timeout"`, `latency_ms`, and, if the executor exposes it, a `phase`
     field (`queued` or `compiling`).
   - Detached completion: the blocking closure owns the `OwnedSemaphorePermit` and sends its result
     through a oneshot. If the receiver is gone, it logs `render.abandoned` (template,
     `compile_ms`) and drops the result (`REQ-HTTP-074`, `REQ-HTTP-075`). This relies on the
     blocking plan's design (see Dependencies).
   - `render_error_kind` and `ApiError`: add the new kind. Do not add a `RenderError` variant in
     core, because the timeout is an application concern. Use an application-level error enum if
     needed.
   - Tests (mock compilers, per `docs/development/testing.md`):
     - A blocking mock that waits on a `std::sync::Barrier` or `std::sync::mpsc` with a 50 ms
       timeout returns `504` with the exact envelope and no `diagnostics`.
     - Permit retention: with concurrency `1`, time out request A; request B must not start
       compiling until A's mock is released. Assert this with a counter in the mock.
     - Queue wait counts: with concurrency `1` and A holding the permit, B times out with `504`
       without its mock ever being called.
     - A fast render under the deadline is unaffected.
     - One real-Typst end-to-end test (`render_real_template_over_http` style) with a
       payload-bounded loop and a small timeout returns `504`. Keep the loop bounded to a few
       seconds so the detached thread ends within the test run.
   - Specs: `http-api.md` (`REQ-HTTP-041` row, `REQ-HTTP-072` to `REQ-HTTP-075`, an example
     envelope), `cli.md` (the `--render-timeout` part of `REQ-CLI-044`),
     `architecture/template-lifecycle.md` Request Lifecycle (deadline step), `architecture/overview.md`
     Error Boundaries (timeouts are not compilation errors), README "CLI".
   - Done when: the focused tests and the full validation commands pass.

6. **Close out.** Update `ROADMAP.md` (a delivered "Operational limits" item under CLI, HTTP, and
   Diagnostics, or a new milestone), check that the specs no longer reference this plan, run
   `python3 scripts/check-doc-links.py`, remove this plan from the
   [plans index](README.md#active-plans), and delete this file.

## Dependencies on other plans

| Plan | Needed from it | Provided to it |
|---|---|---|
| [`blocking-render-execution.md`](blocking-render-execution.md) | **Prerequisite for step 5.** (a) Compilation runs on a blocking thread through an app-level executor that returns a future the handler can drop. (b) The concurrency permit is an `OwnedSemaphorePermit` **moved into the blocking closure**, not held by the awaiting future, so dropping the future (timeout or disconnect) never releases capacity early. (c) A oneshot or equivalent result channel whose closed receiver the closure can detect, to log `render.abandoned`. (d) If it adds a bounded queue with an immediate `503` rejection, its error code and its precedence relative to `render_timeout`. (e) Whether watch-mode and warmup compiles also move off the runtime threads (affects the shutdown risk). | The timeout wrapper, the `504 render_timeout` contract, the D9 runtime exit (without it, detached blocking tasks would stall process exit), and `ShutdownSignal`. |
| [`render-metrics.md`](render-metrics.md) | Metric names and labels, if metrics are emitted instead of or in addition to logs. | Events and counters to expose: timeouts (by phase), abandoned compilations and their duration, `413` rejections, shutdown duration and outcome. `ShutdownSignal` for a final flush. Note that `413` and other pre-render rejections are currently not logged. |
| [`render-result-cache.md`](render-result-cache.md) | A decision on whether an abandoned compilation's successful result may populate the cache (it would make a retry fast), and confirmation that cache hits bypass the render deadline and permits. | The guarantee that timed-out results are otherwise discarded (`REQ-HTTP-074`). |
| [`async-render-jobs.md`](async-render-jobs.md) | Job execution deadline semantics: recommended to reuse `--render-timeout` as the per-job compile deadline, with job status `failed` and code `render_timeout`. Shutdown behavior for queued and running jobs (drain, persist, or fail) within `--shutdown-timeout`. | `ShutdownSignal` (stop accepting jobs, drain workers), the timeout helper, and the D9 runtime exit. The HTTP render timeout does not apply to the `202` submission response. |
| [`pdf-output-destinations.md`](pdf-output-destinations.md) | Whether destination writes are inside or outside the render deadline. Recommended: outside, with their own timeout. Drain behavior for uploads in progress at shutdown. | The shutdown drain covers the whole request, including destination writes, up to `--shutdown-timeout`. |

## Open questions

1. Approve the D1 ADR proposal (non-preemptive response deadline versus process-isolated workers)?
2. Defaults: are `20 s` render timeout, `25 s` shutdown timeout, and `2 MiB` body limit right for
   the intended deployments? Should `--render-timeout` accept `0` to disable the deadline (useful
   when developing heavy templates with `--watch`)?
3. Configuration surface (D3): CLI flags only, or also `typrest.toml`?
4. Readiness during shutdown (D6): is documentation-only (`preStop` sleep) acceptable, or is a lame
   duck delay or `/_readyz` wanted?
5. Exit status after a drain timeout or a second signal: is `1` right, or should a signal-initiated
   shutdown always exit `0` (or `130`/`143` by Unix convention)?
6. Should `SIGTERM` during startup warmup abort immediately (exit `1`) or be honored only after
   binding?
7. Should warmup, `typrest check`, and watch reloads also get a compile deadline, so a template
   with a near-infinite loop cannot hang startup or the watch worker?
8. Units: are whole seconds enough for timeouts, and raw bytes for the body limit, or should the
   flags accept suffixes such as `500ms` and `4MiB` (which needs a parser or a new dependency)?
9. Should `413` and other pre-render rejections be logged (currently `400` rejections are not)?

## Documents to update on completion

- [`docs/specs/http-api.md`](../specs/http-api.md): `REQ-HTTP-041` rows, new `REQ-HTTP-070` to
  `REQ-HTTP-075`, timeout and `413` examples.
- [`docs/specs/cli.md`](../specs/cli.md): command summary, `serve` option table, `REQ-CLI-003`
  clarification, `REQ-CLI-044` to `REQ-CLI-048`.
- [`docs/specs/typrest-toml.md`](../specs/typrest-toml.md): Non-goals (or new `REQ-CFG-050`+ if
  D3-B is chosen).
- [`docs/specs/logging.md`](../specs/logging.md): shutdown example.
- [`docs/architecture/template-lifecycle.md`](../architecture/template-lifecycle.md): request
  deadline, and watch-mode stop on shutdown.
- [`docs/architecture/overview.md`](../architecture/overview.md): Error Boundaries (timeout and
  body limit are not render errors from the compiler).
- [`docs/development/testing.md`](../development/testing.md): container stop check, shutdown
  integration tests.
- [`README.md`](../../README.md): CLI options, error table, Docker stop guidance.
- `Dockerfile`, `docker-compose.yml`, `scripts/check-container.sh`.
- [`ROADMAP.md`](../../ROADMAP.md): delivered status.
- [`docs/adr/`](../adr/README.md): the D1 ADR, only if approved.
- [`docs/plans/README.md`](README.md): add this plan to "Active plans" when adopted, and remove it
  on completion.
