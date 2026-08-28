# syntax=docker/dockerfile:1
# Ghosty multi-arch Docker image (#501)
#
# Build:  docker buildx build --platform linux/amd64,linux/arm64 -t ghosty:latest .
# Run:    docker run --rm -it -e DEEPSEEK_API_KEY -v ghosty-home:/home/ghosty/.ghosty ghosty
#
# The image ships the canonical `ghosty` and `ghosty-tui` command names in a
# minimal runtime layer.
#
# API keys MUST be passed at runtime (never baked into the image):
#   docker run --rm -it -e DEEPSEEK_API_KEY ghosty
# Or mount an env file:
#   docker run --rm -it --env-file .env ghosty

ARG RUST_VERSION=1.88

# ── Stage 1: Build ────────────────────────────────────────────────────
FROM --platform=$BUILDPLATFORM rust:${RUST_VERSION}-slim-bookworm AS builder
ARG TARGETPLATFORM
ARG TARGETARCH
ARG BUILDPLATFORM
ARG GHOSTY_BUILD_SHA

ENV CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    PKG_CONFIG_ALLOW_CROSS=1 \
    PKG_CONFIG_LIBDIR_aarch64_unknown_linux_gnu=/usr/lib/aarch64-linux-gnu/pkgconfig:/usr/share/pkgconfig \
    GHOSTY_BUILD_SHA=${GHOSTY_BUILD_SHA}

RUN if [ "${TARGETARCH}" = "arm64" ] && [ "${BUILDPLATFORM}" != "${TARGETPLATFORM}" ]; then \
      dpkg --add-architecture arm64; \
    fi \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
      pkg-config libdbus-1-dev \
    && if [ "${TARGETARCH}" = "arm64" ] && [ "${BUILDPLATFORM}" != "${TARGETPLATFORM}" ]; then \
      apt-get install -y --no-install-recommends \
        gcc-aarch64-linux-gnu libc6-dev-arm64-cross libdbus-1-dev:arm64; \
    fi \
    && rm -rf /var/lib/apt/lists/*

# Translate Docker platform into Rust target triple.
# linux/amd64  → x86_64-unknown-linux-gnu
# linux/arm64  → aarch64-unknown-linux-gnu
RUN case "${TARGETPLATFORM}" in \
      linux/amd64)  echo x86_64-unknown-linux-gnu  > /rust-target ;; \
      linux/arm64)  echo aarch64-unknown-linux-gnu > /rust-target ;; \
      *)            echo "Unsupported platform: ${TARGETPLATFORM}" >&2; exit 1 ;; \
    esac

RUN rustup target add "$(cat /rust-target)"

WORKDIR /build
COPY . .

# Build the one runtime for the target platform. Expose the same verified
# bytes under both supported command names. --locked keeps the build
# reproducible from the committed lockfile.
RUN --mount=type=cache,id=ghosty-target-${TARGETARCH},target=/build/target,sharing=locked \
    --mount=type=cache,id=ghosty-cargo-registry-${TARGETARCH},target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=ghosty-cargo-git-${TARGETARCH},target=/usr/local/cargo/git,sharing=locked \
    rustup target add "$(cat /rust-target)" \
    && cargo build --release --locked --target "$(cat /rust-target)" \
      -p ghosty-cli \
    && mkdir -p /out \
    && cp target/$(cat /rust-target)/release/ghosty /out/ \
    && cp target/$(cat /rust-target)/release/ghosty /out/ghosty-tui

# ── Stage 2: Runtime ──────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libdbus-1-3 \
    && rm -rf /var/lib/apt/lists/*

# Non-root user with explicit UID/GID for filesystem ownership clarity. Keep
# the legacy state directory for read-fallback migration; v0.9.0 no longer
# ships legacy deepseek command shims.
RUN groupadd --gid 1000 ghosty \
    && useradd --create-home --shell /bin/bash --uid 1000 --gid 1000 ghosty \
    && install -d -m 0700 -o ghosty -g ghosty /home/ghosty/.ghosty \
    && install -d -m 0700 -o ghosty -g ghosty /home/ghosty/.deepseek
USER ghosty
WORKDIR /home/ghosty

COPY --from=builder --chown=ghosty:ghosty /out/ghosty /usr/local/bin/ghosty
COPY --from=builder --chown=ghosty:ghosty /out/ghosty-tui /usr/local/bin/ghosty-tui

# `ghosty` and `ghosty-tui` are two command names for the same runtime.

ENTRYPOINT ["ghosty"]
CMD []
