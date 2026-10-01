#!/bin/bash
set -euo pipefail

# Run outside the source checkout against the three real downloadable assets.
# The only updater channel is GitHub's latest release; no app update manifest exists.
tag=${1:?usage: smoke-published-release.sh TAG SHA}
sha=${2:?usage: smoke-published-release.sh TAG SHA}
[[ "$tag" =~ ^v0\.[0-9]+\.[0-9]+$ ]]
[[ "$sha" =~ ^[0-9a-f]{40}$ ]]
repo=${GITHUB_REPOSITORY:-r90group/allie}
script_dir=$(cd "$(dirname "$0")" && pwd)
root=$(mktemp -d "${TMPDIR:-/tmp}/allie-published-smoke.XXXXXX")
trap 'rm -rf "$root"' EXIT
archive=allie-linux-x64.tar.gz
mkdir -p "$root/download"
gh release download "$tag" --repo "$repo" --dir "$root/download" \
  --pattern "$archive" --pattern SHA256SUMS --pattern "$archive.sigstore.json"
node --input-type=module - "$root/download/SHA256SUMS" <<'NODE'
import fs from 'node:fs';
const manifest = fs.readFileSync(process.argv[2], 'utf8');
if (!/^[0-9a-f]{64}  allie-linux-x64\.tar\.gz\n$/.test(manifest)) {
  throw new Error('Expected exactly the signed archive in the checksum manifest');
}
NODE
(cd "$root/download" && sha256sum --strict --check SHA256SUMS)
cosign verify-blob \
  --bundle "$root/download/$archive.sigstore.json" \
  --certificate-identity "https://github.com/$repo/.github/workflows/release.yml@refs/heads/master" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  "$root/download/$archive"
actual_sha=$(gh api "repos/$repo/commits/$tag" --jq .sha)
[ "$actual_sha" = "$sha" ]
unset GH_TOKEN GITHUB_TOKEN
tar -xzf "$root/download/$archive" -C "$root"
"$script_dir/smoke-release-consumer.sh" "$root/allie" "$tag" "$sha"
printf '{"event":"allie.published_smoke","tag":"%s","sha":"%s","checksum":"verified","signature":"verified","artifact_consumer":"verified"}\n' "$tag" "$sha"
