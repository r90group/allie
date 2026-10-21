# Verify a target and read reports

Engineers run the composed audit and receive evidence, WCAG obligations, release
status and portable CI reporters tied to the consumer's revision.

## Sub-features

- `verify-consumer`: foreign-working-directory run with consumer Git provenance.
- `verify-reporters`: JSON, WCAG JSON, HTML, Markdown, JUnit and SARIF.
- `verify-mobile`: additional mobile screenshot alongside desktop capture.
- `verify-neutral`: incomplete obligations remain `needs_review`.

## How to get to it (user POV)

Run `allie verify --manifest manifest.yml --out verify --project-root <consumer>`;
open its `reporters/` outputs. Lower-level `run/report/release` commands remain
available; the full repository gate covers those entry points separately.

## Driving it with verify.py

Preconditions: Doctor succeeded; synthetic login fixture and a fresh consumer
Git commit are created by the helper.

- **Audit.** Run `.agents/skills/verify-allie/scripts/verify.py drive --evidence "$evidence"`.
  Expect exit 0, one captured state, zero infrastructure failures, matching
  consumer revision, six nonempty reporters, and a mobile screenshot.
- **Interpret.** Read `drive.json`: `consumer.release_status` must be
  `approved` or `needs_review`; the helper accepts neither `blocked` nor error.
  The safe count projection records remaining review rather than hiding it.

## Gotchas

- A green process alone does not prove usable evidence: counts, provenance,
  artifact outputs and release status are asserted separately.
- Raw reporters can contain private target details; only public projection and
  count-level proof leave scratch. Mobile screenshot presence proves capture,
  not that every mobile WCAG obligation passes.
