FROM rust:1-bookworm AS builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin papelon

FROM debian:bookworm-slim

WORKDIR /app
COPY --from=builder /app/target/release/papelon /usr/local/bin/papelon
COPY examples/templates /app/templates

EXPOSE 8080
CMD ["papelon", "serve", "/app/templates", "8080"]
