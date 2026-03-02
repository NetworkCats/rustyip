FROM rust:1.92-slim AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY static ./static

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd -r rustyip && useradd -r -g rustyip rustyip

COPY --from=builder /build/target/release/rustyip /usr/local/bin/rustyip
COPY --from=builder /build/templates /app/templates
COPY --from=builder /build/static /app/static

WORKDIR /app

USER rustyip

ENTRYPOINT ["rustyip"]
