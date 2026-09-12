#!/usr/bin/env bash
# Verify that the PDK's vendored WIT tree still matches the engine's contract.
#
# crates/engine/wit/ is the single source of truth for the moonlit:plugin ABI.
# crates/pdk/ cannot reference it by relative path -- a crate published to
# crates.io has no sibling engine/ -- so it carries a verbatim copy. That copy
# is only as trustworthy as something that checks it has not drifted.
#
# A Rust test (crates/pdk/src/lib.rs, mod wit_drift) already pins
# moonlit-plugin.wit. It does not look at the rest of the tree: host.wit and the
# vendored WASI deps. A silent divergence there means plugin authors build
# against a different world than the engine instantiates, which surfaces as an
# instantiation failure in someone else's pipeline rather than as a test
# failure here.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
canonical="$repo_root/crates/engine/wit"
vendored="$repo_root/crates/pdk/wit"

for dir in "$canonical" "$vendored"; do
    if [[ ! -d "$dir" ]]; then
        echo "check-wit-sync: no such directory: $dir" >&2
        exit 1
    fi
done

# --recursive reports content differences and files present in only one tree,
# so an added or deleted dep counts as drift just as an edited one does.
if ! diff --recursive --unified "$canonical" "$vendored" >&2; then
    cat >&2 <<'EOF'

check-wit-sync: the PDK's vendored WIT has drifted from the canonical contract.

  canonical: crates/engine/wit
  vendored:  crates/pdk/wit

The diff above reads canonical first, so '-' lines are the engine's contract and
'+' lines are the PDK's copy. The engine is authoritative; re-vendor from it:

  rm -rf crates/pdk/wit && cp -R crates/engine/wit crates/pdk/wit

If the contract itself changed on purpose, the package version in
moonlit-plugin.wit likely needs a bump, and the registry's SupportedWorld has to
move in the same deploy window.
EOF
    exit 1
fi

count="$(find "$canonical" -type f -name '*.wit' | wc -l | tr -d ' ')"
echo "check-wit-sync: crates/pdk/wit matches crates/engine/wit (${count} files)."
