# Changelog

All notable changes to Typrest are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `typrest --version` prints the package version.
- Request body limit (`[server].max_body_bytes`) with a JSON `413 payload_too_large` response.
- Graceful shutdown on `SIGTERM` and `SIGINT` with a bounded drain
  (`[server].shutdown_timeout_seconds`).

### Changed

- `typrest serve` binds to `127.0.0.1` by default. The container image and `docker-compose.yml` pass
  `--hostname 0.0.0.0` explicitly.
