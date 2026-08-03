# Web UI Specification

## Status

This document defines the current contract for Typrest's development Web UI. If the implementation
and this specification differ, this document is the source of truth for the next corrective change.

The Web UI is a developer convenience for filesystem mode. It is not a production administration
surface or a replacement for Typrest's HTTP rendering API.

## Goals

The Web UI must let a developer:

- inspect the templates in the prepared in-memory registry;
- compose a test payload using schema-generated controls or raw JSON;
- render through the existing `POST /render/:template` endpoint;
- preview and download the resulting PDF;
- understand schema validation, compilation, export, and network failures.

The Web UI must remain optional, lightweight, client-rendered, and isolated under `webui/`.

## Configuration and Security

The Web UI is controlled by the optional `web_ui` field in `typrest.toml`:

```toml
schema_version = "v1"
templates_root = "templates"
web_ui = true
```

Rules:

- `web_ui` is a boolean.
- Omitting it is equivalent to `web_ui = false`.
- When false, `/webui`, `/webui/*`, and `/_api/*` must not be mounted and therefore return `404`.
- The existing `/healthz`, `/templates`, and `/render/:template` routes are independent of this
  setting.
- `typrest init` omits `web_ui`, leaving new projects disabled by default.

The Web UI has no authentication or authorization. It is intended for local development only.
Developers should bind Typrest to the loopback interface when enabling it:

```bash
typrest serve --hostname 127.0.0.1
```

A Web UI-enabled server must not be exposed directly to an untrusted network.

## Route Contract

The following additional routes exist only when `web_ui = true`.

### Static Routes

| Method | Route | Behavior |
|---|---|---|
| `GET` | `/webui` | Permanently redirects to `/webui/`. |
| `GET` | `/webui/` | Returns the embedded `index.html`. |
| `GET` | `/webui/*` | Returns an exact embedded static asset or `404`. |

The index response uses `Cache-Control: no-cache`. Static asset responses use:

```text
Cache-Control: public, max-age=31536000, immutable
```

Text assets include a UTF-8 content type. Unknown asset paths return `404`; there is no
single-page-application fallback because navigation uses a query parameter rather than a frontend
router.

### Internal Metadata API

Routes under `/_api/` are an internal interface for the bundled Web UI. Their current shapes are
specified so the Rust and TypeScript implementations stay aligned, but they are not a stable API
for external integrations. External systems should use the supported `/templates` and
`/render/:template` routes instead.

Metadata handlers must read from `TemplateRegistry`. They must not scan the filesystem, call
`TemplateLoader`, or reparse `schema.json`.

#### `GET /_api/templates`

Returns prepared templates in registry name order:

```json
{
  "templates": [
    {
      "name": "invoice",
      "hasSchema": true
    },
    {
      "name": "welcome",
      "hasSchema": false
    }
  ]
}
```

Each item contains:

- `name`: the canonical `TemplateName`;
- `hasSchema`: whether the prepared registry entry contains a parsed `Schema`.

An empty registry returns:

```json
{
  "templates": []
}
```

#### `GET /_api/templates/:template`

Returns the template name and its canonical parsed schema:

```json
{
  "name": "invoice",
  "schema": {
    "schemaVersion": "v1",
    "request": {
      "body": {
        "type": "object",
        "required": false,
        "properties": {
          "name": {
            "type": "string",
            "required": true
          }
        }
      }
    }
  }
}
```

A template without `schema.json` returns:

```json
{
  "name": "welcome",
  "schema": null
}
```

Invalid template names return `400` with the standard JSON error envelope:

```json
{
  "error": "invalid_template_name",
  "message": "template name contains invalid character '$' at position 1; only ASCII alphanumeric characters, '-' and '_' are allowed"
}
```

A valid name that is absent from the prepared registry returns `404`:

```json
{
  "error": "template_not_found",
  "message": "Template 'missing' was not found"
}
```

## User Flow

### Template List

Opening `/webui/` loads the internal template list. The initial view must represent:

- loading;
- API failure with a retry action;
- an empty registry;
- one selectable card per prepared template.

Selecting a template loads its metadata and opens the workspace. The selected name is stored in the
`template` query parameter:

```text
/webui/?template=invoice
```

Opening that URL directly restores the selected workspace. Browser back/forward navigation must
follow changes to the query parameter. Selecting or leaving a template clears the previous preview,
render error, and Blob URL.

### Workspace

The workspace uses two panels on wide viewports and stacks them on smaller viewports:

- left: payload editor and render action;
- right: PDF preview, errors, and download action.

Rendering is always explicit. Opening a template or editing a payload must not automatically issue a
render request. The developer renders with the button or `Ctrl+Enter`/`Cmd+Enter`.

## Payload Editor

### Mode Availability

The editor has Form and Raw JSON modes.

- Form mode is available only when the template has a schema whose `request.body.type` is
  `object`.
- Templates without schemas use Raw JSON mode and accept any valid JSON value.
- Schemas with primitive, array, null, or unspecified root types use Raw JSON mode.
- Raw JSON starts as `{}` when there is no object schema-derived initial payload.

### Form Controls

Form mode maps schema fields as follows:

| Schema rule | Control |
|---|---|
| `enum` | Select containing the exact JSON enum values. |
| `type: "string"` | Text input. |
| `type: "number"` | Numeric input accepting decimal values. |
| `type: "boolean"` | Boolean toggle. |
| `type: "null"` | Read-only `null` value. |
| `type: "object"` | Recursively nested field controls. |
| `type: "array"` | JSON textarea that must parse to an array. |
| no `type` | JSON textarea accepting any JSON value. |

Optional schema fields have an explicit inclusion control. Required fields are always included in
Form mode.

Initial values use this precedence:

1. explicit `default`, including `null`;
2. first `enum` value;
3. a type seed for required fields: `""`, `0`, `false`, recursively seeded `{}`, `[]`, or `null`.

Optional fields without a default or enum are initially omitted. When Raw JSON is moved into Form
mode, required, defaulted, and enum-backed fields missing from the raw object are hydrated using the
same rules, including inside nested objects.

### Custom Fields

Form mode allows top-level custom test fields. Each custom field has:

- a key;
- a value entered as JSON.

Before mode switching or rendering, the client must reject:

- an empty key after trimming surrounding whitespace;
- a key matching a top-level schema-generated field;
- duplicate custom keys;
- a custom value that is not valid JSON.

Custom fields are test-payload additions only. They do not change `schema.json`.

### Mode Switching

Form to Raw JSON:

- compose schema and custom fields into one object;
- block the switch while any schema-backed JSON control or custom field is invalid;
- pretty-print the composed object into the Raw JSON editor.

Raw JSON to Form:

- require syntactically valid JSON;
- require an object root;
- map declared keys into schema-generated fields;
- map unknown top-level keys into custom fields;
- hydrate missing required, defaulted, and enum-backed fields.

If conversion is impossible, the current mode and draft remain visible with a clear error.

### Raw JSON Validation

Raw JSON must be parsed in the browser before a render request. Invalid JSON must:

- mark the editor as invalid;
- show the parse error;
- prevent the request.

The valid parsed JSON value, including a primitive or array for schema-less/non-object schemas, is
the request body sent to the render endpoint.

## Rendering and Preview

The only PDF-producing request is:

```http
POST /render/:template
Content-Type: application/json
```

The Web UI must not introduce another compiler, rendering handler, or render-specific metadata API.
The request and error contracts remain those of the existing render endpoint.

On success, the client:

1. reads the PDF response as a browser `Blob`;
2. creates a Blob URL;
3. revokes the previous Blob URL;
4. displays the new URL in a titled `iframe`;
5. exposes a download link named `<template>.pdf`.

The download action saves the latest successful endpoint-produced Blob and does not trigger another
compilation. A Blob URL must also be revoked when leaving a template or destroying the application.

Starting another render aborts the previous in-flight request. Loading another template similarly
aborts the previous metadata request.

## Error Presentation

The UI must distinguish:

- metadata loading failures;
- invalid raw or custom JSON;
- schema validation failures and their `violations`;
- compilation/export failures and their safe diagnostics;
- non-JSON HTTP failures;
- network failures.

The preview panel may continue showing the latest successful PDF when a later render fails. Error
details must never add template source text or physical filesystem paths beyond what the existing
HTTP diagnostic contract exposes.

## Accessibility

The Web UI must provide:

- labels or accessible names for every input and action;
- keyboard-operable template selection, tabs, inclusion controls, render, and download actions;
- correct tab roles and selected state;
- visible focus treatment;
- `role="alert"` or `aria-live` treatment for actionable errors;
- an informative PDF iframe title;
- adequate contrast at supported responsive sizes.

## Frontend and Asset Architecture

The frontend is a client-rendered Vite, Svelte, and TypeScript application styled with Tailwind CSS
and DaisyUI classes. It must not use server-side rendering, a Svelte-specific UI component library,
or a frontend router.

Svelte owns state, payload composition, API calls, request cancellation, and Blob lifecycle.
Presentation should prefer semantic HTML and DaisyUI classes. Additional components should remain
focused on template selection, payload editing, recursive fields, and PDF preview.

The Vite base path is `/webui/`. Its production output is committed under `webui/dist` and embedded
in the Rust server binary in debug and release builds. Normal Rust/Docker builds therefore do not
require Node, but frontend source changes require rebuilding and committing `dist`.

## Non-Goals

The Web UI does not provide:

- authentication, authorization, or SSO;
- production administration;
- Store mode, tags, publishing, or template mutation;
- template source editing;
- automatic, debounced, or realtime rendering;
- SSE, WebSockets, or live filesystem updates;
- a frontend router or server-side rendering;
- a theme switcher;
- an alternative rendering engine.

Refreshing the browser is sufficient after template or schema changes.

## Acceptance Criteria

- All Web UI routes are absent by default and present only with `web_ui = true`.
- Metadata comes exclusively from the prepared registry.
- Schema-backed object templates support Form and Raw JSON workflows.
- Schema-less and non-object-root templates remain renderable through Raw JSON.
- Duplicate/colliding custom keys and invalid JSON are blocked before rendering.
- Every successful preview and download originates from `POST /render/:template`.
- Replaced and abandoned Blob URLs are revoked.
- Existing render routes and error contracts remain unchanged.
- The committed Vite build is embedded and served with the specified content types and cache policy.
