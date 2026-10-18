"""Native Phase 6 CLI/daemon/model-adapter acceptance; fake providers, zero quota.

Requires built workspace binaries. --platform must match the real host; this script
never emulates another OS. Windows runs native .exe/PowerShell ACLs and Ctrl-Break.
macOS runs Mach-O binaries, POSIX permissions and a real process-group signal.
"""
import argparse
import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import signal
import socket
import sqlite3
import subprocess
import sys
import tempfile
import threading
import time
import uuid


def run(platform):
    actual = {"darwin": "macos", "win32": "windows"}.get(sys.platform, "linux")
    if actual != platform:
        raise SystemExit(f"Refusing emulation: host is {actual}, requested {platform}")
    repo = Path(__file__).resolve().parents[1]
    bins = Path(os.environ.get("LAIN_TEST_BIN_DIR", repo / "target/debug"))
    suffix = ".exe" if os.name == "nt" else ""
    lain, laind, codex = (bins / (name + suffix) for name in ("lain", "laind", "lain-test-app-server"))
    for binary in (lain, laind, codex):
        assert binary.is_file(), f"Build workspace first: missing {binary.name}"
    with tempfile.TemporaryDirectory(prefix="lain-native-") as tmp:
        root = Path(tmp)
        workspace = root / "project"
        workspace.mkdir()
        local = root / "local"
        env = {k: v for k, v in os.environ.items() if not k.startswith("LAIN_")}
        env.update(LAIN_LOCAL_DIR=str(local), LAIN_DAEMON_PATH=str(laind), LAIN_CODEX_PATH=str(codex))
        provider_calls = []
        stream_gate = threading.Event()
        class Model(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass
            def do_POST(self):
                assert self.path == "/v1/responses"
                assert self.headers["Authorization"] == "Bearer fixture-actor-credential"
                body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                provider_calls.append(body)
                assert body["store"] is False and body["stream"] is True
                assert "fixture-actor-credential" not in json.dumps(body)
                assert token not in json.dumps(body)
                last = body["input"][-1]
                prompt = last.get("content", "")
                if last.get("type") == "function_call_output":
                    output = [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "已读取 Kernel 的真实工作状态。"}]}]
                elif "work.create" in prompt:
                    output = [{"type": "function_call", "name": "work_create", "call_id": "call_create", "arguments": json.dumps({"workspace": str(workspace), "profile_id": "codex/local", "sandbox": "read_only", "task": "hello"})}]
                elif "work.inspect" in prompt or "work.history" in prompt:
                    import re
                    sid = re.search(r"[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}", prompt).group()
                    history = "work.history" in prompt
                    arguments = {"session_id":sid, **({"after":0} if history else {})}
                    output = [{"type":"function_call", "name":"work_history" if history else "work_inspect", "call_id":"call_read", "arguments":json.dumps(arguments)}]
                elif "work.list" in prompt or "profile.list" in prompt or "profile.inspect" in prompt:
                    name = "profile_inspect" if "profile.inspect" in prompt else "profile_list" if "profile.list" in prompt else "work_list"
                    output = [{"type":"function_call", "name":name, "call_id":"call_read", "arguments":json.dumps({"profile_id":"codex/local"} if name == "profile_inspect" else {})}]
                elif "三百字" in prompt:
                    output = [{"type":"message", "role":"assistant", "content":[{"type":"output_text", "text":"你好，我是 Lain。" + "这是可见的流式对话。" * 30}]}]
                elif "delegate" in last.get("content", ""):
                    output = [{"type": "function_call", "name": "work_create", "call_id": "call_delegate", "arguments": json.dumps({"workspace": str(workspace), "profile_id": "codex/local", "sandbox": "read_only", "task": "notice-gate" if prompt == "delegate-delivery" else "hello"})}]
                elif "inspect" in last.get("content", ""):
                    output = [{"type": "function_call", "name": "work_list", "call_id": "call_list", "arguments": "{}"}]
                elif prompt == "stream-delayed":
                    output = [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "STREAM-BEGIN 你好\n第二行：" + "流式文字。" * 40 + "STREAM-END"}]}]
                elif prompt == "no-preview":
                    output = [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "完成而无预览。"}]}]
                elif prompt == "partial-failure":
                    output = []
                elif prompt == "stream-secret":
                    output = [{"type":"message", "role":"assistant", "content":[{"type":"output_text", "text":"safe prefix fixture-actor-credential tail"}]}]
                else:
                    output = [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "你来了。"}]}]
                payload = json.dumps({"type": "response.completed", "response": {"status": "completed", "output": output, "usage": {"total_tokens": 20}}}, ensure_ascii=False).encode()
                def frame(event):
                    return b"data: " + json.dumps(event, ensure_ascii=False).encode() + b"\n\n"
                text = "".join(p["text"] for item in output if item["type"] == "message" for p in item["content"])
                if prompt == "partial-failure":
                    text = "PARTIAL-ONLY"
                    payload = json.dumps({"type":"response.failed"}).encode()
                deltas = [frame({"type":"response.output_text.delta", "delta": c}) for c in text] if prompt != "no-preview" else []
                if output and output[0]["type"] == "function_call":
                    deltas = [frame({"type":"response.function_call_arguments.delta", "delta":"PRIVATE-TOOL-ARGUMENT-FRAGMENT"})]
                pieces = deltas + [b"data: " + payload + b"\n\n"]
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream")
                self.send_header("Content-Length", str(sum(map(len, pieces))))
                self.end_headers()
                try:
                    for index, wire in enumerate(pieces):
                        # Deliberately split UTF-8 across HTTP writes.
                        for offset in range(0, len(wire), 11):
                            self.wfile.write(wire[offset:offset + 11])
                            self.wfile.flush()
                        if prompt == "stream-delayed" and index == len("STREAM-BEGIN 你好") - 1:
                            assert stream_gate.wait(15), "PTY test did not release provider gate"
                        if "三百字" in prompt and index == 20:
                            time.sleep(1)  # Deterministic preview-before-completion probe.
                except (BrokenPipeError, ConnectionResetError):
                    pass  # Explicit cancellation can close the private provider connection.
        model = ThreadingHTTPServer(("127.0.0.1", 0), Model)
        thread = threading.Thread(target=model.serve_forever, daemon=True)
        thread.start()
        token = ""
        def cli(*args, success=True, extra=None):
            result = subprocess.run([str(lain), *args], env=dict(env, **(extra or {})), capture_output=True, text=True, encoding="utf-8", timeout=30)
            assert result.returncode == 0 if success else result.returncode != 0, result.stderr
            assert not token or token not in result.stdout + result.stderr
            assert "fixture-actor-credential" not in result.stdout + result.stderr
            return json.loads(result.stdout) if result.stdout.strip() else result.stderr
        setup = cli("--json", "setup", "--local")
        assert setup["status"] == "created"
        token = (local / "kernel-token").read_text()
        assert cli("--json", "setup", "--local")["status"] == "reused"
        assert (local / "kernel-token").read_text() == token
        if platform != "windows":
            assert (local / "kernel-token").stat().st_mode & 0o777 == 0o600
            assert local.stat().st_mode & 0o777 == 0o700
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            port = sock.getsockname()[1]
        settings = json.loads((local / "client.json").read_text())
        settings["url"] = f"http://127.0.0.1:{port}"
        (local / "client.json").write_text(json.dumps(settings))
        cfg = root / "actor.json"
        cfg.write_text(json.dumps({"driver": "openai-responses", "model": "fixture", "endpoint": f"http://127.0.0.1:{model.server_port}/v1", "secret_ref": "secret://env/actor", "timeout_seconds": 10, "max_output_tokens": 1024, "max_turn_tokens": 4096}))
        original_config = cfg.read_bytes()
        saved = cli("--json", "config", "actor", "set", "--file", str(cfg))
        assert saved["source"] == "saved_setting" and saved["credential_status"] == "missing"
        assert cfg.read_bytes() == original_config
        def write_secrets(path, text):
            descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            with os.fdopen(descriptor, "w", encoding="utf-8", newline="") as output:
                output.write(text)
            if os.name == "nt":
                acl_script = "$p=$env:LAIN_TEST_ACL_PATH; $a=Get-Acl -LiteralPath $p; $a.SetAccessRuleProtection($true,$true); Set-Acl -LiteralPath $p -AclObject $a"
                result = subprocess.run(["powershell.exe", "-NoProfile", "-NonInteractive", "-Command", acl_script], env=dict(env, LAIN_TEST_ACL_PATH=str(path)), capture_output=True, timeout=15)
                assert result.returncode == 0, "fixture native ACL setup failed"
        secrets_file = local / "actor secrets.env"
        write_secrets(secrets_file, "LAIN_SECRET_ACTOR=fixture-actor-credential\r\n")
        saved = cli("--json", "config", "actor", "env-file", "--file", str(secrets_file))
        assert saved["credential_status"] == "available"
        assert "fixture-actor-credential" not in (local / "client.json").read_text()
        assert cli("--json", "config", "actor")["env_file"] == str(secrets_file.resolve())
        invalid_env = local / "unsupported.env"
        write_secrets(invalid_env, "PATH=do-not-print-sensitive-value\n")
        previous_settings = (local / "client.json").read_bytes()
        rejected = cli("--json", "config", "actor", "env-file", "--file", str(invalid_env), success=False)
        assert "do-not-print-sensitive-value" not in json.dumps(rejected)
        assert (local / "client.json").read_bytes() == previous_settings
        log = root / "daemon.log"
        process = None
        def call(method, path, body=None):
            conn = http.client.HTTPConnection("127.0.0.1", port, timeout=15)
            try:
                conn.request(method, path, None if body is None else json.dumps(body), {"Authorization": "Bearer " + token, "Content-Type": "application/json"})
                res = conn.getresponse()
                data = res.read()
                assert res.status < 300, (res.status, path)
                assert token.encode() not in data and b"fixture-actor-credential" not in data
                return json.loads(data)
            finally:
                conn.close()
        def start(args=(), extra=None):
            nonlocal process
            process = subprocess.Popen([str(lain), "up", *args], env=dict(env, **(extra or {})), stdin=subprocess.DEVNULL, stdout=log.open("ab"), stderr=subprocess.STDOUT,
                                       creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0,
                                       start_new_session=os.name != "nt")
            deadline = time.monotonic() + 20
            while True:
                assert process.poll() is None, "launcher exited before readiness"
                try:
                    if call("GET", "/ready")["state"] == "ready":
                        break
                except (OSError, AssertionError):
                    assert time.monotonic() < deadline, "daemon readiness timeout"
                    time.sleep(0.05)
        def stop():
            if process and process.poll() is None:
                if os.name == "nt":
                    process.send_signal(signal.CTRL_BREAK_EVENT)
                else:
                    os.killpg(process.pid, signal.SIGINT)
                assert process.wait(timeout=20) == 0, "native graceful launcher/daemon shutdown failed"
        try:
            cli("--json", "doctor", success=False)
            start()
            assert cli("--json", "up")["status"] == "already_running"
            assert cli("--json", "status")["kernel"]["state"] == "ready"
            before = len(provider_calls)
            report = cli("--json", "doctor")
            assert report["diagnostics"]["storage"] == "healthy"
            assert report["diagnostics"]["actor"]["status"] == "configured"
            assert report["actor_configuration"] == "matches"
            assert len(provider_calls) == before
            changed = json.loads(cfg.read_text())
            changed["model"] = "fixture-next"
            cfg.write_text(json.dumps(changed))
            mismatch = cli("--json", "doctor", success=False)
            assert mismatch["actor_configuration"] == "restart_required"
            cli("--json", "up", success=False)
            assert process.poll() is None, "configuration change interrupted the existing daemon"
            stop()
            start()
            assert cli("--json", "doctor")["actor_configuration"] == "matches"
            assert call("GET", "/v1/diagnostics")["actor"]["model"] == "fixture-next"
            if platform == "macos":
                from chat_pty_smoke import run_chat_pty
                run_chat_pty(lain, env, call, stream_gate)
            shadow = root / "shadow"
            shadow.mkdir()
            shim = shadow / ("lain.cmd" if os.name == "nt" else "lain")
            shim.write_text("obsolete command fixture")
            conflict = cli("--json", "doctor", extra={"PATH": str(shadow) + os.pathsep + env.get("PATH", "")})
            assert str(shim) in conflict["path_candidates"]
            cli("--json", "up", success=False, extra={"LAIN_API_TOKEN": "different-owner-token-0123456789abcdef"})
            cli("--json", "doctor", success=False, extra={"LAIN_API_TOKEN": "different-owner-token-0123456789abcdef"})
            hello = cli("--json", "ask", "hello", "--timing")
            assert hello["state"] == "completed"
            conversation = hello["conversation_id"]
            inspected = cli("--json", "ask", "inspect", "--conversation", conversation)
            assert inspected["actions"][0]["state"] == "executed"
            pending = cli("--json", "ask", "delegate", "--conversation", conversation)
            assert pending["state"] == "pending_confirmation"
            assert cli("--json", "work", "list") == []
            # Persist the proposal through a native daemon restart before owner confirmation.
            stop()
            start()
            pending = call("GET", f"/v1/actor-turns/{pending['id']}")
            assert pending["state"] == "pending_confirmation"
            action = pending["actions"][0]
            if platform == "macos":
                import select
                master, slave = os.openpty()
                chat = subprocess.Popen([str(lain), "chat", "--conversation", conversation], env=env, stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
                os.close(slave)
                captured = bytearray()
                def until(text):
                    deadline = time.monotonic() + 15
                    while text.encode() not in captured:
                        assert time.monotonic() < deadline, "native chat prompt timeout"
                        if select.select([master], [], [], 0.1)[0]:
                            captured.extend(os.read(master, 65536))
                    assert token.encode() not in captured and b"fixture-actor-credential" not in captured
                try:
                    until("Confirm this exact action?")
                    os.write(master, b"y\n")
                    until("Task submitted; worker outcome will arrive separately.")
                    os.write(master, b"/exit\n")
                    assert chat.wait(timeout=10) == 0
                finally:
                    if chat.poll() is None:
                        chat.kill()
                        chat.wait(timeout=5)
                    os.close(master)
                # Ctrl-C while waiting on ordinary stdin must exit without hanging
                # Tokio shutdown behind an uninterruptible blocking read.
                master, slave = os.openpty()
                chat = subprocess.Popen([str(lain), "chat", "--conversation", conversation], env=env, stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
                os.close(slave)
                captured = bytearray()
                try:
                    until("You>")
                    chat.send_signal(signal.SIGINT)
                    assert chat.wait(timeout=10) == 0
                finally:
                    if chat.poll() is None:
                        chat.kill()
                        chat.wait(timeout=5)
                    os.close(master)
            else:
                cli("--json", "actor", "approve", pending["id"], action["id"])
            deadline = time.monotonic() + 15
            while True:
                completed = call("GET", f"/v1/actor-turns/{pending['id']}")
                if completed["state"] != "running":
                    break
                assert time.monotonic() < deadline
                time.sleep(0.02)
            assert completed["state"] == "completed", completed["error"]
            sid = completed["actions"][0]["result"]["outcome"]["work_session_id"]
            assert cli("--json", "work", "show", sid)["id"] == sid
            assert any(t["id"] == completed["actions"][0]["result"]["outcome"]["turn_id"] for t in cli("--json", "work", "turns", sid))
            import queue
            watch = subprocess.Popen([str(lain), "--json", "work", "watch", sid], env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
            first_line = queue.Queue(1)
            threading.Thread(target=lambda: first_line.put(watch.stdout.readline()), daemon=True).start()
            try:
                assert json.loads(first_line.get(timeout=10))["id"] == sid, "work watch must emit one complete JSON record per line"
            finally:
                watch.terminate()
                watch.wait(timeout=10)
                watch.stdout.close()
            cli("--json", "work", "history", sid)
            cli("--json", "profile", "list")
            resumed = cli("--json", "ask", "inspect", "--conversation", conversation)
            assert resumed["state"] == "completed"
            assert any(c["id"] == conversation for c in cli("--json", "conversations"))
            # Idempotency uses the same conversation, request UUID and text.
            retry = cli("--json", "ask", "inspect", "--conversation", conversation, "--request-id", resumed["request_id"])
            assert retry["id"] == resumed["id"]
            # Exercise the opt-in runner itself against fake local providers;
            # this imports helpers, never invokes --run-real or spends quota.
            from actor_smoke import Acceptance, CheckFailed
            from unittest.mock import patch
            acceptance = Acceptance(lain, root / "acceptance.json", workspace, env, fixture=True)
            def fixture_decision(prompt):
                if prompt.startswith("Denial test:"):
                    return "DENY"
                if prompt.startswith("Separate real delegation:"):
                    return prompt.split("type ", 1)[1].split(" to launch", 1)[0]
                if prompt.startswith("Type RESTARTED"):
                    return "RESTARTED"
                raise AssertionError("unexpected acceptance owner decision")
            with patch("builtins.input", fixture_decision):
                acceptance.run(15)
            assert acceptance.report["checks"]["incremental_text_while_running"] is True
            assert acceptance.report["checks"]["worker_completed_successfully"] is True
            last_id = list(acceptance.report["actor_turns"])[-1]
            supplemental = {"turn_id":last_id, "conversation_id":acceptance.report["conversation_id"],
                            "digest":acceptance.report["actor_turns"][last_id]["digest"]}
            acceptance.report["supplemental_checks"] = {"work.history":dict(supplemental, digest="invalid")}
            try:
                acceptance.continuity()
            except CheckFailed:
                pass
            else:
                raise AssertionError("contradictory supplemental evidence was accepted")
            acceptance.report["supplemental_checks"] = {"work.history":supplemental}
            acceptance.continuity()
            acceptance.save()
            assert token not in acceptance.evidence.read_text() and "fixture-actor-credential" not in acceptance.evidence.read_text()
            if platform == "macos":
                from chat_pty_smoke import run_delivery_pty
                delivery_cid = run_delivery_pty(lain, env, call, workspace, provider_calls)
                delivery_snapshot = cli("--json", "notices", delivery_cid)
                assert delivery_snapshot == call("GET", f"/v1/conversations/{delivery_cid}/notices")
                assert cli("--json", "delegations", delivery_cid)[0]["instruction"] == "notice-gate"
            stop()
            # Explicit env-file selection is per launch; process environment has
            # higher precedence and neither rewrites the saved owner selection.
            before_calls = len(provider_calls)
            selected_settings = (local / "client.json").read_bytes()
            override_cfg = local / "override actor.json"
            override = dict(changed, model="fixture-override")
            override_cfg.write_text(json.dumps(override))
            persona_file = local / "owner persona.md"
            persona_file.write_text("Fixture owner persona, version one.\n")
            override_env = local / "override secrets.env"
            write_secrets(override_env, "LAIN_ACTOR_CONFIG=override actor.json\nLAIN_PERSONA_PATH=owner persona.md\nLAIN_SECRET_ACTOR=fixture-actor-credential\n")
            start(("--env-file", str(override_env)), {"LAIN_ACTOR_CONFIG": str(cfg)})
            assert call("GET", "/v1/diagnostics")["actor"]["model"] == "fixture-next"
            assert cli("--json", "doctor", "--env-file", str(override_env), extra={"LAIN_ACTOR_CONFIG": str(cfg)})["actor_configuration"] == "matches"
            persona_file.write_text("Fixture owner persona, version two.\n")
            assert cli("--json", "doctor", "--env-file", str(override_env), extra={"LAIN_ACTOR_CONFIG": str(cfg)}, success=False)["actor_configuration"] == "restart_required"
            assert process.poll() is None
            stop()
            start(("--env-file", str(override_env)))
            assert call("GET", "/v1/diagnostics")["actor"]["model"] == "fixture-override"
            assert cli("--json", "doctor", "--env-file", str(override_env))["actor_configuration"] == "matches"
            assert cli("--json", "doctor", success=False)["actor_configuration"] == "restart_required"
            assert call("GET", f"/v1/actor-turns/{resumed['id']}") == resumed
            assert cli("--json", "work", "show", sid)["id"] == sid
            stop()
            start()
            assert cli("--json", "doctor")["actor_configuration"] == "matches"
            with patch("builtins.input", fixture_decision):
                acceptance.recovery()
            if platform == "macos":
                assert cli("--json", "notices", delivery_cid) == delivery_snapshot, "notice identity/content/acknowledgement changed after native restart"
            assert acceptance.report["checks"]["daemon_restart_continuity"] is True
            assert len(provider_calls) == before_calls, "configuration/recovery caused inference"
            assert (local / "client.json").read_bytes() == selected_settings
            stop()
            with sqlite3.connect(local / "lain.sqlite3") as db:
                persisted = "\n".join(db.iterdump())
                assert token not in persisted and "fixture-actor-credential" not in persisted
                assert db.execute("PRAGMA user_version").fetchone()[0] == 6
            assert token.encode() not in log.read_bytes() and b"fixture-actor-credential" not in log.read_bytes()
            # A listening unrelated service is never terminated or replaced.
            unrelated = ThreadingHTTPServer(("127.0.0.1", port), type("Unrelated", (BaseHTTPRequestHandler,), {"log_message": lambda *args: None, "do_GET": lambda self: (self.send_response(401), self.end_headers())}))
            threading.Thread(target=unrelated.serve_forever, daemon=True).start()
            cli("--json", "up", success=False)
            assert unrelated.fileno() >= 0
            unrelated.shutdown()
            unrelated.server_close()
        finally:
            stop()
            model.shutdown()
            model.server_close()
    print(f"PASS native {platform}: local setup/reuse/permissions, launcher/shutdown/restart, doctor, authenticated CLI, streamed Actor adapter, confirmation, real fixture work and persistence; zero quota")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", choices=("macos", "windows", "linux"), required=True)
    run(parser.parse_args().platform)
