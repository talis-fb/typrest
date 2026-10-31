# Plan: Explicit Request Body Limit

- **Status:** Implemented; remove this file once the change is committed (see the
  [plans index](README.md#active-plans)).
- **Owner:** Talison Fabio

## Objective

Make the render request body limit explicit and configurable, and make its rejection use the
structured JSON envelope (`REQ-HTTP-040`). It is independent of [graceful shutdown](graceful-shutdown.md)
and [render timeout](render-timeout.md).

## Current behavior

Contracts: [HTTP API](../specs/http-api.md#request-body), [`typrest.toml`](../specs/typrest-toml.md#server).

Before this change, axum's implicit 2 MiB limit applied, and an oversized body was rejected with a
plain-text `413` before the handler ran. Nothing configured the limit.

## Decisions

| # | Decision |
|---|---|
| D1 | Configure with `[server].max_body_bytes` in `typrest.toml`. No `serve` flag. `[server]` already holds the render limits, so limits stay in one place. |
| D2 | Integer bytes, default `2097152` (unchanged behavior). No size-suffix parser. Valid range `1` to `1073741824` (1 GiB). Outside that range is a configuration load error. |
| D3 | Pre-render rejections (`400`, `404`, `422`, `413`) are not logged here. Logging belongs to [render metrics](render-metrics.md). |
| D4 | Body-read failures other than the length limit (for example, a client disconnecting mid-body) return `400 invalid_json` with the message `failed to read request body`. |
| D5 | Error precedence: `invalid_template_id`, then the content-type `invalid_json` check, then `payload_too_large`, then the body-read failure, then JSON-parse `invalid_json`, then render errors. |

Rejected alternative: a global middleware that rewrites every non-JSON error body. It is broader and
belongs to a separate `REQ-HTTP-040` cleanup for framework errors such as `405`.

## Requirements

Delivered. Requirement IDs were claimed in the specifications, which is the source of truth:

- `REQ-HTTP-077`: oversized bodies return `413 payload_too_large`, the body is not parsed, and a body
  of exactly the limit is accepted.
- `REQ-HTTP-078`: error precedence and the body-read failure mapping (D4, D5).
- `REQ-CFG-055`: `max_body_bytes` range validation. The default and table row are in
  `typrest-toml.md`.
- `REQ-CLI-044`: `server.starting` now also reports `max_body_bytes`.
- `REQ-HTTP-041`: `413 payload_too_large` row.

## Implementation

- `crates/typrest/src/config.rs`: `max_body_bytes` in `[server]`, with `DEFAULT_MAX_BODY_BYTES` and
  `MAX_BODY_BYTES_LIMIT`, resolved through `bounded_setting`. Tests cover default, explicit, bounds,
  out-of-range, and wrong types.
- `crates/typrest/src/http/config.rs`: `ServerConfig::max_body_bytes` and `with_max_body_bytes`.
- `crates/typrest/src/http/router.rs`: `DefaultBodyLimit::max` layer.
- `crates/typrest/src/http/handlers/render_template.rs`: the body is extracted as
  `Result<Bytes, BytesRejection>` and mapped by `body_read_error`, after the template-ID and
  content-type checks.
- `crates/typrest/src/http/errors.rs`: `ApiError::payload_too_large`.
- `crates/typrest/src/commands/serve.rs`: passes the value and logs it on `server.starting`.

## Open follow-ups

- Slow-client protection (header or body read timeouts) is out of scope. Use a reverse proxy.
- Framework-generated errors without a JSON envelope (for example `405`) still violate `REQ-HTTP-040`.
  They need a separate fix.
