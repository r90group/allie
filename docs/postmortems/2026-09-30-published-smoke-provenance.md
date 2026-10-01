# Postmortem: Published-bundle smoke without a committed consumer

- **Incident date:** 2026-09-30
- **Status:** Repaired; INF-101 is Done
- **Operational owner:** Allie continuous-release automation
- **Tracker:** Habitat INF-101

## Summary

The first automatic release built and signed a valid bundle, but its installed
consumer smoke ran outside a committed Git checkout. Allie's fail-closed
provenance check rejected verification. The failure class is a release smoke
fixture that does not satisfy the real consumer's provenance contract, exposed
only when the release path first ran on `master`.

## Impact

Observed: `v0.1.82` failed smoke with exit 2. Its candidate was quarantined as a
prerelease; healthy latest remained `v0.1.0`. No failed candidate reached the
healthy channel. There is no evidence of a broken bundle or customer data loss.

## Timeline

| Time (UTC, 2026-09-30) | Observation |
| --- | --- |
| 23:03 | [Run 36788167756](https://github.com/r90group/allie/actions/runs/36788167756) failed installed-artifact smoke; the signed failure alert was received. |
| 23:05 | Agent triage opened INF-101. |
| 23:34 | The normally merged repair triggered [run 36791080480](https://github.com/r90group/allie/actions/runs/36791080480), publishing the `v0.1.83` candidate. |
| Before 23:43 | Downloaded-bundle smoke passed, `v0.1.83` became healthy latest, and INF-101 was closed with revision-bound evidence. |

## Evidence and mechanism

The failing run emitted: `allie: provenance error: project_root is required;
run Allie against a git checkout with at least one commit`. Creating a manifest
alone did not establish the consumer Git identity required by `verify`.

[PR 88](https://github.com/r90group/allie/pull/88), merged as
`d76f023c87ac1edc1dfe67350e11bc9404f7b648`, repaired the fixture rather than
weakening provenance. [`scripts/smoke-published-release.sh`](../../scripts/smoke-published-release.sh)
creates and commits the consumer manifest before `doctor` and `verify`, passes
that checkout explicitly as `--project-root`, and requires emitted evidence to
match its actual Git revision.

The `retract-failed` job's red result is intentional: after the quarantine PATCH,
it reports the smoke failure and exits 1. It is not proof that quarantine failed.
The observed prerelease state and unchanged healthy latest establish recovery.
[The recovery record](https://github.com/r90group/allie/pull/87#issuecomment-5921326287)
on the initial continuous-release PR 87 preserves that evidence; PR 88 is the repair.

## Pokayoke

How can we pokayoke this so this kind of error never happens again?

One smoke owner now constructs the committed consumer before invoking
verification and compares the resulting evidence with that consumer's Git HEAD.
An empty checkout or accidentally verified source/bundle directory cannot satisfy
that comparison. Promotion remains conditional on successful downloaded-artifact
smoke; a failing candidate is quarantined without advancing healthy latest.

The passing regression is the actual signed, downloaded `v0.1.83` journey:
checksum, exact Sigstore issuer/signer and artifact identity verified;
`init → doctor → verify → publication` produced one captured state, zero
infrastructure failures, the real consumer revision, and publication `ready`.
[Smoke](https://github.com/r90group/allie/actions/runs/36791080480/job/110146201995)
and [promotion](https://github.com/r90group/allie/actions/runs/36791080480/job/110146429085)
both passed.

Residual: this closes the missing-consumer-provenance fixture class, not every
possible first-run release-orchestration error. Full pre-merge release-path
exercise is a separate requirement in the current continuous-deployment work;
this postmortem does not claim it already exists. Paid model execution was not
part of the smoke, and accessibility evidence is not a legal-compliance claim.

## Follow-up

The functional repair is [PR 88](https://github.com/r90group/allie/pull/88).
This documentation-only change adds the canonical postmortem link required by
INF-101's closure audit; it does not change release behavior or reopen the incident.
