# CLI Specification

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

- **CLI-001:** The CLI must provide the commands `init`, `check`, and `serve` and no others.
- **CLI-002:** Global options may appear before or after the subcommand.
- **CLI-003:** The process must exit successfully only when the selected command completes
  successfully.
- **CLI-004:** CLI errors must be emitted as structured tracing events, not ad hoc panic text.

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

- **CLI-010:** `check` and `serve` must walk upward from the current working directory and use the
  nearest ancestor containing `typrest.toml`. A missing file must fail the command.
- **CLI-011:** `templates_root` must be resolved relative to that project root. Path and validation
  rules are in the [`typrest.toml` specification](typrest-toml.md).
- **CLI-012:** Template discovery follows the [template layout](templates.md).

### `typrest init`

```text
typrest init [path]
```

- **CLI-020:** With no path, the current directory must be the target. A relative path must resolve
  from the current directory; an absolute path must be used directly.
- **CLI-021:** The command must create `typrest.toml` (see `CONFIG-040`) and a `templates/hello-world`
  template containing `main.typ` and `schema.yml`:

  ```text
  typrest.toml
  templates/
    hello-world/
      main.typ
      schema.yml
  ```

- **CLI-022:** The generated `main.typ` must be:

  ```typst
  #let data = json(bytes(sys.inputs.data))

  = Hello #data.name
  ```

- **CLI-023:** The generated `schema.yml` must be:

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

- **CLI-024:** The command must fail without overwriting when the target exists as a
  non-directory, when `typrest.toml` already exists in the target, or when `templates/` already
  exists in the target.

### `typrest check`

- **CLI-030:** After project discovery, `check` must list, load, prepare, warm-compile, and
  PDF-export every template using the same registry build as server startup.
- **CLI-031:** Failures must be collected and reported in deterministic template-ID order.
- **CLI-032:** Literal, fully versioned `@preview` imports must be downloaded when absent from the
  project-local `.typrest/packages` cache. Package resolution failures must make the check fail.
  See [Template layout](templates.md) (`TEMPLATE-060` to `TEMPLATE-064`).
- **CLI-033:** Warnings must be displayed and must return exit status `0`. Configuration, load,
  schema, compilation, or export failure must return exit status `1`.
- **CLI-034:** `check` must write timestamped lifecycle events to stdout: each template reports when
  compilation starts and completes, and a successful check reports `templates ready: <count>`.
  Missing packages report installation start and completion. Package cache hits use `DEBUG` and are
  hidden by the default filter. Warnings and errors also use stdout.

### `typrest serve`

| Option | Default | Behavior |
|---|---|---|
| `--port <port>` | `8080` | TCP port to bind |
| `--hostname <ipv4>` | `0.0.0.0` | IPv4 bind address; hostnames and IPv6 are not accepted |
| `--watch` | disabled | Reload changed templates while keeping last known good entries |

- **CLI-040:** `serve` must build and warm-compile the registry before binding its socket.
- **CLI-041:** `serve` must fail before listening when project configuration is missing or invalid,
  the templates root is invalid, or any startup template fails.
- **CLI-042:** Startup must use the same package, template-compilation, warning, error, and
  ready-count events as `check`, followed by an ordinary `server.starting` tracing event. Every
  process event uses stdout.
- **CLI-043:** With `--watch`, a successful full rebuild must log a new ready count, and a
  single-template reload must log that template's lifecycle and reload outcome.
  Watch behavior is defined in [Template lifecycle](../architecture/template-lifecycle.md).

### Diagnostic options

| Option | Default | Behavior |
|---|---|---|
| `--log-format pretty\|json` | `pretty` | Human diagnostics or JSON Lines |
| `--color auto\|always\|never` | `auto` | ANSI policy for pretty output |
| `RUST_LOG` | `typrest=info` | Tracing event filtering |

- **CLI-050:** JSON output must never contain ANSI codes.
- **CLI-051:** The CLI does not currently expose `--verbose`. Lifecycle output is classified
  centrally so verbose details can be enabled by that future option without changing package or
  compilation behavior.

See [Compiler diagnostics](diagnostics.md) and [Logging and process output](logging.md).

## Non-goals

Store commands, publishing, remote registries, and environment-variable configuration overrides
are not part of this specification.
