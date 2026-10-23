# Allie verification map

Run from the repository root after Launch and Doctor in [the skill](../SKILL.md).
Every `drive` creates independent synthetic consumer data and fixture ports;
every `published` downloads the real signed artifact. Use a fresh evidence
directory per run, preserve expected exits, and report skipped paths explicitly.

| Feature | Helper mode | Observable proof |
| --- | --- | --- |
| [Install released CLI](release.md) | `published` | Checksum/signature, signed revision, installed consumer |
| [Initialize and diagnose](setup.md) | `doctor`, `drive` | Model-off manifest and usable browser worker |
| [Verify and inspect reports](reports.md) | `drive` | Captured state, consumer provenance, six reporters, mobile |
| [Audit authenticated targets](authenticated-targets.md) | `drive` | Real fixture login reaches dashboard; signed-out blocks |
| [Publish safe evidence](publication.md) | `drive` | Four public files; canonical evidence refused and unchanged |

Proof pairs each CLI action and exit with asserted resulting state. Allie's
own account sign-in is n/a: it has no hosted user account. Target auth is a
separate, synthetic test. Current deliberate limits: no production target
audit, live paid model review, external SSO, or mailbox delivery proof; no
network-throttling/slow-target proof. The full repository gate supplements
these top journeys; it does not turn these limits into passes.
