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

1. Make autonomous discovery a versioned, replay-gated contract
   (`backlog.d/006-autonomous-discovery-replay-gated.md`).
2. Build the complete WCAG obligation ledger and drilldown report contract
   (`backlog.d/007-complete-wcag-drilldown-reporting.md`).
3. Expand the fixture corpus so discovery, generated flows, and release
   enforcement have real falsifiers.

## Next

1. Generate comprehensive Playwright and axe suites from discovered surfaces and
   promote only replayed flows to release-required coverage.
2. Add the model gateway and vision-agent review contract with no live provider
   calls until policy, redaction, and audit receipts are typed and verified.
3. Add WCAG drilldown report views for engineer fix lists, accessibility
   specialist review, and audit summaries.

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
