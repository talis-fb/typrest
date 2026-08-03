# ADR-0005: Inject Request Data Through `sys.inputs`

- **Date:** 2025-11
- **Status:** Accepted

## Context

Writing a temporary JSON file for every render would introduce avoidable disk I/O and leak
filesystem concerns into request compilation.

## Decision

Serialize the JSON payload into the Typst input named `data` instead of writing a temporary JSON
file.

## Alternatives

- Write an `input.json` file per render and load it from the template.

## Consequences

- Rendering performs no per-request input-file I/O.
- Templates that consume request data use:

  ```typst
  #let data = json(bytes(sys.inputs.data))
  ```

- `typrest init` includes this line in generated templates.
- The compiler builds a fresh request-specific Typst library while reusing prepared template state.
