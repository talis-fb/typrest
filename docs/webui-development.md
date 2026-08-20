# Web UI Usage and Development Guide

> Status: Current contributor guide. Normative behavior lives in the
> [Web UI Specification](webui-spec.md).

## Enable the Web UI

Add `web_ui = true` to a Typrest project's configuration:

```toml
schema_version = "v1"
templates_root = "templates"
web_ui = true
```

Start on loopback:

```bash
typrest serve --hostname 127.0.0.1
```

Open `http://127.0.0.1:8080/_webui`. The UI has no authentication. The metadata endpoints are
currently mounted even when the static UI is disabled, so protect the server itself from untrusted
networks.

## Use the Workspace

The initial screen lists prepared templates. A Schema badge identifies a parsed schema; Raw JSON
identifies a schema-less template. Selection is stored in the URL:

```text
http://127.0.0.1:8080/_webui/?template=financial%2Finvoice
```

The UI does not poll for registry changes. Refresh after a template or schema reload.

### Form Mode

Form mode is available for object-root schemas. It creates recursive controls, initializes defaults
and enum values, seeds required fields, and lets optional fields be included or omitted. Arrays and
untyped values use JSON editors.

Top-level custom fields accept JSON values. Strings require quotes; objects, arrays, numbers,
booleans, and null use normal JSON syntax. Empty keys, duplicates, schema collisions, and invalid
values are rejected before rendering.

### Raw JSON Mode

Raw JSON edits the entire request value and is the only mode for schema-less or non-object schemas.
Invalid JSON is blocked locally. Switching from Form produces a formatted object; switching back
maps declared properties to controls and unknown top-level properties to custom fields.

Server-side `RenderService` still performs schema validation and applies defaults before invoking
the compiler.

### Preview and Download

Use **Render PDF** or `Ctrl+Enter`/`Cmd+Enter`. The UI calls `POST /{*template_id}`. A successful
PDF appears in the preview panel, and **Download PDF** saves the same Blob with `/` replaced by
`--` in the filename, without another compilation.

Validation errors show affected fields. Compilation failures show safe locations, hints, and
traces. Network and non-JSON failures are presented separately.

## Frontend Layout

```text
webui/
  src/
    App.svelte
    api.ts
    payload.ts
    components/
  dist/
  package.json
  vite.config.ts
```

Responsibilities:

- `App.svelte`: navigation, metadata loading, rendering, request cancellation, and Blob lifecycle;
- `api.ts`: metadata and render HTTP calls;
- `payload.ts`: schema-derived values, form/raw conversion, and custom-field validation;
- components: template list, recursive fields, editor, and PDF preview.

Use semantic HTML, Tailwind utilities, and DaisyUI classes. Do not add server-side rendering, a
frontend router, or a Svelte-specific component library.

## Local Development

Install the locked dependencies once:

```bash
cd webui
npm ci
```

Run Typrest with a Web UI-enabled project in one terminal:

```bash
typrest serve --hostname 127.0.0.1
```

Run Vite in another:

```bash
cd webui
npm run dev
```

Open the `/_webui/` path printed by Vite, normally
`http://localhost:5173/_webui/`. Vite proxies `/_api` and the non-underscore template namespace to
`http://127.0.0.1:8080`; the Rust server remains responsible for metadata and PDF rendering.

## Validate Changes

```bash
cd webui
npm run check
npm test
npm run build
```

Then run the Rust checks from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`npm run build` writes fingerprinted files under `webui/dist`. Review and commit source and output
together. Never hand-edit `dist`.

## Embedded Asset Workflow

The Rust server embeds `webui/dist` at compile time, preserving single-binary deployment without a
Node dependency in normal Cargo builds. Use this order:

1. change files under `webui/src`;
2. run checks and frontend tests;
3. run the production build;
4. review source and `dist` changes;
5. rebuild and test the Rust server.

## Metadata Endpoints

The frontend reads:

```text
GET /_api/templates
GET /_api/templates/{*template_id}
```

Both routes read prepared registry state and are currently mounted independently of `web_ui`.
They are an internal, unversioned interface for the bundled frontend, not a stable integration API.

## Troubleshooting

### `/_webui` Returns 404

Confirm that `web_ui = true` is in the discovered `typrest.toml` and restart the server after a
configuration change. Static UI routes intentionally return `404` while disabled.

### The UI Looks Stale

Refresh Vite during development. For Rust-served assets, run `npm run build`, rebuild/restart the
Rust binary, and refresh. Template metadata reflects the registry; refresh after watch mode updates
it.

### Form Mode Is Unavailable

Form mode requires `request.body.type: object`. Missing schemas and every other root type use Raw
JSON by design.

### No Templates Are Listed

Run `typrest check` and inspect startup diagnostics. The endpoint reads the prepared registry, not a
live directory scan.

### Rendering Fails

- Fix client-side JSON errors first.
- For `422`, inspect schema violations.
- For `500`, inspect compiler diagnostics and run `typrest check` for annotated output.
- For network failures, confirm the Rust server and Vite proxy target are running.

## Related Documentation

- [Web UI Specification](webui-spec.md)
- [`typrest.toml` Specification](typrest-toml-spec.md)
- [Schema Specification](schema-v1.md)
- [Compiler diagnostics](diagnostics.md)
- [Testing Guide](testing.md)
