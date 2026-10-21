#!/usr/bin/env python3
"""Drive the actual CLI using disposable synthetic consumers; publish count-only proof."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile

REPO = Path(__file__).resolve().parents[4]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["doctor", "drive", "published"])
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    evidence = args.evidence.resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    proof = evidence / f"{args.mode}.json"
    if proof.exists():
        parser.error("use a fresh evidence directory; prior proof must survive")
    cache = Path.home() / ".cache/tmp"
    cache.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    for key in ("OPENROUTER_API_KEY", "OPENAI_API_KEY", "ALLIE_BROWSER_WORKER",
                "ALLIE_AGENTIC_WORKER", "GITHUB_REPOSITORY"):
        env.pop(key, None)
    result = {"mode": args.mode, "status": "in_progress", "commands": [],
              "source_sha": subprocess.check_output(
                  ["git", "rev-parse", "HEAD"], cwd=REPO, text=True).strip(),
              "owner_sign_in": "n/a: local CLI; no hosted Allie identity",
              "model": "disabled; provider keys removed from child environment"}
    with tempfile.TemporaryDirectory(prefix="allie-verify-", dir=cache) as scratch:
        work = Path(scratch)

        def run(label, command, expected=0, extra_env=None):
            command = [str(x) for x in command]
            with subprocess.Popen(command, cwd=work, env=env | (extra_env or {}),
                                  text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                  start_new_session=True) as process:
                try:
                    stdout, stderr = process.communicate(timeout=240)
                except BaseException:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.communicate()
                    result["commands"].append({"action": label, "status": "interrupted; process group killed"})
                    raise
            result["commands"].append({"action": label, "argv": [
                x.replace(str(REPO), "$ALLIE_REPO").replace(str(work), "$RUN")
                for x in command], "expected_exit": expected, "exit": process.returncode})
            if process.returncode != expected:
                # Raw diagnostics stay outside committed evidence, even on failure.
                fd, name = tempfile.mkstemp(prefix="allie-verify-error-", suffix=".log", dir=cache)
                with os.fdopen(fd, "w") as log:
                    log.write(stdout + stderr)
                raise RuntimeError(f"{label}: expected exit {expected}, got {process.returncode}; private log {name}")
            return stdout

        def read(path):
            return json.loads((work / path).read_text())

        binary = REPO / "target/debug/allie"
        try:
            if args.mode == "published":
                tag = run("resolve healthy release", ["gh", "api", "repos/r90group/allie/releases/latest",
                                                       "--jq", ".tag_name"]).strip()
                sha = run("resolve release revision", ["gh", "api", f"repos/r90group/allie/commits/{tag}",
                                                        "--jq", ".sha"]).strip()
                env["TMPDIR"] = str(work)
                output = run("checksum, exact signer, installed consumer journey", [
                    REPO / "scripts/smoke-published-release.sh", tag, sha])
                events = [json.loads(line) for line in output.splitlines()
                          if line.startswith('{"event":"allie.')]
                assert {event["event"] for event in events} == {
                    "allie.artifact_consumer", "allie.published_smoke"}
                result["release"] = {"tag": tag, "sha": sha, "events": events}
            else:
                run("scaffold model-off consumer", [binary, "init", "--manifest", "manifest.yml",
                    "--app-name", "Synthetic verification", "--fixture-dir", REPO / "fixtures/login"])
                assert "enabled: false" in (work / "manifest.yml").read_text()
                run("consumer preflight", [binary, "doctor", "--manifest", "manifest.yml", "--out", "doctor"])
                doctor = read("doctor/doctor.json")
                assert doctor["status"] in ("pass", "warn")
                assert all(check["status"] != "fail" for check in doctor["checks"])
                result["doctor"] = {check["name"]: check["status"] for check in doctor["checks"]}
                if args.mode == "drive":
                    for command in (["git", "init", "-q"], ["git", "add", "manifest.yml"],
                                    ["git", "-c", "user.name=Allie Synthetic", "-c",
                                     "user.email=allie@example.invalid", "-c", "commit.gpgsign=false",
                                     "commit", "-q", "-m", "synthetic consumer"]):
                        run("seed consumer revision", command)
                    run("verify consumer", [binary, "verify", "--manifest", "manifest.yml",
                                             "--out", "verify", "--project-root", work])
                    packet = read("verify/run/evidence.json")
                    report = read("verify/reporters/allie-report.json")
                    consumer_sha = run("read consumer provenance", ["git", "rev-parse", "--short", "HEAD"]).strip()
                    assert packet["run"]["git_sha"] == consumer_sha
                    assert packet["summary"]["states_captured"] == 1
                    assert packet["summary"]["infrastructure_failures"] == 0
                    assert report["status"] in ("approved", "needs_review")
                    for name in ("allie-report.json", "allie-compliance-report.json", "allie-report.html",
                                 "allie-report.md", "junit.xml", "allie.sarif"):
                        assert (work / "verify/reporters" / name).stat().st_size > 0
                    assert list((work / "verify/run/artifacts").glob("*mobile*.png"))
                    result["consumer"] = {"states_captured": 1, "infrastructure_failures": 0,
                                          "release_status": report["status"], "reporters": 6,
                                          "mobile_capture": True, "consumer_provenance": "matched"}
                    raw = work / "verify/run/evidence.json"
                    before = hashlib.sha256(raw.read_bytes()).hexdigest()
                    run("publish count-only summary", [binary, "publication", "--verify-root", "verify", "--out", "public"])
                    run("refuse canonical evidence", [binary, "publication", "--verify-root", "verify",
                        "--out", "refused", "--include", "run/evidence.json"], expected=2)
                    assert read("public/publication-receipt.json")["status"] == "ready"
                    refusal = read("refused/publication-receipt.json")
                    assert refusal["status"] == "refused" and refusal["retryable"]
                    assert refusal["refused"][0]["publication_class"] == "sensitive_local"
                    assert hashlib.sha256(raw.read_bytes()).hexdigest() == before
                    names = {"allie-public-summary.json", "allie-public-summary.md",
                             "publication-receipt.json", "allie-run-manifest.json"}
                    assert {p.name for p in (work / "public").iterdir()} == names
                    assert not (work / "refused/run/evidence.json").exists()
                    for name in names:
                        shutil.copyfile(work / "public" / name, evidence / name)
                    result["publication"] = {"status": "ready", "files": 4, "raw_refusal_exit": 2,
                                             "canonical_unchanged": True}
                    for label, manifest, expected in (
                        ("authenticated", "auth-fixture-flow.yml", 0),
                        ("signed-out", "auth-fixture-flow-negative.yml", 1),
                    ):
                        run(f"verify {label} target", [binary, "verify", "--manifest", REPO / "examples" / manifest,
                            "--out", label, "--project-root", work], expected=expected, extra_env={
                                "ALLIE_AUTH_FIXTURE_USER": "allie@example.com",
                                "ALLIE_AUTH_FIXTURE_PASSWORD": "allie-fixture-pass"})
                        p = read(f"{label}/run/evidence.json")
                        state = next(s for s in p["coverage"]["state_metadata"] if s["id"] == "dashboard")
                        status = read(f"{label}/reporters/allie-report.json")["status"]
                        if expected == 0:
                            assert state["url"].endswith("/dashboard.html") and not state["state_errors"]
                            assert status in ("approved", "needs_review")
                            for artifact in (work / label).rglob("*"):
                                if artifact.is_file():
                                    assert b"allie-fixture-pass" not in artifact.read_bytes()
                        else:
                            assert status == "blocked"
                            assert any("auth-lost" in error for error in state["state_errors"])
                        result[label] = {"release_status": status,
                                         "dashboard_session": expected == 0,
                                         "auth_lost": expected != 0}
                    # Unset credential is deterministic and never contacts a remote target.
                    env.pop("ALLIE_TEST_DO_NOT_SET_SECRET", None)
                    run("missing credential fails closed", [binary, "run", "--manifest",
                        REPO / "examples/trust-missing-credential.yml", "--out", "missing"], expected=2)
                    assert read("missing/evidence.json")["summary"]["status"] == "error"
                    result["missing_credential"] = "error; exit 2"
            result["status"] = "passed"
        except Exception:
            result["status"] = "failed"
            raise
        finally:
            proof.write_text(json.dumps(result, indent=2) + "\n")
    assert proof.is_file()
    print(f"{args.mode}: {result['status']}; evidence retained after scratch cleanup")


if __name__ == "__main__":
    main()
