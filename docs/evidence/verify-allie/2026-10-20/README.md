# Allie verification evidence

Run date: 2026-10-20 America/Chicago (2026-10-21 UTC). Product source and signed
release revision: `4a64a7c40bfd95b584bd759c41328a0c11cd3525`, default branch
`master`; downloaded healthy release [v0.1.90](https://github.com/r90group/allie/releases/tag/v0.1.90).
The skill and scanner fix were exercised from the PR worktree before committing.

| Proof | Result |
| --- | --- |
| [Doctor](doctor.json) | Actual browser-worker launch, fixture and runtime ready; model-off warning retained |
| [Source CLI journey](drive.json) | Fresh consumer revision, one captured state, six reporters, mobile screenshot, zero infrastructure failures |
| [Downloaded artifact](published.json) | Archive checksum and exact Sigstore signer verified before extraction; installed consumer journey passed |
| [Public summary](allie-public-summary.json) | `needs_review`; no blocking deterministic/scripted/infrastructure failure |
| [Public Markdown](allie-public-summary.md) | Count-only projection; remaining WCAG obligations visible |
| [Publication receipt](publication-receipt.json) | `ready`, approved file hashes |
| [Publication manifest](allie-run-manifest.json) | Completed public projection |
| [Repository checks](checks.json) | Full repository gate and symlink regression results |

The `drive` proof includes a real synthetic browser login reaching
`/dashboard.html` with no state errors, signed-out `auth-lost` rejection (exit 1,
release blocked), required credential absence (exit 2, error packet), and raw
evidence publication refusal (exit 2, retryable `sensitive_local`) with unchanged
canonical packet bytes. Child provider keys are removed; no paid model calls.

Owner sign-in: **n/a**, because Allie is a local CLI/artifact product with no
hosted owner account, inbox, or owner-only route. Native GitHub release download
access worked. There is no unmet owner-auth access requirement. Fixture target
authentication is separate and does not claim a real product-owner session.

The only defect found and changed here was the repository secret scanner's
failure on the required skill directory symlink: the first full gate reached
`secret scan failed: an input could not be read` after passing all 158 Rust
tests and Clippy. The scanner now scans directory-alias text as Git stores it
and scans canonical files separately, preserving file-alias content checks.
Its self-test covers a directory alias, secrets in canonical/file-alias content,
and a secret in directory-alias text. Unreadable files still fail closed.

All committed files are count/status evidence or policy-approved public
projections. Canonical DOM, screenshots, traces, credentials, and private target
content are excluded. Helper scratch was removed after each run and these proof
files survived cleanup. Cosign v3.1.4 was installed only in workstation scratch;
its executable SHA256 matched the official GitHub release asset digest
`424e2c97d0b45fd7a6b608a87f090f35633f4c23ece5f092d48bba11aaf4bbfa`.

Residual limits: no production target audit, live paid model review, external
SSO/mailbox delivery, expired/reused magic links, or slow-network scenario. The
synthetic WCAG ledger retains a failing cell and review/not-tested obligations;
the neutral release result is evidence visibility, not complete WCAG coverage.
