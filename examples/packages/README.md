# Packages example project

Renders a bar chart and summary table from a payload, using two `@preview` packages from
[Typst Universe](https://typst.app/universe): `@preview/cetz:0.4.2` and `@preview/cetz-plot:0.1.3`.

```text
typrest.toml
.gitignore                 # excludes the generated .typrest/ package cache
templates/
  sales-chart/             # template ID and route: POST /sales-chart
    main.typ
    schema.yml
    payload.json           # sample request body
```

## Run it

The first `check` or `serve` needs network access to download packages.

```bash
cd examples/packages
typrest check
typrest serve --watch
```

```bash
curl -X POST http://localhost:8080/sales-chart \
  -H 'content-type: application/json' \
  --data @templates/sales-chart/payload.json \
  -o sales-chart.pdf
```

## What it demonstrates

- **Preparation-time downloads** (REQ-TPL-062): the first `typrest check` installs `cetz`, `cetz-plot`,
  and the transitive dependency `oxifmt` into `.typrest/packages` (about 1.5 s here). Later runs
  reuse the cache and finish in well under a second. `.typrest/` is generated and not committed
  (REQ-TPL-064).
- **No network at render time** (REQ-TPL-063): requests never touch the cache or the network. Verified by
  starting the server in an empty network namespace (`unshare -rn`) after the cache was populated;
  `POST /sales-chart` still returned 200.
- **Literal, exact versions** (REQ-TPL-060, REQ-TPL-061): the dependency set is fixed by the source, so
  builds are reproducible and preparation can resolve everything before the server listens.
  Only `@preview` is supported. Messages reported by `typrest check` for bad imports:

  | Import | Error |
  |---|---|
  | `@preview/cetz:9.9.9` | `failed to download package '@preview/cetz:9.9.9': package found, but version 9.9.9 does not exist (latest is 0.5.3)` |
  | `@preview/cetz` | `invalid package import '@preview/cetz': package specification is missing version` |
  | `@preview/cetz:^0.4.2` | ``invalid package import '@preview/cetz:^0.4.2': `^0` is not a valid major version`` |
  | `@local/mine:0.1.0` | `package namespace 'local' is unsupported; only '@preview' packages are supported` |
  | `@preview/no-such-pkg:0.1.0` | `failed to download package '@preview/no-such-pkg:0.1.0': package not found (searched for @preview/no-such-pkg:0.1.0)` |

  All of them exit with status 1.
- **Warm compilation with defaults**: startup compiles each template with an empty payload, so `series`
  is `[]`. The chart package fails on empty data, so the template draws the chart only when
  `series` has rows.

## Invalid payload

A non-numeric `value`:

```bash
curl -X POST http://localhost:8080/sales-chart -H 'content-type: application/json' \
  --data '{"title":"T","period":"P","series":[{"label":"A","value":"ten"}]}'
```

returns `422`:

```json
{"error":"schema_validation_failed","message":"Payload validation failed","violations":[{"field":"$.series[0].value","issue":"expected type number, got string"}]}
```

## Customizing

Change the bar color in `main.typ`, or upgrade a package by changing its exact version in the
import (check that the version exists on Universe; `cetz-plot` versions pin the `cetz` they need).
See [Template layout](../../docs/specs/templates.md) for the package requirements.
