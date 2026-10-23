# Initialize and diagnose a consumer

An engineer scaffolds a manifest and checks the browser worker before auditing.

## Sub-features

- `setup-init`: portable fixture manifest with model review disabled.
- `setup-doctor`: real browser launch and target/runtime readiness.
- `setup-missing-credential`: required credential absence fails closed.

## How to get to it (user POV)

Run `allie init`, then `allie doctor`; use `allie run` for a manifest requiring
an unavailable target credential.

## Driving it with verify.py

Preconditions: source Launch completed and a fresh `$evidence` path is selected.

- **Preflight.** Run `.agents/skills/verify-allie/scripts/verify.py doctor --evidence "$evidence"`.
  Expect exit 0, all Doctor checks `ok` or `warn`, and a model-off manifest.
- **Missing credential.** Run `.agents/skills/verify-allie/scripts/verify.py drive --evidence "$evidence"`.
  Its final `allie run` uses `examples/trust-missing-credential.yml` with
  `ALLIE_TEST_DO_NOT_SET_SECRET` removed. Expect exit 2 and packet status `error`.

## Gotchas

- Doctor warnings about model review being off are expected. A browser failure
  is not a warning to waive; fix prerequisites and rerun.
- Provider keys are removed from every helper child so `init` cannot enable
  paid model review accidentally. Raw diagnostic details remain local.
