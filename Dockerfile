# Typrest container image.
#
# The image serves the bundled examples/minimal project from /project. To serve your own project,
# mount a directory containing typrest.toml over it:
#
#   docker run --rm -p 127.0.0.1:8080:8080 -v "$PWD:/project" typrest
#
# Typrest writes downloaded @preview packages to /project/.typrest, so the mount must be writable
# by the container user (uid 1000 by default; override with --user "$(id -u):$(id -g)").

FROM rust:1-bookworm AS builder

WORKDIR /src
COPY . .
RUN cargo build --release --locked --bin typrest

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --gid 1000 typrest \
    && useradd --uid 1000 --gid typrest --no-create-home --home-dir /project --shell /usr/sbin/nologin typrest

COPY --from=builder /src/target/release/typrest /usr/local/bin/typrest

WORKDIR /project
COPY --chown=typrest:typrest examples/minimal/ /project/

USER typrest
EXPOSE 8080
# Typrest runs as PID 1 and installs its own SIGTERM/SIGINT handlers, so no init process is needed.
# On a stop it drains in-flight requests for [server].shutdown_timeout_seconds (default 25 s), so
# give the container a stop timeout above that (`docker stop -t 30`).
STOPSIGNAL SIGTERM
CMD ["typrest", "serve"]
