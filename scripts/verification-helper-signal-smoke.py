#!/usr/bin/env python3
"""Interrupt real Doctor runs and check owned-process cleanup and retained proof."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile
import time

REPO = Path(__file__).resolve().parents[1]
HELPER = REPO / ".agents/skills/verify-allie/scripts/verify.py"


def group_alive(group):
    for entry in Path("/proc").iterdir():
        if entry.name.isdigit():
            try:
                fields = (entry / "stat").read_text().rsplit(")", 1)[1].split()
                if int(fields[2]) == group and fields[0] != "Z":
                    return True
            except (FileNotFoundError, ProcessLookupError):
                pass
    return False


def kill_group(group):
    try:
        os.killpg(group, signal.SIGKILL)
    except ProcessLookupError:
        pass


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path)
    args = parser.parse_args()
    results = []
    receipts = {}
    cache = Path.home() / ".cache/tmp"
    cache.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="allie-signal-controls-", dir=cache) as temp:
        for sig in (signal.SIGTERM, signal.SIGINT):
            evidence = Path(temp) / sig.name
            group = None
            scratch = None
            with subprocess.Popen([str(HELPER), "doctor", "--evidence", str(evidence)],
                                  cwd=REPO, stdout=subprocess.DEVNULL,
                                  stderr=subprocess.DEVNULL, start_new_session=True) as helper:
                try:
                    deadline = time.monotonic() + 30
                    while time.monotonic() < deadline and helper.poll() is None:
                        children = Path(f"/proc/{helper.pid}/task/{helper.pid}/children")
                        for pid in children.read_text().split():
                            proc = Path("/proc") / pid
                            try:
                                command = (proc / "cmdline").read_bytes().split(b"\0")
                                if command[:2] != [str(REPO / "target/debug/allie").encode(), b"doctor"]:
                                    continue
                                candidate = int(pid)
                                if os.getpgid(candidate) != candidate:
                                    raise RuntimeError("Doctor must have its own process group")
                                owned_scratch = (proc / "cwd").resolve()
                                if owned_scratch.parent != cache or not owned_scratch.name.startswith("allie-verify-"):
                                    raise RuntimeError("unexpected Doctor scratch ownership")
                                group, scratch = candidate, owned_scratch
                                os.killpg(group, signal.SIGSTOP)
                                break
                            except FileNotFoundError:
                                continue
                        if group is not None:
                            break
                        time.sleep(0.002)
                    if group is None:
                        raise RuntimeError("real Doctor child was not observed before the deadline")
                    helper.send_signal(sig)
                    helper.wait(timeout=15)
                    deadline = time.monotonic() + 5
                    while group_alive(group) and time.monotonic() < deadline:
                        time.sleep(0.01)
                    proof = evidence / "doctor.json"
                    receipt = json.loads(proof.read_text()) if proof.is_file() else {}
                    receipts[sig.name] = receipt
                    result = {"signal": sig.name, "helper_exit": helper.returncode,
                              "expected_exit": 128 + sig,
                              "owned_group_stopped": not group_alive(group),
                              "doctor_reaped": not Path(f"/proc/{group}").exists(),
                              "scratch_removed": not scratch.exists(),
                              "proof_retained": proof.is_file(),
                              "receipt_status": receipt.get("status"),
                              "receipt_signal": receipt.get("signal")}
                    result["passed"] = (result["helper_exit"] == result["expected_exit"]
                        and result["owned_group_stopped"] and result["doctor_reaped"]
                        and result["scratch_removed"] and result["proof_retained"]
                        and result["receipt_status"] == "interrupted"
                        and result["receipt_signal"] == sig.name)
                    results.append(result)
                    print(json.dumps(result), flush=True)
                finally:
                    # Only the helper and the isolated group observed as its own child.
                    if group is not None:
                        kill_group(group)
                    if helper.poll() is None:
                        helper.kill()
                    helper.wait(timeout=15)
                    if scratch is not None and scratch.exists():
                        shutil.rmtree(scratch)
    passed = all(item["passed"] for item in results)
    if args.evidence:
        args.evidence.write_text(json.dumps({"status": "passed" if passed else "failed",
            "helper_sha256": hashlib.sha256(HELPER.read_bytes()).hexdigest(),
            "results": results, "receipts": receipts}, indent=2) + "\n")
    raise SystemExit(0 if passed else 1)


if __name__ == "__main__":
    main()
