# Papelon

Papelon is an HTTP server for server-side PDF generation with Typst. It is built for applications that need reliable PDF output without embedding a PDF library or browser automation runtime into the application itself.

Applications integrate with Papelon through HTTP. A service in any language sends JSON to a template endpoint, and Papelon validates the payload, compiles the matching Typst template, and returns PDF bytes.

The current MVP runs in filesystem mode. Templates live in a directory mounted into the server, and each template contains a required `main.typ` file. An optional `schema.json` contract can validate request payloads.

## Quickstart

Start the server with Docker Compose:

```bash
docker compose up --build
```

Render the invoice example:

```bash
curl -X POST \
  http://localhost:8080/render/invoice \
  -H 'Content-Type: application/json' \
  --data-binary @examples/templates/invoice/payload.json \
  --output invoice.pdf
```

List available templates:

```bash
curl http://localhost:8080/templates
```

Run without Docker:

```bash
cargo run -- serve --port 8080 --watch
```

Validate every template without starting the server:

```bash
cargo run -- check
```

## Template Structure

```text
templates/
  invoice/
    main.typ
    schema.json
    assets/
  welcome/
    main.typ
    schema.json
  _shared/
    brand.typ
```

Each template directory must contain `main.typ`; `schema.json` is optional. If it is absent, Papelon skips payload validation and injects the request JSON directly through `sys.inputs.data`. Files under `_shared/` are available to every template.

## Complete Typst Example

```typst
#let data = json(bytes(sys.inputs.data))

= Welcome, #data.name

#data.message
```

The JSON request body is injected into Typst through `sys.inputs.data`.

## schema.json Reference

Papelon supports schema version `v1`. The root object must contain `schemaVersion` and `request.body`.

```json
{
  "schemaVersion": "v1",
  "request": {
    "body": {
      "type": "object",
      "properties": {
        "name": {
          "type": "string",
          "required": true
        }
      }
    }
  }
}
```

Supported field types are `string`, `number`, `boolean`, `object`, `array`, and `null`. Object schemas use `properties`, array schemas use `items`, fields are optional by default, and optional fields can define `default`.

See `docs/schema-v1.md` for the full schema specification.

## HTTP API

### `GET /healthz`

Returns:

```json
{ "status": "ok" }
```

### `GET /templates`

Returns:

```json
{
  "templates": [
    { "name": "invoice" },
    { "name": "welcome" }
  ]
}
```

### `POST /render/:template`

Request:

```bash
curl -X POST \
  http://localhost:8080/render/welcome \
  -H 'Content-Type: application/json' \
  --data-binary @examples/templates/welcome/payload.json \
  --output welcome.pdf
```

Success response:

```text
200 OK
Content-Type: application/pdf
```

Error responses use JSON:

```json
{
  "error": "schema_validation_failed",
  "message": "Payload validation failed",
  "violations": [
    { "field": "$.name", "issue": "required field missing" }
  ]
}
```

Status codes:

- `404`: template not found
- `422`: payload does not match `schema.json` (when present)
- `500`: Typst compilation or PDF export failed

## Compiler Diagnostics

`papelon check`, server startup, watch reloads, and request-time compilation share the same
diagnostic pipeline. Terminal output includes annotated source ranges, all Typst errors and
warnings, hints, and call/import traces.

Use `--log-format json` for JSON Lines logs and `--color auto|always|never` to control terminal
color. Compilation failures returned over HTTP contain structured paths, ranges, hints, and traces,
but never template source excerpts or absolute filesystem paths.

See [docs/diagnostics.md](docs/diagnostics.md) for the output contract and Typst upgrade procedure.

## Development Web UI

Papelon includes an optional browser interface for inspecting prepared templates, editing test
payloads, previewing PDFs, and downloading the latest render. It is disabled by default.

Enable it in `papelon.toml`:

```toml
schema_version = "v1"
templates_root = "templates"
web_ui = true
```

Then run Papelon on the loopback interface and open `http://127.0.0.1:8080/webui`:

```bash
cargo run -- serve --hostname 127.0.0.1
```

The Web UI is a development tool and has no authentication. Do not expose it to an untrusted
network. Its metadata API reads only from the prepared in-memory template registry, and all PDF
generation still uses `POST /render/:template`.

See the [Web UI guide](docs/webui-development.md) for usage and contributor workflows and the
[Web UI specification](docs/webui-spec.md) for the complete behavior and internal route contracts.

Frontend sources and the committed production build live under `webui/`. After changing frontend
source, rebuild and verify it before committing:

```bash
cd webui
npm ci
npm run check
npm test
npm run build
```
