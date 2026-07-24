# CLI Specification

## Goal

The Papelon CLI is the local entrypoint for creating and running a filesystem-mode project.

In the MVP, the CLI has two responsibilities:

- bootstrap a project layout with `papelon init`
- start the HTTP server with `papelon serve`

This document defines the intended CLI contract going forward. If the implementation differs, this file is the source of truth for the next change.

---

## Command Summary

```text
papelon init
papelon serve [--port <port>] [--watch]
```

Notes:

- `papelon init` creates a starter hello-world template example.
- `papelon init` operates on the current working directory.
- `papelon serve` resolves the templates root from `papelon.toml` by default.

---

## Project Model

A Papelon project is a directory that contains:

- a `papelon.toml` file at the project root
- a templates root directory configured in `papelon.toml`

Default layout produced by `papelon init`:

```text
./papelon.toml
./templates/
./templates/hello-world/
```

The templates root is where template directories such as `hello-world/`, `invoice/`, and `welcome/` live.

Example:

```text
my-project/
  papelon.toml
  templates/
    invoice/
      main.typ
      schema.json
    _shared/
      brand.typ
```

---

## `papelon init`

### Purpose

Initialize the current directory as a Papelon project.

### Syntax

```text
papelon init
```

### Behavior

When executed, the command creates:

- `./papelon.toml`
- `./templates/`
- `./templates/hello-world/main.typ`
- `./templates/hello-world/schema.json`

Default `papelon.toml` content:

```toml
schema_version = "v1"
templates_root = "templates"
```

Default `templates/hello-world/schema.json` content:

```json
{
  "schemaVersion": "v1",
  "request": {
    "body": {
      "type": "object",
      "properties": {
        "name": {
          "type": "string",
          "default": "World",
        }
      }
    }
  }
}
```

Default `templates/hello-world/main.typ` content:

```typst
#let data = json(bytes(sys.inputs.data))

= Hello #data.name
```

### Rules

- The command uses the current working directory as the target project root.
- The command must create a starter hello-world template by default.
- The command must fail if `papelon.toml` already exists in the current directory.
- The command must fail if the configured default templates directory already exists as a non-directory path.
- The command may succeed if `templates/` already exists as a directory and is empty or already usable, but this behavior should be explicit in implementation and error messaging. If stricter behavior is preferred, the implementation should fail whenever either target already exists.

### Result

After success, the current directory is a valid Papelon project root.

---

## `papelon serve`

### Purpose

Start the HTTP server in filesystem mode.

### Syntax

```text
papelon serve [--port <port>] [--watch]
```

### Arguments

#### `--port <port>`

Optional TCP port.

Default:

```text
8080
```

#### `--watch`

Optional flag that enables template reload on filesystem changes.

### Resolution Rules

`papelon serve` must:

1. determine the project root
2. read `papelon.toml`
3. resolve `templates_root` relative to the project root
4. start the server using that resolved directory

Example:

```text
papelon serve
```

If run inside:

```text
/workspace/invoices
```

and `/workspace/invoices/papelon.toml` contains:

```toml
schema_version = "v1"
templates_root = "templates"
```

then the server must read templates from:

```text
/workspace/invoices/templates
```

### Failure Cases

`papelon serve` must fail with a clear user-facing error when:

- the project root does not contain `papelon.toml`
- `papelon.toml` is invalid
- `templates_root` does not exist
- `templates_root` is not a directory

---

## Non-Goals For This Spec

This document does not define:

- store mode commands
- template publishing workflows
- remote registries
- project discovery above the current directory
- environment-variable overrides

Those can be specified later if needed.
