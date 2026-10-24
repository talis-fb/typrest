# HTTP API Specification

- **Status:** Current
- **Scope:** Routes, request and response contracts, and the error envelope served by
  `crates/typrest/src/http/`. Static Web UI behavior is in the [Web UI specification](webui.md).

## Purpose

Clients render PDFs with plain HTTP. Handlers read only the prepared registry; see the
[architecture overview](../architecture/overview.md).

## Requirements

### Routes

- **HTTP-001:** The server must mount `GET /_healthz`, `GET /_api/templates`,
  `GET /_api/templates/{*template_id}`, `GET /{*template_id}`, and `POST /{*template_id}`
  regardless of configuration.
- **HTTP-002:** The server must mount `GET /`, `GET /_webui`, `GET /_webui/`, and `GET /_webui/*`
  as the Web UI routes defined in [`WEBUI-ROUTES`](#web-ui-routes) only when `web_ui = true`.
- **HTTP-003:** The server must not expose `/healthz`, `/templates`, `/render/*`, or `/webui/*`
  aliases.
- **HTTP-004:** Request paths whose first segment begins with `_` are reserved for Typrest and must
  not identify a template.

### Health

- **HTTP-010:** `GET /_healthz` must return status `200` with the JSON body `{"status":"ok"}`.

### Rendering

- **HTTP-020:** `GET` and `POST /{*template_id}` must render the template with the ID in the path
  and return status `200` with `Content-Type: application/pdf` and the PDF bytes.
- **HTTP-021:** `POST` must be sent with `Content-Type: application/json` (parameters such as
  `charset` are allowed); otherwise it must fail with `400` and `invalid_json`, even when the body
  is empty. The parsed body value, including a primitive or an array, is the render payload.
- **HTTP-022:** An empty body, including every `GET`, must use `{}` as the payload. Schema
  validation then applies as for any payload, so a schema with required properties returns
  `schema_validation_failed`. A non-empty body that is not valid JSON must fail with `400` and
  `invalid_json`.
- **HTTP-023:** The handler must parse the path as a template ID before touching the registry.
- **HTTP-024:** When the template has a schema, the server must validate the payload and apply
  defaults before compilation. See [Schema v1](schema-v1.md).
- **HTTP-025:** Rendering must read only the prepared registry. It must not read template or
  package files or access the network.

### Metadata

- **HTTP-030:** `GET /_api/templates` must return `{"templates":[{"id":"…","hasSchema":true}]}`
  with entries in template-ID order, and `{"templates":[]}` for an empty registry.
- **HTTP-031:** `GET /_api/templates/{*template_id}` must return `{"id":"…","schema":{…}}` with the
  canonical parsed schema, or `"schema":null` for a schema-less template.
- **HTTP-032:** Metadata handlers must read only the prepared registry.
- **HTTP-033:** An invalid template ID must return `400` with `invalid_template_id`; a valid ID
  with no template must return `404` with `template_not_found`.
- **HTTP-034:** The metadata routes are internal to the bundled Web UI and unversioned. Integrations
  must not assume they are stable.

### Errors

- **HTTP-040:** Every error response must be a JSON object with `error` (stable code) and
  `message`.
- **HTTP-041:** The following codes and statuses must be used:

  | Condition | Status | `error` |
  |---|---|---|
  | Invalid template ID | `400` | `invalid_template_id` |
  | Request body is not valid JSON | `400` | `invalid_json` |
  | No prepared template with that ID | `404` | `template_not_found` |
  | Payload violates the schema | `422` | `schema_validation_failed` |
  | Request-time compilation or PDF export failure | `500` | `compilation_failed` |

- **HTTP-042:** `schema_validation_failed` must include `violations`, a list of
  `{"field":"$.path","issue":"…"}` entries.
- **HTTP-043:** `compilation_failed` must include `diagnostics` and may include `line`, `hints`, and
  `trace`.
- **HTTP-044:** No error response may contain source text or physical filesystem paths. Diagnostic
  locations use virtual paths.

### Security

- **HTTP-050:** The server must not authenticate or authorize any route.
- **HTTP-051:** Deployments that need confidentiality must bind to loopback or place the server
  behind a trusted network boundary. The metadata routes are mounted even when `web_ui = false`, so
  disabling the UI does not hide template IDs or schemas.

### Web UI routes

<a id="web-ui-routes"></a>

- **HTTP-060:** `GET /` and `GET /_webui` must redirect permanently to `/_webui/`.
- **HTTP-061:** `GET /_webui/` must return the embedded `index.html` with `Cache-Control: no-cache`.
- **HTTP-062:** `GET /_webui/*` must return the exact embedded asset or `404`; there is no
  single-page-application fallback.
- **HTTP-063:** Fingerprinted assets must use `Cache-Control: public, max-age=31536000, immutable`,
  and text assets must declare UTF-8.
- **HTTP-064:** With `web_ui = false`, `/_webui`, `/_webui/`, and `/_webui/*` must return `404`.

## Examples

Validation failure:

```json
{
  "error": "schema_validation_failed",
  "message": "Payload validation failed",
  "violations": [
    { "field": "$.name", "issue": "required field missing" }
  ]
}
```

Compilation failure:

```json
{
  "error": "compilation_failed",
  "message": "unknown variable: amunt",
  "line": 14,
  "diagnostics": [
    {
      "severity": "error",
      "message": "unknown variable: amunt",
      "location": {
        "path": "main.typ",
        "range": {
          "start": { "line": 14, "column": 12 },
          "end": { "line": 14, "column": 17 }
        }
      }
    }
  ]
}
```

Template metadata (`GET /_api/templates/financial/invoice`); a schema-less template returns
`{"id":"welcome","schema":null}`:

```json
{
  "id": "financial/invoice",
  "schema": {
    "schemaVersion": "v1",
    "request": {
      "body": {
        "type": "object",
        "required": false,
        "properties": {
          "name": { "type": "string", "required": true }
        }
      }
    }
  }
}
```

Template listing:

```json
{
  "templates": [
    { "id": "financial/invoice", "hasSchema": true },
    { "id": "welcome", "hasSchema": false }
  ]
}
```

## Non-goals

Authentication, versioned metadata APIs, template mutation, publishing, and tagged render routes.

## Related documents

- [Compiler diagnostics](diagnostics.md) for the diagnostic model behind `compilation_failed`.
- [Template layout](templates.md) for template IDs.
- [ADR-0004](../adr/0004-embed-tracked-development-web-ui.md) for why metadata routes are always
  mounted.
