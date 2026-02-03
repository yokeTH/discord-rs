FROM --platform=$BUILDPLATFORM rust:bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libssl3 \
    ca-certificates \
    python3 \
    curl \
    git \
    clang \
    && rm -rf /var/lib/apt/lists/*

COPY rust-toolchain.toml ./
COPY Cargo.toml Cargo.lock ./
COPY apps/bot/Cargo.toml apps/bot/Cargo.toml
COPY libs/stock/Cargo.toml libs/stock/Cargo.toml

RUN mkdir -p apps/bot/src libs/stock/src \
    && printf "fn main(){}" > apps/bot/src/main.rs \
    && printf "fn _stock(){}" > libs/stock/src/lib.rs

RUN cargo build -p bot --release

COPY . .

RUN cargo build -p bot --release

RUN mkdir -p /out/lib /out/etc/ssl/certs \
    && cp /app/target/release/bot /out/bot \
    && cp /etc/ssl/certs/ca-certificates.crt /out/etc/ssl/certs/ca-certificates.crt \
    && cp /usr/lib/*/libssl.so.3 /out/lib/ \
    && cp /usr/lib/*/libcrypto.so.3 /out/lib/ \
    && (ldd /app/target/release/bot || true)

FROM gcr.io/distroless/cc-debian12
ARG APP_VERSION
ENV APP_VERSION=$APP_VERSION

COPY --from=builder /out/etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

COPY --from=builder /out/lib/libssl.so.3 /usr/lib/libssl.so.3
COPY --from=builder /out/lib/libcrypto.so.3 /usr/lib/libcrypto.so.3

COPY --from=builder /out/bot /bot

ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt

USER 65532:65532
ENTRYPOINT ["/bot"]
