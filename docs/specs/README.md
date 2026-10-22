# Specifications

Specifications define the current, externally observable behavior of Typrest. They describe what
the system does, not how it is built; implementation belongs in
[architecture documents](../architecture/overview.md). Tests and code are the executable truth: when
a specification disagrees with them, verify the behavior, then fix the document in the same change.

| Specification | Namespace | Covers |
|---|---|---|
| [CLI](cli.md) | `CLI` | Commands, options, exit status |
| [`typrest.toml`](typrest-toml.md) | `CONFIG` | Project configuration |
| [Template layout](templates.md) | `TEMPLATE` | Discovery, identity, bundles, fonts, packages |
| [Schema v1](schema-v1.md) | `SCHEMA` | Payload schema and validation |
| [HTTP API](http-api.md) | `HTTP` | Routes, responses, errors, security |
| [Compiler diagnostics](diagnostics.md) | none | Diagnostic model and output |
| [Logging](logging.md) | none | Process streams and log formats |
| [Web UI](webui.md) | none | Development UI behavior |

Requirement ID rules are in [Requirement IDs](../README.md#requirement-ids). Use
[`template.md`](template.md) for a new specification.
