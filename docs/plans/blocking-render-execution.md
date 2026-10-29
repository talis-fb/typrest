# Plan: Blocking Render Execution and Render Concurrency Limits

- **Status:** Draft
- **Owner:** Talison Fabio

## Objective

Run request-time Typst compilation and PDF export off the async runtime, and bound how many renders
run and wait at once. Today a few CPU-heavy renders occupy every Tokio worker thread, so unrelated
requests, including `GET /_healthz`, stall behind them. After this work, render CPU time runs on
blocking threads behind an admission limit. Health and metadata routes stay responsive, and an
overloaded server rejects work with a defined response instead of queueing without limit.

This plan is the foundation for [`operational-limits.md`](operational-limits.md) and is consumed by
the other sibling plans listed in [Dependencies on other plans](#dependencies-on-other-plans).

## Current behavior

Contracts: [HTTP API](../specs/http-api.md) (`REQ-HTTP-010`, `REQ-HTTP-020` to `REQ-HTTP-025`,
`REQ-HTTP-041`), [`typrest.toml`](../specs/typrest-toml.md), [CLI](../specs/cli.md)
(`REQ-CLI-040` to `REQ-CLI-043`). Structure: [architecture overview](../architecture/overview.md),
[template lifecycle](../architecture/template-lifecycle.md), and
[compiler reference](../architecture/compiler.md).

Facts that matter for this plan, verified in the code:

- **The runtime.** `crates/typrest/src/main.rs:3` uses `#[tokio::main]`, which is the default
  multi-thread runtime with one worker per core. `crates/typrest/src/http.rs:17-31` calls
  `axum::serve` with no concurrency, timeout, or load-shedding layer.
- **The handler.** `crates/typrest/src/http/handlers/render_template.rs:48-53` awaits
  `RenderService::render` directly on the request task.
- **The render service.** `RenderService::render` (`crates/typrest-core/src/render.rs:16-32`) is
  `async`, but its only awaited call is `PdfCompiler::compile_prepared` (`render.rs:28-31`). The
  Typst implementation of that method (`crates/typrest-core/src/compiler.rs:65-79`) contains no
  await. It calls `world::compile_prepared` (`crates/typrest-core/src/compiler/world.rs:50-79`),
  which runs `typst::compile` and `typst_pdf::pdf` synchronously. The whole compile and export
  therefore runs on a Tokio worker thread.
- **The compiler port.** `PdfCompiler` (`crates/typrest-core/src/compiler.rs:19-28`) declares both
  `prepare` and `compile_prepared` as `async`. `prepare` really awaits `PackageRepository::resolve`;
  `compile_prepared` never needs to.
- **The only blocking offload.** Package download and cache reading use
  `tokio::task::spawn_blocking` in `crates/typrest/src/package/universal_package.rs:126`. No render
  path uses it.
- **Core has no Tokio dependency.** `crates/typrest-core/Cargo.toml` depends on `async-trait` but not
  on `tokio`. Core tests drive async code with `futures_executor::block_on`, for example
  `render.rs:114`. The workspace `tokio` features (`Cargo.toml`) already include `rt-multi-thread`
  and `sync`, so `spawn_blocking` and `Semaphore` are available to the application package.
- **Registry sharing.** `TemplateRegistry::get` (`crates/typrest-core/src/registry.rs:124-130`)
  returns `Arc<PreparedTemplateBundle>`, so a render can own its prepared template independently
  of later registry swaps (`replace_all` at `registry.rs:141-151`, `replace` at
  `registry.rs:153-158`). `PreparedTemplate` is already `Send + Sync`, because the handler future
  holds a reference to it across `.await` and axum requires `Send` futures.
- **The warmup path.** `prepare_template_bundle` (`registry.rs:181-237`) warm-compiles every
  template through the same `compile_prepared` call (`registry.rs:213-219`). It runs during startup
  in `crates/typrest/src/commands/serve.rs:72-78`, in `typrest check`
  (`crates/typrest/src/commands/check.rs:63`), and in watch mode. Watch mode runs on a
  `tokio::spawn` worker (`crates/typrest/src/loader/watch.rs:44`), concurrently with HTTP traffic,
  and calls `TemplateRegistry::build_with_observer` (`watch.rs:160`) or `reload_with_observer`
  (`watch.rs:301`). A watch reload therefore also compiles on a runtime worker while the server
  is live.
- **Error mapping.** `RenderError` has three variants, `TemplateNotFound`, `ValidationError`, and
  `CompilationError`. They are matched exhaustively in `ApiError::from_render_error`
  (`crates/typrest/src/http/errors.rs:284-303`) and in `render_error_kind`
  (`render_template.rs:121-128`). `ApiError` (`errors.rs:264-268`) has no way to set response
  headers.
- **Panics.** A panic inside Typst during a request unwinds the hyper connection task. The client
  gets a dropped connection, not the JSON envelope required by `REQ-HTTP-040`.
- **Typst internals that affect the design.** These were verified in the pinned 0.15.1 sources.
  - Typst parallelizes internally on the **global rayon pool**. See `Engine::parallelize` in
    `typst-library/src/engine.rs:75` and `Deferred` in `typst-utils/src/deferred.rs:21`. One
    compile can therefore use more than one CPU, even when it is entered from a single thread.
  - Typst memoizes through `comemo`. Its caches are process-global and lock-protected, so
    compilation from several threads at once is supported. Typst re-exports `comemo` as
    `typst::comemo`.
  - Nothing in `crates/` calls `comemo::evict`, so the memoization cache is never trimmed.
  - Typst stops a `while` loop after 10 000 iterations (`typst-eval/src/flow.rs:10`). That bounds
    trivial infinite loops but does not bound total compile time.
- **Configuration.** `RawProjectConfig` (`crates/typrest/src/config.rs:19-25`) has only
  `schema_version`, `templates_root`, and `web_ui`. Unknown fields are ignored (`REQ-CFG-003`). The
  [`typrest.toml` non-goals](../specs/typrest-toml.md#non-goals) state that the file defines no
  server settings yet.

## Intended change

1. **A synchronous compile method.** `PdfCompiler::compile_prepared` becomes a synchronous,
   CPU-bound method. `prepare` stays `async`.
2. **An executor port in core.** `typrest-core` defines a runtime-agnostic `RenderExecutor` port.
   `RenderService` hands each request compilation to the executor as an owned `'static` job. Core
   stays free of Tokio.
3. **The Tokio executor.** The application package implements `BlockingRenderExecutor` with Tokio
   `spawn_blocking`, plus a FIFO-fair semaphore of `max_concurrent_renders` slots and an admission
   counter that caps waiting requests at `max_queued_renders`.
4. **Saturation behavior.** When every slot and queue position is taken, a render request fails
   immediately with `503`, error code `render_capacity_exceeded`, and a `Retry-After` header. It is
   never compiled.
5. **Work that never takes a slot.** Template-ID parsing, JSON parsing, registry lookup, and schema
   validation still run before admission. Responses with status `400`, `404`, and `422` therefore
   never wait for capacity and never consume it.
6. **Panics.** A panic during request compilation becomes `500` with the error code
   `render_execution_failed`. The server keeps serving.
7. **Warmup through the executor.** Startup, `check`, and watch-mode warmup compiles go through the
   same executor with a "background" admission class. That class waits for a slot and is never
   rejected, so watch reloads share CPU capacity with requests instead of competing on runtime
   workers.
8. **In-flight renders during a watch swap.** A request in flight keeps the `Arc` to the prepared
   bundle it started with. It completes against that version, even if watch mode swaps the
   registry in the meantime.
9. **Configuration.** An optional `[server]` table in `typrest.toml` configures the two limits.
   Defaults are derived from the host's available parallelism.
10. **Interfaces for sibling plans.** The executor exposes a stats snapshot and an admission
    extension point that the sibling plans build on.

## Out of scope

- Render timeouts, request deadlines, body-size limits, and graceful shutdown or draining. These
  belong to [`operational-limits.md`](operational-limits.md). This plan only provides the hooks
  they need.
- Metrics endpoints and exporters. These belong to [`render-metrics.md`](render-metrics.md).
- Caching rendered PDFs. This belongs to [`render-result-cache.md`](render-result-cache.md).
- Asynchronous job submission. This belongs to [`async-render-jobs.md`](async-render-jobs.md).
- Writing PDFs to external destinations. This belongs to
  [`pdf-output-destinations.md`](pdf-output-destinations.md).
- Cancelling a compilation that has already started. Typst exposes no cooperative cancellation
  hook in `World`.
- Moving `prepare`, `FilesystemLoader` I/O, or schema validation off the runtime. Watch-mode
  preparation and loading still run on a runtime worker. This is a dev-mode cost that is recorded
  as a follow-up.
- Parallelizing startup warmup across templates.
- Changing `comemo` eviction. This is recorded as an open question.
- Per-template or per-client limits, priorities between clients, and rate limiting.

## Affected requirements

### Existing requirements that change

- **`REQ-HTTP-041`:** The error table gains two rows: `503 render_capacity_exceeded` and
  `500 render_execution_failed`.
- **`REQ-HTTP-010`:** The text is unchanged, but it gains a companion requirement
  (`REQ-HTTP-011`, below) that health does not depend on render capacity.
- **`REQ-HTTP-044`:** No textual change. It now also governs the new `render_execution_failed`
  body, which must not echo panic payloads, because those may contain source text or paths.
- **`REQ-CFG-002`:** No textual change. It now also covers a `[server]` table, or a field in it,
  with the wrong type.
- **`REQ-CLI-042`:** The `server.starting` event gains structured fields for the effective limits
  (see `REQ-CLI-044`).
- **[`typrest.toml` non-goals](../specs/typrest-toml.md#non-goals):** The sentence "This format does
  not define ports, hostnames, …" is narrowed. Render capacity becomes a defined setting, while
  ports and hostnames stay CLI options.

### Proposed new requirements

These requirements are *proposed* and are not yet in the specifications. The numbers are the next
free IDs in each domain or group. Wording is final only when the spec is edited.

**HTTP: health.**

- **REQ-HTTP-011 (proposed):** `GET /_healthz` MUST NOT wait for render capacity. It MUST return
  `200` while every render slot is occupied and the render queue is full.

**HTTP: a new "Render capacity" group starting at 070.**

- **REQ-HTTP-070 (proposed):** The server MUST run at most `max_concurrent_renders` request
  compilations, each including its PDF export, at the same time.
- **REQ-HTTP-071 (proposed):** A render request that finds every slot occupied MUST wait for a slot
  in first-come, first-served order. At most `max_queued_renders` requests MAY wait at once.
- **REQ-HTTP-072 (proposed):** A render request that arrives when all slots are occupied and the
  queue is full MUST fail immediately with `503`, error code `render_capacity_exceeded`, and a
  `Retry-After` header in delay-seconds form. The request MUST NOT be compiled.
- **REQ-HTTP-073 (proposed):** Template-ID parsing, JSON parsing, template lookup, and schema
  validation MUST complete before a render waits for capacity. Responses with `400`, `404`, and
  `422` MUST NOT wait for capacity or occupy a slot.
- **REQ-HTTP-074 (proposed):** A compilation that has started MUST keep its slot until it finishes,
  even when the client disconnects. A request that is still waiting in the queue MUST release its
  queue position as soon as its client disconnects.
- **REQ-HTTP-075 (proposed):** A request-time compilation that terminates abnormally MUST return
  `500` with error code `render_execution_failed` and a fixed message. The response MUST NOT
  include `diagnostics` or the panic payload. The server MUST continue serving other requests.
- **REQ-HTTP-076 (proposed):** A request that starts rendering before a watch-mode registry swap
  MUST complete with the prepared template it looked up. Requests that look up the template after
  the swap MUST use the new entry.

**Configuration: a new "`[server]`" group starting at 050.**

The proposed fields are:

| Field | Type | Required | Default |
|---|---|---|---|
| `[server].max_concurrent_renders` | integer | no | the process's available parallelism (`std::thread::available_parallelism`), or `1` when it cannot be determined |
| `[server].max_queued_renders` | integer | no | `2 × max_concurrent_renders` (see the open questions) |

The proposed requirements are:

- **REQ-CFG-050 (proposed):** The `[server]` table MUST be optional. Omitting it, or any field in
  it, MUST use the defaults above.
- **REQ-CFG-051 (proposed):** `max_concurrent_renders` MUST be an integer from `1` to
  `MAX_CONCURRENT_RENDERS_LIMIT` (proposed `256`; see the open questions). A value outside that
  range MUST be a configuration load error.
- **REQ-CFG-052 (proposed):** `max_queued_renders` MUST be an integer from `0` up to a documented
  upper bound. `0` MUST mean that a render is rejected whenever no slot is free.
- **REQ-CFG-053 (proposed):** Unknown keys inside `[server]` MUST be ignored, consistent with
  `REQ-CFG-003`.
- **REQ-CFG-054 (proposed):** The `[server]` settings MUST affect only `typrest serve`.
  `typrest check` MAY use them to size its executor, but its result MUST NOT depend on them.

**CLI.**

- **REQ-CLI-044 (proposed):** `serve` MUST report the effective `max_concurrent_renders` and
  `max_queued_renders` as structured fields of the `server.starting` event.

The sibling plan [`operational-limits.md`](operational-limits.md) is expected to add timeout, body
limit, and shutdown fields to the same `[server]` table. The two plans must agree on the table name
before either one edits the spec.

## Decisions and alternatives

### D1. Where blocking execution and the limit live

> **ADR proposal: needs human approval.** This decision constrains the core boundary and the
> public `PdfCompiler` contract, has two serious alternatives, and is costly to reverse once other
> plans build on it. One ADR, for example "Run request compilation on a bounded blocking executor",
> could cover D1, D2, and D4. Do not create the ADR file until a human approves.

| Option | Pros | Cons |
|---|---|---|
| **A. A core `RenderExecutor` port with an application implementation** *(recommended)* | Keeps core runtime-agnostic and testable with `futures_executor`. Every consumer of `RenderService` (HTTP, async jobs, `check`, watch) gets offloading and limits for free. The limiter is testable with mock compilers. | Adds a new port and a boxed-closure job type. Every `RenderService::new` caller changes. |
| B. Add `tokio` (`rt`, `sync`) to `typrest-core` and call `spawn_blocking` inside `RenderService` | Smallest code change. | Couples core to one runtime. Existing core tests that use `futures_executor::block_on` would panic, because `spawn_blocking` needs a Tokio context. Limits would be configured inside core. |
| C. Offload only in the application, by wrapping `RenderService::render` in `spawn_blocking` plus `futures_executor::block_on` | No core change. | Blocks on an `async` trait method inside a blocking thread, which deadlocks if a future compiler awaits Tokio I/O. Each sibling consumer would have to repeat the wrapper. |
| D. `tokio::task::block_in_place` in the handler | Tiny change. | Panics on the `current_thread` runtime that `#[tokio::test]` uses. Does not bound concurrency. Still occupies the worker slot for the duration. |

**Recommendation:** A. Name the module `crates/typrest-core/src/executor.rs`. Its proposed public
surface uses Typrest-owned types only:

```rust
pub type CompileJob =
    Box<dyn FnOnce() -> Result<Document, CompilationError> + Send + 'static>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Admission {
    /// Interactive request: bounded queue, may be rejected as saturated.
    Request,
    /// Warmup and reloads: waits for a slot, never rejected.
    Background,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("render capacity exceeded")]
    Saturated,
    #[error("render execution failed: {message}")]
    Aborted { message: String },
}

#[async_trait::async_trait]
pub trait RenderExecutor: Send + Sync {
    async fn execute(
        &self,
        admission: Admission,
        job: CompileJob,
    ) -> Result<Result<Document, CompilationError>, ExecutionError>;
}
```

Core also provides `InlineExecutor`, which runs the job on the caller's thread with no limit. It is
gated behind `cfg(any(test, feature = "test-util"))` so that production code cannot use it on a
serving runtime by accident.

### D2. Make `PdfCompiler::compile_prepared` synchronous

> Part of the D1 ADR proposal.

| Option | Pros | Cons |
|---|---|---|
| **A. `fn compile_prepared(&self, &PreparedTemplate, &Value) -> Result<Document, CompilationError>`** *(recommended)* | Matches reality, because the Typst implementation has no await. The job closure can call it directly. Makes the "CPU-bound, never call it on a runtime worker" contract visible in the type. | Changes a public core trait, which affects five mock implementations: `render.rs:71`, `registry.rs:351`, `registry.rs:372`, `http.rs:67`, and `watch.rs:486`. |
| B. Keep it `async` and drive it with `futures_executor::block_on` inside the job | No trait change. | Hides the blocking nature. A future implementation that awaits runtime I/O would deadlock or panic. |
| C. Have the compiler ACL offload internally through an injected spawner | Keeps offloading next to Typst. | Mocks would bypass the limiter, so the core tests could no longer prove it. |

**Recommendation:** A. Update the trait snippet in
[architecture overview](../architecture/overview.md#compiler-anti-corruption-layer) accordingly.

### D3. Blocking mechanism: Tokio `spawn_blocking` or a dedicated rayon pool

| Option | Pros | Cons |
|---|---|---|
| **A. `tokio::task::spawn_blocking` behind a `tokio::sync::Semaphore`** *(recommended)* | Already in use for packages. No new dependency. A `JoinError` reports panics. Works on `current_thread` test runtimes. | Shares Tokio's blocking pool (default 512 threads) with package downloads. Typst's own parallelism still runs on the global rayon pool, so the limit bounds entry points, not total CPU threads. |
| B. A dedicated `rayon::ThreadPool`, bridged with a oneshot channel | `install` would make Typst's nested parallelism use the same pool, which bounds total compile CPU. | Adds a direct dependency and an async bridge. Rayon panics abort the process unless a `panic_handler` is configured. Still needs a semaphore for queue semantics. Interacts with `Deferred`'s `rayon::yield_now`. |
| C. A second Tokio runtime dedicated to renders | Isolates the schedulers. | Still blocks that runtime's workers, so it moves the problem rather than solving it. |

**Recommendation:** A. Revisit B only if measurements show oversubscription between the blocking
threads and the global rayon pool (see [Risks](#risks)).

### D4. Behavior when saturated

> Part of the D1 ADR proposal, because it defines public operational behavior.

| Option | Pros | Cons |
|---|---|---|
| A. Unbounded queue: wait for a semaphore permit forever | Simple. Never rejects. | Latency and memory grow without bound under overload. Clients time out while their work still runs later. |
| B. Fail fast: `503` whenever no slot is free | Predictable latency and no queue memory. | Bursty traffic is rejected even when it would complete quickly. |
| **C. A bounded FIFO queue, with `503`, `Retry-After`, and `render_capacity_exceeded` once the queue is full** *(recommended)* | Absorbs bursts while capping memory and latency. `max_queued_renders = 0` gives B, and a large value approximates A. | Two settings to tune. A queue wait timeout is still needed, and it is delegated to [`operational-limits.md`](operational-limits.md). |

**Recommendation:** C, with the status `503`. The alternative `429` signals client misbehavior,
which does not describe a capacity limit; this remains an open question. Use a fixed `Retry-After`
from a named constant, `RENDER_RETRY_AFTER_SECONDS = 1`.

Implement exact admission with an `AtomicUsize` `admitted` counter, incremented by compare-exchange
only while it stays below `max_concurrent + max_queued`. An RAII `AdmissionGuard` decrements it.
Then acquire an `OwnedSemaphorePermit` from a semaphore with `max_concurrent` permits; Tokio's
semaphore is FIFO-fair. **Move both the guard and the permit into the blocking closure**, so they
are released only when the compilation actually ends (`REQ-HTTP-074`). A waiter that is dropped
before admission releases its guard immediately, so the `execute` future is cancel-safe while it
waits. `Admission::Background` skips the queue cap and only acquires a permit.

### D5. Validation stays on the async task, before admission

| Option | Pros | Cons |
|---|---|---|
| **A. Validate before `execute`** *(recommended)* | `422` never consumes capacity, as `REQ-HTTP-073` requires. Simple. | Validating a very large payload still uses runtime CPU. |
| B. Move validation into the job | Moves all CPU off the runtime. | Invalid requests consume slots, which makes overload easier to trigger. |

**Recommendation:** A. Payload size is bounded by the body limit in
[`operational-limits.md`](operational-limits.md).

### D6. Warmup and watch reloads use the same executor with `Admission::Background`

| Option | Pros | Cons |
|---|---|---|
| **A. Same executor, background class** *(recommended)* | Reload CPU counts against the same limit and stays off runtime workers. Reloads are never spuriously rejected. One code path for startup, `check`, and watch. | A reload can wait behind saturated request traffic. |
| B. Background warmups use `spawn_blocking` without a permit | Reloads are never delayed. | CPU use can exceed the configured limit during reloads. |
| C. Leave warmup inline on the runtime | No change. | Watch reloads keep starving HTTP traffic in development. |

**Recommendation:** A. `TemplateRegistry::build*` and `reload*` take `executor: &dyn RenderExecutor`.
`prepare_template_bundle` wraps the bundle in an `Arc` *before* warmup, so that the `'static` job can
own a clone. `replace` then stores that `Arc`.

### D7. Configuration surface

| Option | Pros | Cons |
|---|---|---|
| **A. An optional `[server]` table in `typrest.toml`** *(recommended for now)* | Versioned with the project. Follows the existing configuration loader. One place for the sibling limits too. | Container operators must edit or mount the file to tune a deployment. |
| B. `serve` flags such as `--max-concurrent-renders` | Per-deployment tuning without editing the project. | Bloats the CLI. The [CLI non-goals](../specs/cli.md#non-goals) currently exclude environment overrides, and flags alone are awkward in Compose. |
| C. Both, with flags overriding the file | Most flexible. | Precedence rules to specify and test. |

**Recommendation:** A in this plan. Whether to add B or C is an open question for the owner.

## Risks

- **A compilation that has started cannot be cancelled.** A disconnected client, or a future render
  timeout, frees the HTTP response but not the CPU. A pathological template can hold a slot for a
  long time, and if every slot is stuck the server returns `503` indefinitely while `/_healthz`
  still returns `200`. This is mitigated, not solved, by
  [`operational-limits.md`](operational-limits.md).
- **CPU oversubscription.** Typst parallelizes on the global rayon pool, so
  `max_concurrent_renders = cores` can run up to cores × (1 + rayon workers) runnable threads. The
  default may need tuning after measurement.
- **The `comemo` cache grows without limit.** No code calls `comemo::evict`. Higher concurrency and
  distinct payloads grow the process-global memo cache faster. This is pre-existing, but this work
  makes sustained load more likely. It needs an owner (see the open questions).
- **The blocking pool is shared.** Package resolution (`universal_package.rs:126`) and render jobs
  share Tokio's blocking pool, whose default cap is 512 threads. `REQ-CFG-051` caps
  `max_concurrent_renders` well below that cap, so that render permits never queue invisibly
  inside Tokio.
- **Container CPU quotas.** `available_parallelism` honors cgroup quotas on Linux, but fractional
  quotas round, and older cgroup v1 setups can report host cores. Document the field for container
  users.
- **Public API churn in core.** `PdfCompiler`, `RenderService::new`, `TemplateRegistry::build*`,
  `reload*`, and `RenderError` all change. There are no known external implementors, but
  `RenderError` gains variants, and every exhaustive match must be updated.
- **Panic consistency.** After a panic, Typst state is only borrowed immutably, and `comemo` locks
  are not poisoned by unwinding. This assumption has not been proven for every Typst-internal lock.
  The recovery test in Step 3 must be kept.
- **Flaky concurrency tests.** Tests must coordinate with channels or barriers, never with sleeps,
  and must use `#[tokio::test(flavor = "multi_thread")]` only where real parallelism is under test.
- **Watch reload latency.** Background warmups wait behind a saturated request load.

## Steps

Each step leaves the workspace compiling and green. Every step is proven done by running:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 scripts/check-doc-links.py && git diff --check
```

The last line is needed only when Markdown changed.

0. **Get the decisions approved (human).** Present the D1, D2, and D4 ADR proposal and the
   [open questions](#open-questions) to the owner. If approved, create the ADR as its own change,
   following [the ADR process](../adr/README.md#adding-a-decision). Steps 2 and later wait for this
   approval. Step 1 is a pure refactor and may proceed.

1. **Make `compile_prepared` synchronous** (`typrest-core`, no behavior change).
   - In `crates/typrest-core/src/compiler.rs`, change the trait method to a synchronous `fn` and
     document its CPU-bound contract. Update `TypstCompiler`'s implementation.
   - Update the call sites `render.rs:28-31` and `registry.rs:213-219`, which drop `.await`.
   - Update the mocks in `render.rs`, `registry.rs` (`WarningCompiler`, `CountingCompiler`),
     `crates/typrest/src/http.rs`, and `crates/typrest/src/loader/watch.rs`.
   - Update the compiler tests at `compiler.rs:242-268` that call `block_on(compile_prepared…)`.
   - Update the trait snippet in [architecture overview](../architecture/overview.md) and the
     paragraph on `compile_prepared` in [compiler reference](../architecture/compiler.md).
   - Done when: the full check passes, and `rg 'compile_prepared\(.*\)\s*\.await' crates` finds
     nothing.

2. **Add the executor port and plumb it through core, with a minimal application adapter.**
   - In core:
     - Add `crates/typrest-core/src/executor.rs` with `RenderExecutor`, `CompileJob`, `Admission`,
       and `ExecutionError` (D1). Add `InlineExecutor` under `test-util`.
     - Export the deliberate public items from `lib.rs`.
     - Add `RenderError::Overloaded` and `RenderError::ExecutionFailed { message }` in `errors.rs`.
     - Change `RenderService::new(registry, compiler, executor: Arc<dyn RenderExecutor>)`. In
       `render`, do lookup, then validation, then build the `'static` job, which captures
       `Arc<dyn PdfCompiler>`, `Arc<PreparedTemplateBundle>`, and the owned payload. Then call
       `executor.execute(Admission::Request, job)` and map the errors.
     - Change `TemplateRegistry::build`, `build_with_observer`, `reload`, and
       `reload_with_observer` to take `&dyn RenderExecutor` and warm up with
       `Admission::Background`. Store the bundle `Arc` created before warmup (D6).
   - Core tests (use `MemoryLoader`, mock compilers, and `futures_executor`):
     - Validation failure and template-not-found never call the executor (use a counting
       executor).
     - `ExecutionError::Saturated` maps to `RenderError::Overloaded`, and `Aborted` maps to
       `ExecutionFailed`.
     - The job keeps the old prepared bundle alive after `replace_all` (`REQ-HTTP-076`). A mock
       executor captures the job, the registry is swapped, the captured job runs, and the test
       asserts it used the old entry.
     - Warmup uses `Admission::Background`.
   - In the application:
     - Add `crates/typrest/src/render_executor.rs`. It is a top-level application module, not
       under `http/`, because `check` and watch use it too. Define `BlockingRenderExecutor`, which
       at this stage only offloads with `spawn_blocking` and maps a panic `JoinError` to `Aborted`
       with the panic message.
     - Wire it in `commands/serve.rs` (registry build, watcher, `RenderService`), in
       `commands/check.rs`, and in `loader/watch.rs`. The watcher holds an
       `Arc<dyn RenderExecutor>`.
     - Map `RenderError::ExecutionFailed` to `500 render_execution_failed` with a fixed message in
       `http/errors.rs`, and log the panic message in `render.failed`. Add `render_error_kind`
       values. Map `Overloaded` to `503` without the header for now; Step 3 adds the header.
   - Application tests:
     - A `#[tokio::test]` (current-thread) router test in `http.rs` whose mock compiler blocks on a
       `std::sync::mpsc` receiver. While one render is blocked, `GET /_healthz` returns `200`
       (`REQ-HTTP-011`). This test would hang before this change.
     - A panicking mock compiler yields `500 render_execution_failed`, and a following request
       succeeds (`REQ-HTTP-075`).
   - Specs: add `REQ-HTTP-011`, `REQ-HTTP-075`, and `REQ-HTTP-076`, plus the
     `render_execution_failed` row of `REQ-HTTP-041`, to the
     [HTTP API spec](../specs/http-api.md).
   - Done when: the full check passes and the new tests pass. Confirm the health test fails when
     temporarily reverted to inline execution.

3. **Add admission control: the semaphore, the bounded queue, and `503` with `Retry-After`.**
   - In `render_executor.rs`:
     - Add `max_concurrent` and `max_queued` (D4), the exact `admitted` counter, the RAII guard,
       and permit ownership moved into the blocking closure.
     - Add `Admission::Background`, which bypasses the queue cap.
     - Add `stats() -> ExecutorStats { max_concurrent, max_queued, in_flight, queued }` for the
       sibling plans.
     - Add the defaults as named constants and functions: `default_max_concurrent_renders()` from
       `available_parallelism`, and `DEFAULT_QUEUE_FACTOR`.
   - In `http/errors.rs`, let `ApiError` carry an optional `Retry-After`. Map `Overloaded` to `503`,
     `render_capacity_exceeded`, and `Retry-After: RENDER_RETRY_AFTER_SECONDS`. Log rejected renders
     as `render.failed` with `error_kind = "render_capacity_exceeded"` at `WARN`.
   - Executor unit tests, using `#[tokio::test(flavor = "multi_thread")]` and channel-gated jobs:
     - In-flight never exceeds `max_concurrent`; record the peak with atomics.
     - The request beyond `max_concurrent + max_queued` gets `Saturated` immediately.
     - `max_queued = 0` rejects as soon as all slots are busy.
     - Dropping a queued `execute` future frees its queue position.
     - Dropping an admitted `execute` future keeps the slot until the job finishes
       (`REQ-HTTP-074`).
     - Waiters are admitted in FIFO order.
     - `Background` waits and is never rejected.
     - A panicking job releases its slot.
   - HTTP tests: a saturated router returns `503` with the JSON envelope and a `Retry-After`
     header, and `/_healthz` still returns `200` while saturated.
   - Specs: add `REQ-HTTP-070` to `REQ-HTTP-074` and the `503` row of `REQ-HTTP-041`. Until Step 4,
     the specs state the defaults as fixed.
   - Done when: the full check passes and `cargo test -p typrest render_executor` passes.

4. **Make the limits configurable in `typrest.toml`.**
   - In `crates/typrest/src/config.rs`, add an optional `RawServerConfig` with
     `#[serde(default)]`, and validate the ranges (`REQ-CFG-051`, `REQ-CFG-052`). Expose the
     resolved values on `ProjectConfig`, for example `render_limits: RenderLimits`. Keep the
     bounds as named constants.
   - Build `BlockingRenderExecutor` from `ProjectConfig` in `serve.rs` and `check.rs`. Emit the
     effective limits on `server.starting` (`REQ-CLI-044`).
   - Tests in `config.rs`:
     - The defaults apply when the table or a field is absent.
     - Explicit values are honored.
     - `0` and out-of-range values for `max_concurrent_renders` are rejected.
     - `max_queued_renders = 0` is accepted.
     - A wrong type is a load error.
     - Unknown keys in `[server]` are ignored.
   - Specs:
     - Update the [`typrest.toml` spec](../specs/typrest-toml.md): field table, `REQ-CFG-050` to
       `REQ-CFG-054`, an example, and narrowed non-goals.
     - Add `REQ-CLI-044` to the [CLI spec](../specs/cli.md).
     - Replace the provisional default wording in the HTTP spec with links to the configuration
       spec.
   - Done when: the full check passes and `cargo test -p typrest config` passes.

5. **Finish the documentation and remove the plan.**
   - Architecture:
     - Update the "Request Lifecycle" and "Watch Mode" sections of
       [template lifecycle](../architecture/template-lifecycle.md): executor, admission classes,
       and in-flight renders across swaps.
     - Update the "Error Boundaries" section and the canonical terms (`RenderExecutor`,
       `Admission`) in [architecture overview](../architecture/overview.md).
     - Note thread-safety and rayon in [compiler reference](../architecture/compiler.md).
   - Add executor test placement to the [testing guide](../development/testing.md), and add an
     invariant to `AGENTS.md`: request compilation goes through `RenderExecutor`, and
     `compile_prepared` is never called on an async worker.
   - Update [`ROADMAP.md`](../../ROADMAP.md), adding a delivered "Render execution" item, and the
     ADR index if the ADR was approved.
   - Delete this plan and remove it from the active list in the [plans index](README.md).
   - Done when: `python3 scripts/check-doc-links.py` and `git diff --check` pass, and
     `rg -n 'max_concurrent_renders|render_capacity_exceeded' docs` shows only canonical homes.

## Open questions

1. **Defaults.** Is `available_parallelism()` the right default for `max_concurrent_renders`, given
   that Typst also uses the rayon pool? Is `2 × max_concurrent_renders` right for
   `max_queued_renders`? What should the upper bounds be? `256` is proposed for concurrency; none
   is proposed yet for the queue.
2. **Status code.** Should saturation use `503`, as recommended, or `429`? Should the error code be
   `render_capacity_exceeded` or something shorter, such as `server_busy`?
3. **`Retry-After`.** Should it be a fixed `1` second or derived from observed render latency?
4. **Configuration surface.** Is `typrest.toml` alone acceptable, or are `serve` flags or
   environment overrides also wanted for container deployments (D7)? Either would change the
   [CLI non-goals](../specs/cli.md#non-goals).
5. **Health.** Should saturation be visible to health checks, for example through a separate
   readiness route, or is that left to [`render-metrics.md`](render-metrics.md)? `REQ-HTTP-010`
   currently says that health is always `200`.
6. **Reserved capacity.** Should watch reloads and future async jobs get reserved capacity, or
   priority over interactive requests, instead of sharing FIFO order?
7. **`comemo` eviction.** Who owns trimming the global Typst memo cache, and with what policy, for
   example `typst::comemo::evict(n)` after each request compile inside the compiler ACL? Should it
   be part of this plan, of [`operational-limits.md`](operational-limits.md), or of a separate
   change?
8. **Warmup parallelism.** Should startup warmup later run templates in parallel through the
   executor? That would shorten startup but change the order of interleaved log output.

## Dependencies on other plans

None of these plans exist yet; they are being written in parallel.

- **[`operational-limits.md`](operational-limits.md)** (render timeouts, body limit, graceful
  shutdown) depends heavily on this plan. It needs:
  - A cancel-safe `RenderExecutor::execute` future while queued, so that a request deadline, such
    as a Tower timeout around the handler, can drop a waiting request and free its queue position.
  - The documented fact that an admitted compilation cannot be cancelled and keeps its slot
    (`REQ-HTTP-074`). A render timeout returns a response early, but capacity frees only when the
    job ends.
  - `BlockingRenderExecutor::stats()`. Shutdown will add `close()`, which stops admitting new work
    and makes `execute` return `Saturated` or a new `ShuttingDown` variant, and `drained()`, a
    future that resolves when in-flight work reaches zero. Both are extensions of the application
    executor and do not change the core port.
  - The shared `[server]` table in `typrest.toml`, so that both plans agree on the table name and
    error code naming.
- **[`render-metrics.md`](render-metrics.md)** needs `ExecutorStats` (`max_concurrent`,
  `max_queued`, `in_flight`, `queued`) as gauges. It also needs distinct phases inside
  `BlockingRenderExecutor` (admission, queue wait, execution) where counters and histograms can be
  recorded: rejections, queue wait time, and execution time. Instrumentation lives in the
  application module; core stays metrics-free.
- **[`render-result-cache.md`](render-result-cache.md)** needs `RenderService::render` to keep the
  order lookup, validate, cache check, then `execute`. A cache hit must return before admission
  and never consume a slot. Insertion happens after `execute` succeeds. The cache key uses the
  validated, defaulted payload.
- **[`async-render-jobs.md`](async-render-jobs.md)** needs job workers to render through the same
  `RenderService` and executor, so that the global limit covers jobs too. It will likely add an
  `Admission` variant, for example `Job`, that queues instead of failing with `503`. The `Admission`
  enum is the extension point; see open question 6.
- **[`pdf-output-destinations.md`](pdf-output-destinations.md)** needs the guarantee that `execute`
  returns an owned `Document` and that its slot is released at that point. Uploads and writes to
  destinations run on the async runtime after `execute` returns, and never while holding a render
  slot.

## Documents to update on completion

- [HTTP API specification](../specs/http-api.md): `REQ-HTTP-011`, `REQ-HTTP-041`, and
  `REQ-HTTP-070` to `REQ-HTTP-076`, plus a `503` example.
- [`typrest.toml` specification](../specs/typrest-toml.md): the `[server]` table, `REQ-CFG-050` to
  `REQ-CFG-054`, and the non-goals.
- [CLI specification](../specs/cli.md): `REQ-CLI-044`.
- [Architecture overview](../architecture/overview.md): the `PdfCompiler` snippet, the
  `RenderExecutor` and `Admission` terms, the lifecycle diagram, and the error boundaries.
- [Template lifecycle](../architecture/template-lifecycle.md): the request lifecycle, warmup through
  the executor, and watch swaps with renders in flight.
- [Compiler reference](../architecture/compiler.md): the synchronous `compile_prepared`,
  thread-safety, and Typst's internal rayon use.
- [Testing guide](../development/testing.md): placement of executor and concurrency tests.
- [`AGENTS.md`](../../AGENTS.md): the critical rule about request compilation through
  `RenderExecutor`.
- [`ROADMAP.md`](../../ROADMAP.md): the delivered item, and follow-ups for moving watch-mode
  preparation and loading I/O off the runtime.
- [ADR index](../adr/README.md): only if the D1/D2/D4 ADR is approved.
- [Plans index](README.md): list this plan while it is active, and remove it when it is deleted.
