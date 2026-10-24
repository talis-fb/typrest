# Specifications

Specifications define the current, externally observable behavior of Typrest. They describe what
the system does, not how it is built; implementation belongs in
[architecture documents](../architecture/overview.md). Tests and code are the executable truth: when
a specification disagrees with them, verify the behavior, then fix the document in the same change.

| Specification | Domain | Covers |
|---|---|---|
| [CLI](cli.md) | `REQ-CLI` | Commands, options, exit status |
| [`typrest.toml`](typrest-toml.md) | `REQ-CFG` | Project configuration |
| [Template layout](templates.md) | `REQ-TPL` | Discovery, identity, bundles, fonts, packages |
| [Schema v1](schema-v1.md) | `REQ-SCH` | Payload schema and validation |
| [HTTP API](http-api.md) | `REQ-HTTP` | Routes, responses, errors, security |
| [Compiler diagnostics](diagnostics.md) | `REQ-DIAG` | Diagnostic model and output |
| [Logging](logging.md) | `REQ-LOG` | Process streams and log formats |
| [Web UI](webui.md) | `REQ-WEB` | Development UI behavior |

Every specification follows [`template.md`](template.md) and carries requirement IDs. The rules are
in [Requirement IDs](../README.md#requirement-ids). Use the template for a new specification.
