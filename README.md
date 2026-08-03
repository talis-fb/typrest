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
  http://127.0.0.1:8080/render/hello-world \
  -H 'Content-Type: application/json' \
  --data '{"name":"Ada"}' \
  --output hello-world.pdf
```

Check the server and inspect prepared templates:

```bash
curl http://127.0.0.1:8080/_healthz
curl http://127.0.0.1:8080/_api/templates
```

The repository's `Dockerfile` and `docker-compose.yml` still use an obsolete positional `serve`
syntax and do not provide the required `typrest.toml`. Docker quickstart support is tracked as open
work in [`ROADMAP.md`](ROADMAP.md).

## Project and Template Layout

`typrest init` creates a project containing `typrest.toml` and a templates root:

```text
my-project/
  typrest.toml
  templates/
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
template.

Templates read the request payload from `sys.inputs.data`:

```typst
#let data = json(bytes(sys.inputs.data))

= Welcome, #data.name
```

See the complete [`schema.yml` v1 specification](docs/schema-v1.md) and the
[`typrest.toml` specification](docs/typrest-toml-spec.md).

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

See the [CLI specification](docs/cli-spec.md) for exact behavior.

## HTTP API

### `GET /_healthz`

```json
{ "status": "ok" }
```

### `GET /_api/templates`

Returns prepared templates in name order:

```json
{
  "templates": [
    { "name": "invoice", "hasSchema": true },
    { "name": "welcome", "hasSchema": false }
  ]
}
```

### `GET /_api/templates/:template`

Returns the prepared template name and its parsed schema, or `"schema": null` when no schema is
present. These metadata endpoints currently remain mounted when the Web UI is disabled.

### `POST /render/:template`

Send a JSON request body with `Content-Type: application/json`. A successful response is
`application/pdf` with status `200`.

Errors use a structured JSON envelope:

```json
{
  "error": "schema_validation_failed",
  "message": "Payload validation failed",
  "violations": [
    { "field": "$.name", "issue": "required field missing" }
  ]
}
```

Common statuses are `400` for an invalid template name or JSON request, `404` for a missing
template, `422` for schema validation failures, and `500` for compilation or PDF export failures.

## Development Web UI

The optional Web UI can inspect prepared templates, edit test payloads, preview PDFs, and download
the latest render. Enable its static routes in `typrest.toml`:

```toml
schema_version = "v1"
templates_root = "templates"
web_ui = true
```

Run on loopback and open `http://127.0.0.1:8080/webui`. The UI has no authentication and must not
be exposed directly to an untrusted network. See the [Web UI guide](docs/webui-development.md) and
[Web UI specification](docs/webui-spec.md).

## Development

The standard Rust validation pass is:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Start with the [documentation index](docs/README.md), [architecture guide](docs/architecture.md),
and [testing guide](docs/testing.md). AI coding agents should also read [`AGENTS.md`](AGENTS.md).

See the repository [`LICENSE`](LICENSE) file for licensing terms.
