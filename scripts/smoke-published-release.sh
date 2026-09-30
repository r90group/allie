#!/bin/bash
set -euo pipefail

# Run outside the source checkout against the three real downloadable assets.
# The only updater channel is GitHub's latest release; no app update manifest exists.
tag=${1:?usage: smoke-published-release.sh TAG SHA}
sha=${2:?usage: smoke-published-release.sh TAG SHA}
[[ "$tag" =~ ^v0\.[0-9]+\.[0-9]+$ ]]
[[ "$sha" =~ ^[0-9a-f]{40}$ ]]
repo=${GITHUB_REPOSITORY:-r90group/allie}
root=$(mktemp -d "${TMPDIR:-/tmp}/allie-published-smoke.XXXXXX")
trap 'rm -rf "$root"' EXIT
archive=allie-linux-x64.tar.gz
mkdir -p "$root/download" "$root/consumer"
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
bundle="$root/allie"
node --input-type=module - "$bundle/release.json" "$bundle/package.json" "$tag" "$sha" <<'NODE'
import fs from 'node:fs';
const metadata = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const worker = JSON.parse(fs.readFileSync(process.argv[3], 'utf8'));
if (metadata.schema !== 'allie.distribution.v1' || metadata.version !== process.argv[4].slice(1) ||
    metadata.git_sha !== process.argv[5] || worker.version !== metadata.version) {
  throw new Error('Signed artifact identity does not match the published revision and version');
}
NODE
if [ "${ALLIE_INSTALL_BROWSER_DEPS:-0}" = 1 ]; then
  "$bundle/node_modules/.bin/playwright" install-deps chromium
fi
unset OPENROUTER_API_KEY OPENAI_API_KEY ALLIE_BROWSER_WORKER ALLIE_AGENTIC_WORKER
cd "$root/consumer"
"$bundle/bin/allie" init --manifest .allie/manifest.yml \
  --app-name "Published Allie Smoke" --fixture-dir "$bundle/fixtures/login"
"$bundle/bin/allie" doctor --manifest .allie/manifest.yml --out .allie/doctor
"$bundle/bin/allie" verify --manifest .allie/manifest.yml --out .allie/verify/latest
"$bundle/bin/allie" publication --verify-root .allie/verify/latest --out .allie/public/latest
node --input-type=module - "$tag" "$sha" <<'NODE'
import fs from 'node:fs';
const evidence = JSON.parse(fs.readFileSync('.allie/verify/latest/run/evidence.json', 'utf8'));
const receipt = JSON.parse(fs.readFileSync('.allie/public/latest/publication-receipt.json', 'utf8'));
if (evidence.run.allie_version !== process.argv[2].slice(1) || evidence.summary.states_captured !== 1 ||
    evidence.summary.infrastructure_failures !== 0 || receipt.status !== 'ready') {
  throw new Error('Published binary failed the fixture verification/public-summary journey');
}
console.log(JSON.stringify({ event: 'allie.published_smoke', tag: process.argv[2], sha: process.argv[3],
  checksum: 'verified', signature: 'verified', artifact_identity: 'verified',
  states_captured: evidence.summary.states_captured, infrastructure_failures: 0, publication: receipt.status }));
NODE
