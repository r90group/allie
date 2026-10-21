#!/usr/bin/env python3
"""Exercise the real scanner with synthetic Git files and event payloads."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scanner", type=Path, default=Path(__file__).with_name("secret-scan.sh"))
    parser.add_argument("--evidence", type=Path)
    args = parser.parse_args()
    scanner = args.scanner.resolve()
    token = "sk-" + "b" * 40
    results = []
    env = os.environ.copy()
    for key in ("GITHUB_EVENT_PATH", "GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"):
        env.pop(key, None)

    with tempfile.TemporaryDirectory(prefix="allie-scanner-controls-") as temp:
        root = Path(temp)

        def git(*command):
            subprocess.run(["git", *command], cwd=root, env=env, check=True, capture_output=True)

        def scan(name, expected, *, secret=False, event=None, input_error=False):
            process = subprocess.run([str(scanner)], cwd=root,
                env=env | ({"GITHUB_EVENT_PATH": str(event)} if event else {}),
                capture_output=True, text=True, timeout=30)
            output = process.stdout + process.stderr
            result = {"case": name, "expected_exit": expected, "actual_exit": process.returncode,
                      "redacted": token not in output,
                      "passed": process.returncode == expected and token not in output}
            if secret:
                result["rule_reported"] = "openai-key" in output
                result["passed"] &= result["rule_reported"]
            if input_error:
                result["input_error"] = "an input could not be read" in process.stderr
                result["reports_clean"] = "are clean" in process.stdout
                result["passed"] &= result["input_error"] and not result["reports_clean"]
            results.append(result)
            print(json.dumps(result), flush=True)

        git("init", "-q")
        canonical = root / ".agents/skills/verify-allie"
        canonical.mkdir(parents=True)
        skill = canonical / "SKILL.md"
        skill.write_text("safe synthetic skill\n")
        alias = root / ".claude/skills/verify-allie"
        alias.parent.mkdir(parents=True)
        alias.symlink_to("../../.agents/skills/verify-allie", target_is_directory=True)
        (root / ".gitignore").write_text("ignored/\n.allie/\n")
        git("add", ".agents", ".claude", ".gitignore")
        scan("tracked-relative-skill-directory-alias-clean", 0)
        skill.write_text("synthetic=" + token + "\n")
        scan("tracked-canonical-content-through-directory-alias", 1, secret=True)
        skill.write_text("safe synthetic skill\n")
        chain = root / "chain"
        chain.symlink_to(".claude/skills/verify-allie", target_is_directory=True)
        git("add", "chain")
        scan("directory-alias-chain-clean", 0)
        skill.write_text("synthetic=" + token + "\n")
        scan("directory-alias-chain-canonical-secret", 1, secret=True)
        skill.write_text("safe synthetic skill\n")
        ignored = root / "ignored"
        ignored.mkdir()
        private = ignored / "payload.txt"
        private.write_text("synthetic=" + token + "\n")
        file_alias = root / "file-alias.txt"
        file_alias.symlink_to("ignored/payload.txt")
        git("add", "file-alias.txt")
        scan("file-alias-content-when-target-is-ignored", 1, secret=True)
        private.write_text("safe\n")
        private.chmod(0)
        try:
            scan("unreadable-file-alias-target", 1, input_error=True)
        finally:
            private.chmod(0o600)
        git("rm", "-q", "--cached", "file-alias.txt")
        file_alias.unlink()
        (ignored / token).mkdir()
        text_alias = root / "directory-link-text"
        text_alias.symlink_to("ignored/" + token, target_is_directory=True)
        git("add", "directory-link-text")
        scan("secret-in-directory-link-text", 1, secret=True)
        git("rm", "-q", "--cached", "directory-link-text")
        text_alias.unlink()
        missing = root / "missing-link"
        missing.symlink_to("absent-target")
        git("add", "missing-link")
        scan("dangling-alias-fails-closed", 1, input_error=True)
        git("rm", "-q", "--cached", "missing-link")
        missing.unlink()
        skill.unlink()
        scan("vanished-tracked-canonical-file", 1, input_error=True)
        skill.write_text("safe\n")
        for name, path, mode in (("file", skill, 0o644), ("directory", canonical, 0o755)):
            path.chmod(0)
            try:
                scan("unreadable-tracked-canonical-" + name, 1, input_error=True)
            finally:
                path.chmod(mode)
        event = ignored / "event.json"
        event.write_text('{"action":"opened"}\n')
        scan("event-valid-json", 0, event=event)
        event_file_alias = ignored / "event-file-alias"
        event_file_alias.symlink_to("event.json")
        scan("event-valid-file-alias", 0, event=event_file_alias)
        event.write_text("synthetic=" + token + "\n")
        scan("event-regular-secret", 1, secret=True, event=event)
        scan("event-file-alias-secret", 1, secret=True, event=event_file_alias)
        event.chmod(0)
        try:
            scan("unreadable-event-file", 1, event=event, input_error=True)
        finally:
            event.chmod(0o600)
        event.unlink()
        scan("vanished-event-file", 1, event=event, input_error=True)
        scan("dangling-event-file-alias", 1, event=event_file_alias, input_error=True)
        scan("event-directory-must-be-rejected", 1, event=ignored, input_error=True)
        event_target = ignored / "event-target"
        event_target.mkdir()
        (event_target / "payload.json").write_text("synthetic=" + token + "\n")
        event_dir_alias = ignored / "event-directory-alias"
        event_dir_alias.symlink_to("event-target", target_is_directory=True)
        scan("event-directory-alias-must-be-rejected", 1, event=event_dir_alias, input_error=True)
        event_target.chmod(0)
        try:
            scan("unreadable-event-directory", 1, event=event_target, input_error=True)
            scan("unreadable-event-directory-alias", 1, event=event_dir_alias, input_error=True)
        finally:
            event_target.chmod(0o700)
        generated = root / ".allie/verify/latest/run/artifacts/console.json"
        generated.parent.mkdir(parents=True)
        generated.write_text("x" * 999995 + " " + token + "\n")
        scan("generated-evidence-secret-crosses-stream-boundary", 1, secret=True)
        generated.chmod(0)
        try:
            scan("unreadable-generated-evidence", 1, input_error=True)
        finally:
            generated.chmod(0o600)

    passed = all(item["passed"] for item in results)
    if args.evidence:
        args.evidence.write_text(json.dumps({"status": "passed" if passed else "failed",
            "scanner_sha256": hashlib.sha256(scanner.read_bytes()).hexdigest(),
            "results": results}, indent=2) + "\n")
    raise SystemExit(0 if passed else 1)


if __name__ == "__main__":
    main()
