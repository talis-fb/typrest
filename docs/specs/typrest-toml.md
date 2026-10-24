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
```

| Field | Type | Required | Default |
|---|---|---|---|
| `schema_version` | string | yes | none |
| `templates_root` | string | yes | none |
| `web_ui` | boolean | no | `false` |

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
backends, authentication, per-template overrides, or environment-specific configuration. Package
support is automatic and always uses `<project>/.typrest/packages`. Update this specification before
adding any related fields.

## Related documents

- [Web UI specification](webui.md)
- [Template layout](templates.md)
