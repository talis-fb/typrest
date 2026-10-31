# Plan: Opt-in Render Result Cache

- **Status:** Draft
- **Owner:** Talison Fabio

## Objective

Let an operator opt in to an in-process cache of rendered output. When the same template, at the
same version, receives the same (validated) payload again, Typrest returns the stored PDF instead of
compiling and exporting it again. The feature is off by default and changes nothing observable when
it is off.

### Terminology

This plan adds a **render-result cache**: a map from a render request (template, template
generation, payload fingerprint) to the finished `Document` (PDF bytes plus warnings). It is not a
**compilation cache**, which would retain intermediate Typst products (parsed sources, evaluated
modules, layout) between compilations. The product vision, the roadmap, and
[`AGENTS.md`](../../AGENTS.md) exclude "a compilation cache" until a decision is recorded. The
render-result cache is a different mechanism, but it sits in the same family and the exclusion
wording is broad enough that this plan treats it as a scope change that needs explicit approval
(step 0). The prepared template state that Typrest already reuses is also not what this plan adds
(see [Current behavior](#current-behavior)).

## Prerequisites (scope and ADR approval)

These are blocking and need a human decision; no implementation step may start before step 0.

1. **Scope change.** [`docs/product/vision.md`](../product/vision.md) (Excluded list, "a compilation
   cache"), [`ROADMAP.md`](../../ROADMAP.md) (final deferred paragraph), the "System Boundary"
   paragraph of [`docs/architecture/overview.md`](../architecture/overview.md), and the Critical
   Architecture Rules of [`AGENTS.md`](../../AGENTS.md) list a compilation cache as out of scope
   "without an explicit decision and scope change". Approval must state that an opt-in,
   in-memory render-result cache is in scope, and that a compilation cache (and any persistent or
   shared cache) stays out of scope.
2. **ADR proposal — needs human approval.** Proposed title: "Add an opt-in in-memory render-result
   cache". It meets all three criteria in [`docs/adr/README.md`](../adr/README.md): serious
   alternatives exist (no cache, reverse-proxy/CDN caching, disk cache, shared/remote cache,
   compilation cache); the choice constrains a public contract (response headers, the assumption
   that output is a pure function of template, generation and payload), a security property
   (payloads and PDFs are retained in process memory), and operational behavior (memory budget).
   Its decision section would fix: the key definition, the exclusion of time-dependent output, the
   in-memory-only storage, and the opt-in default. Next unused number would be `0009`. Agents must
   not create the file until a human approves the proposal.
3. **New dependencies.** `moka` (with the `future` feature) and `sha2` for `typrest-core`
   (see [Decisions](#decisions-and-alternatives)). Adding crates is a reviewable change; confirm in
   step 0. `sha2` is already in `Cargo.lock` (0.10.9 and 0.11.0, the latter through `rust-embed`);
   `moka` is new. Neither may be imported by the compiler ACL, and neither touches Typst version
   pinning.

## Current behavior

Canonical descriptions: [Template lifecycle](../architecture/template-lifecycle.md),
[Compiler](../architecture/compiler.md), [HTTP API](../specs/http-api.md),
[`typrest.toml`](../specs/typrest-toml.md). Facts relevant here:

- **Every request compiles and exports.** The handler parses the body into a `serde_json::Value`
  ([`render_template.rs:40-46`](../../crates/typrest/src/http/handlers/render_template.rs)) and calls
  `RenderService::render` (`:50-53`). `render` looks up the prepared bundle, validates with defaults
  when a schema exists, then always calls `compile_prepared`
  ([`render.rs:16-32`](../../crates/typrest-core/src/render.rs)). The lifecycle document states that
  "each request still performs compilation and export because `sys.inputs.data` is
  request-specific".
- **What is already reused.** `TemplateRegistry` holds immutable `Arc<PreparedTemplateBundle>`
  entries ([`registry.rs:13-16`](../../crates/typrest-core/src/registry.rs)). Prepared state
  contains the parsed `main` `Source`, all virtual files, resolved package sources, and the
  template font book and fonts
  ([`prepared_template.rs:19-24`](../../crates/typrest-core/src/compiler/prepared_template.rs)). Per
  request, `TemplateWorld::new` builds a fresh `Library` with the request inputs
  ([`world.rs:93-102`](../../crates/typrest-core/src/compiler/world.rs)), `typst::compile` runs
  (`:54-55`), and `typst_pdf::pdf` exports (`:69`). Typst memoizes internally with `comemo`, which
  is process-global and keyed by tracked inputs, so identical sub-computations (for example modules
  that do not read `sys.inputs`) may be reused across compilations. Typrest neither controls nor
  evicts that memo: a search of `crates/` finds no `comemo` use and no `evict` call. PDF export is
  not memoized at that level. Whether `comemo` growth is bounded in a long-running server is
  unverified and is outside this plan (see Open questions).
- **No result reuse.** Identical requests recompile. There is no template version or generation:
  `TemplateRegistry::replace` swaps an `Arc` ([`registry.rs:153-158`](../../crates/typrest-core/src/registry.rs));
  `replace_all` copies a candidate registry that was built independently
  (`:44`, `:141-151`), so nothing identifies "same template content".
- **Compilation is synchronous inside an async fn.**
  [`compiler.rs:65-79`](../../crates/typrest-core/src/compiler.rs) calls `world::compile_prepared`
  directly. The sibling plan [render execution (delivered)](../specs/http-api.md#render-capacity) changes
  this.
- **Payload serialization.** The compiler passes `serde_json::to_string(payload)` as
  `sys.inputs.data` ([`world.rs:176-181`](../../crates/typrest-core/src/compiler/world.rs)). The
  workspace `serde_json` does not enable `preserve_order` (`cargo tree` shows only `default`,
  `raw_value`, `std`), so `Map` is a `BTreeMap` and object keys are serialized in sorted order.
  Integers and floats serialize differently (`1` versus `1.0`) and Typst sees different types.
- **Determinism.** `PdfOptions::default()` is used
  ([`world.rs:69`](../../crates/typrest-core/src/compiler/world.rs)), so `timestamp` is `None`
  and `ident` is `Auto` (document ID derived from title and author; verified in
  `typst-pdf-0.15.1/src/lib.rs` and `metadata.rs:60-62`, where a creation date is used only when the
  template sets one or Typrest passes a timestamp). Fonts, sources and packages are fixed per
  prepared template. The one request-independent input is the clock:
  `World::today` returns the current UTC date (offset applied) at
  [`world.rs:141-155`](../../crates/typrest-core/src/compiler/world.rs) (commit `ae091dc`). A template
  that calls `datetime.today()` (directly, or in a package) produces output that depends on the
  day, so a cache keyed only by payload would change its semantics.
- **Warnings.** `Document` carries PDF bytes and warnings
  ([`template.rs:282-311`](../../crates/typrest-core/src/template.rs)). The handler logs each
  warning on every successful render and logs `render.completed`
  ([`render_template.rs:57-77`](../../crates/typrest/src/http/handlers/render_template.rs)).
  `DiagnosticLocation` holds an `Arc<str>` of the source text
  ([`diagnostic.rs`](../../crates/typrest-core/src/diagnostic.rs)).
- **Configuration.** `RawProjectConfig` has `schema_version`, `templates_root`, `web_ui`
  ([`config.rs:19-25`](../../crates/typrest/src/config.rs)); unknown top-level fields are ignored
  (`REQ-CFG-003`).
- **No authentication** (`REQ-HTTP-050`); deployments rely on a trust boundary (`REQ-HTTP-051`).
  `typrest-core` has no `tokio` dependency
  ([`Cargo.toml`](../../crates/typrest-core/Cargo.toml)) and no filesystem access.
- **Wiring.** `serve.rs` builds `RenderService::new(registry, compiler)`
  ([`serve.rs:192-196`](../../crates/typrest/src/commands/serve.rs)); `AppState` holds the service
  ([`state.rs`](../../crates/typrest/src/http/state.rs)); watch mode calls `reload_with_observer` and
  `replace_all` ([`watch.rs:172`, `:301`](../../crates/typrest/src/loader/watch.rs)).

## Intended change

When `[render_cache] enabled = true` in `typrest.toml`:

1. `RenderService` computes a key `(TemplateId, generation, SHA-256 of the serialized validated
   payload)` after validation and before compilation. A hit returns the stored `Document` without
   touching the compiler. A miss compiles once, even for concurrent identical requests
   (single-flight), and stores the result.
2. Only successful documents are cached. Validation errors never reach the cache; compilation
   failures are not cached.
3. Documents whose compilation read the current date, and documents larger than the entry limit, are
   never retained.
4. Entries are bounded by a byte budget (LRU/TinyLFU eviction) and an optional TTL. A reload of a
   template makes its old entries unreachable immediately (generation) and removes them promptly
   (explicit purge).
5. Responses from cache-enabled templates carry `X-Typrest-Cache: hit|miss|bypass`. A client may
   send `Cache-Control: no-cache` (skip lookup, refresh the entry) or `no-store` (skip lookup and
   store).
6. With the option off, behavior, headers and logs are unchanged.

### Where it lives

`typrest-core` (no I/O): `RenderCache`, key and fingerprint, `RenderService` integration, cache
statistics. The application package: `typrest.toml` parsing and validation, wiring, headers,
`Cache-Control` parsing, log fields, watch-mode purge calls. The compiler ACL: only a
`Document` flag reporting that the current date was read. No other Typst-facing change.

## Decisions and alternatives

Each row is a recommendation for review, not a settled product decision.

| # | Topic | Recommendation | Alternatives considered |
|---|---|---|---|
| D1 | Cached artifact | Finished `Document` (bytes plus warnings), keyed by request. | Compilation cache (out of scope, higher memory, less win); cache only the Typst `PagedDocument` (needs Typst types outside ACL). |
| D2 | Key payload | Hash of `serde_json::to_vec(validated payload)`, the same serialization given to Typst. Equal keys then mean equal Typst input by construction. Keys are sorted (no `preserve_order`), so key order does not matter; whitespace is irrelevant after parsing; `1` and `1.0` stay distinct because Typst distinguishes them. Defaults-applied payloads make a body that omits a default share an entry with one that states it. | Raw body bytes (misses on whitespace/order, hashes before parsing); RFC 8785 canonical JSON (more equivalence than Typst has, would be wrong for `1` vs `1.0`). Guard: extract one `pub(crate)` serialization function shared with `build_inputs` and add a test that the fingerprint input equals `sys.inputs.data`. Re-evaluate if `preserve_order` or `arbitrary_precision` is ever enabled. |
| D3 | Hash | SHA-256, full 32 bytes. The cache is shared by all callers and payloads are attacker-controlled; a collision would serve one caller's document (with its data) to another. | Non-cryptographic 64-bit hashes (`siphasher` via `comemo`, `xxhash`): faster, collision-attackable. Storing the full payload as key: exact but memory-heavy; reject. |
| D4 | Template version | A process-unique `u64` generation assigned in `PreparedTemplateBundle::new`/`without_schema` from a static `AtomicU64`. Every prepare/reload yields a new generation, including a full rebuild. Must be process-global, not per registry, because watch mode builds candidate registries independently (`registry.rs:44`) and `replace_all` copies them in. | Content hash of `TemplateSource` plus packages and fonts (precise, survives no-op reloads, but needs hashing in the prepare path and package identity); `Arc::as_ptr` (address reuse is unsafe). Cost of the recommendation: a no-op full rebuild drops every entry; acceptable. |
| D5 | Time-dependent output | Record in `TemplateWorld` whether `today()` was called during the compile (atomic flag); expose `Document::uses_current_date()`. Such documents are returned and may be shared by concurrent in-flight requests, but are not retained (per-entry expiry of zero, or the fallback in step 3). | Cache with expiry at next UTC midnight (wrong across explicit offsets); require operators to exclude such templates by list only (error-prone, packages can call `today`); ignore (changes semantics). Needs a spike: confirm `comemo` revalidation re-invokes `World::today` so the flag is set even when Typst memoized the caller. |
| D6 | Opt-in granularity | Global `[render_cache] enabled` (default `false`) plus an optional `templates` list of template IDs restricting the cache; absent means all templates. | Per-template files (new concept, not in `templates.md`); schema-level flag (the schema is the payload contract, not runtime policy); CLI flag only (all other project policy lives in `typrest.toml`, per `web_ui`). A request header is only an opt-out/refresh, never an opt-in. |
| D7 | Storage | In-memory only, `moka::future::Cache` with a weigher (PDF bytes plus an estimate per warning plus key size), `time_to_live`, and `Expiry` for per-entry decisions. Runtime-agnostic, so `typrest-core` stays free of `tokio`. Provides single-flight through `try_get_with`. | Hand-rolled LRU plus in-flight map (more code, no admission policy; deterministic eviction for tests); `quick_cache` (smaller, weaker TTL/async story); disk or shared/remote store (see D12, out of scope). |
| D8 | Single-flight | Use `try_get_with`: one leader compiles, others wait and share the result or the error. A dropped leader (axum drops handlers on client disconnect) must not fail followers; moka promotes another waiter. Verify with a test. | No coalescing (stampede on cold start or popular payload); detach the leader onto a spawned task so cancellation never wastes the compile (interacts with [render execution (delivered)](../specs/http-api.md#render-capacity); open question). |
| D9 | Eviction and size | Byte-weighted budget `max_bytes`; entries over `max_entry_bytes` are not retained. Admission may decline a new entry under pressure; a "miss that was not stored" is valid. | Entry-count budget (unbounded memory with large PDFs). |
| D10 | Warnings on hit | Store warnings with the document and return them. The handler logs warnings only on a miss (they were logged when produced) and reports `cache = hit` and `warning_count` in `render.completed`. | Replay warnings on every hit (noisy, repeats identical diagnostics); drop warnings from cached entries (loses `warning_count`). |
| D11 | Response metadata | `X-Typrest-Cache: hit\|miss\|bypass`, emitted only when the cache is enabled for that template. | RFC 9211 `Cache-Status: typrest; hit` (standard, more verbose); `ETag` derived from the key with `If-None-Match` support (useful for `GET`, odd for `POST`; defer). No `Cache-Control` is added to PDF responses by this plan. |
| D12 | Scope of storage | Per-process memory. No disk, no Redis/S3, no cross-instance sharing. | Disk cache (payload data at rest needs permissions and encryption decisions; needs Typst version in key); remote cache (new operational model, same family as Store mode). |

Cross-cutting consequences:

- **Multi-instance.** Each instance has its own cache; hit rate falls roughly with instance count
  unless the load balancer routes equal requests to the same instance. Cross-instance reuse is
  out of scope; a reverse proxy or CDN in front of `GET` routes is the supported alternative.
- **Privacy.** Cached entries are PDFs rendered from caller payloads, retained in process memory
  until eviction, TTL, reload or exit (core dumps and swap included). Nothing is persisted. Keys,
  fingerprints and payloads must not be logged (`REQ-LOG-033`). A hit versus a miss, visible in
  latency and in `X-Typrest-Cache`, tells a caller that someone already submitted an identical
  payload; with no authentication (`REQ-HTTP-050`) this is acceptable only inside the trust
  boundary of `REQ-HTTP-051`, and the specification must say so.
- **Memory pressure.** The budget covers cached documents only, not Typst or `comemo` memory;
  document it. Defaults below are proposals that need confirmation.
- **Output destinations.** In [`pdf-output-destinations.md`](pdf-output-destinations.md), the stored
  S3/filesystem object is not the cache: it is a caller-requested side effect with its own path,
  lifecycle and failure modes, and may be overwritten or removed independently. The destination
  write happens after `RenderService` returns a `Document`, on hits as well as misses.
- **Async jobs.** [`async-render-jobs.md`](async-render-jobs.md) jobs call the same
  `RenderService`, so they hit and fill the cache and share single-flight with synchronous
  requests. Job result retention is a separate store and is not this cache.

## Affected requirements

Existing (meaning preserved unless stated):

- `REQ-HTTP-020`: still returns `200` with a PDF; add that a cache-enabled template also returns the
  cache header (cross-reference to the new group).
- `REQ-HTTP-025`: "Rendering MUST read only the prepared registry" stays true; clarify that an
  in-memory render cache is not a file or network read.
- `REQ-HTTP-024`: unchanged; validation and defaults precede the cache lookup.
- `REQ-CFG-003`: the new `render_cache` table is a known field; unknown keys inside it are rejected
  (the "ignore unknown" rule is about top-level fields).
- `typrest-toml.md` Non-goals ("cache paths"): reword; the cache has no paths.
- `REQ-LOG-033`: unchanged; the new log fields must obey it.
- `template-lifecycle.md` and `overview.md`: prose about "each request still performs compilation"
  and "compilation caches are outside the current scope" is updated (architecture documents have no
  IDs).

Proposed new requirements (*proposed*; final wording in the specification when implemented). A new
specification `docs/specs/render-cache.md` with domain `CACHE` is proposed, plus additions to the
HTTP and configuration specifications. Next free IDs: `REQ-HTTP-070+`, `REQ-CFG-050+`,
`REQ-LOG-050+`; `REQ-CACHE` is a new domain.

- *proposed* **REQ-CFG-050:** `render_cache` is an optional table; absent or `enabled = false` MUST
  leave behavior unchanged.
- *proposed* **REQ-CFG-051:** `render_cache.enabled` (boolean, default `false`).
- *proposed* **REQ-CFG-052:** `render_cache.max_bytes` (integer, bytes, default `134217728`
  (128 MiB), proposed) MUST be greater than zero.
- *proposed* **REQ-CFG-053:** `render_cache.max_entry_bytes` (integer, bytes, default `8388608`
  (8 MiB), proposed) MUST be greater than zero and not exceed `max_bytes`.
- *proposed* **REQ-CFG-054:** `render_cache.ttl_seconds` (integer, default `3600`, proposed) MUST be
  greater than zero; `0` and negative values are configuration errors. (Open question: allow "no
  TTL".)
- *proposed* **REQ-CFG-055:** `render_cache.templates` (array of template ID strings, default
  absent meaning all templates) MUST contain valid template IDs; an ID with no prepared template
  MUST be reported as a startup warning, not an error (a template may appear later in watch mode).
- *proposed* **REQ-CFG-056:** Unknown keys inside `render_cache` MUST be a configuration load error.
- *proposed* **REQ-CACHE-010:** A cache entry key MUST consist of the template ID, the template
  generation, and a SHA-256 digest of the payload as serialized for Typst after validation and
  defaults.
- *proposed* **REQ-CACHE-020:** Only successful documents MAY be retained; validation and
  compilation errors MUST NOT be retained.
- *proposed* **REQ-CACHE-021:** A document produced by a compilation that read the current date MUST
  NOT be retained.
- *proposed* **REQ-CACHE-022:** A document larger than `max_entry_bytes` MUST NOT be retained.
- *proposed* **REQ-CACHE-030:** Replacing a template (watch reload or rebuild) MUST make all earlier
  entries for it unreachable before the replacement serves any request, and SHOULD remove them.
- *proposed* **REQ-CACHE-040:** Concurrent identical requests that miss SHOULD cause at most one
  compilation.
- *proposed* **REQ-CACHE-050:** Total retained size MUST NOT exceed `max_bytes` after maintenance
  completes; entries MUST expire after `ttl_seconds`.
- *proposed* **REQ-CACHE-060:** The cache MUST be process-local and in memory; it MUST NOT write to
  disk or the network.
- *proposed* **REQ-CACHE-070:** The cache MUST NOT log or expose payloads, keys or digests.
- *proposed* **REQ-HTTP-070:** For a template covered by an enabled cache, a `200` render response
  MUST include `X-Typrest-Cache` with value `hit`, `miss`, or `bypass`.
- *proposed* **REQ-HTTP-071:** Responses MUST NOT include `X-Typrest-Cache` when the cache is
  disabled or does not cover the template.
- *proposed* **REQ-HTTP-072:** A request with `Cache-Control: no-cache` MUST skip lookup and refresh
  the entry; `no-store` MUST skip lookup and storage; both report `bypass`. (Open question: whether
  clients may bypass at all.)
- *proposed* **REQ-HTTP-073:** A cache hit MUST return bytes identical to the stored miss response.
- *proposed* **REQ-LOG-050:** `render.completed` MUST include a `cache` field (`hit`, `miss`,
  `bypass`) when the cache covers the template.

## Out of scope

- A compilation cache, any `comemo` tuning or eviction policy (separate concern, see Open
  questions).
- Disk, shared, distributed, or remote caches; cache warming at startup; negative caching of errors.
- `ETag`/`If-None-Match`/`304` support, `Cache-Control` on PDF responses (candidate follow-ups).
- Per-template TTL or size overrides, a cache-management API (purge endpoint), authentication.
- Store mode, tags, publishing (unchanged exclusions).

## Dependencies on other plans

Sibling plans are written in parallel; links may not exist yet.

- [render execution (delivered)](../specs/http-api.md#render-capacity). Needs: compilation runs off the
  async workers. Provided by this plan: a `get_or_render(key, compile_future)` shape where the
  compile future is the only code that runs under that plan's blocking execution; hits never enter
  it. Ordering: independent; if both land, the cache wraps the blocking call.
- [`operational-limits.md`](operational-limits.md). Needs: a concurrency permit or queue around
  compilation. Interface requirement: the permit MUST be acquired inside the cache's miss path
  (leader only), so hits and coalesced followers do not consume permits and do not count toward
  queue or timeout limits. Body-size limits bound key-hash input but not entry size; the cache
  adds its own `max_entry_bytes`. A render timeout applies to the leader; followers receive the same
  error. Provided: `RenderCacheConfig` limits are independent of request limits.
- [`render-metrics.md`](render-metrics.md). Provided by this plan: `CacheStatus` (`Hit`, `Miss`,
  `Bypass`), `RenderOutcome`, and `RenderCache::stats()` (hits, misses, coalesced waits,
  evictions by cause, rejected oversize, skipped time-dependent, entry count, weighted size).
  Needed from that plan: its registration mechanism; this plan assumes counters are exposed from
  the application layer and does not choose a metrics library.
- [`async-render-jobs.md`](async-render-jobs.md). Needs: jobs call `RenderService::render` (or
  `render_with_outcome`); no change to job storage. Provided: single-flight deduplicates a job and a
  synchronous request for the same payload.
- [`pdf-output-destinations.md`](pdf-output-destinations.md). Needs: the destination write consumes
  a `Document` and runs regardless of `CacheStatus`. Provided: `Document` is cheap to clone
  (`Bytes`).

## Steps

Each step is sized for one session and one commit, and ends with `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` (plus the
doc checks when documents change).

0. **Scope and ADR approval gate.** Obtain human approval for: the scope change, the ADR proposal,
   the new dependencies, the defaults and the open questions marked blocking (O1-O3). After
   approval, in one documentation commit: create ADR `0009` (status Accepted once approved) and add
   it to the ADR index; amend `docs/product/vision.md`, `ROADMAP.md`, `AGENTS.md` and the
   `overview.md` System Boundary sentence so a render-result cache is in scope while compilation,
   persistent and shared caches remain excluded. Do not write the new specification yet: specs
   describe implemented behavior. Check: `python3 scripts/check-doc-links.py`, `git diff --check`.
1. **Determinism spike and value check.** In `crates/typrest-core/src/compiler/` add the
   `today()`-read flag to `TemplateWorld` and `Document::uses_current_date()` (new
   `pub(crate)` constructor argument). Tests (real Typst, in the ACL, per
   [testing.md](../development/testing.md)): two compiles of the same template and payload yield
   byte-identical PDFs; a template calling `datetime.today()` sets the flag, one that does not
   leaves it clear; a package-internal call sets it; a second compile of a `today()` template (to
   exercise `comemo` revalidation) still sets it. Record a short benchmark (hit versus miss for
   `examples/invoice`) in the PR description to confirm the feature is worth its cost. If the flag
   cannot be made reliable, stop and revisit D5 before continuing.
2. **Generation and fingerprint (core).** Add the static generation counter and accessor to
   `PreparedTemplateBundle`; extract the shared payload serialization used by `build_inputs`; add
   a `payload_fingerprint` function with `sha2` (new `typrest-core` dependency). Tests: generation
   differs for each `new`/`without_schema`; fingerprint equal for key-reordered objects and
   whitespace-only differences (parse first), different for `1` versus `1.0`, equal for omitted
   versus explicit defaults after `validate_with_defaults`; fingerprint input equals the string
   given to Typst. Files: `registry.rs`, `compiler/prepared_template.rs`, new
   `render/fingerprint.rs` (use `render.rs` plus `render/`, no `mod.rs`).
3. **`RenderCache` (core).** New `render/cache.rs`: `RenderCacheConfig { max_bytes,
   max_entry_bytes, ttl, templates }`, `RenderCacheKey`, `RenderCache::get_or_render`, weigher,
   `Expiry` for non-retained documents, `invalidate_template`, `clear`, `stats()`. Add `moka`
   (`future` feature) to `typrest-core` only. First confirm in a scratch test that a zero
   per-entry expiry keeps a value out of the cache while still coalescing concurrent callers; if
   not, use `optionally_get_with` with a leader-held slot, or an in-flight map. Tests with a counting
   mock `PdfCompiler` (pattern in `render.rs` tests; call `run_pending_tasks` before asserting
   eviction): hit skips compile; concurrent identical calls compile once; errors are not stored;
   oversize and time-dependent results are returned but not retained; byte budget eviction; TTL
   (use moka's test clock or a short TTL with `futures-executor` plus sleep helper); cancelled
   leader does not fail followers.
4. **`RenderService` integration (core).** Add `RenderService::with_render_cache(self, cache)`
   (default: none, so existing `new` call sites and tests are unchanged),
   `render_with_outcome(request, CachePolicy) -> Result<RenderOutcome, RenderError>` with
   `CachePolicy { Default, Refresh, NoStore }` and `CacheStatus`, and make `render` delegate with
   `Default`. Order: lookup bundle, validate with defaults, check template coverage, compute key,
   `get_or_render`. Tests with `MemoryLoader`/registry helpers and mock compilers: validation errors
   precede the cache and never reach the compiler; a reload (new generation) misses; templates
   outside the list bypass; `Refresh` replaces the entry; `NoStore` neither reads nor writes; the
   disabled path never constructs a key.
5. **Configuration and wiring (application).** Extend `RawProjectConfig`/`ProjectConfig` in
   `crates/typrest/src/config.rs` with `render_cache` (`deny_unknown_fields` inside the table),
   validation per REQ-CFG-050..056, conversion to `RenderCacheConfig`; wire in `serve.rs`
   (`with_render_cache`) and log one `render_cache.enabled` startup event with limits (no payload
   data). Tests in `config.rs` (valid, defaults, zero/negative values, `max_entry_bytes >
   max_bytes`, unknown key, invalid template ID). Decide whether `typrest check` validates the table
   (it already loads the config).
6. **HTTP behavior.** In `render_template.rs` parse request `Cache-Control` (`no-cache`, `no-store`,
   case-insensitive, comma-separated), call `render_with_outcome`, add `X-Typrest-Cache` for covered
   templates, skip warning logging on hits, and add `cache` to `render.completed`. Constants for
   header and values (no repeated magic strings). Tests in `http.rs` using the existing `app()`
   helpers with a counting compiler: miss then hit with identical bytes and one compile; header
   absent when disabled or not covered; `no-cache` and `no-store`; error responses unchanged and
   carry no cache header; `GET` and empty-body `POST` share an entry.
7. **Watch-mode invalidation.** Pass an `Option<Arc<RenderCache>>` (or a small purge callback) to
   `start_template_watcher`; call `invalidate_template` after a successful `reload` and `clear`
   after `replace_all`. Correctness already follows from generations; this frees memory. Tests in
   `watch.rs` (existing mock-compiler reload tests): entries gone after reload, still served after
   a failed reload (last known good keeps its generation).
8. **Integration with sibling plans, as they land.** Acquire the operational-limits permit inside
   the miss path; wrap the blocking executor inside the leader; feed `stats()` to the metrics
   plan; confirm async jobs and output destinations use `RenderService` unchanged. One small
   commit per sibling, each with a test that a hit does not consume a permit, queue slot, or
   executor thread.
9. **End-to-end and documentation.** Add an end-to-end test on a random port with real Typst
   (tests under `crates/typrest/tests/`): two identical `POST`s, second reports `hit`; a
   `datetime.today()` template always reports `miss`. Write `docs/specs/render-cache.md` from
   [`docs/specs/template.md`](../specs/template.md) with the final requirement IDs, and apply all
   updates in the next section. Delete this plan.

## Risks

- **Stale or wrong output** if the key omits any input that changes the PDF. Inputs today: template
  generation (source, packages, fonts), payload, clock. A future new input (for example request
  headers or locale passed to Typst) must extend the key; state this in the specification.
- **`today()` detection gaps.** If the flag misses some path, cached output goes stale across
  midnight. Mitigated by the step 1 spike, the `templates` list, and the finite TTL.
- **Memory.** The budget excludes Typst and `comemo` memory; enabling the cache raises peak RSS up
  to `max_bytes` plus key and warning overhead. `moka` is also an additional dependency and
  background maintenance.
- **Confidentiality.** See Privacy above; the hit/miss oracle and in-memory retention need an
  explicit statement in the specification and in the ADR.
- **Single-flight failure sharing.** A transient failure (timeout) of the leader is shared with
  waiting followers; acceptable because errors are not cached and the next request retries.
- **Behavior change risk for logs/metrics:** warnings are logged only on a miss; dashboards counting
  warnings per request will undercount.
- **Admission policy** can decline to store a new entry; tests must not assume a miss is always
  followed by a hit under memory pressure.

## Open questions

Marked "(blocking)" must be answered in step 0. No product answer is assumed.

- O1 (blocking). Is the scope change acceptable as worded (render-result cache in scope, other
  caches excluded)? Is a single ADR enough or should key semantics be a second ADR?
- O2 (blocking). Is a global switch plus a `templates` allowlist sufficient, or is per-template
  configuration inside the template directory wanted?
- O3 (blocking). Are the proposed defaults (128 MiB total, 8 MiB per entry, 3600 s TTL) acceptable,
  and should "no TTL" be allowed?
- O4. May clients bypass or refresh the cache with `Cache-Control` request headers (REQ-HTTP-072)?
  Is the unauthenticated denial-of-service angle acceptable (a client can already force compiles by
  varying payloads)?
- O5. Header naming: `X-Typrest-Cache` versus RFC 9211 `Cache-Status`; whether to add `ETag` and
  conditional requests later.
- O6. Should time-dependent documents be excluded (recommended), cached until the next UTC midnight,
  or controlled by an explicit per-template setting?
- O7. Should a leader that loses its client continue (detached task) so the result is cached?
- O8. Should warnings be replayed on hits (D10)?
- O9. Is `comemo` memo growth bounded in a long-running server (no `evict` call found)? If not, a
  separate decision is needed; the result cache does not fix it.
- O10. Whether the cache budget should appear in `GET /_healthz` or only in metrics.

## Documents to update on completion

- New `docs/specs/render-cache.md` (domain `CACHE`); add it to
  [`docs/specs/README.md`](../specs/README.md) and to the domain list in
  [`docs/README.md`](../README.md).
- [`docs/specs/http-api.md`](../specs/http-api.md) (REQ-HTTP-020, -025, new 070 group, non-goals),
  [`docs/specs/typrest-toml.md`](../specs/typrest-toml.md) (new fields table, REQ-CFG-003, 050 group,
  non-goals), [`docs/specs/logging.md`](../specs/logging.md) (REQ-LOG-050 and the `render.completed`
  field).
- [`docs/architecture/template-lifecycle.md`](../architecture/template-lifecycle.md) (Request
  Lifecycle, Purpose) and [`docs/architecture/overview.md`](../architecture/overview.md) (System
  Boundary, domain terms: `RenderCache`, generation; Template Lifecycle), and
  [`docs/architecture/compiler.md`](../architecture/compiler.md) (the `today()` flag, what is and is
  not reused).
- [`docs/product/vision.md`](../product/vision.md), [`ROADMAP.md`](../../ROADMAP.md),
  [`AGENTS.md`](../../AGENTS.md) (done in step 0), and the ADR index
  ([`docs/adr/README.md`](../adr/README.md)).
- [`docs/development/testing.md`](../development/testing.md) only if a new test location or command
  is introduced; [`docs/plans/README.md`](README.md) active plans list while the plan exists.
- `examples/` and [`examples/README.md`](../../examples/README.md) only if an example project gains
  a `[render_cache]` table.
- Delete this plan when the work is complete.
