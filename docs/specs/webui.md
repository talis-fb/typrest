# Web UI specification

- **Status:** Current
- **Scope:** The bundled development UI. Its routes, caching, and metadata endpoints are defined in
  the [HTTP API specification](http-api.md).

## Purpose

The Web UI is a developer convenience for filesystem mode. It lets a developer inspect prepared
templates, compose a payload, render through the existing HTTP endpoint, preview the PDF, and
download the latest result.

It is not a production administration surface, template editor, authentication boundary, or
replacement for the rendering API.

## Requirements

### Configuration and security

- **REQ-WEB-010:** The embedded static UI MUST be enabled with `web_ui = true` in `typrest.toml` and
  MUST default to disabled ([`REQ-CFG-030`, `REQ-CFG-031`](typrest-toml.md)). The UI routes,
  redirects, and caching are `REQ-HTTP-002` and `REQ-HTTP-060` to `REQ-HTTP-064`; the metadata
  routes it reads are `REQ-HTTP-030` to `REQ-HTTP-034`.
- **REQ-WEB-011:** The UI and metadata routes have no authentication or authorization
  (`REQ-HTTP-050`, `REQ-HTTP-051`), so a server exposing project metadata or the Web UI MUST be
  bound to loopback or protected by a trusted external boundary:

  ```bash
  typrest serve --hostname 127.0.0.1
  ```

### Navigation and workspace

- **REQ-WEB-020:** Opening `/_webui/` MUST represent loading, API failure with retry, an empty
  registry, or selectable template cards.
- **REQ-WEB-021:** Selecting a template MUST store its ID in the query string, such as
  `/_webui/?template=financial%2Finvoice`.
- **REQ-WEB-022:** Direct navigation and browser history MUST restore the selection.
- **REQ-WEB-023:** Changing or leaving a template MUST clear the previous render error, preview, and
  Blob URL.
- **REQ-WEB-024:** The workspace MUST place an editor and render action beside a preview and download
  panel on wide screens and stack them on narrow screens.
- **REQ-WEB-025:** Rendering MUST occur only after the render button or `Ctrl+Enter`/`Cmd+Enter`;
  opening a template or editing a value MUST NOT render automatically.

### Payload editor: mode availability

- **REQ-WEB-030:** Form mode MUST be available only for a schema whose request body has
  `type: object`.
- **REQ-WEB-031:** Raw JSON mode MUST always be available.
- **REQ-WEB-032:** Schema-less, primitive, array, null, and untyped roots MUST use Raw JSON by
  default.
- **REQ-WEB-033:** Without an object-derived initial value, Raw JSON MUST start with `{}`.

### Payload editor: form controls

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

- **REQ-WEB-040:** Each schema field MUST be rendered with the control in the table above.
- **REQ-WEB-041:** Optional properties MUST have an inclusion control; required properties MUST
  always be included.
- **REQ-WEB-042:** Initial values MUST use the explicit default, then the first enum member, then the
  type seed for required fields. Optional fields without a default or enum MUST start omitted.

### Payload editor: custom top-level fields

- **REQ-WEB-050:** Form mode MUST permit top-level custom fields whose values are entered as JSON.
- **REQ-WEB-051:** Before switching modes or rendering, the client MUST reject an empty trimmed key, a
  key colliding with a schema property, a duplicate custom key, or invalid JSON.
- **REQ-WEB-052:** Custom fields MUST affect only the test payload.

### Payload editor: mode conversion

- **REQ-WEB-060:** Form to Raw JSON MUST compose schema and custom fields, block invalid editors, and
  pretty-print one object.
- **REQ-WEB-061:** Raw JSON to Form MUST require a valid object, map declared keys to schema controls,
  map unknown top-level keys to custom fields, and hydrate missing required, defaulted, or
  enum-backed fields.
- **REQ-WEB-062:** An impossible conversion MUST leave the current draft and mode visible with a clear
  error.
- **REQ-WEB-063:** Raw JSON MUST parse in the browser before rendering, and the parsed value,
  including a primitive or array, MUST be sent unchanged to the render endpoint.

### Rendering and Blob lifecycle

- **REQ-WEB-070:** The UI's PDF-producing request MUST be a `POST /{*template_id}` with
  `Content-Type: application/json`.
- **REQ-WEB-071:** On success, the client MUST read a PDF Blob, create a Blob URL, revoke the previous
  URL, and display the new URL in a titled iframe.
- **REQ-WEB-072:** The client MUST offer an ID-derived download filename such as
  `financial--invoice.pdf`, and download MUST reuse the existing Blob without compiling again.
- **REQ-WEB-073:** Blob URLs MUST also be revoked when leaving a template or destroying the
  application.
- **REQ-WEB-074:** Starting a render MUST abort the previous render request, and loading another
  template MUST abort the previous metadata request.
- **REQ-WEB-075:** A failed later render MAY leave the latest successful preview visible.

### Error and accessibility contract

- **REQ-WEB-080:** The UI MUST distinguish metadata failures, invalid client JSON, schema violations,
  compiler or export diagnostics, non-JSON HTTP failures, and network failures.
- **REQ-WEB-081:** The UI MUST NOT add source text or physical paths beyond the safe HTTP contract.
- **REQ-WEB-082:** Every input and action MUST have an accessible label or name.
- **REQ-WEB-083:** Template selection, tabs, inclusion controls, rendering, and download MUST be
  keyboard-operable.
- **REQ-WEB-084:** Tabs MUST expose correct roles and state, errors MUST use alert or live semantics,
  focus MUST remain visible, and the PDF iframe MUST have an informative title.

### Frontend and assets

- **REQ-WEB-090:** The client MUST be Vite, Svelte, and TypeScript with Tailwind CSS and DaisyUI
  classes, with no server-side rendering, frontend router, or Svelte-specific component library.
- **REQ-WEB-091:** Vite MUST use `/_webui/` as its base path.
- **REQ-WEB-092:** Production output MUST be committed under `webui/dist` and embedded in debug and
  release server builds. Cargo builds MUST NOT invoke Node.
- **REQ-WEB-093:** Frontend source changes MUST pass `npm run check`, `npm test`, and
  `npm run build`, with the new `dist` included.

## Examples

Selected template in the query string:

```text
/_webui/?template=financial%2Finvoice
```

Render request made by the UI:

```http
POST /financial/invoice
Content-Type: application/json
```

Acceptance criteria:

- Object schemas support Form and Raw JSON workflows; other roots remain renderable as raw JSON.
- Invalid JSON and colliding custom keys are blocked before rendering.
- Every preview and download originates from the UI's `POST /{*template_id}` request.
- Replaced and abandoned Blob URLs are revoked.
- Structured validation, compilation, warning, and PDF response contracts remain unchanged.

Route availability, embedded content types, and cache policy are acceptance criteria of the
[HTTP API specification](http-api.md). Frontend behavior is covered by the tests under `webui/`.

## Non-goals

The UI does not provide authentication, production administration, Store mode, publishing,
template mutation, source editing, automatic rendering, live filesystem updates, SSE, WebSockets,
server-side rendering, a frontend router, or an alternative rendering engine.

## Related documents

- [HTTP API specification](http-api.md)
- [`typrest.toml` specification](typrest-toml.md)
- [Web UI development guide](../development/webui.md)
- [ADR-0004](../adr/0004-embed-tracked-development-web-ui.md): why the UI is embedded and tracked.
