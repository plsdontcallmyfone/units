#!/usr/bin/env bash
# Primes the offline cargo registry cache the build sandbox reads (HOOKLAB_CARGO_HOME), with the
# network, outside the sandbox: `prime-cache.sh <cargo-home> <crate-dir>...`. Each crate's
# `Cargo.lock` is fetched into the cache; a submission whose dependencies are not in the cache then
# fails to build offline with cargo's own message, which the signed fail report carries. Run it for
# the starter template (examples/hook-template) and for any crate set the lab accepts.
set -euo pipefail
home="${1:?usage: prime-cache.sh <cargo-home> <crate-dir>...}"; shift
mkdir -p "$home"
for c in "$@"; do CARGO_HOME="$home" cargo fetch --locked --manifest-path "$c/Cargo.toml"; done
echo "primed $home"
