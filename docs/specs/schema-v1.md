# Schema Specification (`schema.yml`) Version 1

- **Status:** Current
- **Scope:** The payload schema format, validation semantics, and the startup sample payload,
  implemented by the v1 parser and validator in `typrest-core`.

## Purpose

A template may include a schema to validate and default its JSON request body. Without a schema,
Typrest accepts any valid JSON value and injects it directly through `sys.inputs.data`.

## Requirements

### File rules

File discovery and conflicts are defined in [Template layout](templates.md)
(`TEMPLATE-030` to `TEMPLATE-032`).

- **SCHEMA-001:** `schema.yml` is canonical and must use YAML syntax. Legacy `schema.json` must be
  accepted with JSON syntax.
- **SCHEMA-002:** The schema file must be optional.
- **SCHEMA-003:** YAML input must contain exactly one document and may contain comments.
- **SCHEMA-004:** Duplicate mapping keys must be invalid.
- **SCHEMA-005:** Only `true` and `false` must use YAML boolean semantics; values such as `on` and
  `off` remain strings.

### Document shape

When a schema file is present, it must contain:

```yaml
schemaVersion: v1
request:
  body:
    type: object
    properties:
      name:
        type: string
        required: true
```

```text
schema.yml
  schemaVersion: string, required, exactly "v1"
  request: object, required
    body: SchemaField, required
```

- **SCHEMA-010:** `schemaVersion` must be present and exactly `"v1"`. A missing or unsupported
  version must be a `SchemaVersionError`.
- **SCHEMA-011:** `request` and `request.body` must be present. Version v1 defines request-body
  validation only.
- **SCHEMA-012:** Other syntax, type, or structural failures must be `SchemaError` values detected
  while loading the template.
- **SCHEMA-013:** Unknown keys at the root, under `request`, and inside fields are currently
  ignored without a warning.

### `SchemaField`

Every location that accepts a schema field uses the same recursive shape:

| Field | Type | Default | Constraint |
|---|---|---|---|
| `required` | boolean | `false` | Checks whether an object property exists |
| `type` | string | absent | One of `string`, `number`, `boolean`, `object`, `array`, `null` |
| `properties` | object of fields | absent | Allowed only with `type: object` |
| `items` | field | absent | Allowed only with `type: array` |
| `enum` | array of JSON values | absent | Values must have one JSON type |
| `default` | any JSON value | absent | Must match `type` when `type` is present |

- **SCHEMA-020:** All field keys must be optional.
- **SCHEMA-021:** An empty field (`{}`) must accept any existing value.

### Required and missing values

- **SCHEMA-030:** `required` must check existence, not truthiness or nullability.
- **SCHEMA-031:** A missing property with `required: true` must produce `required field missing`.
- **SCHEMA-032:** A missing optional property must be omitted unless it has a default.
- **SCHEMA-033:** An explicit `null` exists and must continue through type and enum validation.
- **SCHEMA-034:** Required checking must happen before default application, so a required property
  is still required when a default is also declared.
- **SCHEMA-035:** `required` is meaningful only for properties nested under an object schema. The
  request body is always present once the HTTP handler has parsed it.

### Types

| Schema type | Accepted JSON value |
|---|---|
| `string` | string |
| `number` | integer or non-integer JSON number |
| `boolean` | `true` or `false` |
| `object` | object |
| `array` | array |
| `null` | `null` |

- **SCHEMA-040:** An invalid `type` value must be a load-time schema error.
- **SCHEMA-041:** Only one type may be declared. Type arrays and union types must not be supported.
- **SCHEMA-042:** Without `type`, no type check must occur, although `enum` and `default` rules
  still apply.

### Objects and properties

```yaml
type: object
properties:
  name:
    type: string
  address:
    type: object
    properties:
      city:
        type: string
        required: true
```

- **SCHEMA-050:** `properties` must validate declared members recursively.
- **SCHEMA-051:** `properties` is valid only with `type: object`. Using it with another type, or
  without `type`, must be a load-time schema error.
- **SCHEMA-052:** Undeclared payload properties must be retained and must not cause failures.
- **SCHEMA-053:** Declared properties must be processed in name order.

### Arrays and items

```yaml
type: array
items:
  type: object
  properties:
    amount:
      type: number
      required: true
```

- **SCHEMA-060:** `items` must apply one recursive field definition to every array element.
- **SCHEMA-061:** `items` is valid only with `type: array`. Using it with another type, or without
  `type`, must be a load-time schema error.
- **SCHEMA-062:** An empty array must be valid.

### Enums

```yaml
type: string
enum: [BRL, USD]
```

- **SCHEMA-070:** `enum` must restrict an existing value by exact JSON equality.
- **SCHEMA-071:** All enum members must have the same JSON type, and when `type` is present every
  member must match it. Violations must be load-time schema errors.
- **SCHEMA-072:** An empty enum is structurally valid; no existing value can satisfy it.

### Defaults

```yaml
type: object
properties:
  displayName:
    type: string
    default: anonymous
```

Input `{}` becomes `{"displayName":"anonymous"}` before compilation.

- **SCHEMA-080:** A default must supply a property that is missing and optional.
- **SCHEMA-081:** A default must never replace an explicit `null`.
- **SCHEMA-082:** When `type` is present, the default must match it; otherwise the schema must fail
  to load. When `type` is absent, a default may be any JSON value.
- **SCHEMA-083:** After insertion, normal type, enum, object, and array validation must continue.

### Validation order

- **SCHEMA-090:** Every field must be processed in this order: (1) check existence and `required`;
  (2) apply `default` to a missing optional value; (3) validate `type`; (4) validate `enum`;
  (5) validate object `properties`; (6) validate array `items`.
- **SCHEMA-091:** If type or enum validation fails for a value, nested validation for that value
  must stop. Independent properties and array items must continue, so a response may contain
  multiple violations.
- **SCHEMA-092:** Violation paths must use JSON root notation, such as `$.customer.name` and
  `$.items[0].amount`.

### Startup sample payload

- **SCHEMA-100:** Startup warmup must derive a deterministic value from each field with this
  precedence: (1) explicit `default`, including `null`; (2) first `enum` member; (3) the type seed:
  `""` for `string`, `0` for `number`, `false` for `boolean`, a recursively generated object,
  `[]` for `array`, or `null`; (4) `null` for an untyped field.
- **SCHEMA-101:** Known object properties must be included recursively, and arrays must warm with an
  empty array.
- **SCHEMA-102:** The generated value must be validated before warmup compilation.
- **SCHEMA-103:** A schema-less template must warm with `{}`.

See [Template lifecycle](../architecture/template-lifecycle.md) for how warmup is used.

### Load-time consistency errors

- **SCHEMA-110:** The following must be rejected while loading and never during request
  validation: invalid YAML or JSON syntax; missing or unsupported `schemaVersion`; missing or
  incorrectly shaped `request.body`; an invalid `type`; `properties` without `type: object`;
  `items` without `type: array`; a typed default with the wrong type; mixed enum member types; an
  enum member incompatible with the declared type.

## Non-goals

Union types (see `SCHEMA-041`) and validation of anything other than the request body.

## Related documents

- [HTTP API](http-api.md) for how violations appear in responses.
- [Template layout](templates.md) for schema file selection.
