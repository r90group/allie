# Install the released CLI

Engineers download the healthy signed Linux bundle and run the same consumer
journey without the Allie source tree supplying runtime assets.

## Sub-features

- `release-trust`: checksum and exact GitHub workflow signer precede extraction.
- `release-identity`: signed version/SHA and worker package agree with the tag.
- `release-consumer`: installed binary captures evidence and publishes a summary.

## How to get to it (user POV)

Use the README's Consumer Contract download procedure or the verification
runbook's `scripts/smoke-published-release.sh TAG SHA` shortcut.

## Driving it with verify.py

Preconditions: native `gh` release download access, Cosign on PATH, Linux x64
with glibc 2.35+, Node and Chromium libraries; Doctor succeeded.

- **Install and use.** Run `.agents/skills/verify-allie/scripts/verify.py published --evidence "$evidence"`.
  Expect exit 0 and `published.json` with `allie.published_smoke` reporting
  checksum/signature/consumer verified and `allie.artifact_consumer` reporting
  one captured state, zero infrastructure failures, and publication `ready`.
- **Identity.** Read its release tag and full SHA; both must match the selected
  release. The wrapper checks the signature before executing bundled code.

## Gotchas

- Current signer: `https://github.com/r90group/allie/.github/workflows/release.yml@refs/heads/master`;
  issuer: `https://token.actions.githubusercontent.com`. Legacy v0.1.0 has a
  different signer and is outside this helper's path.
- A missing Cosign executable or denied release download is unproven installation,
  even when source checks pass. Do not extract or execute unchecked bytes.
- Allie requires no hosted owner account. Native GitHub download authorization
  is tooling access, not an Allie owner session.
