# Letters example project

One project, three templates. It shows how Typrest discovers templates, how grouped IDs map to
routes, and how `_shared/` code is reused by every template.

```text
typrest.toml
templates/
  _shared/                 # not a template: added to every template's source
    letterhead.typ         # letterhead(), recipient-block(), letter-page()
    theme.typ              # colors and font name
  letters/                 # no main.typ here: only groups the route segment
    formal/                # POST /letters/formal
    notice/                # POST /letters/notice
    welcome/               # POST /letters/welcome
      main.typ  schema.yml  payload.json
```

## Run it

```bash
cd examples/letters
typrest check
typrest serve --watch
curl -X POST http://localhost:8080/letters/formal \
  -H 'content-type: application/json' \
  --data @templates/letters/formal/payload.json -o formal.pdf
```

| Template ID | Route | Purpose |
|---|---|---|
| `letters/formal` | `POST /letters/formal` | Business letter with subject, reference, paragraphs and signatory |
| `letters/notice` | `POST /letters/notice` | One-page notice with a `level` (`info`/`warning`) that changes the accent color, plus action list |
| `letters/welcome` | `POST /letters/welcome` | Friendly onboarding letter with numbered next steps |

## What it demonstrates

- **Discovery and grouped IDs**: `letters/` has no `main.typ`, so it only contributes a route
  segment; each child with a `main.typ` is a template (`REQ-TPL-011`). `_shared` is skipped
  during discovery (`REQ-TPL-012`). The web UI at <http://localhost:8080/> lists the three IDs,
  the same data as `GET /_api/templates`.
- **Shared code**: every template does `#import "/_shared/letterhead.typ": *`. Files in `_shared/`
  are added to each template's source under the `_shared/` prefix (`REQ-TPL-022`). If a template
  ships its own file at the same path (for example `letters/formal/_shared/theme.typ`), the shared
  file wins, so do not use `_shared/` paths for template-specific overrides.
- **Intentional schema duplication**: `sender` and `recipient` have the same shape in all three
  `schema.yml` files, written out in full because Schema v1 has no `$ref`. Each template stays
  independently readable and can evolve its own fields. See
  [Schema v1](../../docs/specs/schema-v1.md).
- **Rebuild on shared edits**: with `--watch`, a change to `_shared/` affects every template
  (`REQ-TPL-023`). Changing `accent` in `theme.typ` logs a full rebuild:

  ```text
  INFO compiling template `letters/formal`
  INFO compiling template `letters/notice`
  INFO compiling template `letters/welcome`
  INFO template registry rebuilt
  INFO templates ready: 3
  ```

  The next request to any of the three routes uses the new color.

## Invalid requests

An unknown template ID returns `404` with `template_not_found`
([HTTP API](../../docs/specs/http-api.md)):

```bash
curl -i -X POST http://localhost:8080/letters/legal -H 'content-type: application/json' -d '{}'
```

```json
{"error":"template_not_found","message":"Template 'letters/legal' was not found"}
```

A payload that fails the schema returns `422` with `schema_validation_failed`; for example, posting
`{"date":"x"}` to `/letters/formal` lists `$.sender`, `$.recipient`, `$.subject`, `$.paragraphs`
and `$.signatory` as missing required fields.

## Customizing

Edit colors in `templates/_shared/theme.typ` or the letterhead layout in
`templates/_shared/letterhead.typ` to restyle all letters at once. To add a letter, create
`templates/letters/<name>/` with `main.typ`, `schema.yml` and `payload.json`; it becomes
`POST /letters/<name>` on the next start or watch rebuild.
