# Publish safe evidence

Engineers produce a small public summary while keeping canonical accessibility
evidence local. Explicit requests for raw evidence must be refused safely.

## Sub-features

- `publication-ready`: exactly four approved public files.
- `publication-refuse`: raw packet request exits 2 with retryable refusal.
- `publication-preserve`: refusal leaves the original evidence bytes unchanged.

## How to get to it (user POV)

Run `allie publication --verify-root verify --out public`, or add
`--include run/evidence.json` to request a sensitive artifact explicitly.

## Driving it with verify.py

Preconditions: the helper's synthetic verification completed successfully.

- **Project.** Run `.agents/skills/verify-allie/scripts/verify.py drive --evidence "$evidence"`.
  Expect publication `ready`, and exactly `allie-public-summary.json`,
  `allie-public-summary.md`, `publication-receipt.json`, `allie-run-manifest.json`.
- **Refuse and preserve.** The same run requests the raw packet. Expect exit 2,
  `refused`, `retryable: true`, class `sensitive_local`, no copied raw packet,
  and identical canonical packet SHA256 before/after.

## Gotchas

- The helper preserves the four public files and count-only assertions; it
  never commits canonical screenshots, auth traces, DOM or full reports.
- An expected refusal is a successful negative control, not permission to
  bypass publication policy or publish the source tree.
