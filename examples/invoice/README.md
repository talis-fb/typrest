# Invoice example project

A complete Typrest project that renders a multi-page A4 invoice. It was created with
`typrest init` and then extended.

```text
typrest.toml
templates/
  _shared/theme.typ        # colors, money/date/percent helpers, imported as /_shared/theme.typ
  invoice/                 # template ID and route: POST /invoice
    main.typ
    schema.yml
    payload.json           # sample request body
    assets/logo.svg
```

## Run it

```bash
cd examples/invoice
typrest check
typrest serve --watch
```

Render the sample payload:

```bash
curl -X POST http://localhost:8080/invoice \
  -H 'content-type: application/json' \
  --data @templates/invoice/payload.json \
  -o invoice.pdf
```

With `web_ui = true` the development UI is available at <http://localhost:8080/>.

## What it demonstrates

- **Schema with defaults and enums**: `currency` (`USD`, `EUR`, `BRL`) and `status` (`draft`, `sent`,
  `paid`, `overdue`) are enums; optional fields, including those inside `items`, default to
  empty values so the template never branches on missing keys. See
  [Schema v1](../../docs/specs/schema-v1.md).
- **Calculated totals**: the payload carries raw numbers. The template computes line totals,
  per-line and invoice-level discounts, tax grouped by rate, and the grand total.
- **Layout**: header with logo and status badge, party cards, a striped table whose header repeats
  across pages, a totals panel, payment details, notes, and a "page X of Y" footer.
- **Status watermark**: `draft` and `paid` invoices get a diagonal watermark.
- **Shared code**: locale-aware money formatting and tokens live in `_shared/`, available to
  every template in the project. See [Template layout](../../docs/specs/templates.md).
- **Bundled assets**: the logo is read from the template directory, never from the host
  filesystem.

## Customizing

Edit the colors in `templates/_shared/theme.typ`, replace `templates/invoice/assets/logo.svg`, or add
a currency to the `currencies` map and to the `currency` enum in `schema.yml`.
