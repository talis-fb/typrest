# Web UI Specification

> Status: Current contract for the bundled development UI and its metadata endpoints.

## Purpose and Scope

The Web UI is a developer convenience for filesystem mode. It lets a developer inspect prepared
templates, compose a payload, render through the existing HTTP endpoint, preview the PDF, and
download the latest result.

It is not a production administration surface, template editor, authentication boundary, or
replacement for the rendering API.

## Configuration and Security

Enable the embedded static UI with:

```toml
schema_version = "v1"
templates_root = "templates"
web_ui = true
```

`web_ui` defaults to false. Current route behavior is:

- `/_webui`, `/_webui/`, and `/_webui/*` exist only when `web_ui = true`;
- `/` redirects permanently to `/_webui/` only when `web_ui = true`;
- `/_api/templates` and `/_api/templates/{*template_id}` are mounted regardless of `web_ui`;
- `/_healthz` and `/{*template_id}` are also independent of `web_ui`.

The UI and metadata routes have no authentication or authorization. A server exposing project
metadata or the Web UI must be bound to loopback or protected by a trusted external boundary:

```bash
typrest serve --hostname 127.0.0.1
```

## Route Contract

### Embedded Static Routes

| Method | Route | Behavior |
|---|---|---|
| `GET` | `/` | Permanent redirect to `/_webui/` |
| `GET` | `/_webui` | Permanent redirect to `/_webui/` |
| `GET` | `/_webui/` | Embedded `index.html` |
| `GET` | `/_webui/*` | Exact embedded asset or `404` |

The index uses `Cache-Control: no-cache`. Fingerprinted assets use
`Cache-Control: public, max-age=31536000, immutable`. Text assets include UTF-8 content types.
Unknown paths return `404`; there is no single-page-application fallback.

### `GET /_api/templates`

Returns prepared registry entries in template-ID order:

```json
{
  "templates": [
    { "id": "financial/invoice", "hasSchema": true },
    { "id": "welcome", "hasSchema": false }
  ]
}
```

An empty registry returns `{"templates":[]}`. The handler reads only `TemplateRegistry`; it does
not scan the filesystem or call `TemplateLoader`.

### `GET /_api/templates/{*template_id}`

Returns the canonical parsed schema:

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

A schema-less template returns `{"id":"welcome","schema":null}`. An invalid ID returns `400`
with `invalid_template_id`; a valid missing ID returns `404` with `template_not_found`.

These routes are internal to the bundled UI and are not a versioned integration API, even though
they are currently always mounted.

## Navigation and Workspace

Opening `/_webui/` represents loading, API failure with retry, an empty registry, or selectable
template cards. Selecting a template stores its ID in the query string:

```text
/_webui/?template=financial%2Finvoice
```

Direct navigation and browser history restore selection. Changing or leaving a template clears the
previous render error, preview, and Blob URL.

The workspace uses an editor and render action beside a preview/download panel on wide screens and
stacks them on narrow screens. Rendering occurs only after the button or `Ctrl+Enter`/`Cmd+Enter`;
opening a template or editing a value does not render automatically.

## Payload Editor

### Mode Availability

- Form mode is available only for a schema whose request body has `type: object`.
- Raw JSON mode is always available.
- Schema-less, primitive, array, null, and untyped roots use Raw JSON by default.
- Without an object-derived initial value, Raw JSON starts with `{}`.

### Form Controls

| Schema rule | Control |
|---|---|
| `enum` | Select using exact JSON enum values |
| `type: string` | Text input |
| `type: number` | Decimal-capable numeric input |
| `type: boolean` | Boolean toggle |
| `type: null` | Read-only `null` |
| `type: object` | Recursive nested controls |
| `type: array` | JSON textarea requiring an array |
| no `type` | JSON textarea accepting any JSON value |

Optional properties have an inclusion control; required properties are always included. Initial
values use explicit default, then first enum member, then the type seed for required fields.
Optional fields without a default or enum start omitted.

### Custom Top-level Fields

Form mode permits top-level custom fields whose values are entered as JSON. Before switching modes
or rendering, the client rejects an empty trimmed key, a key colliding with a schema property, a
duplicate custom key, or invalid JSON. Custom fields affect only the test payload.

### Mode Conversion

Form to Raw JSON composes schema and custom fields, blocks invalid editors, and pretty-prints one
object. Raw JSON to Form requires a valid object, maps declared keys to schema controls, maps unknown
top-level keys to custom fields, and hydrates missing required/defaulted/enum-backed fields.

An impossible conversion leaves the current draft and mode visible with a clear error.

Raw JSON must parse in the browser before rendering. The parsed value—including a primitive or
array—is sent unchanged to the render endpoint.

## Rendering and Blob Lifecycle

The bundled UI's PDF-producing request is:

```http
POST /financial/invoice
Content-Type: application/json
```

On success, the client reads a PDF Blob, creates a Blob URL, revokes the previous URL, displays the
new URL in a titled iframe, and offers an ID-derived filename such as
`financial--invoice.pdf` for download. Download reuses the existing Blob and does not compile
again. Blob URLs are also revoked when leaving a template or destroying the application.

Starting a render aborts the previous render request. Loading another template aborts the previous
metadata request. A failed later render may leave the latest successful preview visible.

## Error and Accessibility Contract

The UI distinguishes metadata failures, invalid client JSON, schema violations, compiler/export
diagnostics, non-JSON HTTP failures, and network failures. It never adds source text or physical
paths beyond the safe HTTP contract.

Every input and action has an accessible label or name. Template selection, tabs, inclusion
controls, rendering, and download are keyboard-operable. Tabs expose correct roles and state,
errors use alert/live semantics, focus remains visible, and the PDF iframe has an informative title.

## Frontend and Assets

The client is Vite, Svelte, and TypeScript with Tailwind CSS and DaisyUI classes. It has no
server-side rendering, frontend router, or Svelte-specific component library. Vite uses
`/_webui/` as its base path.

Production output is committed under `webui/dist` and embedded in debug and release server builds.
Rust and Docker builds do not invoke Node. Frontend source changes require `npm run check`,
`npm test`, and `npm run build`, with the new `dist` included.

## Non-goals

The UI does not provide authentication, production administration, Store mode, publishing,
template mutation, source editing, automatic rendering, live filesystem updates, SSE, WebSockets,
server-side rendering, a frontend router, or an alternative rendering engine.

## Acceptance Criteria

- Static UI routes return `404` by default and are mounted only with `web_ui = true`.
- Metadata routes remain available and read exclusively from the prepared registry.
- Object schemas support Form and Raw JSON workflows; other roots remain renderable as raw JSON.
- Invalid JSON and colliding custom keys are blocked before rendering.
- Every preview and download originates from the UI's `POST /{*template_id}` request. The API also
  supports bodyless `GET /{*template_id}` rendering.
- Replaced and abandoned Blob URLs are revoked.
- Embedded files use the specified content types and cache policy.
- Structured validation, compilation, warning, and PDF response contracts remain unchanged.
