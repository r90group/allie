# Allie verification evidence

Run date: 2026-10-20 America/Chicago (2026-10-21 UTC). Product source and signed
release revision: `4a64a7c40bfd95b584bd759c41328a0c11cd3525`, default branch
`master`; downloaded healthy release [v0.1.90](https://github.com/r90group/allie/releases/tag/v0.1.90).
Initial journey proof was exercised from the PR worktree before committing.
The blocker corrections and fresh source journeys are recorded separately below;
the original signed-release and public-projection proof remains unchanged.

| Proof | Result |
| --- | --- |
| [Doctor](doctor.json) | Actual browser-worker launch, fixture and runtime ready; model-off warning retained |
| [Source CLI journey](drive.json) | Fresh consumer revision, one captured state, six reporters, mobile screenshot, zero infrastructure failures |
| [Downloaded artifact](published.json) | Archive checksum and exact Sigstore signer verified before extraction; installed consumer journey passed |
| [Public summary](allie-public-summary.json) | `needs_review`; no blocking deterministic/scripted/infrastructure failure |
| [Public Markdown](allie-public-summary.md) | Count-only projection; remaining WCAG obligations visible |
| [Publication receipt](publication-receipt.json) | `ready`, approved file hashes |
| [Publication manifest](allie-run-manifest.json) | Completed public projection |
| [Initial repository checks](checks.json) | Original full gate and symlink self-test; predates the independent blockers |
| [Blocker correction checks](blocker-fixes/checks.json) | Required gates, 24 scanner CLI controls, real SIGINT/SIGTERM cleanup, packaging checks |
| [Before corrections](blocker-fixes/regressions-before.json) | Old head accepts readable/unreadable directory event aliases; SIGTERM leaks; SIGINT retains an unfinished receipt |
| [Corrected scanner](blocker-fixes/scanner-controls.json) | Canonical/link/file-alias content and generated evidence scanned; malformed/unreadable events rejected |
| [Interrupted receipts](blocker-fixes/signal-controls.json) | Real Doctor groups stopped, direct children reaped, scratch removed; both receipts retain `interrupted` and signal names |
| [Fresh source Doctor](blocker-fixes/doctor.json) and [drive](blocker-fixes/drive.json) | Normal paths pass after cancellation changes; fixture authentication and publication re-exercised |

The `drive` proof includes a real synthetic browser login reaching
`/dashboard.html` with no state errors, signed-out `auth-lost` rejection (exit 1,
release blocked), required credential absence (exit 2, error packet), and raw
evidence publication refusal (exit 2, retryable `sensitive_local`) with unchanged
canonical packet bytes. Child provider keys are removed; no paid model calls.

Owner sign-in: **n/a**, because Allie is a local CLI/artifact product with no
hosted owner account, inbox, or owner-only route. Native GitHub release download
access worked. There is no unmet owner-auth access requirement. Fixture target
authentication is separate and does not claim a real product-owner session.

The initial defect was the repository secret scanner's failure on the required
skill directory symlink: the first full gate reached
`secret scan failed: an input could not be read` after passing all 158 Rust
tests and Clippy. The scanner now scans directory-alias text as Git stores it
and scans canonical files separately, preserving file-alias content checks.
Independent [QA](https://github.com/r90group/allie/pull/99#issuecomment-6305318138)
and [review](https://github.com/r90group/allie/pull/99#issuecomment-6305265943)
then found that this exception also accepted malformed event-payload directory
aliases. The correction scopes link-text handling to Git worktree entries.
The CLI regression rejects readable and unreadable directory event targets,
missing files and dangling aliases, while valid event files still succeed.

The same review found two cancellation failures in the helper: SIGTERM bypassed
cleanup and wrote no proof, while SIGINT retained `status: in_progress`.
Handlers now cover all child launches, kill only the active owned group, reap
the direct child, remove scratch, and finalize retained `interrupted` receipts
with exits 130 or 143. The real CLI signal regression pauses only its own Doctor
group and signals only its own helper. It retains both resulting receipts in
the count-level evidence above. These are builder corrections, not an assertion
that the old independent reports approve the new head; the parent owns delta
QA, review, judging, and merging.

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
