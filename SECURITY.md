# Security Policy

## Reporting a vulnerability

Please report vulnerabilities privately through GitHub's
[private vulnerability reporting](https://github.com/talis-fb/typrest/security/advisories/new).
Do not open a public issue for a security problem.

Include the affected version, a description of the impact, and the steps or template needed to
reproduce it. You can expect an acknowledgement, and a fix or mitigation plan once the report is
confirmed.

## Supported versions

Only the latest released version receives security fixes.

## Deployment model

Typrest has no authentication and no render timeout. Run it behind a trusted boundary, such as a
private network or a reverse proxy that handles authentication and rate limiting.

- `typrest serve` binds to `127.0.0.1` by default. Pass `--hostname 0.0.0.0` only when the network
  in front of the server is trusted, as in a container.
- The metadata routes and the optional Web UI are unauthenticated.
- Templates are trusted input that is compiled on the server. Only load templates you control.
- Package resolution downloads literal `@preview` packages from the Typst Universe during startup.
