#!/usr/bin/env bash
# Build the moonlit image locally from a freshly compiled binary, single-arch,
# then run the assertion suite against it.
#
#   scripts/docker-build-local.sh [image-tag]     # default: moonlit:local
#
# CI does not use this; the release job packages the published binary instead.
set -euo pipefail

TAG="${1:-moonlit:local}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

case "$(uname -m)" in
  x86_64)          ARCH=amd64 ;;
  aarch64 | arm64) ARCH=arm64 ;;
  *) echo "unsupported host architecture: $(uname -m)" >&2; exit 1 ;;
esac

echo "==> Building the moonlit binary (release)"
cargo build --release -p moonlit

echo "==> Staging into dist/linux/$ARCH/"
mkdir -p "dist/linux/$ARCH"
cp target/release/moonlit "dist/linux/$ARCH/moonlit"

VERSION="$(target/release/moonlit --version | awk '{print $2}')"
VCS_REF="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"

echo "==> Building $TAG (linux/$ARCH, moonlit $VERSION)"
docker build \
  --build-arg "MOONLIT_VERSION=$VERSION" \
  --build-arg "VCS_REF=$VCS_REF" \
  -t "$TAG" \
  .

echo "==> Testing $TAG"
scripts/test-docker-image.sh "$TAG" "$VERSION"
