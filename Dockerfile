FROM rust:1-bookworm AS builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin typrest

FROM debian:bookworm-slim

WORKDIR /app
COPY --from=builder /app/target/release/typrest /usr/local/bin/typrest
COPY examples/templates /app/templates

EXPOSE 8080
CMD ["typrest", "serve", "/app/templates", "8080"]
