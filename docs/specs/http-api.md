# HTTP API specification

- **Status:** Current
- **Scope:** Routes, request and response contracts, and the error envelope served by
  `crates/typrest/src/http/`. Static Web UI behavior is in the [Web UI specification](webui.md).

## Purpose

Clients render PDFs with plain HTTP. Handlers read only the prepared registry; see the
[architecture overview](../architecture/overview.md).

## Requirements

### Routes

- **REQ-HTTP-001:** The server MUST mount `GET /_healthz`, `GET /_api/templates`,
  `GET /_api/templates/{*template_id}`, `GET /{*template_id}`, and `POST /{*template_id}`
  regardless of configuration.
- **REQ-HTTP-002:** The server MUST mount `GET /`, `GET /_webui`, `GET /_webui/`, and `GET /_webui/*`
  as the Web UI routes defined in [Web UI routes](#web-ui-routes) only when `web_ui = true`.
- **REQ-HTTP-003:** The server MUST NOT expose `/healthz`, `/templates`, `/render/*`, or `/webui/*`
  aliases.
- **REQ-HTTP-004:** Request paths whose first segment begins with `_` are reserved for Typrest and MUST NOT
  identify a template.

### Health

- **REQ-HTTP-010:** `GET /_healthz` MUST return status `200` with the JSON body `{"status":"ok"}`.

### Rendering

- **REQ-HTTP-020:** `GET` and `POST /{*template_id}` MUST render the template with the ID in the path
  and return status `200` with `Content-Type: application/pdf` and the PDF bytes.
- **REQ-HTTP-021:** `POST` MUST be sent with `Content-Type: application/json` (parameters such as
  `charset` are allowed); otherwise it MUST fail with `400` and `invalid_json`, even when the body
  is empty. The parsed body value, including a primitive or an array, is the render payload.
- **REQ-HTTP-022:** An empty body, including every `GET`, MUST use `{}` as the payload. Schema
  validation then applies as for any payload, so a schema with required properties returns
  `schema_validation_failed`. A non-empty body that is not valid JSON MUST fail with `400` and
  `invalid_json`.
- **REQ-HTTP-023:** The handler MUST parse the path as a template ID before touching the registry.
- **REQ-HTTP-024:** When the template has a schema, the server MUST validate the payload and apply
  defaults before compilation. See [Schema v1](schema-v1.md).
- **REQ-HTTP-025:** Rendering MUST read only the prepared registry. It MUST NOT read template or
  package files or access the network.

### Metadata

- **REQ-HTTP-030:** `GET /_api/templates` MUST return `{"templates":[{"id":"…","hasSchema":true}]}`
  with entries in template-ID order, and `{"templates":[]}` for an empty registry.
- **REQ-HTTP-031:** `GET /_api/templates/{*template_id}` MUST return `{"id":"…","schema":{…}}` with the
  canonical parsed schema, or `"schema":null` for a schema-less template.
- **REQ-HTTP-032:** Metadata handlers MUST read only the prepared registry.
- **REQ-HTTP-033:** An invalid template ID MUST return `400` with `invalid_template_id`; a valid ID
  with no template MUST return `404` with `template_not_found`.
- **REQ-HTTP-034:** The metadata routes are internal to the bundled Web UI and unversioned. Integrations
  MUST NOT assume they are stable.

### Errors

- **REQ-HTTP-040:** Every error response MUST be a JSON object with `error` (stable code) and
  `message`.
- **REQ-HTTP-041:** The following codes and statuses MUST be used:

  | Condition | Status | `error` |
  |---|---|---|
  | Invalid template ID | `400` | `invalid_template_id` |
  | Request body is not valid JSON | `400` | `invalid_json` |
  | No prepared template with that ID | `404` | `template_not_found` |
  | Payload violates the schema | `422` | `schema_validation_failed` |
  | Request-time compilation or PDF export failure | `500` | `compilation_failed` |

- **REQ-HTTP-042:** `schema_validation_failed` MUST include `violations`, a list of
  `{"field":"$.path","issue":"…"}` entries.
- **REQ-HTTP-043:** `compilation_failed` MUST include `diagnostics` and MAY include `line`, `hints`, and
  `trace`.
- **REQ-HTTP-044:** No error response MAY contain source text or physical filesystem paths. Diagnostic
  locations use virtual paths.

### Security

- **REQ-HTTP-050:** The server MUST NOT authenticate or authorize any route.
- **REQ-HTTP-051:** Deployments that need confidentiality MUST bind to loopback or place the server
  behind a trusted network boundary. The metadata routes are mounted even when `web_ui = false`, so
  disabling the UI does not hide template IDs or schemas.

### Web UI routes

<a id="web-ui-routes"></a>

- **REQ-HTTP-060:** `GET /` and `GET /_webui` MUST redirect permanently to `/_webui/`.
- **REQ-HTTP-061:** `GET /_webui/` MUST return the embedded `index.html` with `Cache-Control: no-cache`.
- **REQ-HTTP-062:** `GET /_webui/*` MUST return the exact embedded asset or `404`; there is no
  single-page-application fallback.
- **REQ-HTTP-063:** Fingerprinted assets MUST use `Cache-Control: public, max-age=31536000, immutable`,
  and text assets MUST declare UTF-8.
- **REQ-HTTP-064:** With `web_ui = false`, `/_webui`, `/_webui/`, and `/_webui/*` MUST return `404`.

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
