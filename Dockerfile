# Build stage
FROM rust:1-alpine3.21 AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release

# Production stage
FROM scratch

COPY --from=builder /app/target/release/webhook-forwarder /webhook-forwarder

EXPOSE 8080

ENTRYPOINT ["/webhook-forwarder"]
