# Web UI Usage and Development Guide

This guide covers using Typrest's development Web UI and maintaining its frontend. For the
normative behavior and internal route contracts, see [Web UI Specification](webui-spec.md).

## Enable the Web UI

The Web UI is disabled by default. Add `web_ui = true` to the project's `typrest.toml`:

```toml
schema_version = "v1"
templates_root = "templates"
web_ui = true
```

Start Typrest on the loopback interface:

```bash
cargo run -- serve --hostname 127.0.0.1
```

Open:

```text
http://127.0.0.1:8080/webui
```

The Web UI has no authentication. Do not bind a Web UI-enabled server to a public or untrusted
network.

## Use the Workspace

### Choose a Template

The first screen lists every template prepared during server startup. A Schema badge means the
template has a parsed `schema.json`; Raw JSON means it does not.

Selecting a template opens its workspace and updates the URL:

```text
http://127.0.0.1:8080/webui/?template=invoice
```

This URL can be bookmarked or refreshed. The Web UI does not poll for template changes; refresh the
browser after the registry has been updated.

### Edit With Form Mode

Form mode is available for schemas with an object request body. It:

- creates controls for schema properties;
- nests object properties recursively;
- initializes defaults, enum selections, and required values;
- lets optional fields be included or omitted;
- uses JSON editors for arrays and untyped values.

Use **Add custom field** to include a top-level test value that is not declared by the schema.
Custom values are JSON, so strings require quotes:

```json
"debug"
```

Objects, arrays, booleans, numbers, and null can be entered directly:

```json
{"trace": true}
```

The UI rejects empty keys, duplicate custom keys, schema-field collisions, and invalid JSON before
rendering.

### Edit With Raw JSON Mode

Raw JSON mode edits the complete request value submitted to the render endpoint. It is always
available and is the only mode for:

- templates without `schema.json`;
- schemas whose request root is not an object.

The editor reports invalid JSON before making a request. Switching from Form mode produces a
formatted object. Switching back maps declared properties to form controls and unknown top-level
properties to custom fields.

For a schema-backed template, `RenderService` still performs normal validation and applies schema
defaults before the compiler receives the value through `sys.inputs.data`.

### Render, Preview, and Download

Click **Render PDF** or press:

```text
Ctrl+Enter
Cmd+Enter
```

The Web UI sends the payload to the existing endpoint:

```text
POST /render/:template
```

A successful PDF appears in the preview panel. **Download PDF** saves that same rendered result as
`<template>.pdf`; it does not compile the template again.

Validation errors list the affected payload fields. Compilation errors show safe messages,
locations, and hints returned by the server. Network and non-JSON response failures are shown
separately.

## Frontend Development

The frontend lives under:

```text
webui/
  src/
    main.ts
    App.svelte
    api.ts
    payload.ts
    components/
  dist/
  package.json
  vite.config.ts
```

Responsibilities are intentionally narrow:

- `App.svelte`: template navigation, metadata loading, rendering, and Blob lifecycle;
- `api.ts`: internal metadata and render HTTP calls;
- `payload.ts`: schema-derived values, form/raw conversion, and custom-field validation;
- focused components: template list, recursive payload fields, editor, and PDF preview.

Use ordinary semantic HTML, Tailwind utilities, and DaisyUI classes. Do not add Svelte-specific UI
component libraries. Add another component only when it isolates a meaningful state or rendering
responsibility.

### Install Dependencies

From the repository root:

```bash
cd webui
npm ci
```

`package-lock.json` is committed. `node_modules/` is local and ignored.

### Run the Vite Development Server

Run Typrest with the Web UI enabled in one terminal:

```bash
cargo run -- serve --hostname 127.0.0.1
```

Run Vite in another:

```bash
cd webui
npm run dev
```

Open the `/webui/` path on the address printed by Vite, normally:

```text
http://localhost:5173/webui/
```

The Vite configuration proxies `/_api` and `/render` to `http://127.0.0.1:8080`. The Vite server is
for frontend development only; the Rust server remains responsible for metadata and PDF rendering.

## Validate Frontend Changes

Run:

```bash
cd webui
npm run check
npm test
npm run build
```

These commands:

- type-check Svelte and TypeScript and report accessibility diagnostics;
- run payload composition and validation tests;
- write the production build to `webui/dist`.

The build output is intentionally committed. After running the build, review and include changed
files under `webui/dist`.

Then validate the Rust workspace from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Server tests verify configuration defaults, route gating, embedded assets, and internal metadata
responses.

## Embedded Asset Workflow

The server embeds `webui/dist` into the binary. This preserves Typrest's single-binary deployment
and keeps normal Cargo and Docker builds independent of Node.

The consequence is that changing frontend source alone does not change an already-built Typrest
binary. Use this order:

1. change files under `webui/src`;
2. run `npm run check` and `npm test`;
3. run `npm run build`;
4. commit both source and `webui/dist`;
5. rebuild the Rust binary.

Vite uses `/webui/` as its base path and emits fingerprinted asset names. Do not hand-edit files in
`webui/dist`.

## Internal Metadata API

The frontend reads:

```text
GET /_api/templates
GET /_api/templates/:template
```

These routes expose only names and parsed schemas from the prepared registry. They never read live
template files.

The `/_api/` routes are internal to the bundled Web UI and may change with it. Do not build external
integrations against them; use `/templates` and `/render/:template` for supported integrations.

See [Web UI Specification](webui-spec.md#internal-metadata-api) for the current response shapes.

## Troubleshooting

### `/webui` Returns 404

Confirm that:

- `web_ui = true` is in the `typrest.toml` found by `typrest serve`;
- the server was restarted after changing configuration;
- the running binary contains the Web UI implementation.

When disabled, all `/webui*` and `/_api/*` routes intentionally return `404`.

### The UI Looks Stale

For the Vite development server, refresh the browser.

For the Rust-served UI:

1. run `npm run build`;
2. rebuild/restart Typrest;
3. refresh `/webui/`.

The Rust binary contains a snapshot of `webui/dist` from build time.

### Form Mode Is Disabled

Form mode requires:

```json
{
  "request": {
    "body": {
      "type": "object"
    }
  }
}
```

A missing schema or any other root type uses Raw JSON mode by design.

### No Templates Are Listed

The list reflects the prepared registry, not a live directory scan. Check server startup output and
confirm the templates loaded successfully. If templates changed after startup, use the existing
watch behavior where appropriate or restart the server, then refresh the browser.

### A Render Fails

- Fix client-side JSON errors before retrying.
- For `422`, review the listed schema violations.
- For `500`, review compiler diagnostics and run `typrest check` for annotated source output.
- For network failures, confirm that the Rust server is running and that Vite is proxying to port
  `8080`.

The Web UI never bypasses `RenderService`; a payload that fails through the HTTP API also fails from
the browser.

## Related Documentation

- [Web UI Specification](webui-spec.md)
- [`typrest.toml` Specification](typrest-toml-spec.md)
- [Schema Specification](schema-v1.md)
- [Compiler Diagnostics and Logging](diagnostics.md)
- [Testing Guide](testing.md)
