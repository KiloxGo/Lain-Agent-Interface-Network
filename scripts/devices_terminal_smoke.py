#!/usr/bin/env python3
"""Native Windows Devices navigation, exact destination and zero-effect smoke.

Uses isolated synthetic registrations and an actual ConPTY. No host, model, QQ
or clipboard interaction. Optional PNGs render the actual terminal VT stream.
"""
import argparse
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
    args = parser.parse_args()
    require(os.name == "nt", "native Windows ConPTY required")
    artifacts = args.artifacts.resolve() if args.artifacts else None
    if artifacts:
        require(not artifacts.is_relative_to(Path(__file__).resolve().parents[1]), "artifacts stay outside checkout")
        artifacts.mkdir(parents=True, exist_ok=True)
    smoke = terminal = None
    with tempfile.TemporaryDirectory(prefix="lain-qq-memory-devices-") as directory:
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

            native("init", "--directory", device, "--endpoint", f"http://127.0.0.1:{smoke.port}", "--label", "Desk-PC", "--workspace", smoke.workspace, "--expires-at-ms", int(time.time()*1000)+600000, "--codex-work")
            registration = json.loads((device / "registration.json").read_text(encoding="utf-8"))
            smoke.cli("device", "pair", "--file", device / "registration.json")
            native("claim", "--directory", device)
            did = registration["id"]
            terminal = Terminal([str(smoke.bins / executable("lain")), "desk", "--conversation", conversation], smoke.env)
            terminal.resize(42, 135)

            def visible(text):
                smoke.wait(lambda: text in terminal.text(), "expected UI text absent: " + text)

            def capture(name):
                if artifacts:
                    terminal.image(artifacts / (name + ".png"))

            visible("connected to Kernel")
            terminal.write("keep-chat-draft")
            terminal.write("\t")
            visible("[ Work ]")
            terminal.write("\t")
            visible("Desk-PC")
            visible("执行服务：待确认")
            require("Last report (ms)" not in terminal.text(), "raw inventory still used as default")
            capture("01-computers")
            terminal.write("\r")
            visible("选中目录")
            visible(str(smoke.workspace))
            visible("登记有效")
            require(did not in terminal.text(), "internal device ID in default details")
            capture("02-computer-detail")
            terminal.write("c")
            visible("云端读取成功")
            visible("执行服务：待确认")
            terminal.write("i")
            visible("Start-Codex-Host.cmd")
            capture("03-startup")
            terminal.write("i")
            visible("选中目录")
            terminal.write("\x1b[18~")  # F7.
            visible(did)
            visible("登记版本")
            terminal.write("\x1b[18~")
            visible("选中目录")
            terminal.resize(30, 60)
            visible("滚轮/PgUp/PgDn")
            capture("04-narrow-detail")
            before = terminal.text()
            terminal.write("\x1b[<65;10;10M")
            smoke.wait(lambda: terminal.text() != before, "detail wheel did not scroll")
            terminal.write("\x1b[1;5H")  # Ctrl-Home returns to the beginning.
            terminal.resize(42, 135)
            visible(str(smoke.workspace))
            terminal.write("n")
            visible("1/2 执行位置")
            visible("Desk-PC")
            terminal.write("\r")
            visible("2/2 任务内容")
            visible(str(smoke.workspace))
            terminal.write("unsent-work-draft")
            capture("05-targeted-work")
            terminal.write("\t")
            visible("to Lain")
            visible("keep-chat-draft")
            terminal.write("\x11")
            terminal.process.wait()
            require(smoke.call("GET", f"/v1/devices/{did}/commands") == [], "navigation queued a task")
            require(not smoke.fixture.requests, "navigation called the model")
            report = {"passed": ["friendly_computers", "directory_and_authority", "read_only_cloud_check", "startup_and_diagnostics", "narrow_terminal", "exact_work_destination", "chat_draft_retained", "zero_tasks_or_model_calls"], "native_windows": True}
        finally:
            if terminal:
                terminal.close()
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
