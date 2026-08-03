# CLI Specification

> Status: Current. The implementation lives in `crates/typrest-cli`.

## Purpose

The Typrest CLI manages filesystem-mode projects. It has three responsibilities:

- create a project with `typrest init`;
- validate all templates with `typrest check`;
- start the HTTP server with `typrest serve`.

## Command Summary

```text
typrest [--log-format <pretty|json>] [--color <auto|always|never>] <command>

typrest init [path]
typrest check
typrest serve [--port <port>] [--hostname <ipv4>] [--watch]
```

Global options may appear before or after the subcommand.

## Project Model and Discovery

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

`check` and `serve` walk upward from the current working directory and use the nearest ancestor
containing `typrest.toml`. They resolve `templates_root` relative to that project root. See the
[`typrest.toml` specification](typrest-toml-spec.md) for path and validation rules.

Template discovery walks directories recursively. A non-root directory containing `main.typ`
defines the slash-separated template ID and stops discovery beneath it. Directories beginning with
`_` are skipped during discovery; invalid visible route segments and `templates/main.typ` fail the
project check. Descendant files remain part of a selected template bundle.

## `typrest init`

### Syntax

```text
typrest init [path]
```

With no path, the current directory is the target. A relative path resolves from the current
directory; an absolute path is used directly.

The command creates:

```text
typrest.toml
templates/
  hello-world/
    main.typ
    schema.yml
```

Generated `typrest.toml`:

```toml
schema_version = "v1"
templates_root = "templates"
```

Generated template:

```typst
#let data = json(bytes(sys.inputs.data))

= Hello #data.name
```

Generated schema:

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

The command fails without overwriting when:

- the target exists as a non-directory;
- `typrest.toml` already exists in the target;
- `templates/` already exists in the target.

## `typrest check`

```text
typrest check
```

After project discovery, `check` lists, loads, prepares, warm-compiles, and PDF-exports every
template using the same registry build as server startup. Failures are collected and reported in
deterministic template-ID order.

Literal, fully versioned `@preview` imports are downloaded when absent from the project-local
`.typrest/packages` cache. Package resolution failures make the check fail. Generated `.typrest/`
content should not be committed.

Warnings are displayed and return exit status `0`. Configuration, load, schema, compilation, or
export failure returns exit status `1`.

## `typrest serve`

```text
typrest serve [--port <port>] [--hostname <ipv4>] [--watch]
```

| Option | Default | Behavior |
|---|---|---|
| `--port <port>` | `8080` | TCP port to bind |
| `--hostname <ipv4>` | `0.0.0.0` | IPv4 bind address; hostnames and IPv6 are not accepted |
| `--watch` | disabled | Reload changed templates while keeping last known good entries |

`serve` builds and warm-compiles the registry before binding its socket. It fails before listening
when project configuration is missing or invalid, the templates root is invalid, or any startup
template fails.

Watch behavior is defined in [Startup Template Compilation](startup-template-compilation.md).

## Diagnostic Options

| Option | Default | Behavior |
|---|---|---|
| `--log-format pretty\|json` | `pretty` | Human diagnostics or JSON Lines |
| `--color auto\|always\|never` | `auto` | ANSI policy for pretty output |
| `RUST_LOG` | runtime default | Operational log filtering |

JSON output never contains ANSI codes. See [Compiler Diagnostics and Logging](diagnostics.md).

## Failure Contract

CLI errors are written through the diagnostic reporter rather than returned as ad hoc panic text.
The process exits successfully only when the selected command completes successfully.

Store commands, publishing, remote registries, and environment-variable configuration overrides
are not part of this specification.
