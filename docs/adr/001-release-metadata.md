# ADR 001: One version generator, existing signed artifact custody

Status: proposed; source-only until the normal source and artifact gates pass.

Follow the [release convention](https://github.com/misty-step/harness/blob/master/docs/release-convention.md)
and [foundations](https://foundations.mistystep.io/). Allie is one release unit:
the Rust CLI and its private browser worker ship together, never as two packages.
There is no deployed product site; GitHub Releases and CHANGELOG.md expose notes.

The immutable baseline is v0.1.85 at
04bb1a57d89538bc11c3d317e0f5e7eb16d7e807 (2026-10-01). Align the four source
manifest/lock versions to that already-published version once; the old preparer
at that source produces exactly 0.1.85. This is not a new release or a retag.
The baseline changelog entry preserves the published GitHub-generated notes.

The existing first-parent revision count is an artifact identity sequence, not
Conventional-Commit SemVer. Replace that calculation, not the signed release
engine: pinned release-please owns subsequent versions and CHANGELOG.md through
generated release PRs. Its default policy makes fixes patch, features minor,
and breaking changes major, including 0.x to 1.0.0. It never creates releases.
Non-release changes do not create another artifact/version. All four manifests
and the release manifest must agree; packaging only validates them.

The existing trusted green-master workflow still owns the tag, read-only build,
isolated OIDC signing, exactly three release assets, downloaded signature/runtime
smoke, serialized healthy-channel promotion, and independent quarantine. Only a
version-changing release commit is a publish candidate. An existing identical
tag is a no-op; a conflicting tag, decreasing version, or false release title
refuses. No signing key, credential, publisher, model, or updater is added.
CHANGELOG.md's current entry is embedded in the signed archive and used verbatim
as the GitHub Release body. Publication is not healthy-channel promotion.

Before activation, use squash-only repository settings, PR_TITLE/BLANK, and real
required CI checks. The merging agent approves only native CI runs for the
same-repository generator PR's current head using GitHub's workflow-approval API,
then reads exact-head CI, version/notes/privacy and merges normally. No stand-in
check, admin bypass, credential creation, or second independent review of the
generated notes is allowed. Implementation itself receives independent technical
review. Existing unrelated drafts stay owned by their authors.

Recovery remains the existing prior verified GitHub release, not a data restore.
Rehearse old/new signed artifacts in isolated consumer directories; do not move
the live healthy channel merely to demonstrate rollback. See the
[release/recovery runbook](../verification.md#release-and-recovery).
