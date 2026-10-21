---
name: verify-allie
description: Verify Allie's CLI consumer journey, signed release artifact, accessibility reports, authenticated targets, and publication privacy after changes or when diagnosing a failed audit.
---

# Verify Allie

Allie is an active Rust CLI distributed as a signed Linux x64 bundle through
[GitHub Releases](https://github.com/r90group/allie/releases/latest). Its primary
user is a consuming engineer running `init → doctor → verify → publication`.
Allie has no hosted owner account, inbox, or owner-only page: owner sign-in is
**n/a**. Authentication in `examples/auth-fixture-*.yml` belongs to audited
targets; the synthetic dashboard proof below is not a production owner session.

## Launch

Run from an isolated checkout/worktree, on Linux with Rust, Python 3.9+, Node 22+
and Chromium system libraries. Install and build once:

```sh
npm ci
npx playwright install chromium
cargo build --locked
```

Ready means Doctor below succeeds, including the actual browser-worker smoke.
There is no persistent Allie server. Each helper invocation creates its own
consumer repository and OS-assigned fixture ports under `~/.cache/tmp`.
Concurrent helper runs use distinct evidence directories; run the repository
gate serially within one checkout because its existing smokes share `.allie/`.

## Doctor

From the repository root, choose a fresh evidence directory for each run:

```sh
evidence="$PWD/docs/evidence/verify-allie/$(date -u +%F)/$(date -u +%H%M%S)"
.agents/skills/verify-allie/scripts/verify.py doctor --evidence "$evidence"
```

This invokes the real `init` and `doctor` against a disposable login fixture.
It reads the target, launches the worker, and writes diagnostics only in scratch;
it does not change deployed state. Require exit 0, every check non-failing, and
model review disabled. A model-off warning is expected and retains review needs.
On any surprising result, rerun Doctor into a fresh directory before driving.

## Drive

Read [the feature map](features/README.md), then run the shell-accessible core:

```sh
.agents/skills/verify-allie/scripts/verify.py drive --evidence "$evidence"
```

It invokes the built CLI from a foreign consumer working directory: scaffold,
Doctor, verify and six reporters, desktop/mobile evidence, public publication,
sensitive-evidence refusal, authenticated dashboard and signed-out rejection,
and missing-credential failure. Child provider keys and worker overrides are
removed; all manifests use synthetic local fixtures with the model disabled.
Exit 0 means every assertion passed; `needs_review` remains a neutral release
result, never a claim of complete accessibility or legal compliance.

For the actual deployed artifact, install Cosign on PATH and use an authorized
native `gh` login that can download the release. Run:

```sh
.agents/skills/verify-allie/scripts/verify.py published --evidence "$evidence"
```

This resolves healthy `latest` and its tag commit, then reuses
`scripts/smoke-published-release.sh`: exactly one archive checksum, exact
master-workflow Sigstore identity and issuer **before extraction**, signed
version/revision, and installed `init/doctor/verify/publication` from a fresh
consumer repository. No tag, release, workflow, deployment, or alert is created.
Missing download authorization, Cosign, or browser libraries is an access/setup
gap; report the exact failure and keep the published feature unproven.

## Evidence

The executable helper emits `doctor.json`, `drive.json`, and `published.json`
into the supplied directory. Each records actions, normalized CLI arguments,
expected/actual exits, source revision and count/status assertions. `drive`
also copies exactly the four policy-approved `public_summary` files. A failed
run writes `status: failed` and exits nonzero; prior proof cannot be overwritten.

Canonical DOM, screenshots, auth traces, worker requests, credentials and raw
reports remain local and are removed with scratch. Failure diagnostics are
mode-0600 logs under `~/.cache/tmp`; inspect locally, then delete only the named
log. Commit only the generated count-level proof and public projection after
inspection. Never substitute a sender response, fabricated session, mock worker,
or mere file existence for a real consumer action and its asserted result.

Run the repository's required checks with synthetic inputs:

```sh
env -u OPENROUTER_API_KEY -u OPENAI_API_KEY TMPDIR="$HOME/.cache/tmp" npm run verify
```

The gate additionally covers state actions, storageState, coverage matrices,
workbench lifecycle, output ownership, distribution and offline model adapters.
Fake provider checks in that gate prove adapter contracts, not live model quality.

## Cleanup

The helper removes only its own temporary directory on success or failure;
workers own their ephemeral browser/server lifetimes. Proof stays outside that
directory and is checked after cleanup. An interrupted or timed-out command
fails the run and the helper kills only that command's isolated process group.
Source dependencies and `.allie/` outputs from `npm run verify` remain reusable
in this dedicated worktree. Remove the worktree when finished, preserving the
committed evidence. Never remove another checkout's `.allie/` or kill by name.

## Helpers

`scripts/verify.py` is executable and uses only the Python standard library;
all three invocations appear above. Keep this map current with
`/maintain-verification-skill` when CLI contracts or release paths change.
