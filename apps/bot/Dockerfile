#
# Base (chef)
#
FROM --platform=$BUILDPLATFORM rust:bookworm AS chef
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
    fontconfig \
    fonts-jetbrains-mono \
    unzip \
    wget \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef --locked

#
# Planner
#
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

#
# Cacher
#
FROM chef AS cacher

ARG CACHE_ID=default

COPY --from=planner /app/recipe.json /app/recipe.json

RUN --mount=type=cache,id=${CACHE_ID}-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=${CACHE_ID}-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=${CACHE_ID}-cargo-target,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json

#
# Builder
#
FROM chef AS builder
WORKDIR /app

ARG CACHE_ID=default

ENV FONT_VERSION=3.2.1
ENV FONT_DIR=/usr/share/fonts/JetBrainsMono

RUN mkdir -p "${FONT_DIR}" \
    && wget -O /tmp/JetBrainsMono.zip \
    "https://github.com/ryanoasis/nerd-fonts/releases/download/v${FONT_VERSION}/JetBrainsMono.zip" \
    && unzip -q /tmp/JetBrainsMono.zip -d "${FONT_DIR}" \
    && rm -f /tmp/JetBrainsMono.zip \
    && fc-cache -fv

COPY . .

RUN --mount=type=cache,id=${CACHE_ID}-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=${CACHE_ID}-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=${CACHE_ID}-cargo-target,target=/app/target \
    cargo build -p bot --release \
    && mkdir -p \
    /out/lib \
    /out/etc/ssl/certs \
    /out/usr/share/fonts \
    /out/etc/fonts \
    /out/var/cache/fontconfig \
    && cp /app/target/release/bot /out/bot \
    && cp /etc/ssl/certs/ca-certificates.crt /out/etc/ssl/certs/ca-certificates.crt \
    && cp /usr/lib/*/libssl.so.3 /out/lib/ \
    && cp /usr/lib/*/libcrypto.so.3 /out/lib/ \
    && cp -r /usr/share/fonts/* /out/usr/share/fonts/ \
    && cp -r /etc/fonts/* /out/etc/fonts/ \
    && (cp -r /var/cache/fontconfig/* /out/var/cache/fontconfig/ || true)

#
# Runner
#
FROM gcr.io/distroless/cc-debian12

ARG APP_VERSION
ENV APP_VERSION=$APP_VERSION

# SSL certs
COPY --from=builder /out/etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt

# OpenSSL libs
COPY --from=builder /out/lib/libssl.so.3 /usr/lib/libssl.so.3
COPY --from=builder /out/lib/libcrypto.so.3 /usr/lib/libcrypto.so.3

# Fonts + fontconfig
COPY --from=builder /out/usr/share/fonts /usr/share/fonts
COPY --from=builder /out/etc/fonts /etc/fonts
COPY --from=builder /out/var/cache/fontconfig /var/cache/fontconfig
ENV FONTCONFIG_PATH=/etc/fonts
ENV FONTCONFIG_FILE=/etc/fonts/fonts.conf

# App
COPY --from=builder /out/bot /bot

USER 65532:65532
ENTRYPOINT ["/bot"]
