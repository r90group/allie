# Roadmap

## Current V0 Loop

The first local evidence loop is implemented around:

```sh
cargo run --locked -- run --manifest examples/login-flow.yml --out .allie/runs/latest
```

It reads the checked-in login manifest, serves the local fixture through the
browser worker, runs Playwright plus axe, captures a screenshot, writes raw axe
JSON, emits an `allie.evidence.v0` packet, and generates a local HTML report.

This is a proof foundation, not the autonomous product. The current system does
not yet discover application surfaces, generate tests, run vision AI, complete a
WCAG matrix, or draft remediation.

## Product Target

Allie should let a compliance engineer point it at an application and receive:

1. An autonomously discovered sitemap, product-surface inventory, and likely user
   stories.
2. Generated Playwright and axe coverage that replays through real browser
   evidence before it can enforce release policy.
3. A complete WCAG 2.2 A/AA obligation ledger with drilldown from criterion to
   state, finding, artifact, agentic context, waiver, and remediation.
4. Agentic vision review for criteria that require judgment, with redaction
   receipts and neutral findings until promoted by scripted proof or human
   attestation.
5. Release enforcement and remediation guidance that are packet projections, not
   a separate status model.

## Now

1. Keep the autonomous workbench smoke green as the primary delivery oracle
   (`npm run autonomous:smoke`).
2. Use the generated receipts under `.allie/*/autonomous-smoke/` to inspect
   discovery, replay, WCAG drilldown, agentic review, remediation, and release
   blocking behavior.
3. Harden from local fixture proof toward real staged applications without
   weakening replay, redaction, or model-promotion gates.

## Next

1. Add authenticated staged-app discovery and changed-surface inference.
2. Add live provider adapters behind the offline model gateway contract.
3. Add richer remediation patch adapters, before/after packet comparison, and
   reviewer attestations.

## Later

1. Enable approved live multimodal provider calls behind the model gateway.
2. Add remediation branch drafting with evidence-linked source hints and replay
   proof.
3. Wire GitHub Checks, PR comments, and hosted evidence viewer from the same
   packets.
4. Add SME review workbench, reviewer attestations, and promotion workflows.
5. Add browser extension capture companion, multi-repo dashboard, and trends.

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
