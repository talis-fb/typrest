# Receipt example project

A point-of-sale receipt on a narrow custom page: 80 mm wide, with a height that grows to fit the
content. It was created with `typrest init` and then extended.

```text
typrest.toml
templates/
  receipt/                 # template ID and route: POST /receipt
    main.typ
    schema.yml
    payload.json           # full sample request body
    payload-minimal.json   # only required fields; every default is applied
```

## Run it

```bash
cd examples/receipt
typrest check
typrest serve --watch
```

Render the sample payloads:

```bash
curl -X POST http://localhost:8080/receipt \
  -H 'content-type: application/json' \
  --data @templates/receipt/payload.json \
  -o receipt.pdf

curl -X POST http://localhost:8080/receipt \
  -H 'content-type: application/json' \
  --data @templates/receipt/payload-minimal.json \
  -o receipt-minimal.pdf
```

With `web_ui = true` the development UI is available at <http://localhost:8080/>.

## What it demonstrates

- **Custom page size**: `#set page(width: 80mm, height: auto)` produces a single tall page, like a
  thermal-printer roll. The font is `DejaVu Sans Mono`, which ships with Typst.
- **Defaults inside array items**: `items[].note` defaults to `""`, so the template reads
  `l.note` on every line without checking for it. See [Schema v1](../../docs/specs/schema-v1.md)
  (`REQ-SCH-080`).
- **Enum with default**: `payment_method` accepts `cash`, `card`, `pix` or `transfer` and
  defaults to `card`. `tip` (0), `tax_rate` (10), `footer` and `store.phone` also have defaults.
- **Calculated totals**: the payload carries raw numbers. Unit prices are tax-inclusive, so the
  template computes the subtotal, the tax portion (`subtotal * rate / (100 + rate)`), the tip, and
  the grand total. `tax_rate` is an addition to the minimal schema so the rate is not hard-coded.
- **Number formatting**: a small `money` helper rounds to two decimals and groups thousands
  (`1,234.50`), and dotted leaders (`box(width: 1fr, repeat[.])`) join names to prices.
- **Defaults proof**: `payload-minimal.json` omits every optional field and still renders.

## Invalid payload

A string quantity is rejected before the template runs:

```bash
curl -i -X POST http://localhost:8080/receipt \
  -H 'content-type: application/json' \
  -d '{"store":{"name":"X","address":["a"]},"items":[{"name":"Tea","quantity":"two","unit_price":3}]}'
```

The response is `422` with one violation at `$.items[0].quantity`:

```json
{"error":"schema_validation_failed","message":"Payload validation failed","violations":[{"field":"$.items[0].quantity","issue":"expected type number, got string"}]}
```

See the [HTTP API](../../docs/specs/http-api.md) for the error envelope.

## Customizing

Change the width in `#set page(...)`, edit the `tax_rate` default in `schema.yml`, or add a payment
method to the `payment_method` enum.
