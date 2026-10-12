"""Opt-in, quota-consuming smoke test against the real daemon and Codex login.

Build laind, then run: python scripts/codex_smoke.py --run-real
Uses private temporary workspaces and a fresh API token; never prints raw traffic.
"""
import argparse
import concurrent.futures
from contextlib import closing
import http.client
import json
import os
from pathlib import Path
import queue
import secrets
import shutil
import signal
import socket
import sqlite3
import subprocess
import tempfile
import threading
import time


class Stream:
    def __init__(self, port, token):
        self.connection = http.client.HTTPConnection("127.0.0.1", port, timeout=240)
        self.connection.request("GET", "/v1/events", headers={"Authorization": "Bearer " + token})
        self.response = self.connection.getresponse()
        assert self.response.status == 200
        self.events = queue.Queue()
        self.thread = threading.Thread(target=self.read, daemon=True)
        self.thread.start()

    def read(self):
        kind, data = "", ""
        try:
            for raw in self.response:
                line = raw.decode().rstrip("\r\n")
                if line.startswith("event:"):
                    kind = line[6:].strip()
                elif line.startswith("data:"):
                    data = line[5:].strip()
                elif not line and data:
                    self.events.put((kind, json.loads(data)))
                    kind, data = "", ""
        except (OSError, ValueError, http.client.HTTPException):
            pass

    def wait(self, predicate, seconds=180):
        deadline = time.monotonic() + seconds
        while True:
            kind, value = self.events.get(timeout=max(0.01, deadline - time.monotonic()))
            assert kind != "resync_required", "event stream lagged"
            if predicate(kind, value):
                return value
            if time.monotonic() >= deadline:
                raise TimeoutError("expected event not observed")

    def close(self):
        if self.connection.sock:
            try:
                self.connection.sock.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass
        # Shutdown wakes the reader. Let it finish before closing the response
        # underneath http.client's chunk parser (which races on Python 3.14).
        self.thread.join(timeout=2)
        self.connection.close()


class Daemon:
    def __init__(self, root, binary, idle_timeout=300, extra_env=None):
        self.root, self.binary = root, binary
        self.token = secrets.token_hex(32)
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            self.port = sock.getsockname()[1]
        self.log = []
        self.process = None
        self.idle_timeout = idle_timeout
        self.extra_env = dict(extra_env or {})

    def start(self):
        env = dict(os.environ, LAIN_API_TOKEN=self.token,
                   LAIN_DATABASE_PATH=str(self.root / "state.sqlite3"),
                   LAIN_LISTEN=f"127.0.0.1:{self.port}",
                   LAIN_WORKER_IDLE_TIMEOUT=str(self.idle_timeout))
        env.update(self.extra_env)
        self.process = subprocess.Popen([str(self.binary)], env=env, stdout=subprocess.DEVNULL,
                                        stderr=subprocess.PIPE,
                                        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0)
        ready = threading.Event()
        def drain():
            for line in self.process.stderr:
                self.log.append(line)
                if b'"message":"Kernel ready"' in line:
                    ready.set()
        threading.Thread(target=drain, daemon=True).start()
        assert ready.wait(120), "daemon did not become ready"
        assert self.call("GET", "/ready")["state"] == "ready"
        assert self.call("GET", "/v1/backends")[0]["available"]

    def call(self, method, path, body=None):
        connection = http.client.HTTPConnection("127.0.0.1", self.port, timeout=90)
        try:
            connection.request(method, path, None if body is None else json.dumps(body),
                               {"Authorization": "Bearer " + self.token, "Content-Type": "application/json"})
            response = connection.getresponse()
            value = json.loads(response.read())
            assert response.status < 300, f"API {method} {path}: HTTP {response.status}, {value.get('error')}"
            return value
        finally:
            connection.close()

    def stream(self):
        return Stream(self.port, self.token)

    def stop(self):
        if self.process and self.process.poll() is None:
            self.process.send_signal(signal.CTRL_BREAK_EVENT if os.name == "nt" else signal.SIGTERM)
            try:
                assert self.process.wait(timeout=45) == 0, "daemon shutdown failed"
            finally:
                if self.process.poll() is None:
                    self.process.kill()
                    self.process.wait(timeout=10)
        assert self.token.encode() not in b"".join(self.log), "daemon token leaked to logs"

    def diagnostics(self):
        for line in self.log:
            try:
                record = json.loads(line)
                if record.get("level") in ("WARN", "ERROR"):
                    print("daemon:", record.get("fields", {}).get("error", "worker warning"), flush=True)
            except ValueError:
                pass

    def identities(self):
        with closing(sqlite3.connect(self.root / "state.sqlite3")) as db:
            return {sid: json.loads(value)["provider_session_id"]
                    for sid, value in db.execute("SELECT session_id, snapshot FROM worker_state")}

    def crash_worker(self):
        # Select only a direct child of this smoke-test daemon, never another Codex session.
        if os.name == "nt":
            command = (f"$worker = Get-CimInstance Win32_Process | Where-Object {{ $_.ParentProcessId -eq {self.process.pid} -and $_.Name -eq 'codex.exe' }} | Select-Object -First 1; "
                       "if ($null -eq $worker) { exit 1 }; Stop-Process -Id $worker.ProcessId -Force -ErrorAction Stop")
            subprocess.run(["powershell", "-NoProfile", "-Command", command], check=True, capture_output=True)
        elif Path("/proc").is_dir():
            for entry in Path("/proc").iterdir():
                try:
                    fields = (entry / "stat").read_text().rsplit(")", 1)[1].split()
                    if int(fields[1]) == self.process.pid:
                        os.kill(int(entry.name), signal.SIGKILL)
                        return
                except (OSError, ValueError, IndexError):
                    continue
            raise RuntimeError("owned worker process not found")
        else:
            raise RuntimeError("crash smoke supports Windows or Linux")


def run(binary):
    with tempfile.TemporaryDirectory(prefix="lain-real-smoke-") as directory:
        root = Path(directory)
        # Windows cannot relink an executable while it is running.
        private_binary = root / binary.name
        shutil.copy2(binary, private_binary)
        daemon = Daemon(root, private_binary)
        streams = []
        try:
            daemon.start()
            stream = daemon.stream()
            streams.append(stream)
            stream.wait(lambda kind, _: kind == "snapshot")
            sessions = []
            for name in ("alpha", "beta"):
                workspace = root / name
                workspace.mkdir()
                sessions.append(daemon.call("POST", "/v1/work-sessions", {"workspace": str(workspace)})["id"])
            paths = [f"/v1/work-sessions/{sid}" for sid in sessions]
            def prompt(path, text):
                return daemon.call("POST", path + "/turns", {"text": text})
            def completed(turn):
                event = stream.wait(lambda kind, e: kind == "event" and e.get("kind") == "turn_completed" and e["turn"]["id"] == turn["id"])
                assert event["turn"]["state"] == "completed", "real turn did not complete successfully"
            for path, marker in zip(paths, ("LainAlpha", "LainBeta")):
                completed(prompt(path, f"Reply with exactly {marker}. Do not call tools."))
            identities = daemon.identities()
            assert len(set(identities.values())) == 2
            completed(prompt(paths[0], "Repeat your previous reply exactly. Do not call tools."))
            print("PASS: two real threads, structured completion, same-session follow-up", flush=True)

            # These two turns stay active long enough to observe scoped cancellation.
            instruction = "Run a shell command that waits for 120 seconds, then prints done. Use Start-Sleep on Windows or sleep on Unix. Do not shorten the wait."
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
                turns = list(executor.map(lambda path: prompt(path, instruction), paths))
            observed = set()
            def both_active(kind, event):
                if kind == "event" and event.get("kind") in ("output_progress", "command_started", "approval_requested"):
                    if event.get("turn_id") in {turn["id"] for turn in turns} or event.get("approval", {}).get("turn_id") in {turn["id"] for turn in turns}:
                        observed.add(event["work_session_id"])
                return observed == set(sessions)
            stream.wait(both_active)
            for path in paths:
                assert daemon.call("GET", path)["state"] in ("running", "waiting_for_approval")
            daemon.call("POST", paths[0] + "/cancel")
            cancelled = stream.wait(lambda kind, e: kind == "event" and e.get("kind") == "turn_completed" and e["turn"]["id"] == turns[0]["id"])
            assert cancelled["turn"]["state"] == "cancelled"
            assert daemon.call("GET", paths[1])["state"] in ("running", "waiting_for_approval")
            stream.close()
            assert daemon.call("GET", paths[1])["state"] in ("running", "waiting_for_approval")
            stream = daemon.stream()
            streams.append(stream)
            snapshot = stream.wait(lambda kind, _: kind == "snapshot")
            assert len(snapshot["sessions"]) == 2
            daemon.call("POST", paths[1] + "/cancel")
            cancelled = stream.wait(lambda kind, e: kind == "event" and e.get("kind") == "turn_completed" and e["turn"]["id"] == turns[1]["id"])
            assert cancelled["turn"]["state"] == "cancelled"
            print("PASS: scoped real cancellation, client detach/reconnect snapshot", flush=True)

            turn = prompt(paths[0], "Request explicit owner approval to run one harmless shell command outside the sandbox: echo LainApproval. Set sandbox_permissions=require_escalated with a justification. Do not run other commands and do not avoid the approval request.")
            event = stream.wait(lambda kind, e: kind == "event" and e.get("kind") == "approval_requested")
            approval = event["approval"]
            assert approval["work_session_id"] == sessions[0]
            daemon.call("POST", paths[0] + f"/approvals/{approval['id']}/decision", {"decision": "approve_once"})
            completed(turn)
            print("PASS: real owner approval round trip", flush=True)

            daemon.crash_worker()
            event = stream.wait(lambda kind, e: kind == "event" and e.get("kind") == "backend_disconnected")
            crashed = f"/v1/work-sessions/{event['work_session_id']}"
            lost = daemon.call("GET", crashed)
            assert lost["state"] == "idle" and lost["runtime"] == "lost"
            daemon.call("POST", crashed + "/resume")
            assert daemon.identities() == identities
            print("PASS: owned Codex process crash and explicit resume", flush=True)

            stream.close()
            daemon.stop()
            daemon.start()
            assert daemon.identities() == identities
            assert {s["id"] for s in daemon.call("GET", "/v1/work-sessions")} == set(sessions)
            stream = daemon.stream()
            streams.append(stream)
            stream.wait(lambda kind, _: kind == "snapshot")
            for path in paths:
                assert daemon.call("GET", path)["state"] == "idle"
            completed(prompt(paths[0], "Reply exactly LainResumed. Do not call tools."))
            print("PASS: daemon restart, same provider identities, resumed real turn", flush=True)
        except BaseException:
            daemon.diagnostics()
            raise
        finally:
            for stream in streams:
                stream.close()
            daemon.stop()


def run_phase3(binary, client):
    """Two short real turns; empty detach, CLI lazy resume, history and restart."""
    with tempfile.TemporaryDirectory(prefix="lain-presence-smoke-") as directory:
        root = Path(directory)
        private_binary = root / binary.name
        shutil.copy2(binary, private_binary)
        daemon = Daemon(root, private_binary, idle_timeout=2)
        streams = []
        def cli(*args):
            output = subprocess.run([str(client), "--json", *args],
                                    env=dict(os.environ, LAIN_URL=f"http://127.0.0.1:{daemon.port}", LAIN_API_TOKEN=daemon.token),
                                    capture_output=True, timeout=90, check=True)
            assert daemon.token.encode() not in output.stdout + output.stderr
            return json.loads(output.stdout)
        def detached(path):
            deadline = time.monotonic() + 30
            while time.monotonic() < deadline:
                value = daemon.call("GET", path)
                if value["runtime"] == "detached":
                    assert value["state"] == "idle"
                    return
                time.sleep(0.1)
            raise TimeoutError("idle worker did not detach")
        try:
            daemon.start()
            workspace = root / "workspace"
            workspace.mkdir()
            session = cli("work", "start", "--workspace", str(workspace))
            sid = session["id"]
            path = f"/v1/work-sessions/{sid}"
            identity = daemon.identities()
            detached(path)
            assert cli("work", "show", sid)["runtime"] == "detached"
            assert cli("work", "list")[0]["runtime"] == "detached"
            print("PASS: unused real thread survives idle detach; CLI inspection leaves it detached", flush=True)
            for marker in ("LainPresence", "LainStillHere"):
                stream = daemon.stream()
                streams.append(stream)
                stream.wait(lambda kind, _: kind == "snapshot")
                turn = cli("work", "send", sid, f"Reply exactly {marker}. Do not call tools.")
                result = stream.wait(lambda kind, e: kind == "event" and e.get("kind") == "turn_completed" and e["turn"]["id"] == turn["id"])
                assert result["turn"]["state"] == "completed"
                detached(path)
                assert daemon.identities() == identity
                history = cli("work", "history", sid)["events"]
                assert any(e["kind"] == "assistant_message_completed" and marker in e["text"] for e in history)
                assert all(e["kind"] != "output_progress" and e["history_order"] for e in history)
                assert len({e["id"] for e in history}) == len(history)
                stream.close()
                daemon.stop()
                daemon.start()
                assert daemon.identities() == identity
                assert cli("work", "show", sid)["runtime"] == "detached"
                recovered = cli("work", "history", sid)["events"]
                assert recovered == history
                print("PASS: CLI lazy real follow-up, durable completed output, idle reboot without attachment", flush=True)
        except BaseException:
            daemon.diagnostics()
            raise
        finally:
            for stream in streams:
                stream.close()
            daemon.stop()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-real", action="store_true", required=True)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/laind.exe" if os.name == "nt" else "target/debug/laind"))
    parser.add_argument("--phase3", action="store_true", help="focused idle/history/CLI acceptance with two short real turns")
    parser.add_argument("--client", type=Path, default=Path("target/debug/lain.exe" if os.name == "nt" else "target/debug/lain"))
    args = parser.parse_args()
    if args.phase3:
        run_phase3(args.binary.resolve(), args.client.resolve())
    else:
        run(args.binary.resolve())
