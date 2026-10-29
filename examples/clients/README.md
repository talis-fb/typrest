# Clients

Three small clients that call the Typrest HTTP API: `curl.sh`, `client.ts` (Node `fetch`), and
`client.py` (Python `urllib`). None has dependencies. This folder is not a Typrest project and has no
`typrest.toml`.

Each client POSTs the [invoice example](../invoice/README.md) payload to `/invoice`, saves the PDF,
and on a non-2xx response prints the error envelope and exits with status 1.

```text
examples/clients/
  curl.sh      bash + curl (jq is optional, used to pretty-print errors)
  client.ts    Node 20+ fetch, no dependencies
  client.py    Python 3 standard library
```

## Run it

Start the invoice server in one terminal:

```bash
cd examples/invoice
typrest serve --port 8111
```

Then, from `examples/clients`, in another:

```bash
export TYPREST_URL=http://localhost:8111   # default: http://localhost:8080

./curl.sh                                          # writes invoice.pdf
node --experimental-strip-types client.ts          # writes invoice.pdf
python3 client.py                                  # writes invoice.pdf
```

Each script takes optional arguments: `[payload.json] [output.pdf]`. The default payload is
`../invoice/templates/invoice/payload.json`. Verified with Node 24 and
`node --experimental-strip-types client.ts` (Node 22.6+; on Node 20 compile with `tsc` or strip the
types first, since the file uses only type annotations and interfaces).

## Error handling

Failures return a JSON envelope ([`REQ-HTTP-040`](../../docs/specs/http-api.md)) with `error` and
`message`. Schema failures add `violations[]` of `{field, issue}` ([`REQ-HTTP-042`](../../docs/specs/http-api.md));
compilation failures add `diagnostics[]` ([`REQ-HTTP-043`](../../docs/specs/http-api.md)). The clients print all of them.

Try a payload with a string quantity:

```bash
python3 - <<'PY'
import json
d = json.load(open("../invoice/templates/invoice/payload.json"))
d["items"][0]["quantity"] = "two"
json.dump(d, open("/tmp/bad.json", "w"))
PY
./curl.sh /tmp/bad.json
```

Expected output (HTTP 422, exit status 1; identical for all three clients):

```text
request failed: HTTP 422
schema_validation_failed: Payload validation failed
  $.items[0].quantity: expected type number, got string
```

The raw body is:

```json
{"error":"schema_validation_failed","message":"Payload validation failed","violations":[{"field":"$.items[0].quantity","issue":"expected type number, got string"}]}
```

Network failures (server down) print `request error: ...` and also exit 1.

## What it demonstrates

- Sending `Content-Type: application/json`; other types fail with `invalid_json` ([`REQ-HTTP-021`](../../docs/specs/http-api.md)).
- Treating any non-2xx as a structured error rather than writing the body to a `.pdf`.
- Configuring the base URL through `TYPREST_URL`.

## Customizing

Change the template ID in the URL (`/invoice`) and pass another payload to render a different
template. Routes and error codes are in the [HTTP API specification](../../docs/specs/http-api.md).
