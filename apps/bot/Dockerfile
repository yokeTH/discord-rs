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
    fontconfig \
    fonts-jetbrains-mono \
    && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build -p bot --release

RUN mkdir -p \
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
    && (cp -r /var/cache/fontconfig/* /out/var/cache/fontconfig/ || true) \
    && (ldd /app/target/release/bot || true)

FROM gcr.io/distroless/cc-debian12
ARG APP_VERSION
ENV APP_VERSION=$APP_VERSION

COPY --from=builder /out/etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt

COPY --from=builder /out/lib/libssl.so.3 /usr/lib/libssl.so.3
COPY --from=builder /out/lib/libcrypto.so.3 /usr/lib/libcrypto.so.3

COPY --from=builder /out/usr/share/fonts /usr/share/fonts
COPY --from=builder /out/etc/fonts /etc/fonts
COPY --from=builder /out/var/cache/fontconfig /var/cache/fontconfig
ENV FONTCONFIG_PATH=/etc/fonts
ENV FONTCONFIG_FILE=/etc/fonts/fonts.conf

COPY --from=builder /out/bot /bot

USER 65532:65532
ENTRYPOINT ["/bot"]
