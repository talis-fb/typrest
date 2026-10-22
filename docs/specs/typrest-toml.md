# `typrest.toml` Specification

- **Status:** Current. The parser lives in `crates/typrest/src/config.rs`.
- **Scope:** The project configuration file of a filesystem-mode Typrest project.

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
```

| Field | Type | Required | Default |
|---|---|---|---|
| `schema_version` | string | yes | none |
| `templates_root` | string | yes | none |
| `web_ui` | boolean | no | `false` |

- **CONFIG-001:** The file must be named exactly `typrest.toml` and live at the project root.
- **CONFIG-002:** Invalid TOML, or a field with the wrong type, must be a configuration load error.
- **CONFIG-003:** Unknown top-level fields must be ignored.

### `schema_version`

- **CONFIG-010:** The only supported value must be `"v1"`.
- **CONFIG-011:** A missing, non-string, or unsupported value must fail project loading.

### `templates_root`

`templates_root` identifies the directory containing template folders.

- **CONFIG-020:** The value must be resolved relative to the directory containing `typrest.toml`.
- **CONFIG-021:** Absolute paths must be rejected.
- **CONFIG-022:** `.` and internal `..` components must be normalized.
- **CONFIG-023:** Paths whose normalization would leave the project root must be rejected.
- **CONFIG-024:** The resolved path must exist and be a directory.

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

- **CONFIG-030:** `web_ui` must default to `false`.
- **CONFIG-031:** `web_ui = true` must mount the embedded Web UI routes; `web_ui = false` must
  leave them unmounted. All other routes, including the metadata routes, are independent of the
  flag. Route behavior is defined in [HTTP API](http-api.md) (`HTTP-002`, `HTTP-064`).
- **CONFIG-032:** The Web UI has no authentication. Bind to loopback when enabling it; see
  `HTTP-051`.

  ```bash
  typrest serve --hostname 127.0.0.1
  ```

### Generated default

- **CONFIG-040:** `typrest init` must generate a configuration with the Web UI enabled:

  ```toml
  schema_version = "v1"
  templates_root = "templates"
  web_ui = true
  ```

## Non-goals

This format does not define ports, hostnames, watch settings, package policy or cache paths, store
backends, authentication, per-template overrides, or environment-specific configuration. Package
support is automatic and always uses `<project>/.typrest/packages`. Update this specification before
adding any related fields.

## Related documents

- [Web UI specification](webui.md)
- [Template layout](templates.md)
