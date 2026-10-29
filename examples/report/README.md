# Report example project

A multi-page A4 report: cover page, table of contents, numbered sections with nested bullets and
tables, a running header, and a "Page X of Y" footer.

```text
typrest.toml
templates/
  _shared/theme.typ        # colors and heading styles, imported as /_shared/theme.typ
  report/                  # template ID and route: POST /report
    main.typ
    schema.yml
    payload.json           # sample request body (7 sections, 6 pages)
```

## Run it

```bash
cd examples/report
typrest check
typrest serve --watch
```

```bash
curl -X POST http://localhost:8080/report \
  -H 'content-type: application/json' \
  --data @templates/report/payload.json \
  -o report.pdf
```

With `web_ui = true` the development UI is available at <http://localhost:8080/>.

## What it demonstrates

- **Nested schema**: `sections[]` is an array of objects, each with `bullets[]` (default `[]`) and an
  optional `table` object whose `rows` are an array of arrays of strings. See
  [Schema v1](../../docs/specs/schema-v1.md).
- **Optional parts without branching on missing keys**: `bullets` defaults to an empty list, so the
  template only checks its length; `table` has no default and is rendered only when present
  (`"table" in s`).
- **Table of contents**: `show_toc` (boolean, default `true`) toggles `outline()`. Typst resolves page
  numbers across layout passes, so they match the pages.
- **Running header and footer**: the header shows the title and date, the footer shows
  "Page X of Y". The cover page has neither, and numbering restarts at 1 on the first body page.
- **Long tables**: figures are made breakable and `table.header` repeats the header row on every page.
- **Shared code**: colors and heading styles live in `_shared/`. See
  [Template layout](../../docs/specs/templates.md).

## Invalid payload

`show_toc` must be a boolean. Sending `"show_toc": "yes"` returns `422`:

```json
{"error":"schema_validation_failed","message":"Payload validation failed","violations":[{"field":"$.show_toc","issue":"expected type boolean, got string"}]}
```

## Customizing

Change `accent` and the other tokens in `templates/_shared/theme.typ`, or edit `apply-headings` to
restyle headings. Only `title`, `author`, `date`, `summary`, and `sections` are required.
