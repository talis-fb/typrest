# Schema Specification (`schema.json`) - Version 1

## Goal

The optional `schema.json` file defines the expected contract for a JSON value. A template without this file accepts any JSON payload and receives it directly through `sys.inputs.data`; no payload validation is performed.

In version `v1`, only request body validation (`request.body`) is supported.

Every schema is composed of a root object containing a version and a schema for the body.

The `schemaVersion` field is required.

Example:

```json
{
  "schemaVersion": "v1",
  "request": {
    "body": {
      "type": "object",
      "properties": {
        "name": {
          "type": "string",
          "required": true
        }
      }
    }
  }
}
```

---

# Structure

```text
schema.json
 ├── schemaVersion (string)
 └── request
      └── body (Schema)
```

The `body` field is always a `Schema`.

Any place in the language that expects a `Schema` uses exactly the same structure.

---

# Schema

A Schema is an object that describes how to validate a single JSON value.

All schema fields are optional.

An empty schema:

```json
{}
```

is valid and accepts any value.

---

# `required` Field

Type:

```text
boolean
```

Default:

```text
false
```

Responsibility:

Determines whether the value must exist.

Rules:

- `required` checks existence only.
- If the value does not exist (`undefined`) and `required` is `false`, the schema is considered valid.
- If the value does not exist and `required` is `true`, validation fails.
- If the value exists (including `null`), validation continues normally.

---

# `type` Field

Type:

```text
string
```

Required:

No.

Supported values:

- string
- number
- boolean
- object
- array
- null

Responsibility:

Checks the value type.

Rules:

- A schema may have only one type.
- Type lists are not allowed.
- If the value exists, its type must exactly match the type defined in `type`.
- If `type` is not provided, no type validation is performed.

---

# `properties` Field

Available only when:

```text
type == object
```

Type:

```text
object<string, Schema>
```

Each property represents the schema of an object field.

Example:

```json
{
  "type": "object",
  "properties": {
    "name": {
      "type": "string"
    },
    "age": {
      "type": "number"
    }
  }
}
```

Using `properties` when `type` is not `object` is a schema error.

---

# `items` Field

Available only when:

```text
type == array
```

Type:

```text
Schema
```

Defines the schema of each array element.

Example:

```json
{
  "type": "array",
  "items": {
    "type": "string"
  }
}
```

Using `items` when `type` is not `array` is a schema error.

---

# `enum` Field

Type:

```text
array
```

Responsibility:

Restricts the value to a fixed set of possibilities.

Rules:

- All enum values must have exactly the same type.
- If `type` is defined, all enum values must have exactly that type.
- If any type mismatch exists, the schema is invalid.
- Comparison is done by exact equality.

Example:

```json
{
  "type": "string",
  "enum": [
    "BRL",
    "USD"
  ]
}
```

Invalid example:

```json
{
  "type": "string",
  "enum": [
    "BRL",
    10
  ]
}
```

---

# `default` Field

Responsibility:

Defines a default value for the field.

Rules:

- The value defined in `default` must have exactly the same type defined in `type`.
- Otherwise, the schema is invalid.
- The default value is applied only when the field does not exist (`undefined`).
- The default value never replaces `null`.
- After applying the default value, validation continues normally.

Example:

Input:

```json
{}
```

Schema:

```json
{
  "type": "string",
  "default": "anonymous"
}
```

Result:

```json
"anonymous"
```

Input:

```json
null
```

Result:

```json
null
```

---

# Validation Order

Every schema must be processed in exactly this order:

1. Check existence (`required`).
2. Apply `default` if the value is `undefined`.
3. Validate type (`type`).
4. Validate `enum`.
5. If `type == object`, validate `properties`.
6. If `type == array`, validate `items`.

No implementation may change this order.

---

# Consistency Rules

The following cases make the schema invalid before execution:

- `properties` used without `type: "object"`.
- `items` used without `type: "array"`.
- `default` incompatible with `type`.
- Any `enum` value incompatible with `type`.
- Values of different types inside `enum`.
- Invalid value for `type`.

These errors must be detected while loading the schema, never during payload validation.
