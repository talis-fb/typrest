# ADR-0005: Derive Template IDs and Direct Routes from Folders

- **Date:** 2026-08
- **Status:** Accepted

## Context

Template names were previously a single flat ASCII segment (letters, digits, `-`, `_`) rendered
under `/render`. Flat names force every filesystem template into one directory level and expose
renders through an extra `/render` prefix. A folder-derived convention makes route ownership visible in the
project tree and gives templates familiar application-style paths. The convention must preserve
self-contained compiler bundles, reserve room for internal HTTP endpoints, and avoid overlapping
filesystem template roots before Store mode is designed.

## Decision

Represent template identity with `TemplateId`, a canonical slash-separated sequence of safe ASCII
segments. Segments may contain ASCII alphanumerics, `-`, and `_`, but may not be empty or begin with
`_`. Leading-underscore URL paths are reserved for Typrest internals.

Filesystem discovery walks non-underscore directories recursively. A non-root directory containing
`main.typ` defines its relative path as a template ID and stops route discovery below that point.
Nested entrypoints below the selected root are silently treated as source. `templates/main.typ` and
invalid visible route segments fail discovery. Once selected, the template directory becomes the
virtual source root and all descendant files, including underscore directories, are bundled.

Render templates directly with `POST /{*template_id}`. Keep health and metadata under `/_healthz`
and `/_api`; move the optional Web UI to `/_webui`. When enabled, `GET /` redirects to `/_webui/`.
Metadata identifies templates with `id` and uses a catch-all detail route.

Keep root `templates/_shared/` resolution in the filesystem loader under ADR-0001. Global shared
files are inserted after local files and retain the existing same-path precedence. Route-topology
and shared changes build and atomically swap a complete candidate registry in watch mode.

## Alternatives

- Keep flat names or retain `/render`, which preserves an API namespace but loses the direct
  folder-to-route model.
- Register every nested `main.typ`, which creates overlapping filesystem source roots.
- Fail on nested entrypoints, which prevents templates from carrying arbitrary descendant source.
- Treat underscore folders as routes, which would force a permanent collision list for internal
  endpoints.

## Consequences

- Filesystem layout and public render paths match directly.
- Adding or removing an ancestor `main.typ` can replace an entire route subtree, so watch updates
  require atomic registry reconciliation.
- Template IDs remain logical storage keys; future Store refs and versions stay separate.
- Store mode can return the same self-contained bundle shape without reproducing grouping folders.
- `/render/*` and `/webui/*` have no compatibility aliases in this pre-release contract.
