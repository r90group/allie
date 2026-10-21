# Audit an authenticated target

Engineers give Allie a target login recipe; Allie must reach the gated page
and must block if the session is lost, including an HTTP-200 login wall.

## Sub-features

- `target-login`: browser submits synthetic credentials and reaches dashboard.
- `target-signed-out`: missing session records `auth-lost` and blocks release.
- `target-secret`: injected credential value stays out of positive-run artifacts.

## How to get to it (user POV)

Use `allie verify` with `examples/auth-fixture-flow.yml` or the signed-out
`examples/auth-fixture-flow-negative.yml`. `allie run` and storageState are
additional entry points exercised by `npm run auth:smoke` in the full gate.

## Driving it with verify.py

Preconditions: Doctor succeeded. Credentials belong only to the repository's
disposable fixture; no real owner's credentials or inbox are involved.

- **Sign in and audit.** Run `.agents/skills/verify-allie/scripts/verify.py drive --evidence "$evidence"`.
  It supplies fixture variables by environment, submits `#email`, `#password`
  and `button[type=submit]`, waits for `#dashboard`, then audits `/dashboard.html`.
  Expect no state errors, dashboard final URL, neutral/approved release, and
  no injected password bytes in any positive artifact.
- **Signed-out failure.** The same run attempts the dashboard without creating
  a session. Expect verify exit 1, release `blocked`, and `auth-lost` state error.

## Gotchas

- Allie itself has no sign-in; this is target-auth behavior, not real owner
  authentication proof. Real SSO/inbox access and expired/reused links are not
  covered by this synthetic session fixture.
- The signed-out recipe intentionally never submits a password. It proves
  missing-session rejection; it does not prove wrong-password validation.
- Login fixture source contains its own test password. Keep canonical negative
  artifacts local; committed evidence contains counts and statuses only.
