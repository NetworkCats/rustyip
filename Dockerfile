# Build Stage
FROM rust:1.93.1-alpine3.23 AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY static ./static

RUN cargo build --release

# Runtime Stage
FROM alpine:3.23.3

RUN apk add --no-cache ca-certificates \
    && addgroup -S -g 1001 rustyip \
    && adduser -S -u 1001 -G rustyip rustyip

COPY --from=builder /build/target/release/rustyip /usr/local/bin/rustyip
COPY --from=builder /build/templates /app/templates
COPY --from=builder /build/static /app/static

WORKDIR /app
RUN chown -R rustyip:rustyip /app

USER rustyip

ENTRYPOINT ["rustyip"]
