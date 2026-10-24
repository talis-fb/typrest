# Template layout specification

- **Status:** Current
- **Scope:** How filesystem-mode templates are discovered, identified, and bundled, and how templates
  import packages and fonts. Implemented by `crates/typrest-core/src/template_id.rs` and
  `crates/typrest/src/loader/`.

## Purpose

A template's folder path is its public identity. This specification defines which folders become
templates, what each template can see, and what authors may import.

## Requirements

### Identity

- **REQ-TPL-001:** A template ID MUST be a slash-separated sequence of one or more segments.
- **REQ-TPL-002:** A segment MUST be non-empty and contain only ASCII letters, digits, `-`, or `_`.
- **REQ-TPL-003:** A segment MUST NOT begin with `_`. Leading-underscore paths are reserved for
  Typrest.
- **REQ-TPL-004:** The ID of a filesystem template MUST equal its path relative to
  `templates_root`. It is also the render route (`POST /{*template_id}`).

  Valid: `invoice`, `invoice-v2`, `invoice_v2`, `financial/invoice`. Invalid: `../invoice`,
  `invoice//v2`, `_shared`, names containing spaces, and non-ASCII names.

### Discovery

- **REQ-TPL-010:** Discovery MUST walk `templates_root` recursively in deterministic ID order.
- **REQ-TPL-011:** A non-root directory containing `main.typ` MUST define a template, and
  discovery MUST stop beneath it. Directories without `main.typ` only group route segments.
- **REQ-TPL-012:** Discovery MUST skip directories whose names begin with `_`.
- **REQ-TPL-013:** A `main.typ` directly in `templates_root` MUST fail discovery.
- **REQ-TPL-014:** A visited directory whose name does not begin with `_` and is not a valid ID
  segment MUST fail discovery, whether or not it contains a template.
- **REQ-TPL-015:** An empty `templates_root` MUST produce an empty registry and MAY start
  successfully.

### Bundle contents

- **REQ-TPL-020:** The template directory MUST be the source root. All descendant files,
  including underscore directories and nested `main.typ` files, MUST be part of its source with
  paths relative to that directory. Nested `main.typ` files are source, not templates.
- **REQ-TPL-021:** The entrypoint MUST be `main.typ`.
- **REQ-TPL-022:** Files in `templates_root/_shared/` MUST be added to every template's source
  under the `_shared/` prefix. When a path collides with a template's own file, the shared file
  MUST win.
- **REQ-TPL-023:** A change to `_shared/` or to route topology MUST be treated as affecting every
  template in watch mode. See [template lifecycle](../architecture/template-lifecycle.md).
- **REQ-TPL-024:** Template source MUST be self-contained: Typst MUST NOT read the physical
  filesystem during compilation.

### Schema files

- **REQ-TPL-030:** A template MAY contain `schema.yml` or the legacy `schema.json`, not both; a
  template containing both MUST fail to load.
- **REQ-TPL-031:** `schema.yaml` MUST fail to load with an instruction to rename it.
- **REQ-TPL-032:** A template without a schema file MUST accept any valid JSON payload.

  The schema format is defined in [Schema v1](schema-v1.md).

### Fonts

- **REQ-TPL-040:** Lowercase `.ttf` and `.otf` files anywhere in a template's source MUST be
  available as fonts to that template only. Fonts under `_shared/` are available to every template.
- **REQ-TPL-041:** Typrest MUST NOT discover operating-system fonts.

### Payload access

- **REQ-TPL-050:** The request payload MUST be available to Typst as the input `data`, a JSON
  string. Templates read it with:

  ```typst
  #let data = json(bytes(sys.inputs.data))
  ```

### Packages

- **REQ-TPL-060:** A template MAY import packages with literal, fully versioned `@preview`
  imports, such as `#import "@preview/cetz:0.5.2"`. Direct and transitive imports MUST resolve.
- **REQ-TPL-061:** Other namespaces, imports without an exact version, and dynamic import
  expressions MUST NOT be supported and MUST fail preparation.
- **REQ-TPL-062:** Missing packages MUST be downloaded during `check`, startup, and watch
  preparation into `<project>/.typrest/packages`. Failure MUST fail preparation.
- **REQ-TPL-063:** Render requests MUST NOT read the package cache or access the network.
- **REQ-TPL-064:** `.typrest/` is generated and SHOULD NOT be committed.

## Examples

```text
templates/
  financial/
    invoice/             # template ID: financial/invoice
      main.typ
      schema.yml
      assets/logo.svg
  welcome/               # template ID: welcome
    main.typ
  _shared/
    brand.typ            # imported by any template as _shared/brand.typ
```

## Non-goals

Store-backed templates, template versions or tags, overlapping template roots, other package
namespaces, custom registries, and package lockfiles.

## Related documents

- [ADR-0001](../adr/0001-resolve-shared-during-loading.md): why `_shared/` is resolved by the loader.
- [ADR-0002](../adr/0002-inject-request-data-through-sys-inputs.md): why data is passed as an input.
- [ADR-0005](../adr/0005-derive-template-ids-from-folder-routes.md): why IDs come from folders.
- [ADR-0006](../adr/0006-resolve-preview-packages-during-preparation.md): why packages resolve
  during preparation.
- [Compiler architecture](../architecture/compiler.md).
