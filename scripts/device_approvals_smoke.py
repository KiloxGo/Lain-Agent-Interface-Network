"""Native W3a evidence/decision protocol; synthetic reports, no Worker or model."""
import argparse
from contextlib import closing
import copy
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import time
import uuid

from qq_memory_smoke import Smoke, executable, require


def fingerprint(evidence):
    a = evidence["pending_approvals"][0]
    scope = [evidence["execution_id"], evidence["session_id"], evidence["turn_id"],
             a["id"], a["kind"], a["description"], a["truncated"], a["choices"]]
    encoded = json.dumps(scope, ensure_ascii=False, separators=(",", ":")).encode()
    return hashlib.sha256(b"lain-device-approval-v1\0" + encoded).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    args = parser.parse_args()
    smoke = None
    with tempfile.TemporaryDirectory(prefix="lain-device-approvals-") as directory:
        root = Path(directory).resolve()
        try:
            smoke = Smoke(root, args.bin_dir.resolve(), None, 300)
            smoke.setup(None)
            smoke.fixture.login_release.set()
            smoke.start()
            device_dir = root / "device"
            device_env = {k: v for k, v in smoke.env.items()
                          if not k.upper().startswith(("LAIN_", "OPENAI_", "DEEPSEEK_", "ANTHROPIC_"))}
            device_env["LAIN_LOCAL_DIR"] = str(root / "no-owner-setup")
            keys = []

            def cli(*items, success=True, owner=False):
                result = subprocess.run(
                    [str(smoke.bins / executable("lain")), "--json", *map(str, items)],
                    env=smoke.env if owner else device_env, capture_output=True,
                    text=True, encoding="utf-8", timeout=45,
                )
                require(not any(key in result.stdout + result.stderr for key in (*smoke.credentials, *keys)),
                        "credential in protocol output")
                require((result.returncode == 0) == success, "unexpected CLI outcome: " + str(items[:3]))
                return json.loads(result.stdout) if success else None

            def file(name, value):
                path = root / name
                path.write_text(json.dumps(value, ensure_ascii=False), encoding="utf-8")
                return path

            cli("device", "init", "--directory", device_dir, "--endpoint",
                f"http://127.0.0.1:{smoke.port}", "--label", "synthetic-approval-device",
                "--workspace", smoke.workspace, "--expires-at-ms", int(time.time()*1000)+600000)
            r = json.loads((device_dir / "registration.json").read_text(encoding="utf-8"))
            keys.append(json.loads((device_dir / "device-credential.json").read_text(encoding="utf-8"))["token"])
            did = r["id"]
            cli("device", "pair", "--file", device_dir / "registration.json", owner=True)
            cli("device", "claim", "--directory", device_dir)

            def queue(name):
                task = {"request_id": str(uuid.uuid4()), "expected_device_revision": 2,
                        "workspace_id": r["workspaces"][0]["id"], "profile_id": "codex/local",
                        "sandbox": "read_only", "task": "Synthetic evidence only; no Worker invocation.",
                        "expires_at_ms": int(time.time()*1000)+180000}
                return cli("device", "command", "queue", did, "--file", file(name, task), owner=True)["id"]

            def report(cid, seq, kind, evidence, success=True):
                value = {"request_id": str(uuid.uuid4()), "sequence": seq, "kind": kind,
                         "detail": "Synthetic reported fact, not verified native execution.", "execution": evidence}
                return cli("device", "command", "event", "--directory", device_dir, cid,
                           "--file", file(f"event-{cid}-{seq}-{kind}.json", value), success=success)

            def wait_evidence():
                e = {"execution_id": str(uuid.uuid4()), "session_id": str(uuid.uuid4()),
                     "turn_id": str(uuid.uuid4()), "phase": "waiting_for_approval", "result": None,
                     "pending_approvals": [{"id": str(uuid.uuid4()), "kind": "command_execution",
                         "description": "核对 fixture；仅此请求", "truncated": False,
                         "choices": ["approve_once", "deny", "cancel"], "fingerprint": ""}]}
                e["pending_approvals"][0]["fingerprint"] = fingerprint(e)
                return e

            def begin(cid, e):
                preparing = {**e, "turn_id": None, "phase": "preparing", "pending_approvals": []}
                report(cid, 1, "accepted", preparing)
                report(cid, 2, "started", e)

            cid = queue("first-work.json")
            e = wait_evidence()
            begin(cid, e)
            a = e["pending_approvals"][0]
            require(cli("device", "command", "inspect", "--directory", device_dir, cid)["execution"] == e,
                    "normalized execution binding differs")
            smoke.passed("separate_principal_and_exact_normalized_execution_scope")
            d = {"request_id": str(uuid.uuid4()), "fingerprint": a["fingerprint"], "choice": "approve_once"}
            decision_file = file("decision.json", d)
            cli("device", "command", "decide", did, cid, a["id"], "--file", decision_file, success=False)
            receipt = cli("device", "command", "decide", did, cid, a["id"], "--file", decision_file, owner=True)
            require(cli("device", "command", "fetch-decisions", "--directory", device_dir, cid) == [receipt],
                    "bound device did not receive exact decision")
            require(cli("device", "command", "show", did, cid, owner=True)["execution"] == e,
                    "recording decision falsely claimed Worker application")
            smoke.passed("only_owner_decides_and_receipt_does_not_claim_applied")

            smoke.stop()
            smoke.start()
            require(cli("device", "command", "decide", did, cid, a["id"], "--file", decision_file, owner=True) == receipt,
                    "lost owner acknowledgement changed receipt after restart")
            require(cli("device", "command", "decisions", did, cid, owner=True) == [receipt], "restart lost ledger")
            cli("device", "command", "decide", did, cid, a["id"], "--file",
                file("changed-choice.json", {**d, "choice": "deny"}), owner=True, success=False)
            smoke.passed("restart_and_lost_receipt_preserve_immutable_human_choice")

            secret = copy.deepcopy(e)
            secret["pending_approvals"][0]["description"] = keys[0]
            secret["pending_approvals"][0]["fingerprint"] = fingerprint(secret)
            report(cid, 3, "progress", secret, success=False)
            require(not (device_dir / "command-events" / f"{cid}-3.request.json").exists(),
                    "secret persisted before validation")
            result = {**e, "phase": "completed", "pending_approvals": [], "result": {
                "output_tail": "Synthetic Worker-reported tail, not a goal acceptance.", "command_failures": 1,
                "reported_paths": ["reports/fixture.txt"], "paths_withheld": 1, "history_truncated": True}}
            report(cid, 3, "succeeded", result)
            require(cli("device", "command", "show", did, cid, owner=True)["execution"]["result"] == result["result"],
                    "result lost failure/truncation facts")
            require(not (smoke.workspace / "reports" / "fixture.txt").exists(), "test invented artifact bytes")
            smoke.passed("secret_rejected_before_journal_and_reported_results_remain_factual")

            second = queue("second-work.json")
            e2 = wait_evidence()
            begin(second, e2)
            current = cli("device", "command", "show", did, second, owner=True)
            cli("device", "command", "cancel", did, second, "--revision", current["revision"], owner=True)
            d2 = {**d, "request_id": str(uuid.uuid4()), "fingerprint": e2["pending_approvals"][0]["fingerprint"]}
            cli("device", "command", "decide", did, second, e2["pending_approvals"][0]["id"],
                "--file", file("late-decision.json", d2), owner=True, success=False)
            require(cli("device", "command", "show", did, second, owner=True)["state"] == "running",
                    "cancellation request claimed stopped")
            cli("device", "revoke", did, "--revision", 2, owner=True)
            cli("device", "command", "fetch-decisions", "--directory", device_dir, cid, success=False)
            require(cli("device", "command", "decisions", did, cid, owner=True) == [receipt], "owner lost audit receipt")
            smoke.passed("cancel_blocks_new_permission_and_revoke_blocks_device_receipt_access")

            require(not smoke.call("GET", "/v1/work-sessions") and not smoke.fixture.requests,
                    "evidence/approval protocol started a Worker or model")
            smoke.stop()
            with closing(sqlite3.connect(smoke.local / "state.db")) as db:
                require(db.execute("PRAGMA user_version").fetchone()[0] == 26, "wrong schema")
                require(db.execute("PRAGMA integrity_check").fetchone()[0] == "ok", "storage integrity")
                require(db.execute("SELECT count(*) FROM device_approval_decisions").fetchone()[0] == 1, "extra choice persisted")
                require(db.execute("SELECT count(*) FROM device_executions").fetchone()[0] == 2, "execution correlation lost")
            smoke.passed("durable_schema26_ledgers_without_worker_model_or_real_qq_effects")
            output = {"checks": smoke.checks, "schema": 26, "paid_calls": 0, "model_requests": 0,
                      "worker_executions": 0, "real_qq_sends": 0, "host": os.name}
        finally:
            if smoke:
                smoke.stop()
                if smoke.fixture:
                    smoke.fixture.model_release.set()
                    smoke.fixture.login_release.set()
                    smoke.fixture.close()
    output["cleanup"] = not root.exists()
    output["binary_sha256"] = {name: hashlib.sha256((args.bin_dir / executable(name)).read_bytes()).hexdigest()
                               for name in ["lain", "laind"]}
    print(json.dumps(output))


if __name__ == "__main__":
    main()
