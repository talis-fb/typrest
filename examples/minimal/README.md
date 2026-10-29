# Minimal example project

The smallest valid Typrest project: one template, no shared code, no assets.

```text
typrest.toml
templates/
  hello/                   # template ID and route: POST /hello
    main.typ
    schema.yml
    payload.json           # sample request body
```

## 60 seconds to a PDF

Either copy this folder, or generate a project with `typrest init my-project` (it creates
`typrest.toml` and a `hello-world` template). See the [CLI specification](../../docs/specs/cli.md).

```bash
cd examples/minimal
typrest check            # compile every template; exits 0 when all are valid
typrest serve --watch    # listens on http://localhost:8080
```

In another terminal:

```bash
curl -X POST http://localhost:8080/hello \
  -H 'content-type: application/json' \
  --data @templates/hello/payload.json \
  -o hello.pdf
```

## Invalid payloads

A missing `name` returns `422`:

```bash
curl -i -X POST http://localhost:8080/hello -H 'content-type: application/json' \
  --data '{"greeting": "Hi"}'
# {"error":"schema_validation_failed","message":"Payload validation failed",
#  "violations":[{"field":"$.name","issue":"required field missing"}]}
```

A `greeting` outside the enum also returns `422`; omitting it is fine and defaults to `Hello`:

```bash
curl -i -X POST http://localhost:8080/hello -H 'content-type: application/json' \
  --data '{"name": "Ada", "greeting": "Yo"}'
# "violations":[{"field":"$.greeting","issue":"invalid enum value \"Yo\""}]
```

## What it demonstrates

- The required files: `typrest.toml`, and per template `main.typ` plus `schema.yml`.
- `name` is `required`; `greeting` is an enum with a `default`. See
  [Schema v1](../../docs/specs/schema-v1.md).
- The template reads its payload with `json(bytes(sys.inputs.data))`. See
  [Template layout](../../docs/specs/templates.md).
