# CLI specification

- **Status:** Current
- **Scope:** The `typrest` command line: project discovery, `init`, `check`, `serve`, and global
  options, implemented in `crates/typrest/src/commands/`.

## Purpose

The Typrest CLI manages filesystem-mode projects. It creates a project with `typrest init`, validates
all templates with `typrest check`, and starts the HTTP server with `typrest serve`.

## Requirements

### Command summary

```text
typrest [--log-format <pretty|json>] [--color <auto|always|never>] <command>

typrest init [path]
typrest check
typrest serve [--port <port>] [--hostname <ipv4>] [--watch]
```

- **REQ-CLI-001:** The CLI MUST provide the commands `init`, `check`, and `serve` and no others.
- **REQ-CLI-002:** Global options MAY appear before or after the subcommand.
- **REQ-CLI-003:** The process MUST exit successfully only when the selected command completes
  successfully.
- **REQ-CLI-004:** CLI errors MUST be emitted as structured tracing events, not ad hoc panic text.

### Project model and discovery

A Typrest project contains a `typrest.toml` at its root and a templates directory selected by
`templates_root`:

```text
project/
  typrest.toml
  .typrest/                 # generated package cache, when needed
    packages/
  templates/
    financial/
      invoice/
        main.typ
        schema.yml
    _shared/
      brand.typ
```

- **REQ-CLI-010:** `check` and `serve` MUST walk upward from the current working directory and use the
  nearest ancestor containing `typrest.toml`. A missing file MUST fail the command.
- **REQ-CLI-011:** `templates_root` MUST be resolved relative to that project root. Path and validation
  rules are in the [`typrest.toml` specification](typrest-toml.md).
- **REQ-CLI-012:** Template discovery MUST follow the [template layout](templates.md).

### `typrest init`

```text
typrest init [path]
```

- **REQ-CLI-020:** With no path, the current directory MUST be the target. A relative path MUST resolve
  from the current directory; an absolute path MUST be used directly.
- **REQ-CLI-021:** The command MUST create `typrest.toml` (see `REQ-CFG-040`) and a `templates/hello-world`
  template containing `main.typ` and `schema.yml`:

  ```text
  typrest.toml
  templates/
    hello-world/
      main.typ
      schema.yml
  ```

- **REQ-CLI-022:** The generated `main.typ` MUST be:

  ```typst
  #let data = json(bytes(sys.inputs.data))

  = Hello #data.name
  ```

- **REQ-CLI-023:** The generated `schema.yml` MUST be:

  ```yaml
  schemaVersion: v1
  request:
    body:
      type: object
      properties:
        name:
          type: string
          default: World
  ```

- **REQ-CLI-024:** The command MUST fail without overwriting when the target exists as a
  non-directory, when `typrest.toml` already exists in the target, or when `templates/` already
  exists in the target.

### `typrest check`

- **REQ-CLI-030:** After project discovery, `check` MUST list, load, prepare, warm-compile, and
  PDF-export every template using the same registry build as server startup.
- **REQ-CLI-031:** Failures MUST be collected and reported in deterministic template-ID order.
- **REQ-CLI-032:** Literal, fully versioned `@preview` imports MUST be downloaded when absent from the
  project-local `.typrest/packages` cache. Package resolution failures MUST make the check fail.
  See [Template layout](templates.md) (`REQ-TPL-060` to `REQ-TPL-064`).
- **REQ-CLI-033:** Warnings MUST be displayed and MUST return exit status `0`. Configuration, load,
  schema, compilation, or export failure MUST return exit status `1`.
- **REQ-CLI-034:** `check` MUST write timestamped lifecycle events to stdout: each template reports when
  compilation starts and completes, and a successful check reports `templates ready: <count>`.
  Missing packages report installation start and completion. Package cache hits use `DEBUG` and are
  hidden by the default filter. Warnings and errors also use stdout.

### `typrest serve`

| Option | Default | Behavior |
|---|---|---|
| `--port <port>` | `8080` | TCP port to bind |
| `--hostname <ipv4>` | `0.0.0.0` | IPv4 bind address; hostnames and IPv6 are not accepted |
| `--watch` | disabled | Reload changed templates while keeping last known good entries |

- **REQ-CLI-040:** `serve` MUST build and warm-compile the registry before binding its socket.
- **REQ-CLI-041:** `serve` MUST fail before listening when project configuration is missing or invalid,
  the templates root is invalid, or any startup template fails.
- **REQ-CLI-042:** Startup MUST use the same package, template-compilation, warning, error, and
  ready-count events as `check`, followed by an ordinary `server.starting` tracing event. Every
  process event MUST use stdout.
- **REQ-CLI-043:** With `--watch`, a successful full rebuild MUST log a new ready count, and a
  single-template reload MUST log that template's lifecycle and reload outcome.
  Watch behavior is defined in [Template lifecycle](../architecture/template-lifecycle.md).

- **REQ-CLI-044:** `serve` MUST report the effective `max_concurrent_renders` and
  `max_queued_renders` as structured fields of the `server.starting` event.

### Diagnostic options

| Option | Default | Behavior |
|---|---|---|
| `--log-format pretty\|json` | `pretty` | Human diagnostics or JSON Lines |
| `--color auto\|always\|never` | `auto` | ANSI policy for pretty output |
| `RUST_LOG` | `typrest=info` | Tracing event filtering |

- **REQ-CLI-050:** JSON output MUST NOT contain ANSI codes.
- **REQ-CLI-051:** The CLI MUST NOT expose a `--verbose` option. Lifecycle output is classified
  centrally so a future option can enable verbose details without changing package or
  compilation behavior.

See [Compiler diagnostics](diagnostics.md) and [Logging and process output](logging.md).

## Examples

None. The generated project in REQ-CLI-021 to REQ-CLI-023 is the canonical example.

## Non-goals

Store commands, publishing, remote registries, and environment-variable configuration overrides
are not part of this specification.

## Related documents

- [`typrest.toml` specification](typrest-toml.md)
- [Template layout](templates.md)
- [Compiler diagnostics](diagnostics.md)
- [Logging and process output](logging.md)
- [Template lifecycle](../architecture/template-lifecycle.md)
