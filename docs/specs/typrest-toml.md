# `typrest.toml` specification

- **Status:** Current
- **Scope:** The project configuration file of a filesystem-mode Typrest project, parsed in
  `crates/typrest/src/config.rs`.

## Purpose

`typrest.toml` configures a filesystem-mode project. It is found by `typrest check` and
`typrest serve` as described in the [CLI specification](cli.md#project-model-and-discovery).

## Requirements

### Location and format

```text
project/
  typrest.toml
  templates/
```

```toml
schema_version = "v1"
templates_root = "templates"
web_ui = false

[server]
max_concurrent_renders = 4
max_queued_renders = 8
shutdown_timeout_seconds = 25
max_body_bytes = 2097152
```

| Field | Type | Required | Default |
|---|---|---|---|
| `schema_version` | string | yes | none |
| `templates_root` | string | yes | none |
| `web_ui` | boolean | no | `false` |
| `[server].max_concurrent_renders` | integer | no | the process's available parallelism (`std::thread::available_parallelism`), or `1` when it cannot be determined |
| `[server].max_queued_renders` | integer | no | `2 × max_concurrent_renders`, capped at `1024` |
| `[server].shutdown_timeout_seconds` | integer | no | `25` |
| `[server].max_body_bytes` | integer | no | `2097152` (2 MiB) |

- **REQ-CFG-001:** The file MUST be named exactly `typrest.toml` and live at the project root.
- **REQ-CFG-002:** Invalid TOML, or a field with the wrong type, MUST be a configuration load error.
- **REQ-CFG-003:** Unknown top-level fields MUST be ignored.

### `schema_version`

- **REQ-CFG-010:** The only supported value MUST be `"v1"`.
- **REQ-CFG-011:** A missing, non-string, or unsupported value MUST fail project loading.

### `templates_root`

`templates_root` identifies the directory containing template folders.

- **REQ-CFG-020:** The value MUST be resolved relative to the directory containing `typrest.toml`.
- **REQ-CFG-021:** Absolute paths MUST be rejected.
- **REQ-CFG-022:** `.` and internal `..` components MUST be normalized.
- **REQ-CFG-023:** Paths whose normalization would leave the project root MUST be rejected.
- **REQ-CFG-024:** The resolved path MUST exist and be a directory.

```toml
# Valid
templates_root = "./templates"
```

```toml
# Invalid: absolute
templates_root = "/srv/templates"
```

```toml
# Invalid: escapes the project
templates_root = "../shared-templates"
```

### `web_ui`

- **REQ-CFG-030:** `web_ui` MUST default to `false`.
- **REQ-CFG-031:** `web_ui = true` MUST mount the embedded Web UI routes; `web_ui = false` MUST
  leave them unmounted. All other routes, including the metadata routes, are independent of the
  flag. Route behavior is defined in [HTTP API](http-api.md) (`REQ-HTTP-002`, `REQ-HTTP-064`).
- **REQ-CFG-032:** The Web UI has no authentication, so a server with `web_ui = true` SHOULD
  bind to loopback; see `REQ-HTTP-051`.

  ```bash
  typrest serve --hostname 127.0.0.1
  ```

### `[server]`

<a id="server"></a>

The `[server]` table sets render capacity, the maximum request body size, and the shutdown
drain deadline. The behavior it controls is defined in [HTTP API](http-api.md#render-capacity)
(`REQ-HTTP-070` to `REQ-HTTP-074`), [Request body](http-api.md#request-body)
(`REQ-HTTP-077`, `REQ-HTTP-078`), and [CLI](cli.md#shutdown) (`REQ-CLI-045` to
`REQ-CLI-048`). Ports and hostnames remain `serve` options.

- **REQ-CFG-050:** The `[server]` table MUST be optional. Omitting it, or any field in it, MUST use
  the defaults in the table above.
- **REQ-CFG-051:** `max_concurrent_renders` MUST be an integer from `1` to `256`. A value outside
  that range MUST be a configuration load error.
- **REQ-CFG-052:** `max_queued_renders` MUST be an integer from `0` to `1024`. A value outside that
  range MUST be a configuration load error. `0` MUST mean that a render is rejected whenever no
  slot is free.
- **REQ-CFG-053:** Unknown keys inside `[server]` MUST be ignored, consistent with `REQ-CFG-003`.
- **REQ-CFG-054:** The `[server]` settings MUST affect only `typrest serve` (request-time rendering
  and shutdown). `typrest check` MAY use the render limits to size its executor, but its result MUST
  NOT depend on them.
- **REQ-CFG-055:** `max_body_bytes` MUST be an integer from `1` to `1073741824` (1 GiB). A value
  outside that range MUST be a configuration load error.
- **REQ-CFG-056:** `shutdown_timeout_seconds` MUST be an integer from `1` to `3600`: the maximum time
  `serve` drains in-flight requests after `SIGTERM` or `SIGINT`. A value outside that range, or a
  non-integer, MUST be a configuration load error. Operators SHOULD keep it at or above any render
  timeout and below the container stop timeout (Docker `stop_grace_period`, Kubernetes
  `terminationGracePeriodSeconds`).

Typst also parallelizes inside one compilation, so `max_concurrent_renders` bounds concurrent
compilations, not total CPU threads. Container operators should set it explicitly when CPU quotas
are fractional or reported inaccurately, because the default follows the process's available
parallelism.

### Generated default

- **REQ-CFG-040:** `typrest init` MUST generate a configuration with the Web UI enabled:

  ```toml
  schema_version = "v1"
  templates_root = "templates"
  web_ui = true
  ```

## Examples

The snippets inside the requirement groups above are the examples. They agree with the
requirements they follow.

## Non-goals

This format does not define ports, hostnames, watch settings, package policy or cache paths, store
backends, authentication, per-template overrides, or environment-specific configuration. The only
server settings are the render capacity limits, the request body limit, and the shutdown
timeout in `[server]`. Package
support is automatic and always uses `<project>/.typrest/packages`. Update this specification before
adding any related fields.

## Related documents

- [Web UI specification](webui.md)
- [HTTP API](http-api.md#render-capacity) for the render capacity behavior.
- [Template layout](templates.md)
