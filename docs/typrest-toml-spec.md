# `typrest.toml` Specification

> Status: Current. The parser lives in `crates/typrest-cli/src/config.rs`.

## Purpose and Location

`typrest.toml` configures a filesystem-mode Typrest project. It must be named exactly
`typrest.toml` and live at the project root discovered by `typrest check` and `typrest serve`.

```text
project/
  typrest.toml
  templates/
```

## Format

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

Unknown top-level fields are ignored. Invalid TOML or a field with the wrong type is a configuration
load error.

## `schema_version`

The only supported value is `v1`:

```toml
schema_version = "v1"
```

A missing, non-string, or unsupported value fails project loading.

## `templates_root`

`templates_root` identifies the directory containing template folders:

```toml
templates_root = "templates"
```

Resolution rules:

- resolve the value relative to the directory containing `typrest.toml`;
- reject absolute paths;
- normalize `.` and internal `..` components;
- reject paths whose normalization would leave the project root;
- require the resolved path to exist and be a directory.

Examples:

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

## `web_ui`

`web_ui` controls the embedded development UI and defaults to false:

```toml
web_ui = true
```

When true, the server additionally mounts `/webui`, `/webui/`, and `/webui/*`. The current
`/_api/templates` and `/_api/templates/:template` metadata routes remain mounted whether this flag
is true or false. `POST /render/:template` and `GET /_healthz` are also independent of the flag.

The Web UI has no authentication. Bind to loopback when enabling it:

```bash
typrest serve --hostname 127.0.0.1
```

See the [Web UI Specification](webui-spec.md) for its complete route and security contract.

## Generated Default

`typrest init` omits `web_ui`, so generated projects contain:

```toml
schema_version = "v1"
templates_root = "templates"
```

## Non-goals

This format does not define ports, hostnames, watch settings, store backends, authentication,
per-template overrides, or environment-specific configuration. Update this specification before
adding any of those fields.
