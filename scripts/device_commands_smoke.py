"""Native W2b control-plane rehearsal; fake OneBot/model, no Worker execution."""
import argparse
import hashlib
import http.client
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import time
import uuid
from contextlib import closing

from qq_memory_smoke import Smoke, executable, require


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    args = parser.parse_args()
    smoke = None
    with tempfile.TemporaryDirectory(prefix="lain-device-commands-") as directory:
        root = Path(directory).resolve()
        try:
            smoke = Smoke(root, args.bin_dir.resolve(), None, 300)
            smoke.setup(None)
            smoke.fixture.login_release.set()
            smoke.start()
            device_dir = root / "device"
            # No administrator environment or saved setup in any device invocation.
            device_env = {k: v for k, v in smoke.env.items() if not k.upper().startswith("LAIN_")}
            device_env["LAIN_LOCAL_DIR"] = str(root / "no-administrator-setup")
            credential = []

            def cli(*items, success=True):
                result = subprocess.run(
                    [str(smoke.bins / executable("lain")), "--json", *map(str, items)],
                    env=device_env, capture_output=True, text=True, encoding="utf-8", timeout=45,
                )
                require(not any(s in result.stdout + result.stderr for s in (*smoke.credentials, *credential)),
                        "credential in device CLI output")
                require((result.returncode == 0) == success, "device CLI result mismatch: " + str(items[:2]))
                return json.loads(result.stdout) if success else None

            def file(name, body):
                path = root / name
                path.write_text(json.dumps(body), encoding="utf-8")
                return path

            cli("device", "init", "--directory", device_dir, "--endpoint",
                f"http://127.0.0.1:{smoke.port}", "--label", "synthetic-command-device",
                "--workspace", smoke.workspace, "--expires-at-ms", int(time.time()*1000)+600000)
            registration = json.loads((device_dir / "registration.json").read_text())
            credential.append(json.loads((device_dir / "device-credential.json").read_text())["token"])
            did = registration["id"]
            smoke.cli("device", "pair", "--file", device_dir / "registration.json")
            cli("device", "claim", "--directory", device_dir)
            cli("device", "report", "--directory", device_dir, "--sequence", 1)
            smoke.passed("separate_device_principal_without_administrator_setup")

            def queue(name):
                task = {"request_id": str(uuid.uuid4()), "expected_device_revision": 2,
                        "workspace_id": registration["workspaces"][0]["id"], "profile_id": "codex/local",
                        "sandbox": "read_only", "task": "Synthetic work intent; do not execute a Worker.",
                        "expires_at_ms": int(time.time()*1000)+180000}
                path = file(name, task)
                c = smoke.cli("device", "command", "queue", did, "--file", path)
                require(smoke.cli("device", "command", "queue", did, "--file", path) == c,
                        "duplicate admission changed command identity")
                return c

            first = queue("first-work.json")
            cid = first["id"]
            page = cli("device", "command", "fetch", "--directory", device_dir)
            require(page["through"] == 1 and page["changes"][0]["command_id"] == cid, "queued command missing")
            require(cli("device", "command", "inspect", "--directory", device_dir, cid) == first,
                    "device command differs from immutable owner intent")
            smoke.passed("durable_intent_dedup_and_metadata_resync")

            def post(command_id, seq, kind, name):
                value = {"request_id": str(uuid.uuid4()), "sequence": seq, "kind": kind,
                         "detail": "Synthetic device observation, not real execution evidence."}
                path = file(name, value)
                receipt = cli("device", "command", "event", "--directory", device_dir, command_id, "--file", path)
                return value, path, receipt

            event, path, receipt = post(cid, 1, "accepted", "accepted.json")
            journal = device_dir / "command-events"
            saved = journal / f"{cid}-1.request.json"
            require(json.loads(saved.read_text()) == event, "immutable device event not journaled")
            local_receipt = journal / f"{cid}-1.receipt.json"
            require(local_receipt.is_relative_to(root), "unsafe synthetic receipt path")
            local_receipt.unlink()  # Crash after server commit, before local receipt retention.
            require(cli("device", "command", "receipt", "--directory", device_dir, cid, event["request_id"]) == receipt,
                    "lost receipt not inspectable")
            require(cli("device", "command", "event", "--directory", device_dir, cid, "--file", path) == receipt,
                    "same event replay changed receipt")
            require(local_receipt.exists(), "lost local receipt was not recovered")
            changed = {**event, "request_id": str(uuid.uuid4())}
            cli("device", "command", "event", "--directory", device_dir, cid,
                "--file", file("changed-event.json", changed), success=False)
            smoke.passed("event_journal_lost_receipt_recovery_without_reexecution")

            post(cid, 2, "started", "started.json")
            current = smoke.cli("device", "command", "show", did, cid)
            cancelling = smoke.cli("device", "command", "cancel", did, cid, "--revision", current["revision"])
            require(cancelling["cancel_requested"] and cancelling["state"] == "running", "cancel falsely claimed process stopped")
            _, _, stopped = post(cid, 3, "cancelled", "cancelled.json")
            require(stopped["state"] == "cancelled", "device cancellation evidence missing")
            smoke.passed("cancel_request_is_separate_from_device_stop_receipt")

            second = queue("second-work.json")
            _, _, accepted = post(second["id"], 1, "accepted", "second-accepted.json")
            before = cli("device", "command", "fetch", "--directory", device_dir)["next_after"]
            smoke.stop()
            smoke.start()
            state = cli("device", "command", "inspect", "--directory", device_dir, second["id"])
            require(state["state"] == "accepted" and state["last_event"] == accepted, "restart requeued or changed accepted work")
            page = cli("device", "command", "fetch", "--directory", device_dir, "--after", before)
            require(not page["changes"], "restart invented a duplicate change")
            smoke.passed("kernel_restart_preserves_receipts_and_consumer_cursor")

            connection = http.client.HTTPConnection("127.0.0.1", smoke.port, timeout=5)
            try:
                connection.request("POST", f"/v1/devices/{did}/commands", body="{}",
                                   headers={"Authorization": "Bearer " + credential[0], "Content-Type": "application/json"})
                response = connection.getresponse()
                require(response.status == 401, "device key gained administrator command authority")
                response.read()
            finally:
                connection.close()
            smoke.cli("device", "revoke", did, "--revision", 2)
            cli("device", "command", "inspect", "--directory", device_dir, second["id"], success=False)
            require(smoke.cli("device", "command", "show", did, second["id"])["state"] == "unknown",
                    "device revocation claimed that accepted execution stopped")
            require(not smoke.call("GET", "/v1/work-sessions") and not smoke.fixture.requests,
                    "command protocol started Worker/model execution")
            smoke.passed("revocation_rechecks_authority_and_retains_execution_uncertainty")
            smoke.stop()
            with closing(sqlite3.connect(smoke.local / "state.db")) as db:
                require(db.execute("PRAGMA user_version").fetchone()[0] == 26, "schema mismatch")
                require(db.execute("PRAGMA integrity_check").fetchone()[0] == "ok", "storage integrity")
            report = {"checks": smoke.checks, "schema": 26, "paid_calls": 0, "model_requests": 0,
                      "real_qq_sends": 0, "worker_executions": 0, "host": os.name}
        finally:
            if smoke:
                smoke.stop()
                if smoke.fixture:
                    smoke.fixture.model_release.set()
                    smoke.fixture.login_release.set()
                    smoke.fixture.close()
    report["cleanup"] = not root.exists()
    report["binary_sha256"] = {name: hashlib.sha256((args.bin_dir / executable(name)).read_bytes()).hexdigest()
                               for name in ["lain", "laind"]}
    print(json.dumps(report))


if __name__ == "__main__":
    main()
