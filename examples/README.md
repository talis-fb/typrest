# Examples

Each folder is a complete Typrest project you can run with `typrest check` and `typrest serve --watch`
from inside it, except [`clients`](clients/README.md), which calls a running server. Every project
documents a sample request and a deliberately invalid one.

| Example | Shows |
|---|---|
| [`minimal`](minimal/README.md) | The smallest valid project: one template, a two-field schema, one `curl`. Start here. |
| [`invoice`](invoice/README.md) | A full multi-page invoice: calculated totals, shared module, bundled logo, status watermark. |
| [`receipt`](receipt/README.md) | An 80mm auto-height page, arrays with defaults inside items, enum, number formatting. |
| [`report`](report/README.md) | Long-form layout: table of contents, running header, "page X of Y", repeating table headers. |
| [`resume`](resume/README.md) | Deeply nested schema, optional sections hidden through defaults, accent enum. |
| [`certificate`](certificate/README.md) | Bundled OFL fonts, landscape page, SVG asset, per-template fonts. |
| [`letters`](letters/README.md) | Several templates in one project with grouped routes (`/letters/formal`) and a shared letterhead. |
| [`packages`](packages/README.md) | Pinned `@preview` packages (cetz charts), preparation-time download, no network at render time. |
| [`ticket`](ticket/README.md) | A small custom page size, a QR code from a package, and how `null` is validated. |
| [`i18n`](i18n/README.md) | Payload-driven localization (labels, dates, numbers) with no runtime library. |
| [`schema-showcase`](schema-showcase/README.md) | Every Schema v1 feature, plus the exact error for each kind of invalid payload. |
| [`clients`](clients/README.md) | Calling the HTTP API from `curl`, Node and Python, and handling the error envelope. |

`packages` and `ticket` download Typst packages on the first `check` or `serve`, so they need network
access once. See the [template layout specification](../docs/specs/templates.md).
