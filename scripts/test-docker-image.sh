#!/usr/bin/env bash
# Assertion suite for the moonlit Docker image.
#
#   scripts/test-docker-image.sh <image-ref> <expected-version>
#
# Runs against ANY image ref: a locally built one, or one already pushed to
# Docker Hub. The publish workflow deliberately runs it against the pushed
# artifact, so what ships is what was tested.
set -euo pipefail

IMAGE="${1:?usage: test-docker-image.sh <image-ref> <expected-version>}"
EXPECTED_VERSION="${2:?usage: test-docker-image.sh <image-ref> <expected-version>}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PASSED=0

fail() { printf 'FAIL: %b\n' "$*" >&2; exit 1; }
pass() { PASSED=$((PASSED + 1)); echo "  ok  $*"; }

# Run a shell command inside the image, bypassing the moonlit ENTRYPOINT.
in_image() { docker run --rm --entrypoint sh "$IMAGE" -c "$1"; }

echo "Testing image: $IMAGE (expecting moonlit $EXPECTED_VERSION)"

# 1. The image carries the exact binary we expect.
if ! actual="$(docker run --rm "$IMAGE" --version 2>&1)"; then
  fail "1: 'docker run $IMAGE --version' failed:\n$actual"
fi
[ "$actual" = "moonlit $EXPECTED_VERSION" ] \
  || fail "1: --version was '$actual', expected 'moonlit $EXPECTED_VERSION'"
pass "1: --version reports moonlit $EXPECTED_VERSION"

# 2. A bare run prints help and succeeds (ENTRYPOINT + CMD wired correctly).
docker run --rm "$IMAGE" >/dev/null 2>&1 \
  || fail "2: bare 'docker run' did not exit 0"
help_output="$(docker run --rm "$IMAGE" 2>&1)"
grep -q "^Usage: moonlit" <<<"$help_output" \
  || fail "2: bare 'docker run' did not print the usage banner"
pass "2: bare run prints help, exit 0"

# 3. Non-root by default. A root image would silently root-own everything it
#    writes into a mounted workspace.
[ "$(in_image 'id -u')" = "1000" ] || fail "3: uid is not 1000"
[ "$(in_image 'id -un')" = "moonlit" ] || fail "3: user is not 'moonlit'"
pass "3: runs as moonlit (1000)"

# 4. git is present. A plugin granted exec: ["git"] can only run programs that
#    exist in the image; without this, a base-image bump silently breaks them.
in_image 'git --version' >/dev/null || fail "4: git is missing"
pass "4: git is present"

# 5. HOME is set and writable, and the cache dir exists. Docker does NOT set
#    HOME on USER — it inherits /root from the base image. Without an explicit
#    ENV, moonlit writes its plugin cache and 0600 credentials into an
#    unwritable /root and fails as a confusing resolve error.
[ "$(in_image 'echo $HOME')" = "/home/moonlit" ] || fail "5: HOME is not /home/moonlit"
in_image 'test -w "$HOME"' || fail "5: HOME is not writable"
in_image 'test -w /home/moonlit/.cache/moonlit' || fail "5: cache dir is not writable"
docker run --rm --user 1001:0 --entrypoint sh "$IMAGE" -c 'touch "$HOME/.probe"' \
  || fail "5: HOME is not writable under a mismatched uid with gid 0"
pass "5: HOME set and writable, cache dir writable, and writable under an arbitrary uid with gid 0"

# 6. CA certificates, without which every oci:// and https:// resolve fails TLS.
in_image 'test -s /etc/ssl/certs/ca-certificates.crt' || fail "6: CA bundle missing or empty"
pass "6: CA bundle present"

# 7. End to end: a real pipeline, a real WASI Preview 2 component, inside this
#    container. Assertions 1-6 test packaging; only this tests that the engine
#    works here. Staged into a temp dir so no second copy of the wasm is tracked.
workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT
cp "$REPO_ROOT/tests/docker/release.yml" "$workdir/release.yml"
cp "$REPO_ROOT/engine/tests/fixtures/test_plugin.wasm" "$workdir/test_plugin.wasm"
# mktemp -d is 0700, and on most CI runners the host uid differs from the image's,
# so the container could not otherwise enter the mount at all.
chmod 0755 "$workdir"

run_e2e() {
  local label="$1"
  shift
  local out
  if ! out="$(docker run --rm "$@" -v "$workdir:/work" "$IMAGE" run --output plain 2>&1)"; then
    fail "7: pipeline run ($label) exited non-zero:\n$out"
  fi
  grep -q "SUCCESS" <<<"$out" || fail "7: pipeline output ($label) had no SUCCESS row:\n$out"
}

run_e2e "default user"
run_e2e "host uid with gid 0" --user "$(id -u):0"
pass "7: e2e pipeline runs a wasm plugin to SUCCESS (default user and mismatched uid)"

# 8. OCI metadata, and the architecture the manifest claims.
labels="$(docker image inspect "$IMAGE" --format '{{json .Config.Labels}}')"
for label in \
  org.opencontainers.image.title \
  org.opencontainers.image.version \
  org.opencontainers.image.revision \
  org.opencontainers.image.source \
  org.opencontainers.image.licenses
do
  grep -q "$label" <<<"$labels" || fail "8: missing label $label"
done
grep -q "\"org.opencontainers.image.version\":\"$EXPECTED_VERSION\"" <<<"$labels" \
  || fail "8: image.version label does not match $EXPECTED_VERSION"
pass "8: OCI labels present and versioned"

echo "All $PASSED assertions passed for $IMAGE"
