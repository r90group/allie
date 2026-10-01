#!/bin/bash
set -euo pipefail

# Same credential-free production artifact path for PRs and trusted master.
# Version preparation changes only this disposable CI checkout's four manifests.
dist=${1:-dist}
tag=$(node scripts/prepare-release.mjs)
sha=$(git rev-parse HEAD)
scripts/package-release.sh "$dist"
scripts/release-checksums.sh "$dist"
(cd "$dist" && sha256sum --strict --check SHA256SUMS)
root=$(mktemp -d "${TMPDIR:-/tmp}/allie-release-runtime-smoke.XXXXXX")
trap 'rm -rf "$root"' EXIT
mkdir -p "$root/work"
tar -xzf "$dist/allie-linux-x64.tar.gz" -C "$root"
# Preserve the actual Debian 12 minimum-runtime init proof without Node/browser
# dependencies: this catches an incompatible native binary before publication.
docker run --rm \
  -v "$root/allie:/opt/allie:ro" \
  -v "$root/work:/work" \
  -w /work \
  debian:bookworm-slim \
  /opt/allie/bin/allie init --manifest .allie/manifest.yml --app-name "Release Runtime Smoke"
test -s "$root/work/.allie/manifest.yml"
scripts/smoke-release-consumer.sh "$root/allie" "$tag" "$sha"
printf 'Release preflight passed: %s %s (archive, checksum, Debian 12 init, committed consumer)\n' "$tag" "$sha"
