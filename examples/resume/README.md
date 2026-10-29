# Resume example project

A one-page A4 resume with a two-column layout. It teaches deeply nested payloads
(`experience[]` containing `highlights[]`), optional sections that disappear when their data is
empty, and an enum that selects a color theme.

```text
typrest.toml
templates/
  _shared/theme.typ        # ink/muted colors and the `accents` map (blue, green, slate)
  resume/                  # template ID and route: POST /resume
    main.typ
    schema.yml
    payload.json           # sample request body
```

## Run it

```bash
cd examples/resume
typrest check
typrest serve --watch
```

```bash
curl -X POST http://localhost:8080/resume \
  -H 'content-type: application/json' \
  --data @templates/resume/payload.json \
  -o resume.pdf
```

With `web_ui = true` the development UI is available at <http://localhost:8080/>.

## What it demonstrates

- **Deep nesting with defaults**: `experience[].highlights` defaults to `[]` and `end` defaults to
  `Present`, so the template never checks for missing keys. Defaults apply inside array items. See
  [Schema v1](../../docs/specs/schema-v1.md).
- **Optional sections**: `summary`, `experience`, `education`, and `skills` default to empty values.
  The template renders a section only when it has content, so
  `{"basics": {"name": "A", "headline": "B", "email": "a@example.com"}}` is a valid payload and
  produces just the header and contact block.
- **Enum-driven theme**: `accent` (`blue`, `green`, `slate`; default `blue`) indexes the `accents` map
  in `_shared/theme.typ`.
- **Layout**: a `grid` with a sidebar and a main column, and skills drawn as rounded pills.
  See [Template layout](../../docs/specs/templates.md) for `_shared/` rules.

## Invalid payload

An `accent` outside the enum is rejected before rendering:

```bash
curl -i -X POST http://localhost:8080/resume -H 'content-type: application/json' \
  -d '{"basics":{"name":"A","headline":"B","email":"a@example.com"},"accent":"pink"}'
```

```text
HTTP/1.1 422 Unprocessable Entity
{"error":"schema_validation_failed","message":"Payload validation failed",
 "violations":[{"field":"$.accent","issue":"invalid enum value \"pink\""}]}
```

The message names the rejected value; the allowed values are the `enum` list in `schema.yml`.

## Customizing

Add a color pair to `accents` in `templates/_shared/theme.typ` and the same key to the `accent` enum in
`schema.yml`. Content longer than one page flows onto a second page.
