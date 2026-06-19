# Repository Instructions

This repo is Rust-first. Keep non-Rust surfaces behind narrow process or schema boundaries.

## Gates

Run before claiming repo changes are complete:

```sh
cargo fmt --check
cargo test --locked
npm run worker:smoke
```

The browser worker smoke is part of the gate; keep it green when worker or fixture behavior changes.

## Design Rules

- Treat `SPEC.md` as the product contract.
- Treat the evidence packet as the core interface.
- Keep Playwright/axe implementation details behind a worker adapter.
- Do not spread OpenRouter/provider details outside the model gateway.
- Do not claim legal compliance; report evidence, status, confidence, and residual review needs.
- Do not block releases on model-only findings.
- Do not weaken deterministic gates to make a run green.

## Closeout

Every meaningful change should state:

- the exact product behavior or doc contract changed;
- the command that verified it;
- residual unverified paths.
