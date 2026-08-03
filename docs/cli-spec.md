# CLI Specification

## Goal

The Typrest CLI is the local entrypoint for creating and running a filesystem-mode project.

In the MVP, the CLI has two responsibilities:

- bootstrap a project layout with `typrest init`
- validate every template with `typrest check`
- start the HTTP server with `typrest serve`

This document defines the intended CLI contract going forward. If the implementation differs, this file is the source of truth for the next change.

---

## Command Summary

```text
typrest init [path]
typrest check [--log-format <pretty|json>] [--color <auto|always|never>]
typrest serve [--port <port>] [--watch]
```

Notes:

- `typrest init` creates a starter hello-world template example.
- `typrest init` operates on the current working directory when no path is provided.
- `typrest init path` initializes the provided target directory, relative or absolute.
- `typrest check` uses the same project discovery and warmup compilation as `typrest serve`.
- `typrest serve` resolves the nearest ancestor `typrest.toml` from the current working directory.

---

## Project Model

A Typrest project is a directory that contains:

- a `typrest.toml` file at the project root
- a templates root directory configured in `typrest.toml`

Default layout produced by `typrest init`:

```text
./typrest.toml
./templates/
./templates/hello-world/
```

The templates root is where template directories such as `hello-world/`, `invoice/`, and `welcome/` live.

Example:

```text
my-project/
  typrest.toml
  templates/
    invoice/
      main.typ
      schema.yml
    _shared/
      brand.typ
```

---

## `typrest init`

### Purpose

Initialize the current directory or a target directory as a Typrest project.

### Syntax

```text
typrest init [path]
```

### Behavior

When executed, the command creates:

- `typrest.toml` in the chosen target root
- `templates/` in the chosen target root
- `templates/hello-world/main.typ`
- `templates/hello-world/schema.yml`

Default `typrest.toml` content:

```toml
schema_version = "v1"
templates_root = "templates"
```

Default `templates/hello-world/schema.yml` content:

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

Default `templates/hello-world/main.typ` content:

```typst
#let data = json(bytes(sys.inputs.data))

= Hello #data.name
```

### Rules

- The command uses the current working directory as the target project root when no path is provided.
- When a path is provided, the command uses that path as the target project root. Relative paths resolve from the current working directory, and absolute paths are used as-is.
- The command must create a starter hello-world template by default.
- The command must fail if `typrest.toml` already exists in the target root.
- The command must fail if `templates/` already exists in the target root.
- The command must fail if the target root already exists as a non-directory path.

### Result

After success, the chosen target root is a valid Typrest project root.

---

## `typrest serve`

### Purpose

Start the HTTP server in filesystem mode.

### Syntax

```text
typrest serve [--port <port>] [--hostname <ipv4>] [--watch]
```

### Arguments

#### `--port <port>`

Optional TCP port.

Default:

```text
8080
```

#### `--hostname <ipv4>`

Optional IPv4 address to bind to.

Default:

```text
0.0.0.0
```

#### `--watch`

Optional flag that enables template reload on filesystem changes.

### Resolution Rules

`typrest serve` must:

1. determine the nearest project root by walking upward from the current working directory
2. read `typrest.toml`
3. resolve `templates_root` relative to the project root
4. start the server using that resolved directory

Example:

```text
typrest serve
```

If run inside:

```text
/workspace/invoices
```

and `/workspace/invoices/typrest.toml` contains:

```toml
schema_version = "v1"
templates_root = "templates"
```

then the server must read templates from:

```text
/workspace/invoices/templates
```

### Failure Cases

`typrest serve` must fail with a clear user-facing error when:

- the project root does not contain `typrest.toml`
- `typrest.toml` is invalid
- `templates_root` does not exist
- `templates_root` is not a directory

---

## `typrest check`

### Purpose

Validate all templates without starting the HTTP server.

### Behavior

`typrest check` resolves the nearest project exactly like `typrest serve`, then loads, prepares,
warm-compiles, and PDF-exports every template. It reports all template failures in deterministic
name order rather than stopping at the first failure.

Warnings are displayed but return exit status `0`. Configuration, load, schema, compilation, and
export failures return exit status `1`.

### Global Output Options

- `--log-format pretty|json`: annotated human output by default, or JSON Lines for automation.
- `--color auto|always|never`: controls pretty output; JSON output never contains ANSI codes.
- `RUST_LOG`: controls operational log verbosity.

---

## Non-Goals For This Spec

This document does not define:

- store mode commands
- template publishing workflows
- remote registries
- environment-variable overrides

Those can be specified later if needed.
