# Plan: PDF Output Destinations (Filesystem and S3)

- **Status:** Draft
- **Owner:** Talison Fabio

## Objective

Let a client ask Typrest to write the rendered PDF to an operator-configured **output destination**
(a filesystem directory first, an S3-compatible bucket second) and receive a small JSON receipt
with the object key, size, checksum, and, where configured, a path or URL. Today every successful
render streams the PDF bytes back in the HTTP response. Large PDFs, archival workflows, and callers
that only forward a link to someone else would no longer have to download and re-upload the bytes.

Inline PDF responses stay the default. Delivery to a destination is an explicit per-request opt-in,
so existing clients are unaffected.

## Prerequisites (scope and ADR approval)

This feature conflicts with documented scope, and nothing may be implemented until step 0 is
approved:

- the [product vision](../product/vision.md#scope) excludes "S3 or other remote backends";
- [`ROADMAP.md`](../../ROADMAP.md) lists S3 as out of scope in its closing paragraph about packages
  and Store mode;
- [`AGENTS.md`](../../AGENTS.md#critical-architecture-rules) forbids implementing "S3" without an
  explicit decision and scope change;
- the [architecture overview](../architecture/overview.md#system-boundary) lists "remote backends"
  as outside the current scope;
- the vision's [high-level requirements](../product/vision.md#high-level-requirements) and
  `REQ-HTTP-025` say rendering requests must not read files or access the network.

Those exclusions were written for **template storage** (Store mode: content storage, manifests,
publishing, tags, remote template backends) and for package sources. This plan is about **output
delivery**: where a finished PDF goes after a successful render. The two concerns are different:

| Concern | Store mode (still excluded) | Output delivery (this plan) |
|---|---|---|
| Direction | Reads template source into the registry | Writes rendered PDF bytes out |
| When | Startup and watch reloads | After a successful request-time render |
| Affects the registry | Yes | No |
| Affects compilation | Yes | No; compilation stays I/O-free |

Even so, the exclusion lists name S3 without qualification, and the "no network during rendering"
rule is a stated high-level requirement. A human must therefore approve a scope change that:

1. splits "S3 or other remote backends" in the vision into "remote **template** backends (Store
   mode)", which stay excluded, and "output destinations", which become included;
2. narrows the request-time I/O rule: **compilation** never reads files or accesses the network;
   only the post-render delivery stage, and only for a destination the request selected, may write
   to the filesystem or network;
3. accepts the ADR proposals in [Decisions and alternatives](#decisions-and-alternatives).

Phase 1 (filesystem only) adds no remote backend and could be approved without phase 2 (S3). The
plan is sequenced so that phase 2 can be rejected or postponed without rework.

## Current behavior

- **Rendering contract.** `GET` and `POST /{*template_id}` return `200`, `application/pdf`, and the
  PDF bytes (`REQ-HTTP-020`); the body is the raw render payload (`REQ-HTTP-021`, `REQ-HTTP-022`);
  rendering reads only the registry and never touches files or the network (`REQ-HTTP-025`). See
  [HTTP API, rendering](../specs/http-api.md#rendering).
- **Error envelope.** Fixed set of five codes (`REQ-HTTP-040` to `REQ-HTTP-044`); no physical paths
  in error responses (`REQ-HTTP-044`). See [HTTP API, errors](../specs/http-api.md#errors).
- **No authentication** on any route (`REQ-HTTP-050`, `REQ-HTTP-051`).
- **Handler.** `crates/typrest/src/http/handlers/render_template.rs:16-107` parses the ID (`:23-29`),
  enforces the JSON content type for `POST` (`:32-38`), parses the payload (`:40-46`), calls
  `RenderService::render` (`:50-53`), logs `render.completed`/`render.failed` (`:68-76`, `:95-103`),
  and builds the PDF response in `pdf_response` (`:130-136`). Query strings and request headers
  other than `Content-Type` are ignored.
- **Routing.** One catch-all render route for `GET` and `POST`
  (`crates/typrest/src/http/router.rs:16-19`).
- **State.** `AppState` holds `registry`, `render_service`, and `log_format`
  (`crates/typrest/src/http/state.rs:5-10`).
- **Core render path.** `RenderService::render` looks up the prepared bundle, validates and applies
  defaults, and compiles (`crates/typrest-core/src/render.rs:5-34`). `Document` is `Bytes` plus
  warnings (`crates/typrest-core/src/template.rs:49-78`).
- **Port pattern.** Core already defines an application-implemented port with
  `PackageRepository` (`crates/typrest-core/src/package.rs:8-14`), implemented in
  `crates/typrest/src/package/universal_package.rs`. Core has no HTTP or filesystem dependency
  ([architecture overview](../architecture/overview.md#workspace-responsibilities)).
- **Configuration.** `typrest.toml` has only `schema_version`, `templates_root`, and `web_ui`
  ([specification](../specs/typrest-toml.md)); parsing is in `crates/typrest/src/config.rs:11-25`
  (`ProjectConfig`, `RawProjectConfig`) and `:44-86` (`load_project_config`); `templates_root`
  path rules are in `:123-153`. Unknown top-level fields are ignored (`REQ-CFG-003`). The spec's
  non-goals exclude "store backends" and "per-template overrides".
- **Startup.** `run_serve` loads config, builds the registry, and binds only after every template
  compiles (`crates/typrest/src/commands/serve.rs:19-198`; `REQ-CLI-040`, `REQ-CLI-041`).
  `run_check` (`crates/typrest/src/commands/check.rs`) performs the same build without binding.
- **Errors.** `ApiError` and `ErrorBody` (`crates/typrest/src/http/errors.rs:16-56`, `:86-96`)
  carry `error`, `message`, and optional `violations`, `line`, `diagnostics`.
- **Logging.** JSON logs must not contain payloads or PDF bytes (`REQ-LOG-033`).
- **Web UI.** Posts to `/{id}` and reads a PDF Blob (`webui/src/api.ts:95`; `REQ-WEB-070`,
  `REQ-WEB-071`).
- **Container.** The image runs as uid 1000 with `/project` as the working directory
  ([`Dockerfile`](../../Dockerfile)); [`docker-compose.yml`](../../docker-compose.yml) bind-mounts
  `examples/invoice` read-write and has no other services.
- **Tests.** HTTP router tests use `tower::ServiceExt::oneshot` with mock compilers in
  `crates/typrest/src/http.rs:33-1093`; the [testing guide](../development/testing.md) forbids tests
  that require Docker or Podman.

## Intended change

### Summary

1. Operators declare named destinations under `[outputs.<name>]` in `typrest.toml`.
2. A `POST` render request that carries the header `Typrest-Output: <name>` is validated against
   the destination **before** rendering, rendered exactly as today, then written to the destination
   under a server-generated key. The response is `201 Created` with a JSON receipt instead of the
   PDF.
3. Requests without the header behave exactly as today.
4. Destinations are validated before the socket binds; a broken destination fails startup.
5. Core owns the port (`OutputSink`) and the pure domain types (keys, key patterns, receipts);
   the application package owns the filesystem and S3 implementations, configuration, HTTP mapping,
   and logging.

### API sketch

Request (destination delivery):

```http
POST /financial/invoice HTTP/1.1
Content-Type: application/json
Typrest-Output: archive
Typrest-Output-Name: INV-2026-0042

{"customer":"Ada","amount":120}
```

Response:

```http
HTTP/1.1 201 Created
Content-Type: application/json

{
  "output": "archive",
  "kind": "filesystem",
  "template": "financial/invoice",
  "key": "financial/invoice/2026/10/29/INV-2026-0042.pdf",
  "size": 48213,
  "sha256": "9f2c5e…",
  "contentType": "application/pdf",
  "path": "/srv/typrest/outputs/financial/invoice/2026/10/29/INV-2026-0042.pdf"
}
```

Field presence:

| Field | When present |
|---|---|
| `output`, `kind`, `template`, `key`, `size`, `sha256`, `contentType` | Always |
| `path` | `kind = "filesystem"` and `link = "path"` |
| `bucket` | `kind = "s3"` (open question: see [Q7](#open-questions)) |
| `url` | `link = "public"` or `link = "presigned"` |
| `urlExpiresAt` | `link = "presigned"` (RFC 3339 UTC) |

Field names use camelCase, like the existing `hasSchema`. No `Location` header is sent, because a
presigned URL in a header is more likely to be logged by intermediaries and a filesystem path is not
a URI (open question [Q3](#open-questions)).

New error codes (all in the existing envelope, `REQ-HTTP-040`):

| Condition | Status | `error` |
|---|---|---|
| `Typrest-Output` names no configured destination | `400` | `unknown_output` |
| `Typrest-Output` on `GET`; `Typrest-Output-Name` missing, invalid, or not used by the key pattern; header not valid ASCII | `400` | `invalid_output_request` |
| Key already exists and `on_conflict = "fail"` | `409` | `output_conflict` |
| Filesystem write failed | `500` | `output_write_failed` |
| S3 write failed | `502` | `output_write_failed` |
| Write exceeded the destination timeout | `504` | `output_timeout` |

Validation order: template ID (`REQ-HTTP-023`), content type, output selection (header, method,
destination, name), JSON payload, render (schema then compile), key generation, write. Output
selection is checked before rendering so a misconfigured client never wastes a compilation.
Messages name the destination but never contain physical paths, endpoints, credentials, or
presigned URLs (extends `REQ-HTTP-044`).

### `typrest.toml` sketch

```toml
schema_version = "v1"
templates_root = "templates"

# Phase 1
[outputs.archive]
kind = "filesystem"
root = "/srv/typrest/outputs"            # absolute, or relative to the project root
key = "{template}/{year}/{month}/{day}/{name}.pdf"
link = "path"                            # "none" | "path" | "public"
on_conflict = "fail"                     # "fail" | "replace"

# Phase 2
[outputs.invoices-s3]
kind = "s3"
bucket = "acme-invoices"
region = "eu-west-1"
endpoint = "http://minio:9000"           # optional, for MinIO, R2, and other S3-compatible stores
force_path_style = true                  # optional, default false
key = "typrest/{template}/{uuid}.pdf"
link = "presigned"                       # "none" | "public" | "presigned"
presign_expiry_seconds = 900
write_timeout_seconds = 30
```

New fields (all proposed; defaults marked "proposed" need confirmation in step 0):

| Field | Type | Required | Default | Notes |
|---|---|---|---|---|
| `outputs` | table of tables | no | empty | Keys are destination names |
| `outputs.<name>` name | table key | — | — | Proposed rule: `[a-z0-9][a-z0-9_-]{0,63}` |
| `kind` | string | yes | — | `"filesystem"` (phase 1), `"s3"` (phase 2) |
| `key` | string | no | `"{template}/{uuid}.pdf"` (proposed) | Key pattern; see below |
| `link` | string | no | `"none"` | Allowed values depend on `kind` |
| `on_conflict` | string | no | `"fail"` | `"fail"` or `"replace"` |
| `write_timeout_seconds` | integer | no | `30` (proposed) | Upper bound for one write |
| `root` | string | filesystem: yes | — | Directory that must exist at startup |
| `public_base_url` | string | when `link = "public"` | — | `http`/`https`; `url = base + key` |
| `bucket` | string | s3: yes | — | |
| `region` | string | s3: no | credential-chain default | |
| `endpoint` | string | s3: no | AWS | `http`/`https` URL |
| `force_path_style` | boolean | s3: no | `false` | Needed by most MinIO setups |
| `presign_expiry_seconds` | integer | when `link = "presigned"` | `900` (proposed) | 1 to 604800 (S3 SigV4 maximum) |
| `verify_on_startup` | boolean | s3: no | `true` | `HeadBucket` before binding |

Rules:

- Tables under `outputs` reject unknown fields (`serde(deny_unknown_fields)`), unlike the top level
  (`REQ-CFG-003`). A typo in a delivery setting must fail loudly, and fields such as
  `secret_access_key` must never be silently accepted.
- Credentials never appear in `typrest.toml`. S3 credentials come from the standard AWS chain:
  environment variables, web identity token, ECS/EKS container credentials, EC2 instance metadata,
  and, if the chosen client supports it, shared profiles.
- A filesystem `root` may be absolute (outputs usually live on a separate volume) or relative to the
  project root. It must exist, be a directory, be writable, and must not be inside `templates_root`
  or `<project>/.typrest`: a destination inside `templates_root` would feed rendered files back into
  template discovery and watch mode.
- `typrest init` output is unchanged (`REQ-CFG-040`); destinations are opt-in.

### Key patterns

A key pattern is a `/`-separated string with placeholders:

| Placeholder | Value |
|---|---|
| `{template}` | Template ID, keeping its `/` segments |
| `{year}`, `{month}`, `{day}` | UTC render date, zero-padded |
| `{timestamp}` | UTC `YYYYMMDDTHHMMSSZ` |
| `{uuid}` | Server-generated UUID (v7 proposed, so keys sort by time) |
| `{sha256}` | Lowercase hex SHA-256 of the PDF bytes |
| `{name}` | Client-supplied `Typrest-Output-Name`, after validation |

Rules (validated at configuration load, and again on every rendered key):

- A pattern contains at least one of `{uuid}`, `{sha256}`, or `{name}`, so two renders do not
  collide by construction.
- Unknown placeholders, unbalanced braces, a leading `/`, empty segments, and `.` or `..` segments
  are configuration errors.
- Literal characters are limited to ASCII letters, digits, `-`, `_`, `.`, and `/`. A segment must
  not start with `.` (temporary files use a leading `.`, see [filesystem writes](#filesystem-writes)).
- A rendered key is at most 1024 bytes (the S3 limit).
- `{name}` is a single segment matching `[A-Za-z0-9_-][A-Za-z0-9._-]{0,127}`. It cannot contain `/`,
  so a client can never choose a directory, an absolute path, or traverse upward. A request that
  sends `Typrest-Output-Name` to a destination whose pattern has no `{name}` is rejected, and a
  pattern with `{name}` makes the header required.

The client never supplies a path or a full key. This is the main path-traversal control; the
filesystem implementation adds a second check that the joined path stays under `root`.

### Filesystem writes

- Write to a temporary file `.<uuid>.tmp` in the final directory, `fsync` it, then publish it:
  - `on_conflict = "fail"`: `hard_link(temp, final)` (fails atomically if `final` exists), then
    remove `temp`;
  - `on_conflict = "replace"`: `rename(temp, final)`.
- `fsync` the parent directory after publishing. Create intermediate directories with
  `create_dir_all` inside `root` only.
- Run the blocking work on `tokio::task::spawn_blocking`, never on the async worker threads.
- On any failure, remove the temporary file best-effort. A crash can leave a `.…tmp` file; cleanup
  of such leftovers is out of scope and documented.
- File permissions follow the process umask. Retention and cleanup are out of scope.

### S3 writes (phase 2)

- One `PutObject` per render with `Content-Type: application/pdf` and a SHA-256 checksum. PDFs are
  well below the 5 GiB single-PUT limit, so multipart upload is out of scope.
- `on_conflict = "fail"` uses a conditional write (`If-None-Match: *`). AWS S3, MinIO, and
  Cloudflare R2 support it; destinations whose service does not must use `"replace"` (open question
  [Q9](#open-questions)).
- Retries are left to the client library's standard retry policy. The key is fixed before the first
  attempt, so a retry overwrites the same object rather than producing a duplicate.
- `link = "presigned"` signs a `GET` URL locally after a successful write; no extra network call.

### Core abstraction (crate boundary)

`typrest-core` gains a module `output` (`crates/typrest-core/src/output.rs` plus `output/`), with no
filesystem, network, or HTTP dependency:

```rust
/// Validated relative object key (`a/b/c.pdf`).
pub struct OutputKey(String);

/// Validated client-supplied name for `{name}`.
pub struct OutputName(String);

/// Parsed key pattern; `render` is pure.
pub struct KeyPattern { /* segments */ }

pub struct KeyContext<'a> {
    pub template: &'a TemplateId,
    pub rendered_at: time::OffsetDateTime,
    pub id: Uuid,                 // injected for deterministic tests
    pub sha256: &'a str,
    pub name: Option<&'a OutputName>,
}

/// What a sink returns after a successful write.
pub struct StoredOutput {
    pub key: OutputKey,
    pub size: u64,
    pub sha256: String,
    pub path: Option<String>,     // set only by filesystem sinks with `link = "path"`
    pub url: Option<String>,
    pub url_expires_at: Option<time::OffsetDateTime>,
}

#[async_trait::async_trait]
pub trait OutputSink: Send + Sync {
    /// Startup check: called once before the server binds.
    async fn verify(&self) -> Result<(), OutputSinkError>;

    /// Write `bytes` under `key`. Implementations enforce `on_conflict`.
    async fn write(&self, key: &OutputKey, bytes: Bytes) -> Result<StoredOutput, OutputSinkError>;
}

#[derive(Debug, thiserror::Error)]
pub enum OutputSinkError {
    Conflict,          // -> 409 output_conflict
    Timeout,           // -> 504 output_timeout
    Unavailable { .. },// -> 500/502 output_write_failed; carries a safe detail for logs only
    Invalid { .. },    // configuration or verification failure
}

/// Named destinations built at startup; immutable afterwards.
pub struct OutputDestinations { /* BTreeMap<DestinationName, Destination> */ }
pub struct Destination { pub name, pub kind: &'static str, pub pattern: KeyPattern,
                         pub sink: Arc<dyn OutputSink> }
```

Exact names are proposals; the implementation step may refine them but must keep the boundary.
Core also exposes a `test-util` `MemoryOutputSink` (records writes, can be told to fail), mirroring
`MemoryLoader`. `RenderService` does not change: delivery composes after it, in a small core
function or method (`OutputDestinations::deliver(&self, name, template, document, name_header)`)
that hashes the bytes, renders the key, and calls the sink. Both the HTTP handler and a future
async job worker call this one function (see [Dependencies on other plans](#dependencies-on-other-plans)).

New core dependencies: `uuid` (v7 feature) and `sha2` (already present transitively in
`Cargo.lock`). Neither performs I/O beyond OS randomness for UUID generation. Alternative: generate
the UUID in the application and pass it in; this keeps core deterministic and is acceptable too.

Application package additions (`crates/typrest`):

- `src/output.rs` + `src/output/filesystem.rs` (phase 1) and `src/output/s3.rs` (phase 2, behind a
  Cargo feature);
- `src/config.rs`: `outputs` parsing into an owned `OutputConfig` per destination;
- `src/commands/serve.rs` and `check.rs`: build and verify destinations;
- `src/http/state.rs`: `AppState.outputs: Arc<OutputDestinations>`;
- `src/http/handlers/render_template.rs`: output selection and receipt response;
- `src/http/errors.rs`: mapping of `OutputSinkError` and selection errors.

## Affected requirements

### Existing requirements that change

| ID | Change |
|---|---|
| `REQ-HTTP-020` | Add "unless the request selects an output destination (`REQ-HTTP-070`)". |
| `REQ-HTTP-025` | Narrow to compilation: the render stage reads only the registry; only the delivery stage of a request that selected a destination may write to that destination. |
| `REQ-HTTP-041` | Add the five new rows (`unknown_output`, `invalid_output_request`, `output_conflict`, `output_write_failed`, `output_timeout`). |
| `REQ-HTTP-044` | Also forbid physical paths, endpoints, credentials, and presigned URLs in output error responses. |
| `REQ-HTTP-051` | Mention that an unauthenticated client can fill configured destinations. |
| `REQ-CFG-003` | Clarify that unknown fields are ignored at the top level only; `outputs` tables are strict. |
| `REQ-CLI-030` | `check` also validates output destination configuration (static checks, no network). |
| `REQ-CLI-041` | `serve` also fails before listening when a destination is invalid or fails verification. |
| `REQ-LOG-033` | Add presigned URLs and credentials to the forbidden JSON content. |

The `typrest.toml` non-goals ("store backends", "per-template overrides") and the vision's
high-level requirement "Rendering requests must not read files or access the network" are prose,
not IDs, and change with the scope decision.

### Proposed new requirements (*proposed*, numbering uses the next free groups)

HTTP ([`http-api.md`](../specs/http-api.md), new group "Output delivery"):

- **REQ-HTTP-070** *(proposed)*: A `POST` render request with `Typrest-Output: <name>` MUST, after a
  successful render, write the PDF to that destination and return `201` with a JSON receipt instead
  of PDF bytes.
- **REQ-HTTP-071** *(proposed)*: Without `Typrest-Output`, rendering MUST behave as in
  `REQ-HTTP-020` to `REQ-HTTP-025`.
- **REQ-HTTP-072** *(proposed)*: `Typrest-Output` on `GET` MUST fail with `400`
  `invalid_output_request` before rendering.
- **REQ-HTTP-073** *(proposed)*: Output selection, including destination lookup and
  `Typrest-Output-Name` validation, MUST be checked before rendering.
- **REQ-HTTP-074** *(proposed)*: The receipt MUST contain `output`, `kind`, `template`, `key`,
  `size`, `sha256`, and `contentType`, and MUST contain `path`, `bucket`, `url`, and `urlExpiresAt`
  only as defined by the destination's `link`.
- **REQ-HTTP-075** *(proposed)*: When the write fails, the server MUST NOT return the PDF bytes and
  MUST return the matching output error from `REQ-HTTP-041`.

Configuration ([`typrest-toml.md`](../specs/typrest-toml.md), new group `outputs`):

- **REQ-CFG-050** *(proposed)*: `outputs` MUST be optional and default to no destinations.
- **REQ-CFG-051** *(proposed)*: Destination names MUST match the name rule; others MUST be a
  configuration load error.
- **REQ-CFG-052** *(proposed)*: Unknown fields inside a destination MUST be a configuration load
  error.
- **REQ-CFG-053** *(proposed)*: `kind` MUST be `"filesystem"` or `"s3"`; a kind not compiled into
  the binary MUST be a configuration load error that names the kind.
- **REQ-CFG-054** *(proposed)*: A filesystem `root` MUST resolve relative to the project root when
  relative, MUST exist and be a directory, and MUST NOT be inside `templates_root` or `.typrest`.
- **REQ-CFG-055** *(proposed)*: Credentials MUST NOT be configurable in `typrest.toml`.
- **REQ-CFG-056** *(proposed)*: An invalid `key` pattern (see `REQ-OUT-010` to `REQ-OUT-015`) MUST
  be a configuration load error.

Output destinations (new specification `docs/specs/output-destinations.md`, new domain `OUT`, to be
added to the [specification index](../specs/README.md)):

- **REQ-OUT-001** *(proposed)*: Destinations MUST be fixed at startup; watch mode MUST NOT reload
  them.
- **REQ-OUT-010** to **REQ-OUT-015** *(proposed)*: placeholder set, uniqueness rule, character set,
  segment rules, 1024-byte limit, and `{name}` rule from [Key patterns](#key-patterns).
- **REQ-OUT-020** *(proposed)*: A client MUST NOT be able to choose a directory, absolute path, or
  key outside the destination; only `{name}` is client-controlled.
- **REQ-OUT-030** *(proposed)*: A filesystem write MUST be published atomically, so a reader never
  observes a partial file at the final key.
- **REQ-OUT-031** *(proposed)*: `on_conflict = "fail"` MUST NOT overwrite an existing object.
- **REQ-OUT-040** *(proposed)*: `serve` MUST verify every destination before binding (filesystem:
  writable probe; S3: `HeadBucket` unless `verify_on_startup = false`).
- **REQ-OUT-050** *(proposed)*: A write MUST be abandoned and reported as `output_timeout` after
  `write_timeout_seconds`.
- **REQ-OUT-060** *(proposed)*: S3 credentials MUST come from the environment or platform credential
  chain.

CLI ([`cli.md`](../specs/cli.md)):

- **REQ-CLI-035** *(proposed)*: `check` MUST validate destination configuration statically and MUST
  NOT contact remote destinations.

Logging ([`logging.md`](../specs/logging.md), new group):

- **REQ-LOG-050** *(proposed)*: Every write MUST emit `output.write.completed` or
  `output.write.failed` with `template`, `output`, `kind`, `key`, `size`, `latency_ms`, and, on
  failure, `error_kind`.
- **REQ-LOG-051** *(proposed)*: Logs MUST NOT contain presigned URLs or credentials.

## Decisions and alternatives

### D1. Delivery is a post-render stage, not part of compilation (ADR proposal A)

Recommendation: keep `PdfCompiler` and `RenderService` unchanged and compose delivery after them.
Alternatives: give the compiler a sink (mixes Typst ACL with I/O, rejected); make delivery a
separate process or sidecar that receives the PDF (no change to Typrest, but the user asked for it in
Typrest; noted as the zero-scope alternative).

### D2. Port in core, implementations in the application (part of ADR proposal A)

Recommendation: `OutputSink` trait and pure types in `typrest-core`, implementations in
`crates/typrest`, like `PackageRepository`. This keeps core free of filesystem and HTTP access and
lets async jobs reuse the same port. Alternative: everything in the application package; simpler,
but async jobs and future workers would duplicate key generation and receipt logic.

### D3. How the client opts in

| Option | Pros | Cons |
|---|---|---|
| **Header `Typrest-Output` (recommended)** | Body and schema untouched; works with any payload type; no new route | Less discoverable than a route; must be documented |
| Query `?output=archive` | Easy in curl and browsers | Leaks into access logs and caches; GET-friendly, which is wrong for a write |
| `Accept: application/json` negotiation | Standard HTTP | Cannot name a destination; many clients send `*/*`; ambiguous with JSON errors |
| Body envelope `{"data":…,"output":…}` | Self-describing | Breaks `REQ-HTTP-021` and schema validation of the raw body; rejected |
| New route, such as `POST /_outputs/{name}/{*template_id}` | Explicit, separate contract | New reserved route family (`REQ-HTTP-004`); duplicates the render route |

The `X-` prefix is avoided per RFC 6648. Delivery is `POST`-only because `GET` must stay safe.

### D4. Destination selection policy

Recommendation for this plan: per request only, by name, among operator-configured destinations.
Alternatives deferred: a global or per-template default destination (would change the response type
for existing clients unless the header still selects it; conflicts with the `typrest.toml`
non-goal "per-template overrides"); client-supplied arbitrary paths or buckets (rejected for
security).

### D5. Key naming

Recommendation: operator-defined patterns with server-generated `{uuid}` (v7) by default and an
optional sanitized `{name}`. Alternatives: fixed `{template}/{uuid}.pdf` only (simplest, but users
asked for meaningful names); fully client-supplied keys (rejected, traversal and overwrite risk);
content-addressed only (`{sha256}`), which interacts with caching (see D10).

### D6. Response shape and status

Recommendation: `201 Created` with a JSON receipt, no `Location` header. Alternatives: `200` (fine,
less precise); `303 See Other` to the URL (only works for `link = "public"`/`"presigned"`, breaks
filesystem destinations, and many clients follow redirects automatically).

### D7. Links

Recommendation: `link = "none"` by default (key only); `"path"` for filesystem (explicitly exposes a
physical path, an operator choice), `"public"` (base URL plus key) for both kinds, `"presigned"` for
S3. Presigning is local (no network) and its expiry is reported.

### D8. S3 client library (ADR proposal B, phase 2 only)

| Crate | Pros | Cons |
|---|---|---|
| **`object_store` (recommended)** | One interface for S3 (and later GCS/Azure), `InMemory` for tests, conditional put (`PutMode::Create`), local presigning (`Signer`), configurable retries and timeouts, S3-compatible endpoints | Uses `reqwest`/`rustls`, a second TLS stack next to the vendored OpenSSL used by the `typst-kit` downloader; shared AWS profile support is limited compared with the AWS SDK |
| `aws-sdk-s3` + `aws-config` | Official, complete credential chain including profiles and SSO, every S3 feature | Largest dependency tree, compile time, and binary growth of the three |
| `rust-s3` | Small | Smaller maintenance base, weaker credential-chain coverage, fewer guarantees |

Before the ADR is accepted, step 8 measures release binary size and clean build time with each
candidate and records the numbers in the ADR. S3 support sits behind a Cargo feature `s3` of the
`typrest` package; whether it is on by default (and in the container image) is open question
[Q5](#open-questions).

### D9. Startup verification

Recommendation: verify before binding (filesystem: create and delete a probe file under `root`;
S3: `HeadBucket`, skippable with `verify_on_startup = false` because it requires `s3:ListBucket`).
`typrest check` performs only static validation, so CI runs of `check` need no credentials.

### D10. Relation to the result cache

Recommendation: destinations are not a cache. A cache hit produces a `Document` that is then
delivered like any render. Using `{sha256}` keys with a future `on_conflict = "keep"` (return the
existing object) could make delivery idempotent, but PDFs are only byte-identical if Typst output is
deterministic for equal inputs, which is not verified (open question [Q8](#open-questions)).

### ADR proposals (require human approval; do not create files before approval)

- **ADR proposal A: "Deliver rendered PDFs to configured output destinations."** Context: inline
  delivery only; scope exclusion of remote backends written for Store mode. Decision: D1, D2, D3,
  D4, and the narrowed request-time I/O rule. Alternatives: sidecar delivery, body envelope, new
  route, client-supplied paths. Consequences: new public contract (header, receipt, error codes), a
  new security property (unauthenticated writes to operator destinations), and a changed vision
  requirement. Meets all three ADR criteria in [`docs/adr/README.md`](../adr/README.md).
- **ADR proposal B: "Use `object_store` for S3 output destinations."** Only if phase 2 is approved.
  Records the measured binary-size and build-time comparison and the credential-chain behavior.

D5 to D7 and D9 are specification-level choices and do not need separate ADRs.

## Out of scope

- Store mode and every form of remote template storage (still excluded).
- Retention, lifecycle, cleanup of outputs or orphaned temporary files, quotas, and garbage
  collection (use S3 lifecycle rules or external jobs).
- Serving stored files from Typrest (no download route); `link = "public"` assumes something else
  serves them.
- GCS, Azure Blob, SFTP, WebDAV, and other destination kinds.
- Multipart uploads, server-side encryption settings, object tags, ACLs, and custom metadata
  (open question [Q10](#open-questions) for SSE).
- Authentication or per-client authorization of destinations.
- Web UI changes (the UI keeps rendering inline; see [Q6](#open-questions)).
- Delivering the same PDF to several destinations in one request.

## Steps

Phase 0: approval.

0. **Approval gate.** Present this plan, ADR proposals A and B, and the open questions to the
   owner. Do not implement anything until the owner approves the scope change and ADR proposal A
   (and, for phase 2, ADR proposal B) and answers or explicitly defers Q1 to Q5. Done when the
   decisions are recorded in this plan (update the Decisions section and remove answered
   questions).

Phase 1: abstraction and filesystem destination.

1. **Scope and ADR documents.** Create ADR A (next free number, currently `0009`) from
   [`docs/adr/template.md`](../adr/template.md) with the approved wording and add it to the ADR
   index. Update [vision](../product/vision.md) (split the S3 exclusion; amend the high-level
   requirement), [`ROADMAP.md`](../../ROADMAP.md) (new "Output destinations" section with phase 1
   and phase 2 items; keep S3 for Store mode excluded), [`AGENTS.md`](../../AGENTS.md) (critical
   rules: S3 allowed only as an output destination; core stays I/O-free through `OutputSink`), and
   the [architecture overview](../architecture/overview.md) (system boundary, domain terms, error
   boundaries, invariants). Add this plan to the active list in
   [`docs/plans/README.md`](README.md). Checks: `python3 scripts/check-doc-links.py`,
   `git diff --check`.
2. **Core output domain.** Add `crates/typrest-core/src/output.rs` and `output/` with `OutputKey`,
   `OutputName`, `KeyPattern` (parse and render), `KeyContext`, `StoredOutput`, `OutputSink`,
   `OutputSinkError`, `OutputDestinations`, the delivery function, and `MemoryOutputSink` behind the
   existing `test-util` feature. Add `uuid` (v7) and `sha2` to the workspace and core manifests.
   Export the public types intentionally from `lib.rs`. Tests: pattern parsing (each placeholder,
   unknown placeholder, braces, `.`/`..`, leading `/`, uniqueness rule, character set), key
   rendering with a fixed clock and UUID, 1024-byte limit, `OutputName` validation (traversal
   strings such as `../x`, `a/b`, `.hidden`, empty, 129 bytes, non-ASCII), delivery with
   `MemoryOutputSink` (receipt fields, sha256 correctness, error passthrough). Checks:
   `cargo test -p typrest-core output`, clippy, fmt; confirm core gained no `std::fs`, `tokio::fs`,
   or HTTP dependency (`rg -n 'std::fs|tokio::fs|axum|reqwest' crates/typrest-core/src`).
3. **Configuration parsing.** Extend `RawProjectConfig` and `ProjectConfig` in
   `crates/typrest/src/config.rs` with `outputs` (strict tables, filesystem kind only; `kind = "s3"`
   returns "not supported by this build" until phase 2). Resolve `root` (absolute or project-relative),
   reject roots inside `templates_root` or `.typrest`, validate `link`, `on_conflict`,
   `public_base_url`, `write_timeout_seconds`, and the key pattern via core. Tests with `tempfile`
   for every rule, including `secret_access_key` rejected as an unknown field. Update
   [`typrest-toml.md`](../specs/typrest-toml.md) (`REQ-CFG-003`, `REQ-CFG-050`+, non-goals) in the
   same commit. Checks: `cargo test -p typrest config`.
4. **Filesystem sink.** Add `crates/typrest/src/output.rs` and `output/filesystem.rs` implementing
   `OutputSink` with the [filesystem write](#filesystem-writes) algorithm on `spawn_blocking`, the
   joined-path containment check, the startup probe, `tokio::time::timeout` around the write, and
   `link` handling. Tests (tempdir): writes the bytes at the expected key; intermediate directories
   created; `fail` conflict returns `Conflict` and keeps the original bytes; `replace` overwrites;
   no temporary file remains after success or failure; read-only root fails `verify` and `write`
   (skip when running as root); `path` only with `link = "path"`; a pre-existing symlinked
   subdirectory pointing outside `root` is rejected or documented (decide in this step and record
   it in the spec). Checks: `cargo test -p typrest output`.
5. **Startup and check wiring.** Build `OutputDestinations` in `run_serve`
   (`crates/typrest/src/commands/serve.rs`) after configuration load, verify every destination
   before `crate::http::serve`, and log `output.destination.ready` per destination (name and kind
   only). `run_check` performs static validation only. Add `outputs` to `AppState`. Tests in
   `crates/typrest/tests/check.rs` and serve-level tests: invalid destination fails before binding
   with a clear error event; a project without `outputs` behaves exactly as before. Update
   [`cli.md`](../specs/cli.md) (`REQ-CLI-030`, `REQ-CLI-035`, `REQ-CLI-041`). Checks:
   `cargo test -p typrest --test check`, `cargo test --workspace`.
6. **HTTP delivery.** In `render_template.rs`, parse `Typrest-Output` and `Typrest-Output-Name`
   (header name constants), enforce `POST`, resolve the destination and validate the name before
   rendering, call delivery after a successful render, and return `201` with the receipt
   (`serde` struct with camelCase). Map errors in `errors.rs`. Emit `output.write.completed` and
   `output.write.failed`; keep `render.completed` unchanged. Router tests in
   `crates/typrest/src/http.rs` with `MemoryOutputSink`: no header returns the PDF (regression);
   header returns `201` and the receipt; unknown destination `400 unknown_output` and the compiler
   is not called; `GET` with header `400`; name missing, invalid, or unexpected `400`; schema
   failure still `422` and nothing written; compilation failure `500 compilation_failed` and nothing
   written; `Conflict` `409`; failure `500`/`502`; timeout `504`; error bodies contain no root path.
   One end-to-end test with a real filesystem sink and the real compiler on a random port. Update
   [`http-api.md`](../specs/http-api.md), [`logging.md`](../specs/logging.md), and create
   `docs/specs/output-destinations.md` plus its index entry. Checks: `cargo test -p typrest http`,
   full suite, doc links.
7. **Container, examples, and phase 1 wrap-up.** Add a commented filesystem destination example to
   an example project (or a new `examples/output-destinations`, indexed in
   [`examples/README.md`](../../examples/README.md)), a `README.md` section, and a commented
   `docker-compose.yml` volume (for example `./outputs:/outputs`, writable by uid 1000). Mark the
   phase 1 ROADMAP items done. Optionally extend `scripts/check-container.sh` with one delivery
   check. Checks: full Rust suite, doc links, `scripts/check-container.sh` when a container engine
   is available.

Phase 2: S3 destination (only after ADR proposal B is approved).

8. **Client spike and ADR B.** On a throwaway branch, build release binaries with `object_store`
   (`aws` feature) and with `aws-sdk-s3` + `aws-config`; record binary size, clean build time, and
   added crate count; confirm MinIO path-style access, conditional put, and presigning. Write
   ADR B with the numbers, after approval. Checks: numbers recorded in the ADR.
9. **S3 sink.** Add the `s3` Cargo feature and `crates/typrest/src/output/s3.rs` wrapping the chosen
   client: configuration fields from the [sketch](#typresttoml-sketch), credential chain, `HeadBucket`
   verification, conditional put, checksum, timeout, `public`/`presigned` links, and mapping to
   `OutputSinkError` (`502` mapping lives in `errors.rs` keyed by kind). Enable `kind = "s3"` in
   configuration parsing. Unit tests inject an in-memory store behind the same wrapper (with
   `object_store`, `InMemory`) to cover key layout, conflict behavior, and receipt fields; presign
   tests check URL shape and expiry with fixed credentials. Checks: `cargo test -p typrest --features
   s3 output`, clippy for both feature sets.
10. **MinIO integration.** Add integration tests gated on an environment variable such as
    `TYPREST_TEST_S3_ENDPOINT` (skipped otherwise, so `cargo test` never needs Docker), a CI job
    that starts MinIO as a service container and runs them, and an optional `minio` service under a
    Compose profile (`profiles: ["s3"]`) with documented environment credentials. Update
    [`testing.md`](../development/testing.md) and the container docs. Checks: the new CI job is
    green; `cargo test --workspace` unchanged without the variable.
11. **Completion.** Finalize specifications, ROADMAP, and architecture documents; delete this plan
    and remove it from the plans index. Checks: full validation commands from
    [`AGENTS.md`](../../AGENTS.md#validation-commands).

## Risks

- **Unauthenticated writes.** Any client that reaches the server can write PDFs to every configured
  destination (`REQ-HTTP-050`), filling disks or incurring storage cost. Mitigations: destinations
  are opt-in, keys are server-controlled, `REQ-HTTP-051` loopback guidance, and limits from the
  operational-limits plan. Authentication remains out of scope.
- **Scope creep toward Store mode.** An `OutputSink` that can write to S3 is tempting to reuse for
  template storage. The ADR must state that the port is write-only and output-specific.
- **Second TLS stack and binary growth** with any S3 client; measured in step 8.
- **Filesystem durability and portability.** `hard_link` no-clobber and directory `fsync` behave
  differently on network filesystems and some container volume drivers; atomicity is only
  guaranteed on local POSIX filesystems. Symlinks inside `root` can redirect writes.
- **Partial success.** A render succeeds but the write fails: the PDF is lost and the client must
  retry the whole request. Returning the PDF inline as a fallback was rejected (it would make the
  response type unpredictable), but it is open question [Q4](#open-questions).
- **Orphans.** S3 retries after an ambiguous failure can leave an object the client never learned
  about; filesystem crashes can leave `.…tmp` files.
- **Presigned URL leakage.** URLs are bearer tokens; they must stay out of logs and metrics labels.
- **Conditional-put support** varies across S3-compatible services.
- **Startup coupling.** A temporarily unreachable bucket blocks startup when verification is on.

## Open questions

- **Q1.** Is the scope split (output delivery included, Store mode excluded) acceptable, and does
  phase 2 (S3) go ahead or stop after phase 1?
- **Q2.** Opt-in mechanism: header `Typrest-Output` (recommended), query parameter, or a separate
  route? Should a project-wide default destination exist at all?
- **Q3.** `201` without `Location` (recommended) or `200`? Should a `Location` header be sent for
  `public` links only?
- **Q4.** On write failure, error only (recommended) or fall back to returning the PDF inline?
- **Q5.** Is the `s3` Cargo feature enabled by default and in the container image?
- **Q6.** Should the development Web UI offer destination delivery (it would need a metadata route
  listing destination names, which discloses configuration)?
- **Q7.** Should S3 receipts include `bucket`, and should filesystem receipts ever include `path` by
  default?
- **Q8.** Is Typst PDF output byte-identical for identical inputs (needed for content-addressed keys
  and an `on_conflict = "keep"` mode)?
- **Q9.** Must `on_conflict = "fail"` be supported for S3-compatible services without conditional
  writes, and if so, how (accept a race-prone `HeadObject` check, or refuse the configuration)?
- **Q10.** Are server-side encryption (SSE-S3, SSE-KMS), storage class, or object metadata (template
  ID, request ID) required for phase 2?
- **Q11.** Proposed defaults: `write_timeout_seconds = 30`, `presign_expiry_seconds = 900`, key
  pattern `{template}/{uuid}.pdf`, UUID v7. Confirm or replace.
- **Q12.** Should the filesystem destination create a missing `root` at startup, or must it exist?
  Should file mode be configurable?
- **Q13.** Should `typrest check` gain an opt-in flag to verify destinations (including network)?

## Dependencies on other plans

These sibling plans are being written in parallel; links may not resolve until they land.

| Plan | This plan needs | This plan provides |
|---|---|---|
| [Blocking render execution](blocking-render-execution.md) | A render call that returns a `Document` after the compile work leaves the blocking pool, so the compile concurrency permit is released before the write starts | Writes are async I/O (filesystem work on `spawn_blocking`, S3 on the async runtime) and must not hold a compile permit |
| [Operational limits](operational-limits.md) | Agreement on how the global request timeout relates to `write_timeout_seconds`, on the timeout error code (this plan proposes `output_timeout`), and on any concurrency limit for in-flight writes | Per-destination write timeout; write latency is a separate stage from compile latency |
| [Render metrics](render-metrics.md) | The metrics recorder interface and naming conventions | Proposed series: `typrest_output_writes_total{output,kind,outcome}`, `typrest_output_write_duration_seconds{output,kind}`, `typrest_output_bytes_total{output,kind}`; no keys, names, or URLs as labels |
| [Render result cache](render-result-cache.md) | A cache that sits before delivery and yields a `Document` | Destinations are not a cache (D10); `{sha256}` keys are available if deterministic output is confirmed |
| [Async render jobs](async-render-jobs.md) | Whether job results are stored in memory or in a destination, and how a job request selects one | The shared "render output" abstraction: `OutputSink`, `OutputDestinations::deliver`, and `StoredOutput`, whose JSON receipt (`REQ-HTTP-074`) can be embedded in a job status response. Whichever plan lands first defines these types in core, and the other reuses them instead of adding a second storage port |

## Documents to update on completion

- [Product vision](../product/vision.md): scope split and the amended high-level requirement.
- [`ROADMAP.md`](../../ROADMAP.md): new delivered section; S3 exclusion limited to Store mode.
- [`AGENTS.md`](../../AGENTS.md): critical architecture rules and documentation navigation row for
  the new specification.
- [Architecture overview](../architecture/overview.md): system boundary, workspace
  responsibilities, canonical domain terms (`OutputDestination`, `OutputSink`, `OutputKey`,
  `KeyPattern`, `StoredOutput`), error boundaries, invariants.
- [HTTP API](../specs/http-api.md), [`typrest.toml`](../specs/typrest-toml.md),
  [CLI](../specs/cli.md), [Logging](../specs/logging.md), and the new
  `docs/specs/output-destinations.md` with its [index](../specs/README.md) entry and the `OUT`
  domain in [`docs/README.md`](../README.md#requirement-ids).
- [Testing guide](../development/testing.md): output tests, the MinIO job, and its environment
  variable.
- [ADR index](../adr/README.md): ADR A and, for phase 2, ADR B.
- `README.md`, [`docker-compose.yml`](../../docker-compose.yml) comments and volumes,
  [`Dockerfile`](../../Dockerfile) comments, and [`examples/README.md`](../../examples/README.md) if
  an example is added.
- [Plans index](README.md): add this plan when approved; remove it and delete this file when done.
