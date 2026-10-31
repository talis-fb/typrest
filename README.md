# Typrest

Typrest is an HTTP service for server-side PDF generation with [Typst](https://typst.app/).
Applications in any language send JSON over HTTP; Typrest validates the payload when a schema is
present, compiles the selected template, and returns PDF bytes.

The current release supports filesystem mode. Templates are loaded from a project directory,
prepared and warm-compiled before startup completes, and retained in memory for rendering. Store
backends, template publishing, tags, and authentication are not implemented.

## Quickstart from Source

Prerequisites: a Rust toolchain compatible with edition 2024 and `curl`.

Build Typrest and create a temporary project:

```bash
cargo build
TYPREST_BIN="$PWD/target/debug/typrest"
"$TYPREST_BIN" init /tmp/typrest-demo
cd /tmp/typrest-demo
"$TYPREST_BIN" check
```

Start the server on the loopback interface:

```bash
"$TYPREST_BIN" serve --hostname 127.0.0.1
```

In another terminal, render the generated template:

```bash
curl -X POST \
  http://127.0.0.1:8080/hello-world \
  -H 'Content-Type: application/json' \
  --data '{"name":"Ada"}' \
  --output hello-world.pdf
```

Check the server and inspect prepared templates:

```bash
curl http://127.0.0.1:8080/_healthz
curl http://127.0.0.1:8080/_api/templates
```

## Quickstart with Docker

Prerequisites: Docker (or Podman) and `curl`. The image builds Typrest from source and serves the
bundled [`examples/minimal`](examples/minimal/README.md) project from `/project`:

```bash
docker build -t typrest .
docker run --rm -p 127.0.0.1:8080:8080 typrest
```

In another terminal, render the `hello` template:

```bash
curl -X POST \
  http://127.0.0.1:8080/hello \
  -H 'Content-Type: application/json' \
  --data @examples/minimal/templates/hello/payload.json \
  --output hello.pdf
```

To serve your own project, mount the directory containing `typrest.toml` over `/project`:

```bash
docker run --rm -p 127.0.0.1:8080:8080 -v "$PWD:/project" typrest
```

The mount must be writable: Typrest downloads `@preview` packages into `/project/.typrest`. The
container runs as uid/gid 1000; add `--user "$(id -u):$(id -g)"` if your files belong to another
user.

`docker compose up` serves [`examples/invoice`](examples/invoice/README.md) with `--watch`, so
template edits on the host reload without restarting the container. Both commands publish the port
on the loopback interface only, because the development Web UI is unauthenticated.

## Project and Template Layout

`typrest init` creates a project containing `typrest.toml` and a templates root:

```text
my-project/
  typrest.toml
  templates/
    financial/
      invoice/
        main.typ
        schema.yml
        assets/
          logo.svg
    welcome/
      main.typ
    _shared/
      brand.typ
```

Every template requires `main.typ`. A `schema.yml` file is optional. Legacy `schema.json` remains
accepted, but a template cannot contain both formats. Without a schema, any valid JSON value is
passed directly to Typst without payload validation. Files under `_shared/` are available to every
template. Directories without `main.typ` group route segments; the first directory containing
`main.typ` defines the template ID and stops route discovery below it. Directories beginning with
`_` are skipped during discovery but remain ordinary source when they are below an active template.

Templates read the request payload from `sys.inputs.data`:

```typst
#let data = json(bytes(sys.inputs.data))

= Welcome, #data.name
```

Templates may also import literal, fully versioned packages from Typst Universe:

```typst
#import "@preview/cetz:0.5.2"
```

`check`, startup, and watch preparation download missing `@preview` packages into the project-local
`.typrest/packages` cache. Prepared package files remain in memory, so render requests do not read
the cache or access the network. Other namespaces and dynamic package import expressions are not
yet supported; generated `.typrest/` content should not be committed.

See the [template layout](docs/specs/templates.md), [`schema.yml` v1](docs/specs/schema-v1.md), and
[`typrest.toml`](docs/specs/typrest-toml.md) specifications.

## CLI

```text
typrest init [path]
typrest check
typrest serve [--hostname <ipv4>] [--port <port>] [--watch]
```

`check` and `serve` search the current directory and its ancestors for the nearest
`typrest.toml`. Both load, prepare, compile, and PDF-export every template; `serve` binds its socket
only after this startup validation succeeds.

Global diagnostic options are available before or after a subcommand:

```text
--log-format <pretty|json>
--color <auto|always|never>
```

See the [CLI specification](docs/specs/cli.md) for exact behavior.

## HTTP API

| Method | Route | Purpose |
|---|---|---|
| `POST`, `GET` | `/{*template_id}` | Render a template and return `application/pdf` |
| `GET` | `/_healthz` | Health check, `{"status":"ok"}` |
| `GET` | `/_api/templates` | List prepared templates |
| `GET` | `/_api/templates/{*template_id}` | Template ID and parsed schema |

`POST` requires `Content-Type: application/json`. Request bodies are limited to
`[server].max_body_bytes` (default 2 MiB); larger bodies return `413 payload_too_large`. Errors use a
structured JSON envelope:

```json
{
  "error": "schema_validation_failed",
  "message": "Payload validation failed",
  "violations": [
    { "field": "$.name", "issue": "required field missing" }
  ]
}
```

See the [HTTP API specification](docs/specs/http-api.md) for the complete contract, status codes,
and security notes.

## Development Web UI

The optional Web UI can inspect prepared templates, edit test payloads, preview PDFs, and download
the latest render. Enable its static routes in `typrest.toml`:

```toml
schema_version = "v1"
templates_root = "templates"
web_ui = true
```

Run on loopback and open `http://127.0.0.1:8080/_webui`. The UI has no authentication and must not
be exposed directly to an untrusted network. See the [Web UI guide](docs/development/webui.md) and
[Web UI specification](docs/specs/webui.md).

## Development

The standard Rust validation pass is:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Start with the [documentation index](docs/README.md), [architecture overview](docs/architecture/overview.md),
and [testing guide](docs/development/testing.md). See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the contribution workflow. AI coding agents should also read [`AGENTS.md`](AGENTS.md).

Copyright 2026 Talison Fabio.

Licensed under the [Apache License, Version 2.0](LICENSE).
