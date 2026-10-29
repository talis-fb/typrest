# Schema showcase example project

One template, `schema-demo`, whose `schema.yml` exercises every Schema v1 feature, plus payloads that
pass and payloads that fail. The template prints the payload it receives (after validation and
defaults) as a table, so you can see which defaults were applied.

```text
typrest.toml
payloads/
  valid.json               # exercises every declared field
  defaults-only.json       # only the required fields; everything else is defaulted
  invalid/                 # one file per failure class (see below)
templates/
  schema-demo/             # template ID and route: POST /schema-demo
    main.typ
    schema.yml             # heavily commented
    payload.json           # sample request body (same as payloads/valid.json)
```

## Run it

```bash
cd examples/schema-showcase
typrest check
typrest serve --watch
```

```bash
curl -X POST http://localhost:8080/schema-demo \
  -H 'content-type: application/json' \
  --data @payloads/defaults-only.json \
  -o demo.pdf
```

Render `valid.json` and `defaults-only.json` and compare the two tables. In the second, `status`,
`priority`, `rating`, `published`, `tags`, `lines`, `matrix`, and the nested `author.address.*` and
`author.email` all come from defaults. Typst shows JSON `null` as `none`.

## What it demonstrates

| Feature | Where in `schema.yml` | Requirements |
|---|---|---|
| Six types | `string`, `number`, `boolean`, `object`, `array`, and `type: null` (`archived_at`) | `REQ-SCH-040`, `REQ-SCH-041` |
| `required` | `title`, `author`, `author.name`, `lines[].sku` | `REQ-SCH-030` to `REQ-SCH-035` |
| `default`, including `null` and `[]` | `archived_at`, `note`, `tags`, `lines`, `matrix` | `REQ-SCH-080` to `REQ-SCH-083` |
| Defaults at depth and inside items | `author.address.*`, `lines[].qty` | `REQ-SCH-080` |
| String and number `enum` | `status`, `priority` | `REQ-SCH-070`, `REQ-SCH-071` |
| Nested `object` | `author.address` | `REQ-SCH-050` |
| Array of objects, array of arrays | `lines`, `matrix` | `REQ-SCH-060` |
| Untyped field | `extra`, `note` | `REQ-SCH-021`, `REQ-SCH-042` |
| Undeclared keys are kept | `"undeclared"` in `valid.json` | `REQ-SCH-052` |
| Validation order | comments at the top of `properties` | `REQ-SCH-090`, `REQ-SCH-091` |

Full format: [Schema v1](../../docs/specs/schema-v1.md).

## Invalid payloads

Each file in `payloads/invalid/` was sent to a running server:

```bash
curl -i -X POST http://localhost:8080/schema-demo \
  -H 'content-type: application/json' --data @payloads/invalid/wrong-type.json
```

Every response is the envelope from [HTTP API](../../docs/specs/http-api.md) (`REQ-HTTP-041`,
`REQ-HTTP-042`): `{"error": ..., "message": ..., "violations": [{"field": ..., "issue": ...}]}`.
Paths use `REQ-SCH-092` notation.

| Payload file | Failure class | Status | `error` | `field` | `issue` |
|---|---|---|---|---|---|
| `wrong-type.json` | wrong type | 422 | `schema_validation_failed` | `$.rating` | `expected type number, got string` |
| `bad-enum.json` | bad string enum | 422 | `schema_validation_failed` | `$.status` | `invalid enum value "archived"` |
| `bad-number-enum.json` | bad number enum | 422 | `schema_validation_failed` | `$.priority` | `invalid enum value 4` |
| `missing-required.json` | missing required | 422 | `schema_validation_failed` | `$.title` | `required field missing` |
| `nested-path.json` | nested path, two violations on one item | 422 | `schema_validation_failed` | `$.lines[1].qty` and `$.lines[1].sku` | `expected type number, got string` and `required field missing` |
| `null-for-string.json` | explicit `null` where `string` is expected | 422 | `schema_validation_failed` | `$.author.name` | `expected type string, got null` |
| `null-for-boolean.json` | explicit `null` is not replaced by the default | 422 | `schema_validation_failed` | `$.published` | `expected type boolean, got null` |

Notes:

- A `null` is a present value, so it is validated rather than defaulted (`REQ-SCH-033`,
  `REQ-SCH-081`). Omit the key to get the default.
- Independent fields keep validating, so one response can list several violations (`REQ-SCH-091`).
  Sending `"rating": "x"` together with `"status": "nope"` returns both.
- A body that is not JSON (`{bad`) is a different class: `400` with `invalid_json`
  (`REQ-HTTP-041`).

## Load-time schema errors

These are rejected when the template loads (`REQ-SCH-110`), so `typrest check` and `serve` fail
before listening and no request ever sees them. They are shown as snippets, not committed files,
because they would break `check`. Messages below are the log lines `typrest check` printed for each
snippet placed in this template's `schema.yml`; each is prefixed with
``template `schema-demo`: invalid schema: ``.

| Snippet (inside `request.body`) | Message |
|---|---|
| `properties: {a: {type: string}}` (no `type: object`) | `invalid schema at $: properties is only allowed when type is object` |
| `type: object` with `a: {items: {type: string}}` (no `type: array`) | `invalid schema at $.a: items is only allowed when type is array` |
| `type: text` | `unknown variant `text`, expected one of `string`, `number`, `boolean`, `object`, `array`, `null`` |
| `a: {type: number, default: abc}` | `invalid schema at $.a: default value must have type number` |
| `a: {enum: [a, 1]}` | `invalid schema at $.a: enum values must all have the same type` |
| `a: {type: number, enum: [a, b]}` | `invalid schema at $.a: enum value must have type number` |
| `type: [string, number]` (union, `REQ-SCH-041`) | `invalid type: sequence, expected string or map` |
| missing `request.body` (`request: {}`) | `missing field `body`` |
| missing `schemaVersion` | `template `schema-demo`: schemaVersion is missing` (no "invalid schema" prefix) |
| duplicate mapping key (`REQ-SCH-004`) | a YAML error naming `duplicate mapping key: a` |

## Customizing

Add a field to `templates/schema-demo/schema.yml`, save (watch mode reloads), and send a payload that
breaks it to see the violation. The template prints whatever keys the validated payload contains, so
it needs no edits.
