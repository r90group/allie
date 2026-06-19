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

1. Harden V0 trust boundaries and failure taxonomy.
2. Add standards profile mapping for `wcag22-aa`.
3. Make verification boring in CI and local runbooks.

## Next

1. Add deterministic PR/CI exit semantics.
2. Add DOM and accessibility tree artifact capture behind redaction policy.
3. Add model-gateway policy types, but keep provider calls disabled by default.
4. Add fixture packet golden tests once the trust-boundary taxonomy lands.

## Later

1. OpenRouter-backed multimodal first-pass review.
2. GitHub Checks integration.
3. Hosted evidence viewer.
4. SME review workbench.
5. Remediation PR drafting.
6. Browser extension capture companion.
7. Multi-repo dashboard and trends.

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
