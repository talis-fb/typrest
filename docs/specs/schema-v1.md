# Schema v1 specification (`schema.yml`)

- **Status:** Current
- **Scope:** The payload schema format, validation semantics, and the startup sample payload,
  implemented by the v1 parser and validator in `typrest-core`.

## Purpose

A template may include a schema to validate and default its JSON request body. Without a schema,
Typrest accepts any valid JSON value and injects it directly through `sys.inputs.data`.

## Requirements

### File rules

File discovery and conflicts are defined in [Template layout](templates.md)
(`REQ-TPL-030` to `REQ-TPL-032`).

- **REQ-SCH-001:** `schema.yml` is canonical and MUST use YAML syntax. Legacy `schema.json` MUST be
  accepted with JSON syntax.
- **REQ-SCH-002:** The schema file MUST be optional.
- **REQ-SCH-003:** YAML input MUST contain exactly one document and MAY contain comments.
- **REQ-SCH-004:** Duplicate mapping keys MUST be invalid.
- **REQ-SCH-005:** Only `true` and `false` MUST use YAML boolean semantics; values such as `on` and
  `off` remain strings.

### Document shape

When a schema file is present, it MUST contain:

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

- **REQ-SCH-010:** `schemaVersion` MUST be present and exactly `"v1"`. A missing or unsupported
  version MUST be a `SchemaVersionError`.
- **REQ-SCH-011:** `request` and `request.body` MUST be present. Version v1 defines request-body
  validation only.
- **REQ-SCH-012:** Other syntax, type, or structural failures MUST be `SchemaError` values detected
  while loading the template.
- **REQ-SCH-013:** Unknown keys at the root, under `request`, and inside fields MUST be
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

- **REQ-SCH-020:** All field keys MUST be optional.
- **REQ-SCH-021:** An empty field (`{}`) MUST accept any existing value.

### Required and missing values

- **REQ-SCH-030:** `required` MUST check existence, not truthiness or nullability.
- **REQ-SCH-031:** A missing property with `required: true` MUST produce `required field missing`.
- **REQ-SCH-032:** A missing optional property MUST be omitted unless it has a default.
- **REQ-SCH-033:** An explicit `null` exists and MUST continue through type and enum validation.
- **REQ-SCH-034:** Required checking MUST happen before default application, so a required property
  is still required when a default is also declared.
- **REQ-SCH-035:** `required` is meaningful only for properties nested under an object schema. The
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

- **REQ-SCH-040:** An invalid `type` value MUST be a load-time schema error.
- **REQ-SCH-041:** Only one type MAY be declared. Type arrays and union types MUST NOT be supported.
- **REQ-SCH-042:** Without `type`, no type check MUST occur, although `enum` and `default` rules
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

- **REQ-SCH-050:** `properties` MUST validate declared members recursively.
- **REQ-SCH-051:** `properties` is valid only with `type: object`. Using it with another type, or
  without `type`, MUST be a load-time schema error.
- **REQ-SCH-052:** Undeclared payload properties MUST be retained and MUST NOT cause failures.
- **REQ-SCH-053:** Declared properties MUST be processed in name order.

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

- **REQ-SCH-060:** `items` MUST apply one recursive field definition to every array element.
- **REQ-SCH-061:** `items` is valid only with `type: array`. Using it with another type, or without
  `type`, MUST be a load-time schema error.
- **REQ-SCH-062:** An empty array MUST be valid.

### Enums

```yaml
type: string
enum: [BRL, USD]
```

- **REQ-SCH-070:** `enum` MUST restrict an existing value by exact JSON equality.
- **REQ-SCH-071:** All enum members MUST have the same JSON type, and when `type` is present every
  member MUST match it. Violations MUST be load-time schema errors.
- **REQ-SCH-072:** An empty enum is structurally valid; no existing value can satisfy it.

### Defaults

```yaml
type: object
properties:
  displayName:
    type: string
    default: anonymous
```

Input `{}` becomes `{"displayName":"anonymous"}` before compilation.

- **REQ-SCH-080:** A default MUST supply a property that is missing and optional. This MUST apply at any depth,
  including properties of objects inside array items.
- **REQ-SCH-081:** A default MUST never replace an explicit `null`.
- **REQ-SCH-082:** When `type` is present, the default MUST match it; otherwise the schema MUST fail
  to load. When `type` is absent, a default MAY be any JSON value.
- **REQ-SCH-083:** After insertion, normal type, enum, object, and array validation MUST continue.

### Validation order

- **REQ-SCH-090:** Every field MUST be processed in this order: (1) check existence and `required`;
  (2) apply `default` to a missing optional value; (3) validate `type`; (4) validate `enum`;
  (5) validate object `properties`; (6) validate array `items`.
- **REQ-SCH-091:** If type or enum validation fails for a value, nested validation for that value
  MUST stop. Independent properties and array items MUST continue, so a response MAY contain
  multiple violations.
- **REQ-SCH-092:** Violation paths MUST use JSON root notation, such as `$.customer.name` and
  `$.items[0].amount`.

### Startup sample payload

- **REQ-SCH-100:** Startup warmup MUST derive a deterministic value from each field with this
  precedence: (1) explicit `default`, including `null`; (2) first `enum` member; (3) the type seed:
  `""` for `string`, `0` for `number`, `false` for `boolean`, a recursively generated object,
  `[]` for `array`, or `null`; (4) `null` for an untyped field.
- **REQ-SCH-101:** Known object properties MUST be included recursively, and arrays MUST warm with an
  empty array.
- **REQ-SCH-102:** The generated value MUST be validated before warmup compilation.
- **REQ-SCH-103:** A schema-less template MUST warm with `{}`.

See [Template lifecycle](../architecture/template-lifecycle.md) for how warmup is used.

### Load-time consistency errors

- **REQ-SCH-110:** The following MUST be rejected while loading and never during request
  validation: invalid YAML or JSON syntax; missing or unsupported `schemaVersion`; missing or
  incorrectly shaped `request.body`; an invalid `type`; `properties` without `type: object`;
  `items` without `type: array`; a typed default with the wrong type; mixed enum member types; an
  enum member incompatible with the declared type.

## Examples

The schema in [Document shape](#document-shape) is a complete minimal example. It agrees with
`REQ-SCH-010` and `REQ-SCH-011`.

## Non-goals

Union types (see `REQ-SCH-041`) and validation of anything other than the request body.

## Related documents

- [HTTP API](http-api.md) for how violations appear in responses.
- [Template layout](templates.md) for schema file selection.
