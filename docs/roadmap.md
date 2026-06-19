# Roadmap

## Current V0 Loop

The first local evidence loop is implemented around:

```sh
cargo run --locked -- run --manifest examples/login-flow.yml --out .allie/runs/latest
```

It reads the checked-in login manifest, serves the local fixture through the
browser worker, runs Playwright plus axe, captures a screenshot, writes raw axe
JSON, emits an `allie.evidence.v0` packet, and generates a local HTML report.

## Now

1. Expand DOM and accessibility tree artifact capture behind redaction policy.
2. Add model-gateway adapter tests while keeping provider calls disabled by
   default.
3. Add fixture packet golden tests for larger flow coverage.

## Next

1. Wire `github-check.json` into a hosted GitHub Checks integration.
2. Shape the hosted evidence viewer from local packet/report semantics.
3. Add trend ledger indexing across repeated evidence runs.

## Later

1. OpenRouter-backed multimodal first-pass review.
2. SME review workbench.
3. Remediation PR drafting.
4. Browser extension capture companion.
5. Multi-repo dashboard and trends.

## First Acceptance Slice

The first slice is complete when this command works against a checked-in fixture:

```sh
allie run --manifest examples/login-flow.yml --out .allie/runs/latest
```

Required evidence:

- JSON packet;
- HTML report;
- Playwright route state;
- axe results;
- at least one screenshot;
- deterministic exit code;
- replay instructions.
