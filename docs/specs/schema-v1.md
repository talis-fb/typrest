# Schema Specification (`schema.yml`) Version 1

> Status: Normative for the implemented v1 parser and validator in `typrest-core`.

## Purpose and File Rules

A template may include a schema to validate and default its JSON request body. Without a schema,
Typrest accepts any valid JSON value and injects it directly through `sys.inputs.data`.

File selection rules:

- `schema.yml` is canonical and uses YAML syntax;
- legacy `schema.json` is accepted with JSON syntax;
- a template containing both files fails to load;
- the unsupported spelling `schema.yaml` fails with an instruction to rename it;
- the schema file itself is optional.

YAML input must contain exactly one document and may contain comments. Duplicate mapping keys are
invalid. Only `true` and `false` use YAML boolean semantics; values such as `on` and `off` remain
strings.

## Document Shape

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

Version v1 defines request-body validation only. Missing `schemaVersion` is a
`SchemaVersionError`; an unsupported version is also a `SchemaVersionError`. Other syntax, type,
or structural failures are `SchemaError` values detected while loading the template.

Unknown keys are currently ignored without a warning at the root, under `request`, and inside
fields.

## `SchemaField`

Every location that accepts a schema field uses the same recursive shape:

| Field | Type | Default | Constraint |
|---|---|---|---|
| `required` | boolean | `false` | Checks whether an object property exists |
| `type` | string | absent | One of `string`, `number`, `boolean`, `object`, `array`, `null` |
| `properties` | object of fields | absent | Allowed only with `type: object` |
| `items` | field | absent | Allowed only with `type: array` |
| `enum` | array of JSON values | absent | Values must have one JSON type |
| `default` | any JSON value | absent | Must match `type` when `type` is present |

All fields are optional. An empty field accepts any existing value:

```yaml
{}
```

## Required and Missing Values

`required` checks existence, not truthiness or nullability.

- A missing property with `required: true` produces `required field missing`.
- A missing optional property is omitted unless it has a default.
- An explicit `null` exists and continues through type and enum validation.
- Required checking happens before default application, so a required property is still required
  even if a default is also declared.

`required` is meaningful for properties nested under an object schema. The request body itself is
always present once the HTTP handler has parsed it.

## Types

Supported values are:

| Schema type | Accepted JSON value |
|---|---|
| `string` | string |
| `number` | integer or non-integer JSON number |
| `boolean` | `true` or `false` |
| `object` | object |
| `array` | array |
| `null` | `null` |

Only one type may be declared. Type arrays or union types are unsupported. Without `type`, no type
check occurs, although `enum` and `default` rules still apply.

## Objects and Properties

`properties` recursively validates declared members and is valid only with `type: object`:

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

Undeclared payload properties are retained and do not cause validation failures. Declared
properties are processed in name order because schemas use an ordered map.

Using `properties` with any other type, or without `type`, is a load-time schema error.

## Arrays and Items

`items` applies one recursive field definition to every array element and is valid only with
`type: array`:

```yaml
type: array
items:
  type: object
  properties:
    amount:
      type: number
      required: true
```

An empty array is valid. Using `items` with any other type, or without `type`, is a load-time schema
error.

## Enums

`enum` restricts an existing value by exact JSON equality:

```yaml
type: string
enum: [BRL, USD]
```

Rules:

- all enum members must have the same JSON type;
- when `type` is present, every member must match it;
- an empty enum is structurally valid but no existing value can satisfy it;
- mixed types or a member incompatible with `type` are load-time schema errors.

## Defaults

A default supplies an optional property that is missing:

```yaml
type: object
properties:
  displayName:
    type: string
    default: anonymous
```

Input `{}` becomes `{"displayName":"anonymous"}` before compilation. Defaults never replace an
explicit `null`. When `type` is present, the default must match it; otherwise the schema fails to
load. A default may be any JSON value when `type` is absent.

After insertion, normal type, enum, object, and array validation continues.

## Validation Order

Every field is processed in this order:

1. check existence and `required`;
2. apply `default` to a missing optional value;
3. validate `type`;
4. validate `enum`;
5. validate object `properties`;
6. validate array `items`.

If type or enum validation fails for a value, nested validation for that value stops. Independent
properties and array items continue so a response can contain multiple violations. Paths use JSON
root notation such as `$.customer.name` and `$.items[0].amount`.

## Startup Sample Payload

Startup warmup derives a deterministic value from each field:

1. explicit default;
2. first enum member;
3. type seed: `""`, `0`, `false`, recursively generated object, `[]`, or `null`;
4. `null` for an untyped field.

This generated value is validated before warmup compilation. A schema-less template warms with
`{}`. See [Startup Template Compilation](startup-template-compilation.md).

## Load-time Consistency Errors

The following are rejected while loading, never during request validation:

- invalid YAML or JSON syntax;
- missing or unsupported `schemaVersion`;
- missing or incorrectly shaped `request.body`;
- an invalid `type` value;
- `properties` without `type: object`;
- `items` without `type: array`;
- a typed default with the wrong type;
- mixed enum member types;
- an enum member incompatible with the declared type.
