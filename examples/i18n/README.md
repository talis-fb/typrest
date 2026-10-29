# i18n example project

Payload-driven localization with no runtime i18n library. One template, `statement`, renders an A4
account statement in English, Brazilian Portuguese, Spanish, or German, chosen by the `locale` field
of the request body.

```text
typrest.toml
templates/
  _shared/i18n.typ         # label dictionaries, number/money/date formatters, imported as /_shared/i18n.typ
  statement/               # template ID and route: POST /statement
    main.typ
    schema.yml
    payload.json           # sample request body (same as payload-en.json)
    payload-en.json
    payload-pt-BR.json
    payload-es.json
    payload-de.json
```

## Run it

```bash
cd examples/i18n
typrest check
typrest serve --watch
```

Render each locale:

```bash
for l in en pt-BR es de; do
  curl -X POST http://localhost:8080/statement \
    -H 'content-type: application/json' \
    --data @templates/statement/payload-$l.json \
    -o statement-$l.pdf
done
```

With `web_ui = true` the development UI is available at <http://localhost:8080/>.

## What it demonstrates

- **Locale as data**: `locale` is an enum (`en`, `pt-BR`, `es`, `de`, default `en`) and `currency` is an
  enum (`USD`, `EUR`, `BRL`, `GBP`). Locale and currency are independent, so a German reader can see
  USD amounts. See [Schema v1](../../docs/specs/schema-v1.md).
- **Dictionaries in `_shared/`**: `i18n.typ` holds the labels per locale and a `t(locale, key)` lookup that
  falls back to English, then to the key itself, so a missing translation never fails a render.
  See [Template layout](../../docs/specs/templates.md).
- **Formatters**: `number`, `money`, `short-date`, and `long-date` apply the locale's thousands and
  decimal separators, symbol placement (`$ 1,234.50`, `R$ 1.234,50`, `1.234,50 €`), and date order
  (`03/31/2026`, `31/03/2026`, `31.03.2026`).
- **Language settings**: `set text(lang:, region:)` follows the locale, so hyphenation and
  language-sensitive typography match.
- **Computed values**: the payload carries raw numbers; the template computes running balances,
  total credits and debits, and the closing balance.

Transaction descriptions are user data and are shown as sent; only the template's own text is translated.

## Invalid payload

```bash
curl -i -X POST http://localhost:8080/statement \
  -H 'content-type: application/json' \
  -d '{"locale":"fr","account":{"holder":"A","number":"1"},"transactions":[]}'
```

Returns `422` with `"error":"schema_validation_failed"` and the violation
`{"field":"$.locale","issue":"invalid enum value \"fr\""}`. The enum is what rejects unknown
locales at the edge; the fallback in `t()` is a second line of defense for incomplete dictionaries.

## Customizing

To add a locale (for example `fr`):

1. Add `fr` to the `locale` enum in `templates/statement/schema.yml`.
2. Add an `fr` entry to `locales` in `templates/_shared/i18n.typ`: `lang`, `region`, separators,
   `symbol-after`, `date-format`, month names, and the `labels` dictionary. Missing labels fall back to English.
3. If its long-date word order differs from the existing ones, extend `long-date`.

Add a currency by extending the `currency` enum and the `symbols` map.
