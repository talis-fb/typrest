# Template Layout Specification

- **Status:** Current
- **Scope:** How filesystem-mode templates are discovered, identified, and bundled, and how templates
  import packages and fonts. Implemented by `crates/typrest-core/src/template_id.rs` and
  `crates/typrest/src/loader/`.

## Purpose

A template's folder path is its public identity. This specification defines which folders become
templates, what each template can see, and what authors may import.

## Requirements

### Identity

- **TEMPLATE-001:** A template ID must be a slash-separated sequence of one or more segments.
- **TEMPLATE-002:** A segment must be non-empty and contain only ASCII letters, digits, `-`, or `_`.
- **TEMPLATE-003:** A segment must not begin with `_`. Leading-underscore paths are reserved for
  Typrest.
- **TEMPLATE-004:** The ID of a filesystem template must equal its path relative to
  `templates_root`. It is also the render route (`POST /{*template_id}`).

  Valid: `invoice`, `invoice-v2`, `invoice_v2`, `financial/invoice`. Invalid: `../invoice`,
  `invoice//v2`, `_shared`, names containing spaces, and non-ASCII names.

### Discovery

- **TEMPLATE-010:** Discovery must walk `templates_root` recursively in deterministic ID order.
- **TEMPLATE-011:** A non-root directory containing `main.typ` must define a template, and
  discovery must stop beneath it. Directories without `main.typ` only group route segments.
- **TEMPLATE-012:** Discovery must skip directories whose names begin with `_`.
- **TEMPLATE-013:** A `main.typ` directly in `templates_root` must fail discovery.
- **TEMPLATE-014:** A visited directory whose name does not begin with `_` and is not a valid ID
  segment must fail discovery, whether or not it contains a template.
- **TEMPLATE-015:** An empty `templates_root` must produce an empty registry and may start
  successfully.

### Bundle contents

- **TEMPLATE-020:** The template directory must be the source root. All descendant files,
  including underscore directories and nested `main.typ` files, must be part of its source with
  paths relative to that directory. Nested `main.typ` files are source, not templates.
- **TEMPLATE-021:** The entrypoint must be `main.typ`.
- **TEMPLATE-022:** Files in `templates_root/_shared/` must be added to every template's source
  under the `_shared/` prefix. When a path collides with a template's own file, the shared file
  must win.
- **TEMPLATE-023:** A change to `_shared/` or to route topology must be treated as affecting every
  template in watch mode. See [template lifecycle](../architecture/template-lifecycle.md).
- **TEMPLATE-024:** Template source must be self-contained: Typst must not read the physical
  filesystem during compilation.

### Schema files

- **TEMPLATE-030:** A template may contain `schema.yml` or the legacy `schema.json`, not both; a
  template containing both must fail to load.
- **TEMPLATE-031:** `schema.yaml` must fail to load with an instruction to rename it.
- **TEMPLATE-032:** A template without a schema file must accept any valid JSON payload.

  The schema format is defined in [Schema v1](schema-v1.md).

### Fonts

- **TEMPLATE-040:** Lowercase `.ttf` and `.otf` files anywhere in a template's source must be
  available as fonts to that template only. Fonts under `_shared/` are available to every template.
- **TEMPLATE-041:** Typrest must not discover operating-system fonts.

### Payload access

- **TEMPLATE-050:** The request payload must be available to Typst as the input `data`, a JSON
  string. Templates read it with:

  ```typst
  #let data = json(bytes(sys.inputs.data))
  ```

### Packages

- **TEMPLATE-060:** A template may import packages with literal, fully versioned `@preview`
  imports, such as `#import "@preview/cetz:0.5.2"`. Direct and transitive imports must resolve.
- **TEMPLATE-061:** Other namespaces, imports without an exact version, and dynamic import
  expressions must not be supported and must fail preparation.
- **TEMPLATE-062:** Missing packages must be downloaded during `check`, startup, and watch
  preparation into `<project>/.typrest/packages`. Failure must fail preparation.
- **TEMPLATE-063:** Render requests must not read the package cache or access the network.
- **TEMPLATE-064:** `.typrest/` is generated and should not be committed.

## Example

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
