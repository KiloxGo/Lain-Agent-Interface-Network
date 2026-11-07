"""Native Windows compact task cards and Work result reader; synthetic state only.

No interactive clipboard access, paid provider or real QQ. Optional pictures render
the actual ConPTY VT stream and are labelled as synthetic by the shared renderer.
"""
import argparse
import json
import os
from pathlib import Path
import signal
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
    args = parser.parse_args()
    require(os.name == "nt", "native Windows ConPTY required")
    artifacts = args.artifacts.resolve() if args.artifacts else None
    if artifacts:
        require(not artifacts.is_relative_to(Path(__file__).resolve().parents[1]), "artifacts stay outside checkout")
        artifacts.mkdir(parents=True, exist_ok=True)
    smoke = terminal = host = log = None
    with tempfile.TemporaryDirectory(prefix="lain-qq-memory-activity-") as directory:
        try:
            root = Path(directory)
            smoke = Smoke(root, args.bin_dir.resolve(), None, 600)
            smoke.setup(None)
            smoke.env.pop("LAIN_NAPCAT_CONFIG", None)
            smoke.env.pop("LAIN_PORTRAIT_FILE", None)
            smoke.start()
            conversation = smoke.call("POST", "/v1/conversations", expected=201)["id"]
            device = root / "device"
            env = {k: v for k, v in smoke.env.items() if not k.upper().startswith(("LAIN_", "OPENAI_", "DEEPSEEK_", "ANTHROPIC_"))}
            env["LAIN_LOCAL_DIR"] = str(root / "no-owner-config")

            def native(*items):
                reply = subprocess.run([str(smoke.bins / executable("lain")), "--json", "device", *map(str, items)], env=env, capture_output=True, text=True, encoding="utf-8", timeout=30)
                require(reply.returncode == 0, "isolated device setup failed")
                return json.loads(reply.stdout)

            native("init", "--directory", device, "--endpoint", f"http://127.0.0.1:{smoke.port}", "--label", "fixture-pc", "--workspace", smoke.workspace, "--expires-at-ms", int(time.time()*1000)+600000, "--codex-work")
            registration = json.loads((device / "registration.json").read_text(encoding="utf-8"))
            smoke.cli("device", "pair", "--file", device / "registration.json")
            native("claim", "--directory", device)
            native("host-configure", "--directory", device, "--codex-path", smoke.bins / executable("lain-test-app-server"))
            did = registration["id"]

            def queue(task):
                return smoke.call("POST", f"/v1/conversations/{conversation}/device-work/{did}", {
                    "request_id": str(uuid.uuid4()), "expected_device_revision": 2,
                    "workspace_id": registration["workspaces"][0]["id"], "profile_id": "codex/local",
                    "sandbox": "read_only", "task": task, "expires_at_ms": int(time.time()*1000)+180000,
                })["id"]

            def command(cid):
                return smoke.call("GET", f"/v1/devices/{did}/commands/{cid}")

            log = open(root / "host.log", "wb")
            startup = subprocess.STARTUPINFO()
            startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
            startup.wShowWindow = subprocess.SW_HIDE
            host = subprocess.Popen([str(smoke.bins / executable("lain-device-agent")), "--directory", str(device)], env=env, stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT, creationflags=subprocess.CREATE_NEW_PROCESS_GROUP, startupinfo=startup)
            old = queue("historical-result-only")
            smoke.wait(lambda: command(old)["state"] == "succeeded", "historical fixture did not complete")
            terminal = Terminal([str(smoke.bins / executable("lain")), "desk", "--conversation", conversation], smoke.env)
            terminal.resize(50, 160)

            def visible(text):
                smoke.wait(lambda: text in terminal.text(), "expected UI text absent: " + text)

            def capture(name):
                if artifacts:
                    terminal.image(artifacts / (name + ".png"))

            visible("connected to Kernel")
            terminal.write("\t")
            visible("historical-result-only")  # Remote baseline has arrived.
            terminal.write("\x1b")
            visible("to Lain")
            require("historical-result-only" not in terminal.text(), "old result was replayed into Chat")
            terminal.write("keep-chat-draft")
            live = queue("activity-result")
            smoke.wait(lambda: command(live)["state"] == "succeeded", "live fixture did not complete")
            visible("Codex · 已返回结果")
            visible("查看结果")
            require("Kernel state" not in terminal.text() and "RESULT BOLD" not in terminal.text(), "Chat contains an expanded device dump")
            capture("compact-chat-card")
            terminal.write("\x1b[18~")  # F7: activity menu, not task submission.
            visible("Enter 查看/展开")
            terminal.write("\r")
            visible("RESULT BOLD")
            visible("let result = 42;")
            require("**RESULT BOLD**" not in terminal.text(), "Work output was not Markdown-rendered")
            capture("markdown-work-result")
            # A second turn must keep both requests and receipts, including after UI restart.
            parent = command(live)
            device_revision = smoke.call("GET", f"/v1/devices/{did}")["device"]["revision"]
            child = smoke.call("POST", f"/v1/conversations/{conversation}/device-work/{did}/{live}/followups", {
                "request_id": str(uuid.uuid4()), "expected_device_revision": device_revision,
                "expected_parent_revision": parent["revision"], "text": "activity-followup",
                "expires_at_ms": int(time.time()*1000)+180000,
            })["id"]
            smoke.wait(lambda: command(child)["state"] == "succeeded", "follow-up fixture did not complete")
            terminal.write("\x1b")
            visible("keep-chat-draft")
            terminal.write("\t")
            visible("activity-followup")
            visible("fixture-pc")
            visible(str(smoke.workspace))
            capture("work-task-context")
            terminal.write("\r")
            visible("SECOND RESULT")
            terminal.write("\x1b[1;5H")  # Ctrl-Home: read the earlier turn in this family.
            visible("RESULT BOLD")
            visible("activity-result")
            visible("activity-followup")
            capture("two-turn-history")
            terminal.write("\x1b[1;5F")  # Ctrl-End restores latest-follow mode.
            visible("SECOND RESULT")
            terminal.write("\x1bOQ")  # F2 copy menu, deliberately do not copy.
            visible("Markdown 原文")
            terminal.write("\x1b")
            terminal.write("\x1b[18~")
            visible("Device evidence")
            visible(child)
            capture("explicit-diagnostics")
            terminal.write("\x1b[18~")
            visible("SECOND RESULT")
            terminal.write("\x1b")
            visible("keep-chat-draft")
            terminal.write("\x11")
            smoke.wait(lambda: not terminal.process.isalive(), "UI did not exit")
            terminal.close()
            terminal = Terminal([str(smoke.bins / executable("lain")), "desk", "--conversation", conversation], smoke.env)
            terminal.resize(50, 160)
            visible("connected to Kernel")
            terminal.write("\t")
            visible("activity-followup")
            terminal.write("\r")
            visible("SECOND RESULT")
            terminal.write("\x1b[1;5H")
            visible("RESULT BOLD")
            visible("activity-result")
            terminal.write("\x1b")
            visible("to Lain")
            require("Codex · 已返回结果" not in terminal.text(), "reconnect replayed a historical completion")
            terminal.write("\x11")
            smoke.wait(lambda: not terminal.process.isalive(), "reconnected UI did not exit")
            require(not smoke.fixture.requests, "presentation invoked conversational inference")
            report = {"native_windows_conpty": True, "historical_snapshot_quiet": True,
                      "compact_live_card": True, "keyboard_open_result": True,
                      "work_markdown": True, "work_copy_menu": True, "explicit_diagnostics": True,
                      "chat_draft_preserved": True, "reconnect_no_replay": True,
                      "task_title_time_registered_directory": True, "two_turn_history_after_restart": True,
                      "clipboard_access": False, "paid_calls": 0, "model_requests": 0,
                      "worker_backend": "structured_fixture_only"}
        except Exception:
            if terminal and artifacts:
                terminal.image(artifacts / "failure.png")
                (artifacts / "failure.txt").write_text(terminal.text(), encoding="utf-8")
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
                    host.wait(timeout=10)
            if log:
                log.close()
            if smoke:
                smoke.stop()
                if smoke.fixture:
                    smoke.fixture.close()
    report["temporary_state_cleaned"] = True
    if artifacts:
        (artifacts / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report), flush=True)


if __name__ == "__main__":
    main()
