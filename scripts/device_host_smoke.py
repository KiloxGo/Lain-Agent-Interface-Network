"""Native W3b cloud/device service with structured fake Codex; no paid providers."""
import argparse
from contextlib import closing
import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import signal
import sqlite3
import subprocess
import tempfile
import threading
import time
import uuid
import hashlib

from qq_memory_smoke import Smoke, executable, require


class Proxy:
    def __init__(self, upstream):
        self.block = threading.Event()
        self.lose_started = threading.Event()
        self.upstream = upstream
        fixture = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_GET(self):
                self.forward()

            def do_POST(self):
                self.forward()

            def forward(self):
                if fixture.block.is_set():
                    self.send_response(503)
                    self.send_header("Content-Length", "0")
                    self.end_headers()
                    return
                body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
                conn = http.client.HTTPConnection("127.0.0.1", fixture.upstream, timeout=30)
                try:
                    headers = {k: v for k, v in self.headers.items() if k.lower() not in {"host", "connection"}}
                    conn.request(self.command, self.path, body=body, headers=headers)
                    reply = conn.getresponse()
                    data = reply.read()
                    if self.command == "POST" and self.path.endswith("/events") and fixture.lose_started.is_set() and json.loads(body)["kind"] == "started":
                        fixture.lose_started.clear()
                        self.close_connection = True  # Commit succeeded; only the acknowledgement is lost.
                        return
                    self.send_response(reply.status)
                    self.send_header("Content-Type", "application/json")
                    self.send_header("Content-Length", str(len(data)))
                    self.end_headers()
                    self.wfile.write(data)
                except (BrokenPipeError, ConnectionResetError):
                    pass
                finally:
                    conn.close()

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.server.daemon_threads = True
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()

    def close(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=3)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    args = parser.parse_args()
    smoke = proxy = host = None
    log_handles = []
    with tempfile.TemporaryDirectory(prefix="lain-device-host-") as directory:
        root = Path(directory).resolve()
        try:
            smoke = Smoke(root, args.bin_dir.resolve(), None, 600)
            smoke.setup(None)
            smoke.fixture.login_release.set()
            smoke.start()
            channel = smoke.cli("channel", "list")[0]["binding"]
            smoke.channel = channel["id"]
            smoke.conversation = channel["conversation_id"]
            smoke.wait(lambda: smoke.snapshot()["connection"]["phase"] == "online", "synthetic QQ identity not online")
            proxy = Proxy(smoke.port)
            device = root / "device"
            native_env = {k: v for k, v in smoke.env.items()
                          if not k.upper().startswith(("LAIN_", "OPENAI_", "DEEPSEEK_", "ANTHROPIC_"))}
            native_env["LAIN_LOCAL_DIR"] = str(root / "no-owner-setup")
            keys = list(smoke.credentials)

            def cli(*items, success=True):
                r = subprocess.run([str(smoke.bins / executable("lain")), "--json", "device", *map(str, items)],
                                   env=native_env, capture_output=True, text=True, encoding="utf-8", timeout=60)
                require(not any(k in r.stdout+r.stderr for k in keys), "credential in device CLI output")
                require((r.returncode == 0) == success, "unexpected device CLI result")
                return json.loads(r.stdout) if success else None

            cli("init", "--directory", device, "--endpoint", f"http://127.0.0.1:{proxy.server.server_port}",
                "--workspace", smoke.workspace, "--label", "synthetic-native-host", "--expires-at-ms", int(time.time()*1000)+600000)
            registration = json.loads((device / "registration.json").read_text(encoding="utf-8"))
            keys.append(json.loads((device / "device-credential.json").read_text(encoding="utf-8"))["token"])
            did = registration["id"]
            smoke.cli("device", "pair", "--file", device / "registration.json")
            cli("claim", "--directory", device)
            if os.name == "nt":
                # Reading an already protected device must not spawn an ACL shell helper:
                # a helper can outlive a killed host and retain its inherited log handle.
                empty_path = root / "empty-status-path"
                empty_path.mkdir()
                status_env = native_env.copy()
                status_env["PATH"] = os.pathsep.join((str(empty_path), str(Path(os.environ["SYSTEMROOT"]) / "System32")))
                require(shutil.which("powershell.exe", path=status_env["PATH"]) is None, "ACL regression PATH contains PowerShell")
                model_requests = len(smoke.fixture.requests)
                workers = smoke.call("GET", "/v1/work-sessions")
                status = subprocess.run([str(smoke.bins / executable("lain")), "--json", "device", "status", "--directory", str(device)],
                                        env=status_env, cwd=empty_path, capture_output=True, text=True, encoding="utf-8", timeout=60)
                require(not any(k in status.stdout + status.stderr for k in keys), "credential in PATH-isolated device status output")
                require(not (device / "execution").exists() and smoke.call("GET", "/v1/work-sessions") == workers
                        and len(smoke.fixture.requests) == model_requests, "read-only device status started execution, Worker or model")
                require(status.returncode == 0, "protected device status requires PowerShell: " + status.stderr.strip())
                require(json.loads(status.stdout)["device"]["registration"]["id"] == did, "PATH-isolated status changed device identity")
                smoke.passed("windows_protected_device_status_without_powershell_or_execution")
            host_bin = smoke.bins / executable("lain-device-agent")
            no_policy = subprocess.run([str(host_bin), "--directory", str(device)], env=native_env,
                                       capture_output=True, text=True, encoding="utf-8", timeout=30)
            require(no_policy.returncode != 0 and not (device / "execution").exists(), "host started without explicit local policy")
            cli("host-configure", "--directory", device, "--codex-path", smoke.bins / executable("lain-test-app-server"),
                "--offline-grace-seconds", 2)
            cli("host-configure", "--directory", device, "--codex-path", smoke.bins / executable("lain-test-app-server"), success=False)
            policy_bytes = (device / "host-policy.json").read_bytes()
            smoke.passed("explicit_owner_only_native_policy_no_overwrite_or_auto_install")

            def start_host():
                nonlocal host
                output = open(root / f"host-{len(log_handles)}.log", "wb")
                log_handles.append(output)
                if os.name == "nt":
                    startup = subprocess.STARTUPINFO()
                    startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
                    startup.wShowWindow = subprocess.SW_HIDE
                    kwargs = {"creationflags": subprocess.CREATE_NEW_PROCESS_GROUP, "startupinfo": startup}
                else:
                    kwargs = {"start_new_session": True}
                host = subprocess.Popen([str(host_bin), "--directory", str(device)], env=native_env,
                                        stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT, **kwargs)

            def crash_host():
                nonlocal host
                if host and host.poll() is None:
                    host.kill()
                    host.wait(timeout=15)
                host = None

            def queue(text, sandbox="read_only", seconds=180):
                task = {"request_id": str(uuid.uuid4()), "expected_device_revision": 2,
                        "workspace_id": registration["workspaces"][0]["id"], "profile_id": "codex/local",
                        "sandbox": sandbox, "task": text, "expires_at_ms": int(time.time()*1000)+seconds*1000}
                path = root / f"task-{task['request_id']}.json"
                path.write_text(json.dumps(task), encoding="utf-8")
                return smoke.cli("device", "command", "queue", did, "--file", path)["id"]

            def current(cid):
                return smoke.call("GET", f"/v1/devices/{did}/commands/{cid}")

            def wait(cid, state, seconds=60):
                try:
                    return smoke.wait(lambda: (c if c["state"] in state else None) if (c := current(cid)) else None,
                                      "native host command did not reach " + str(state), seconds=seconds)
                except AssertionError:
                    c = current(cid)
                    print(json.dumps({"diagnostic_state": c["state"], "phase": c.get("execution", {}).get("phase"),
                                      "host_exit": host.poll() if host else None}), flush=True)
                    for output in log_handles:
                        output.flush()
                        text = Path(output.name).read_text(encoding="utf-8", errors="replace")[-8192:]
                        for key in keys:
                            text = text.replace(key, "[redacted]")
                        print(text, flush=True)
                    raise

            def waiting(cid):
                return smoke.wait(lambda: (c if c.get("execution", {}).get("pending_approvals") else None) if (c := current(cid)) else None,
                                  "native approval was not reported", seconds=60)

            def decide(cid, command, choice):
                a = command["execution"]["pending_approvals"][0]
                d = {"request_id": str(uuid.uuid4()), "fingerprint": a["fingerprint"], "choice": choice}
                path = root / f"decision-{d['request_id']}.json"
                path.write_text(json.dumps(d), encoding="utf-8")
                return smoke.cli("device", "command", "decide", did, cid, a["id"], "--file", path)

            def native_turn(cid):
                c = current(cid)
                sid = c.get("execution", {}).get("session_id")
                if not sid:
                    return None
                with closing(sqlite3.connect(device / "execution" / "worker.db")) as db:
                    row = db.execute("SELECT snapshot FROM worker_state WHERE session_id=?", (sid,)).fetchone()
                return json.loads(row[0]) if row else None

            start_host()
            blocked = queue("w3b-write-denied", "workspace_write")
            wait(blocked, {"rejected"})
            first = queue("w3b-native-result")
            done = wait(first, {"succeeded"})
            require(done["execution"]["result"]["output_tail"] == "w3b-native-result", "original Worker result missing")
            require(not smoke.call("GET", "/v1/work-sessions"), "device session copied into cloud Supervisor")
            burst = queue("command-output-burst")
            burst_done = wait(burst, {"succeeded"})
            require(burst_done["execution"]["result"]["output_tail"] == "FINAL SUMMARY 你好",
                    "command output burst lost the final device result")
            require(not burst_done["execution"]["result"]["history_truncated"],
                    "transient command progress was treated as missing history")
            smoke.passed("command_output_burst_preserves_device_completion_and_final_summary")
            smoke.passed("native_version_profile_workspace_policy_and_true_structured_turn_result")

            bounded = queue("history-output")
            result = wait(bounded, {"succeeded"})["execution"]["result"]
            require(len(result["output_tail"].encode()) <= 2048 and result["history_truncated"]
                    and len(result["reported_paths"]) == 8 and result["paths_withheld"] >= 8,
                    "native result limits/truncation facts lost")
            require(not any((smoke.workspace / p).exists() for p in result["reported_paths"]), "reported references treated as verified artifacts")
            smoke.passed("bounded_native_results_keep_truncation_and_reported_only_artifact_references")

            approval = queue("approve")
            pending = waiting(approval)
            require(pending["execution"]["pending_approvals"][0]["kind"] == "command", "provider approval object leaked")
            require(current(approval)["state"] == "running", "request auto-approved")
            allow_receipt = decide(approval, pending, "approve_once")
            allowed = wait(approval, {"succeeded"})
            require(allowed["execution"]["approval_reports"] == [{"approval_id": allow_receipt["approval_id"], "decision": allow_receipt["decision"], "stage": "applied"}],
                    "final native result lost exact Worker application evidence")
            denied = queue("approve")
            deny_receipt = decide(denied, waiting(denied), "deny")
            denied_result = wait(denied, {"succeeded"})  # Fixture completes a denied turn; no claim that the goal succeeded.
            require(denied_result["execution"]["approval_reports"] == [{"approval_id": deny_receipt["approval_id"], "decision": deny_receipt["decision"], "stage": "applied"}],
                    "denial application was inferred instead of reported exactly")
            smoke.passed("explicit_current_human_approval_and_denial_applied_by_existing_supervisor")

            unconfirmed = queue("approve-unconfirmed")
            unconfirmed_receipt = decide(unconfirmed, waiting(unconfirmed), "approve_once")
            unconfirmed_result = wait(unconfirmed, {"succeeded"})
            require(unconfirmed_result["execution"]["approval_reports"] == [{
                "approval_id": unconfirmed_receipt["approval_id"],
                "decision": unconfirmed_receipt["decision"], "stage": "received"}],
                "turn completion falsely confirmed an unacknowledged permission response")
            smoke.passed("native_completed_turn_without_permission_resolution_retains_received_not_applied")

            hidden = queue("file-approval")
            pending = waiting(hidden)
            request = pending["execution"]["pending_approvals"][0]
            require(request["truncated"] and "approve_once" not in request["choices"], "incomplete file scope could be approved")
            decide(hidden, pending, "deny")
            wait(hidden, {"succeeded"})
            smoke.passed("incomplete_native_scope_cannot_offer_remote_approval")

            parallel = queue("wait")
            wait(parallel, {"running"})
            smoke.input_turn("Synthetic QQ conversation remains available while a device Worker runs")
            c = current(parallel)
            require(c["state"] == "running", "chat implicitly stopped native work")
            smoke.cli("device", "command", "cancel", did, parallel, "--revision", c["revision"])
            wait(parallel, {"cancelled"})
            smoke.passed("cloud_chat_remains_available_and_cancel_waits_for_actual_terminal_evidence")

            shutdown = queue("wait")
            wait(shutdown, {"running"})
            if os.name == "nt":
                host.send_signal(signal.CTRL_BREAK_EVENT)
            else:
                os.killpg(host.pid, signal.SIGTERM)
            require(host.wait(timeout=25) == 0, "native host did not drain orderly shutdown")
            require(current(shutdown)["state"] == "unknown", "service shutdown fabricated cancellation")
            start_host()
            wait(shutdown, {"cancelled"})
            smoke.passed("native_service_shutdown_drains_owned_workers_and_restart_reconciles_unknown")

            uncertain_cancel = queue("reject-cancel")
            c = wait(uncertain_cancel, {"running"})
            smoke.cli("device", "command", "cancel", did, uncertain_cancel, "--revision", c["revision"])
            wait(uncertain_cancel, {"unknown"})
            smoke.passed("rejected_interrupt_and_process_stop_do_not_forge_cancelled")

            recovery = queue("wait")
            before = wait(recovery, {"running"})
            identity = json.loads((smoke.workspace / "fixture-history.json").read_text(encoding="utf-8"))
            crash_host()
            start_host()
            after = wait(recovery, {"cancelled"})
            resumed = json.loads((smoke.workspace / "fixture-history.json").read_text(encoding="utf-8"))
            require(after["execution"]["session_id"] == before["execution"]["session_id"] and resumed["id"] == identity["id"] and len(resumed["turns"]) == 1,
                    "restart created a replacement thread or prompt")
            smoke.passed("host_crash_resumes_bound_session_without_new_thread_or_turn")

            lost = queue("lost-prompt-ack")
            unknown = wait(lost, {"unknown"})
            accepted = json.loads((smoke.workspace / "fixture-history.json").read_text(encoding="utf-8"))
            crash_host()
            start_host()
            follow = queue("w3b-after-unknown")
            wait(follow, {"succeeded"})  # New command proves the prior uncertain one was processed, not blocking startup.
            require(current(lost)["state"] == "unknown" and len(accepted["turns"]) == 1,
                    "unmapped accepted turn was replayed or declared completed")
            require(native_turn(lost)["turns"][0]["provider_turn_id"] is None, "uncertain provider correlation invented")
            smoke.passed("lost_prompt_receipt_stays_unknown_and_never_replays_submission")

            proxy.lose_started.set()
            lost_event = queue("wait")
            wait(lost_event, {"cancelled", "unknown"})
            require(not proxy.lose_started.is_set(), "receipt-loss fault was not exercised")
            require(len(native_turn(lost_event)["turns"]) == 1, "lost HTTP acknowledgement duplicated native work")
            smoke.passed("lost_cloud_event_receipt_reuses_immutable_outbox_without_duplicate_effects")

            partition = queue("wait")
            wait(partition, {"running"})
            proxy.block.set()
            smoke.wait(lambda: (r if r and r["turns"][0]["turn"]["state"] in {"cancelled", "unknown"} else None) if (r := native_turn(partition)) else None,
                       "network loss did not stop owned native work", seconds=35)
            proxy.block.clear()
            wait(partition, {"cancelled", "unknown"}, seconds=40)
            smoke.passed("bounded_network_loss_stops_work_and_reports_outcome_after_reconnect")

            changed = queue("wait")
            wait(changed, {"running"})
            policy = json.loads(policy_bytes)
            policy["workspace_write"] = True
            (device / "host-policy.json").write_text(json.dumps(policy), encoding="utf-8")
            wait(changed, {"cancelled", "unknown"})
            (device / "host-policy.json").write_bytes(policy_bytes)
            smoke.passed("changed_local_policy_stops_effects_instead_of_silently_expanding_scope")

            revoked = queue("wait")
            wait(revoked, {"running"})
            smoke.cli("device", "revoke", did, "--revision", 2)
            smoke.wait(lambda: host.poll() is not None, "revoked host did not exit", seconds=35)
            require(current(revoked)["state"] == "unknown", "revocation falsely confirmed physical cancellation")
            with closing(sqlite3.connect(device / "execution" / "worker.db")) as db:
                require(db.execute("PRAGMA integrity_check").fetchone()[0] == "ok", "native storage integrity")
                require(db.execute("SELECT count(*) FROM lain_conversations").fetchone()[0] == 0, "device created another Lain conversation")
                require(db.execute("SELECT count(*) FROM channel_bindings").fetchone()[0] == 0, "device created a channel persona")
            for path in (device / "execution").rglob("*"):
                if path.is_file():
                    require(not any(k.encode() in path.read_bytes() for k in keys), "credential persisted in native operational state")
            smoke.passed("revocation_closes_device_authority_and_native_store_contains_no_persona_or_credentials")
            for output in log_handles:
                output.flush()
                require(not any(k.encode() in Path(output.name).read_bytes() for k in keys), "credential in host diagnostics")
            with closing(sqlite3.connect(smoke.local / "state.db")) as db:
                schema = db.execute("PRAGMA user_version").fetchone()[0]
            report = {"checks": smoke.checks, "model_requests": len(smoke.fixture.requests), "paid_calls": 0,
                      "real_qq_sends": 0, "worker_backend": "structured_fixture_only", "schema": schema, "host": os.name}
        finally:
            if host and host.poll() is None:
                host.kill()
                host.wait(timeout=15)
            for output in log_handles:
                output.close()
            if proxy:
                proxy.close()
            if smoke:
                smoke.stop()
                if smoke.fixture:
                    smoke.fixture.model_release.set()
                    smoke.fixture.login_release.set()
                    smoke.fixture.close()
    report["cleanup"] = not root.exists()
    report["binary_sha256"] = {n: hashlib.sha256((args.bin_dir / executable(n)).read_bytes()).hexdigest()
                               for n in ["lain", "laind", "lain-device-agent", "lain-test-app-server"]}
    print(json.dumps(report))


if __name__ == "__main__":
    main()
