"""Native Windows Work N -> exact direct task -> Chat; isolated ConPTY/Kernel.

No personal account, QQ traffic, Worker start or model call. Optional pictures are
renders of the actual synthetic ConPTY VT stream, not desktop screenshots.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

from desk_terminal_smoke import Terminal
from qq_memory_smoke import Smoke, executable, require


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    parser.add_argument("--artifacts", type=Path)
    parser.add_argument("--test-client", type=Path, required=True,
                        help="compiled client unit-test executable for its existing private Windows clipboard fixture")
    args = parser.parse_args()
    require(os.name == "nt", "native Windows ConPTY required")
    artifacts = args.artifacts.resolve() if args.artifacts else None
    if artifacts:
        require(not artifacts.is_relative_to(Path(__file__).resolve().parents[1]), "artifacts stay outside checkout")
        artifacts.mkdir(parents=True, exist_ok=True)
    smoke = terminal = None
    with tempfile.TemporaryDirectory(prefix="lain-qq-memory-work-entry-") as temp, tempfile.TemporaryDirectory(prefix="lain-ui-terminal-") as clipboard_root:
        try:
            root = Path(temp)
            smoke = Smoke(root, args.bin_dir.resolve(), None, 600)
            smoke.setup(None)
            smoke.env.pop("LAIN_NAPCAT_CONFIG", None)
            smoke.env.pop("LAIN_PORTRAIT_FILE", None)
            smoke.start()
            conversation = smoke.call("POST", "/v1/conversations", expected=201)["id"]
            device = root / "pc"
            env = {k: v for k, v in smoke.env.items() if not k.upper().startswith(("LAIN_", "OPENAI_", "DEEPSEEK_", "ANTHROPIC_"))}
            env["LAIN_LOCAL_DIR"] = str(root / "device-only-no-owner")
            def native(*items):
                result = subprocess.run([str(smoke.bins / executable("lain")), "--json", "device", *map(str, items)], env=env, capture_output=True, text=True, encoding="utf-8", timeout=30)
                require(result.returncode == 0, "isolated native device operation failed")
                return json.loads(result.stdout)
            native("init", "--directory", device, "--endpoint", f"http://127.0.0.1:{smoke.port}", "--label", "fixture-pc", "--workspace", smoke.workspace, "--expires-at-ms", int(time.time()*1000)+600000, "--codex-work")
            smoke.cli("device", "pair", "--file", device / "registration.json")
            native("claim", "--directory", device)
            task = "read this project\nkeep exact original text"
            terminal_env=smoke.env.copy()
            terminal_env.update(LAIN_TERMINAL_COPY_FIXTURE="isolated-fixture",
                                LAIN_TERMINAL_COPY_RECEIPT=str(Path(clipboard_root)/"clipboard-receipt.jsonl"),
                                LAIN_TERMINAL_COPY_CONVERSATION=conversation,
                                LAIN_TERMINAL_PASTE_FIXTURE=task)
            terminal = Terminal([str(args.test_client.resolve()), "--exact", "desk::clipboard::tests::native_terminal_private_clipboard", "--ignored", "--nocapture"], terminal_env)
            def visible(text):
                smoke.wait(lambda: text in terminal.text(), "terminal did not show " + text)
            def capture(name):
                if artifacts:
                    terminal.image(artifacts / (name + ".png"))
            visible("connected to Kernel")
            terminal.write("keep-chat-draft\t")
            visible("N 新建任务")
            terminal.write("N")
            visible("fixture-pc")
            capture("choose-project")
            terminal.write("\r")
            visible("任务内容")
            # Crossterm's native Windows key-record path can lose LF from raw
            # ConPTY injection. Explicit F9 reads the actual private clipboard,
            # without changing or reading the user's interactive clipboard.
            terminal.write("\x1b[20~")
            visible("keep exact original text")
            terminal.write("\r")
            visible("Unchanged")
            endpoint = f"/v1/conversations/{conversation}/device-work"
            require(not smoke.call("GET", endpoint)["work"], "opening/review queued work")
            for _ in range(12):
                if "Ctrl-Y 确认创建" in terminal.text():
                    break
                terminal.write("\x1b[6~")
                time.sleep(.1)
            visible("Ctrl-Y 确认创建")
            capture("review-exact-task")
            terminal.write("\x19")
            smoke.wait(lambda: len(smoke.call("GET", endpoint)["work"]) == 1, "explicit creation did not queue one task")
            command = smoke.call("GET", endpoint)["work"][0]["command"]
            if artifacts and command["input"]["task"] != task:
                (artifacts / "paste-diagnostic.json").write_text(json.dumps({"expected":task,"received":command["input"]["task"]},ensure_ascii=True,indent=2),encoding="utf-8")
            require(command["input"]["task"] == task, "multiline paste/original text changed")
            require(command["input"]["sandbox"] == "read_only" and command["state"] == "queued", "default scope/state changed")
            visible("排队中")
            visible("[ Work ]")
            capture("queued-awaiting-device")
            terminal.write("\x1b")
            visible("keep-chat-draft")
            terminal.write("\x11")
            smoke.wait(lambda: not terminal.process.isalive(), "frontend did not close")
            require(len(smoke.call("GET", endpoint)["work"]) == 1, "frontend exit changed work")
            require(not smoke.fixture.requests, "direct work used the conversational model")
            require(not smoke.call("GET", f"/v1/conversations/{conversation}?recent=true")["turns"], "direct work fabricated an Actor turn")
            report = {"native_platform": "Windows ConPTY", "exact_direct_task": True, "multiline_paste": True, "private_clipboard_station": True, "interactive_clipboard_touched": False, "chat_draft_preserved": True, "task_survives_client_exit": True, "queued_tasks": 1, "model_calls": 0, "worker_starts": 0, "paid_calls": 0}
        except Exception:
            if terminal and artifacts:
                terminal.image(artifacts / "failure.png")
                (artifacts / "failure.txt").write_text(terminal.text(), encoding="utf-8")
            raise
        finally:
            if terminal:
                terminal.close()
            if smoke:
                smoke.stop()
                if smoke.fixture:
                    smoke.fixture.close()
    report["temporary_state_cleaned"] = True
    report["binary_sha256"] = {name: hashlib.sha256((args.bin_dir / executable(name)).read_bytes()).hexdigest() for name in ["lain", "laind"]}
    report["binary_sha256"]["test_client"]=hashlib.sha256(args.test_client.read_bytes()).hexdigest()
    if artifacts:
        (artifacts / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report), flush=True)


if __name__ == "__main__":
    main()
