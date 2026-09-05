# syntax=docker/dockerfile:1

# This Dockerfile PACKAGES a prebuilt moonlit binary; it compiles nothing.
# The build context must already contain:
#   dist/linux/amd64/moonlit
#   dist/linux/arm64/moonlit
# CI fills those from the GitHub Release. For a local image, use
# scripts/docker-build-local.sh, which runs cargo first.
#
# A bare `docker build .` on a clean checkout fails with
# "COPY failed: no source files were specified". That is expected.

FROM debian:trixie-slim

ARG TARGETARCH
ARG MOONLIT_VERSION=dev
ARG VCS_REF=unknown

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends ca-certificates git; \
    rm -rf /var/lib/apt/lists/*

COPY dist/linux/${TARGETARCH}/moonlit /usr/local/bin/moonlit

RUN set -eux; \
    groupadd --gid 1000 moonlit; \
    useradd --uid 1000 --gid 1000 --create-home --shell /bin/bash moonlit; \
    install -d -o moonlit -g moonlit /work /home/moonlit/.cache/moonlit; \
    moonlit --version

# Load-bearing: Docker does not set HOME on USER, it inherits /root from the
# base image. moonlit resolves its plugin cache via $XDG_CACHE_HOME else
# $HOME/.cache, and its 0600 credentials file via $HOME.
ENV HOME=/home/moonlit

USER moonlit
WORKDIR /work

LABEL org.opencontainers.image.title="Moonlit" \
      org.opencontainers.image.description="Build and release automation powered by Rust and sandboxed WebAssembly plugins." \
      org.opencontainers.image.version="${MOONLIT_VERSION}" \
      org.opencontainers.image.revision="${VCS_REF}" \
      org.opencontainers.image.source="https://github.com/wolfware-labs/moonlit" \
      org.opencontainers.image.url="https://moonlitbuild.dev/" \
      org.opencontainers.image.documentation="https://moonlitbuild.dev/" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0" \
      org.opencontainers.image.vendor="Wolfware LLC"

ENTRYPOINT ["moonlit"]
CMD ["--help"]
