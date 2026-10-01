#!/bin/bash
set -euo pipefail

# Internal artifact proof, not an installation/update trust boundary. The published
# wrapper must authenticate downloaded bytes before passing an unpacked bundle.
bundle=$(realpath "${1:?usage: smoke-release-consumer.sh BUNDLE TAG SHA}")
tag=${2:?usage: smoke-release-consumer.sh BUNDLE TAG SHA}
sha=${3:?usage: smoke-release-consumer.sh BUNDLE TAG SHA}
[[ "$tag" =~ ^v0\.[0-9]+\.[0-9]+$ ]]
[[ "$sha" =~ ^[0-9a-f]{40}$ ]]
root=$(mktemp -d "${TMPDIR:-/tmp}/allie-consumer-smoke.XXXXXX")
trap 'rm -rf "$root"' EXIT
node --input-type=module - "$bundle/release.json" "$bundle/package.json" "$tag" "$sha" <<'NODE'
import fs from 'node:fs';
const metadata = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const worker = JSON.parse(fs.readFileSync(process.argv[3], 'utf8'));
if (metadata.schema !== 'allie.distribution.v1' || metadata.version !== process.argv[4].slice(1) ||
    metadata.git_sha !== process.argv[5] || worker.version !== metadata.version) {
  throw new Error('Artifact identity does not match the expected revision and version');
}
NODE
if [ "${ALLIE_INSTALL_BROWSER_DEPS:-0}" = 1 ]; then
  "$bundle/node_modules/.bin/playwright" install-deps chromium
fi
unset GH_TOKEN GITHUB_TOKEN OPENROUTER_API_KEY OPENAI_API_KEY ALLIE_BROWSER_WORKER ALLIE_AGENTIC_WORKER
cd "$root"
"$bundle/bin/allie" init --manifest .allie/manifest.yml \
  --app-name "Allie Artifact Smoke" --fixture-dir "$bundle/fixtures/login"
git init -q
git config user.email "allie-smoke@example.invalid"
git config user.name "Allie Smoke"
git add .allie/manifest.yml
git commit -q -m "consumer fixture manifest"
"$bundle/bin/allie" doctor --manifest .allie/manifest.yml --out .allie/doctor
"$bundle/bin/allie" verify --manifest .allie/manifest.yml \
  --project-root "$root" --out .allie/verify/latest
"$bundle/bin/allie" publication --verify-root .allie/verify/latest --out .allie/public/latest
node --input-type=module - "$tag" "$sha" <<'NODE'
import fs from 'node:fs';
import { execFileSync } from 'node:child_process';
const consumerSha = execFileSync('git', ['rev-parse', '--short', 'HEAD'], { encoding: 'utf8' }).trim();
const evidence = JSON.parse(fs.readFileSync('.allie/verify/latest/run/evidence.json', 'utf8'));
const receipt = JSON.parse(fs.readFileSync('.allie/public/latest/publication-receipt.json', 'utf8'));
if (evidence.run.allie_version !== process.argv[2].slice(1) || evidence.summary.states_captured !== 1 ||
    evidence.run.git_sha !== consumerSha || evidence.summary.infrastructure_failures !== 0 || receipt.status !== 'ready') {
  throw new Error('Artifact binary failed the fixture verification/public-summary journey');
}
console.log(JSON.stringify({ event: 'allie.artifact_consumer', tag: process.argv[2], sha: process.argv[3],
  artifact_identity: 'verified', consumer_sha: consumerSha, states_captured: evidence.summary.states_captured,
  infrastructure_failures: 0, publication: receipt.status }));
NODE
