"""Native Windows conversation -> device Codex -> permission -> result acceptance.

Uses two real Kernel processes, the existing structured fake Codex, fake Responses
and ConPTY. No QQ, paid inference, service installation or personal state. This is
same-machine acceptance, not cross-device TLS or real-provider acceptance.
"""
import argparse
from contextlib import closing
import hashlib
import json
import os
from pathlib import Path
import signal
import sqlite3
import subprocess
import tempfile
import time
import uuid

from desk_terminal_smoke import Terminal
from qq_memory_smoke import Smoke, executable, require


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    parser.add_argument("--artifacts", type=Path)
    parser.add_argument("--portrait-file", type=Path)
    args = parser.parse_args()
    require(os.name == "nt", "native Windows ConPTY is required")
    artifacts = args.artifacts.resolve() if args.artifacts else None
    if artifacts:
        require(not artifacts.is_relative_to(Path(__file__).resolve().parents[1]), "artifacts must stay outside checkout")
        artifacts.mkdir(parents=True, exist_ok=True)
    smoke = terminal = host = output = None
    report = {}
    with tempfile.TemporaryDirectory(prefix="lain-qq-memory-w4-") as directory:
        root = Path(directory)
        try:
            smoke = Smoke(root, args.bin_dir.resolve(), None, 600)
            smoke.setup(None)
            smoke.env.pop("LAIN_NAPCAT_CONFIG", None)
            smoke.env.pop("LAIN_PORTRAIT_FILE", None)
            if args.portrait_file:
                smoke.env["LAIN_PORTRAIT_FILE"] = str(args.portrait_file.resolve())
            smoke.start()
            conversation = smoke.call("POST", "/v1/conversations", expected=201)["id"]
            device = root / "device"
            native_env = {k: v for k, v in smoke.env.items()
                          if not k.upper().startswith(("LAIN_", "OPENAI_", "DEEPSEEK_", "ANTHROPIC_"))}
            native_env["LAIN_LOCAL_DIR"] = str(root / "no-owner-setup")

            def native(*items):
                result = subprocess.run([str(smoke.bins / executable("lain")), "--json", "device", *map(str, items)],
                                        env=native_env, capture_output=True, text=True, encoding="utf-8", timeout=60)
                require(result.returncode == 0, "native device setup failed: " + " ".join(map(str, items[:2])))
                return json.loads(result.stdout)

            native("init", "--directory", device, "--endpoint", f"http://127.0.0.1:{smoke.port}",
                   "--workspace", smoke.workspace, "--label", "w4-native-device", "--expires-at-ms", int(time.time()*1000)+600000)
            registration = json.loads((device / "registration.json").read_text(encoding="utf-8"))
            did = registration["id"]
            smoke.cli("device", "pair", "--file", device / "registration.json")
            native("claim", "--directory", device)
            native("host-configure", "--directory", device, "--codex-path", smoke.bins / executable("lain-test-app-server"),
                   "--offline-grace-seconds", 30)

            def host_check():
                def device_files():
                    files = {}
                    for p in device.rglob("*"):
                        if p.is_file():
                            relative = p.relative_to(device)
                            if relative in {Path("execution/host.lock"), Path("execution/worker.db.lock")}:
                                # Both service and Store hold File::try_lock for
                                # their lifetime. Windows denies reading these
                                # two exact files. Do not bypass either lock.
                                stat = p.stat()
                                files[str(relative)] = ("owner-lock", stat.st_dev, stat.st_ino, stat.st_size, stat.st_mtime_ns)
                            else:
                                try:
                                    files[str(relative)] = p.read_bytes()
                                except PermissionError as error:
                                    raise AssertionError("unexpected locked diagnostic fixture file: " + str(relative)) from error
                    return files

                before = device_files()
                result = subprocess.run([str(smoke.bins / executable("lain-device-agent")), "--check", "--directory", str(device)],
                                        env=native_env, capture_output=True, text=True, encoding="utf-8", timeout=60)
                require(result.returncode == 0, "read-only native host check is unavailable")
                checked = json.loads(result.stdout)
                after = device_files()
                changed = sorted(k for k in before.keys() | after.keys() if before.get(k) != after.get(k))
                require(not changed, "files changed during host check: " + ", ".join(changed))
                require(checked["service_availability"] == "not_proven_by_prerequisite_check", "prerequisites were mislabeled online")
                require(checked["start_argv"] == ["lain-device-agent", "--directory", str(device)], "start arguments lost native path identity")
                return checked

            checked = host_check()
            require(checked["ready_for_explicit_start"] and checked["backend_version"] == "0.160.0", "valid configured host failed diagnostics")
            require(not (device / "execution").exists() and not smoke.call("GET", "/v1/work-sessions") and not smoke.fixture.requests,
                    "diagnostic started Worker state or model")
            smoke.passed("read_only_host_check_verifies_native_prerequisites_and_cloud_authority_without_starting_service")
            output = open(root / "native-host.log", "wb")
            startup = subprocess.STARTUPINFO()
            startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
            startup.wShowWindow = subprocess.SW_HIDE
            def start_host():
                return subprocess.Popen([str(smoke.bins / executable("lain-device-agent")), "--directory", str(device)],
                                        env=native_env, stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT,
                                        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP, startupinfo=startup)

            def stop_host():
                if host and host.poll() is None:
                    host.send_signal(signal.CTRL_BREAK_EVENT)
                    host.wait(timeout=25)

            host = start_host()

            def respond(body):
                markers = {"delegate-fixture", "after-result-fixture", "parallel-fixture"}
                prompt = next(i["content"] for i in reversed(body["input"])
                              if i.get("role") == "user" and isinstance(i.get("content"), str) and i["content"] in markers)
                if prompt == "delegate-fixture":
                    return [{"type": "function_call", "name": "device_queue", "call_id": str(uuid.uuid4()),
                             "arguments": json.dumps({"device_id": did, "expected_device_revision": 2,
                                 "workspace_id": registration["workspaces"][0]["id"], "profile_id": "codex/local",
                                 "sandbox": "read_only", "task": "approve", "expires_at_ms": int(time.time()*1000)+180000})}]
                return [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "fixture reply: " + prompt}]}]

            smoke.fixture.respond = respond

            def current(cid):
                return smoke.call("GET", f"/v1/devices/{did}/commands/{cid}")

            def wait_state(cid, states):
                return smoke.wait(lambda: c if (c := current(cid))["state"] in states else None,
                                  "remote task did not reach " + str(states), seconds=60)

            def waiting(cid):
                return smoke.wait(lambda: c if (c := current(cid)).get("execution", {}).get("pending_approvals") else None,
                                  "native permission not reported", seconds=60)

            def settled_receipt(cid, observed):
                # A cloud GET can see an event before the device has finished
                # atomically publishing its matching local receipt. Choose a
                # quiescent boundary for whole-tree no-write comparison.
                event = observed["last_event"]["event"]
                path = device / "execution" / cid / ("receipt-" + str(event["sequence"]) + ".json")
                smoke.wait(lambda: path.exists() and json.loads(path.read_text(encoding="utf-8"))["event"] == event,
                           "matching native receipt has not been durably published", seconds=30)

            def visible(text):
                smoke.wait(lambda: text in terminal.text(), "terminal did not show " + text, seconds=30)

            def capture(name):
                if artifacts:
                    time.sleep(0.15)
                    terminal.image(artifacts / (name + ".png"))

            def open_terminal():
                return Terminal([str(smoke.bins / executable("lain")), "desk", "--conversation", conversation], smoke.env)

            def review_permission(choice):
                terminal.write("\t")
                visible("device tasks")
                terminal.write("\r")
                visible("Device task")
                smoke.wait(lambda: "No recorded decision" in terminal.text(), "decision receipts not loaded")
                # Native ConPTY accepts SS3 F4; CSI14 is dropped before the app.
                terminal.write("\x1bOS")
                visible("Remote Worker permission")
                for _ in range(30):
                    if "Ctrl-D deny" in terminal.text():
                        break
                    terminal.write("\x1b[6~")
                    time.sleep(0.03)
                visible("Ctrl-D deny")
                capture("remote-permission-" + choice)
                terminal.write("\x19" if choice == "approve_once" else "\x04")

            def direct_queue(text):
                work = {"request_id": str(uuid.uuid4()), "expected_device_revision": 2,
                        "workspace_id": registration["workspaces"][0]["id"], "profile_id": "codex/local",
                        "sandbox": "read_only", "task": text, "expires_at_ms": int(time.time()*1000)+180000}
                path = root / ("work-" + work["request_id"] + ".json")
                path.write_text(json.dumps(work), encoding="utf-8")
                queued = smoke.cli("device", "command", "queue", did, "--file", path, "--conversation", conversation)
                return queued, path

            terminal = open_terminal()
            visible("lain // wired")
            terminal.write("delegate-fixture\r")
            visible("action awaits review")
            require(not smoke.call("GET", f"/v1/devices/{did}/commands"), "proposal executed before owner review")
            terminal.write("/review\r")
            visible("Outbound task: Changed")
            capture("dispatch-comparison")
            for _ in range(60):
                if "Ctrl-Y approve once" in terminal.text():
                    break
                terminal.write("\x1b[6~")
                time.sleep(0.04)
            visible("Ctrl-Y approve once")
            capture("exact-actor-device-dispatch")
            terminal.write("\x19")
            commands = smoke.wait(lambda: smoke.call("GET", f"/v1/devices/{did}/commands"), "confirmed work was not queued")
            cid = commands[0]["id"]
            command = waiting(cid)
            settled_receipt(cid, command)
            checked = host_check()
            require(checked["ready_for_explicit_start"], "diagnostic could not inspect a running host")
            page = smoke.call("GET", f"/v1/conversations/{conversation}/device-work")
            require(page["work"][0]["command"]["id"] == cid and page["work"][0]["origin"]["action_id"], "Actor dispatch lost its origin")
            history = smoke.call("GET", f"/v1/conversations/{conversation}?recent=true")
            action = history["turns"][0]["actions"][0]
            require(command["input"]["request_id"] == action["id"] and command["input"]["task"] == "approve", "stable exact dispatch changed")
            smoke.passed("natural_actor_dispatch_requires_review_and_keeps_exact_task_stable_identity_and_origin")

            terminal.write("retained chat draft")
            review_permission("deny")
            wait_state(cid, {"succeeded"})
            receipts = smoke.call("GET", f"/v1/devices/{did}/commands/{cid}/approvals")
            require(len(receipts) == 1 and receipts[0]["decision"]["choice"] == "deny", "explicit deny receipt missing")
            require(current(cid)["execution"]["approval_reports"] == [{"approval_id": receipts[0]["approval_id"], "decision": receipts[0]["decision"], "stage": "applied"}],
                    "denial did not return native exact application evidence")
            visible("recorded")
            terminal.write("\x1b")
            visible("retained chat draft")
            terminal.write("\x15after-result-fixture\r")
            visible("fixture reply: after-result-fixture")
            require(cid in json.dumps(smoke.fixture.requests[-1]["input"]) and "untrusted_device_execution_evidence" in json.dumps(smoke.fixture.requests[-1]["input"]), "ordinary Lain conversation missed fresh remote evidence")
            require(not smoke.call("GET", "/v1/work-sessions"), "remote work became a fabricated cloud session")
            capture("chat-with-remote-result")
            smoke.passed("explicit_remote_denial_and_factual_result_return_preserve_chat_draft_without_model_wakes")

            allowed, _ = direct_queue("approve")
            waiting(allowed["id"])
            review_permission("approve_once")
            wait_state(allowed["id"], {"succeeded"})
            receipts = smoke.call("GET", f"/v1/devices/{did}/commands/{allowed['id']}/approvals")
            require(receipts[0]["decision"]["choice"] == "approve_once", "allow receipt missing")
            require(current(allowed["id"])["execution"]["approval_reports"] == [{"approval_id": receipts[0]["approval_id"], "decision": receipts[0]["decision"], "stage": "applied"}],
                    "allow did not return native exact application evidence")
            with closing(sqlite3.connect(device / "execution/worker.db")) as db:
                state = json.loads(db.execute("SELECT snapshot FROM worker_state WHERE session_id=?", (current(allowed["id"])["execution"]["session_id"],)).fetchone()[0])
            require(any(a["decision"] == "approve_once" and a["status"] == "resolved" for a in state["approvals"]), "native Supervisor did not apply owner choice")
            terminal.write("\x12")  # Read the actual task/decision snapshots, no POST.
            visible("Worker application: reported confirmed")
            visible("Device receipt: reported received")
            capture("permission-application-reported")
            smoke.passed("explicit_remote_allow_reaches_the_existing_native_supervisor")

            original = current(allowed["id"])
            session = original["execution"]["session_id"]

            def local_worker():
                with closing(sqlite3.connect(device / "execution/worker.db")) as db:
                    return json.loads(db.execute("SELECT snapshot FROM worker_state WHERE session_id=?", (session,)).fetchone()[0])

            original_thread = local_worker()["provider_session_id"]
            terminal.write("\x1b")
            visible("lain // wired")
            terminal.write("Lain draft survives remote follow-up")
            terminal.write("\t\r")
            visible("Device task")
            terminal.write("\x1b[17~")  # F6 on native terminals.
            visible("Follow-up to same remote Worker")
            exact_followup = "second exact instruction"
            terminal.write(exact_followup + "\r")
            visible("Remote Worker follow-up")
            for _ in range(60):
                if "Ctrl-Y send exact follow-up" in terminal.text():
                    break
                terminal.write("\x1b[6~")
                time.sleep(0.04)
            visible("Ctrl-Y send exact follow-up")
            capture("same-session-followup-review")
            terminal.write("\x19")
            followup_base = f"/v1/conversations/{conversation}/device-work/{did}"
            child = smoke.wait(lambda: smoke.call("GET", f"{followup_base}/{allowed['id']}/followups"), "explicit follow-up was not recorded")
            child = wait_state(child["id"], {"succeeded"})
            require(child["input"]["task"] == exact_followup and child["execution"]["session_id"] == session,
                    "follow-up changed text or Worker session")
            require(child["execution"]["turn_id"] != original["execution"]["turn_id"], "follow-up reused old turn")
            state = local_worker()
            require(state["provider_session_id"] == original_thread and len(state["turns"]) == 2, "follow-up created a new provider thread or repeated a turn")
            capture("same-session-followup-result")
            terminal.write("\x1b")
            visible("Lain draft survives remote follow-up")
            require(len(smoke.fixture.requests) == 2, "direct Worker follow-up woke Lain model")
            smoke.passed("terminal_followup_preserves_actual_session_provider_thread_new_turn_and_lain_draft_without_model_wake")

            def continue_cli(parent, text):
                request = {"request_id": str(uuid.uuid4()), "expected_device_revision": 2,
                           "expected_parent_revision": parent["revision"], "text": text,
                           "expires_at_ms": min(registration["expires_at_ms"], int(time.time()*1000)+180000)}
                file = root / ("followup-" + request["request_id"] + ".json")
                file.write_text(json.dumps(request), encoding="utf-8")
                queued = smoke.cli("device", "command", "continue", did, parent["id"], "--conversation", conversation, "--file", file)
                return queued, file

            settled_receipt(child["id"], child)
            stop_host()
            host = start_host()
            third, third_file = continue_cli(current(child["id"]), "third exact instruction after native restart")
            third = wait_state(third["id"], {"succeeded"})
            replay = smoke.cli("device", "command", "continue", did, child["id"], "--conversation", conversation, "--file", third_file)
            require(replay["id"] == third["id"], "exact follow-up retry created another command")
            state = local_worker()
            require(third["execution"]["session_id"] == session and state["provider_session_id"] == original_thread and len(state["turns"]) == 3,
                    "native host restart replaced thread/session or replayed follow-up")
            capture("same-session-after-native-host-restart")
            smoke.passed("native_host_restart_and_exact_followup_retry_keep_original_provider_thread_and_three_distinct_turns")

            lost, lost_file = continue_cli(current(third["id"]), "lost-prompt-ack")
            lost = wait_state(lost["id"], {"unknown"})
            settled_receipt(lost["id"], lost)
            before_history = json.loads((smoke.workspace / "fixture-history.json").read_text(encoding="utf-8"))
            require(before_history["id"] == original_thread and len(before_history["turns"]) == 4, "lost receipt fixture did not accept one original-thread turn")
            stop_host()
            host = start_host()
            replay = smoke.cli("device", "command", "continue", did, third["id"], "--conversation", conversation, "--file", lost_file)
            require(replay["id"] == lost["id"], "unknown follow-up got a replacement command")
            smoke.wait(lambda: current(lost["id"])["state"] == "unknown", "unknown follow-up was revived")
            require(json.loads((smoke.workspace / "fixture-history.json").read_text(encoding="utf-8")) == before_history,
                    "restart or exact retry replayed an uncertain native submit")
            smoke.call("POST", f"{followup_base}/{lost['id']}/followups", {"request_id": str(uuid.uuid4()),
                       "expected_device_revision": 2, "expected_parent_revision": lost["revision"], "text": "must not replay",
                       "expires_at_ms": int(time.time()*1000)+60000}, expected=409)

            # Deliberately synthetic historical device observations: cloud facts
            # cannot create/adopt a missing native session or parent journal.
            stop_host()
            historical, _ = direct_queue("synthetic history only; never execute")
            historical_session = str(uuid.uuid4())
            historical_execution, historical_turn = str(uuid.uuid4()), str(uuid.uuid4())
            for sequence, kind, phase in [(1, "accepted", "preparing"), (2, "started", "running"), (3, "succeeded", "completed")]:
                proof = {"execution_id": historical_execution, "session_id": historical_session,
                         "turn_id": None if sequence == 1 else historical_turn, "phase": phase,
                         "pending_approvals": [], "result": None}
                if sequence == 3:
                    proof["result"] = {"output_tail": "Synthetic observation only; no native execution", "command_failures": 0,
                                       "reported_paths": [], "paths_withheld": 0, "history_truncated": True}
                event = {"request_id": str(uuid.uuid4()), "sequence": sequence, "kind": kind,
                         "detail": "Synthetic fixture observation, not native execution proof", "execution": proof}
                event_file = root / ("historical-event-" + str(sequence) + ".json")
                event_file.write_text(json.dumps(event), encoding="utf-8")
                native("command", "event", "--directory", device, historical["id"], "--file", event_file)
            missing, _ = continue_cli(current(historical["id"]), "must not create replacement Worker")
            host = start_host()
            missing = wait_state(missing["id"], {"rejected"})
            require(missing.get("execution") is None and not (device / "execution" / missing["id"] / "launch.json").exists(),
                    "missing original binding produced native execution evidence/effects")
            with closing(sqlite3.connect(device / "execution/worker.db")) as db:
                require(db.execute("SELECT count(*) FROM work_sessions").fetchone()[0] == 2, "synthetic parent was adopted as a real native session")

            terminal.write("\x15")

            running, file = direct_queue("wait")
            active = wait_state(running["id"], {"running"})
            require(json.loads((smoke.workspace / ("fixture-thread-" + original_thread + ".json")).read_text(encoding="utf-8")) == before_history,
                    "unknown follow-up replayed before the next independent command")
            require(current(lost["id"])["state"] == "unknown", "unknown child was declared complete")
            smoke.passed("unknown_followup_ack_keeps_original_thread_and_never_replays_or_accepts_another_turn")
            require(host.poll() is None, "one unavailable continuation stopped unrelated native work")
            smoke.passed("missing_original_native_parent_is_rejected_without_replacement_session_and_service_keeps_working")
            terminal.write("parallel-fixture\r")
            visible("fixture reply: parallel-fixture")
            require(current(running["id"])["state"] == "running", "Lain chat stopped independent Worker work")
            model_count = len(smoke.fixture.requests)
            terminal.write("\x11")
            smoke.wait(lambda: not terminal.process.isalive(), "terminal did not close")
            terminal.close()
            require(current(running["id"])["state"] == "running", "closing frontend stopped Worker")
            terminal = open_terminal()
            visible("lain // wired")
            visible(running["id"])
            require(len(smoke.fixture.requests) == model_count, "frontend restore invoked a model")
            replay = smoke.cli("device", "command", "queue", did, "--file", file, "--conversation", conversation)
            require(replay["id"] == running["id"] and current(running["id"])["execution"]["session_id"] == active["execution"]["session_id"], "reconnect/retry created another task or session")
            capture("reconnected-with-independent-worker")
            smoke.passed("parallel_chat_frontend_close_restore_and_exact_retry_do_not_repeat_work")

            smoke.stop()
            smoke.start()
            terminal.write("\x12")
            visible("lain // wired")
            require(current(running["id"])["execution"]["session_id"] == active["execution"]["session_id"], "Kernel restart replaced remote session")
            terminal.write("cancel navigation draft\t")
            visible("device tasks")
            terminal.write("\r")
            visible("Device task")
            visible(running["id"])
            terminal.write("\x1b[19~")  # F8: inspect exact task, no mutation.
            visible("Request cancellation of this device task")
            terminal.write("\x1b")
            require(not current(running["id"])["cancel_requested"], "closing cancel review stopped work")
            terminal.write("\x1b")
            visible("cancel navigation draft")
            terminal.write("\t")
            visible("device tasks")
            terminal.write("\r")
            visible(running["id"])
            terminal.write("\x1b[19~")
            visible("Request cancellation of this device task")
            for _ in range(60):
                if "Ctrl-K request cancellation" in terminal.text():
                    break
                terminal.write("\x1b[6~")
                time.sleep(0.04)
            visible("Ctrl-K request cancellation")
            capture("explicit-task-cancel-review")
            terminal.write("\x0b")
            final = wait_state(running["id"], {"cancelled"})
            settled_receipt(running["id"], final)
            require(final["execution"]["phase"] == "cancelled", "cancel acknowledgement was treated as execution evidence")
            terminal.write("\x12")
            visible("Cancel requested: true")
            visible("Device reports cancellation for task")
            capture("device-reported-cancellation")
            terminal.write("\x1b")
            visible("cancel navigation draft")
            require(len(smoke.fixture.requests) == model_count, "recovery/cancel generated background inference")
            require(not smoke.fixture.sent and smoke.fixture.connection is None, "fixture unexpectedly connected/sent QQ")
            smoke.passed("Kernel_restart_retains_remote_identity_and_cancel_waits_for_native_terminal_evidence")
            smoke.stop()
            checked = host_check()
            require(not checked["ready_for_explicit_start"] and checked["cloud_authority"] == "unavailable", "unreachable Kernel was mislabeled ready/online")
            smoke.start()
            smoke.passed("read_only_diagnostics_report_unreachable_cloud_without_claiming_online_or_mutating_state")
            report = {"checks": smoke.checks, "fake_model_calls": model_count, "paid_calls": 0, "qq_sends": 0,
                      "platform": "native Windows ConPTY; isolated same-machine cloud/device", "native_gui_font_acceptance": False}
        except Exception:
            if artifacts and terminal:
                terminal.image(artifacts / "failure.png")
                (artifacts / "failure.txt").write_text(terminal.text(), encoding="utf-8")
            if artifacts:
                import re
                host_log = root / "native-host.log"
                if host_log.exists():
                    safe_log = re.sub(r"lain-device-[0-9a-f]{64}", "[device credential redacted]", host_log.read_text(encoding="utf-8", errors="replace"))
                    if smoke:
                        for credential in smoke.credentials:
                            safe_log = safe_log.replace(credential, "[fixture credential redacted]")
                    (artifacts / "native-host-redacted.log").write_text(safe_log, encoding="utf-8")
                if smoke:
                    try:
                        facts = smoke.call("GET", f"/v1/conversations/{conversation}/device-work")
                        (artifacts / "device-work-failure.json").write_text(json.dumps(facts, ensure_ascii=False, indent=2), encoding="utf-8")
                    except Exception:
                        pass  # Preserve original failure if the cloud is deliberately offline.
            raise
        finally:
            if terminal:
                terminal.close()
            if host and host.poll() is None:
                host.send_signal(signal.CTRL_BREAK_EVENT)
                try:
                    host.wait(timeout=25)
                except subprocess.TimeoutExpired:
                    host.kill()
                    host.wait(timeout=15)
            if output:
                output.close()
            if smoke:
                smoke.stop()
                if smoke.fixture:
                    smoke.fixture.close()
    report["temporary_state_cleaned"] = not root.exists()
    report["binary_sha256"] = {n: hashlib.sha256((args.bin_dir / executable(n)).read_bytes()).hexdigest()
                               for n in ["lain", "laind", "lain-device-agent", "lain-test-app-server"]}
    if artifacts:
        (artifacts / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=2), flush=True)


if __name__ == "__main__":
    main()
