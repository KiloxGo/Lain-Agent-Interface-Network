"""Windows ConPTY acceptance with the deterministic worker, no model calls.

Build all three binaries, install pywinpty in an isolated environment, then run.
The normal cargo tests need no Python package. Only this optional host check does.
"""
import os
from pathlib import Path
import shutil
import tempfile
import threading
import time
from codex_smoke import Daemon


def wait(predicate, seconds=20):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.05)
    raise TimeoutError("terminal acceptance condition not observed")


def run():
    assert os.name == "nt", "this optional smoke uses Windows ConPTY"
    from winpty import PtyProcess
    # Run the ignored guard test inside a real terminal, including unwind/error.
    guard = PtyProcess.spawn([shutil.which("rtk"), "proxy", "cargo", "test", "--locked", "-p", "lain-client", "--bin", "lain", "terminal_guard_restores_on_error_and_panic", "--", "--ignored", "--nocapture"], dimensions=(35, 120))
    guard_output = []
    try:
        def drain_guard():
            try:
                while True:
                    guard_output.append(guard.read(65536))
            except (EOFError, OSError):
                pass
        guard_reader = threading.Thread(target=drain_guard, daemon=True)
        guard_reader.start()
        wait(lambda: not guard.isalive(), seconds=90)
        guard_reader.join(timeout=3)
        assert "1 passed" in "".join(guard_output), "native terminal error/panic guard check failed"
        print("PASS: native terminal raw mode restored after error and caught panic", flush=True)
    finally:
        guard.close(force=True)
    binaries = Path(os.environ.get("LAIN_TEST_BIN_DIR", "target/debug")).resolve()
    os.environ["LAIN_CODEX_PATH"] = str(binaries / "lain-test-app-server.exe")
    with tempfile.TemporaryDirectory(prefix="lain-terminal-smoke-") as directory:
        root = Path(directory)
        private = root / "laind.exe"
        shutil.copy2(binaries / "laind.exe", private)
        daemon = Daemon(root, private, idle_timeout=1)
        proc = None
        output = []
        try:
            daemon.start()
            session = daemon.call("POST", "/v1/work-sessions", {"workspace": str(root)})
            sid = session["id"]
            base = f"/v1/work-sessions/{sid}"
            wait(lambda: daemon.call("GET", base)["runtime"] == "detached")
            env = dict(os.environ, LAIN_URL=f"http://127.0.0.1:{daemon.port}", LAIN_API_TOKEN=daemon.token)
            proc = PtyProcess.spawn([str(binaries / "lain.exe"), "work", "attach", sid], env=env, dimensions=(35, 120))
            def drain():
                try:
                    while True:
                        output.append(proc.read(65536))
                except (EOFError, OSError):
                    pass
            reader = threading.Thread(target=drain, daemon=True)
            reader.start()
            wait(lambda: "Timeline" in "".join(output) and "detached" in "".join(output))
            assert daemon.call("GET", base)["runtime"] == "detached"
            proc.setwinsize(24, 80)
            proc.setwinsize(35, 120)
            proc.write("terminal-followup\r")
            wait(lambda: any(t["state"] == "completed" for t in daemon.call("GET", base + "/turns")))
            wait(lambda: daemon.call("GET", base)["runtime"] == "detached")
            history = daemon.call("GET", base + "/history")["events"]
            assert any(e.get("text") == "terminal-followup" for e in history)
            daemon.call("POST", base + "/turns", {"text": "approve"})
            wait(lambda: any(a["status"] == "pending" for a in daemon.call("GET", base + "/approvals")))
            wait(lambda: "APPROVAL" in "".join(output))
            proc.write("\x04")  # Ctrl-D: deny the offered approval.
            wait(lambda: daemon.call("GET", base + "/approvals")[0]["status"] == "resolved")
            assert daemon.call("GET", base + "/approvals")[0]["decision"] == "deny"
            proc.write("\x11")  # Ctrl-Q: exit, never cancel the worker implicitly.
            wait(lambda: not proc.isalive())
            reader.join(timeout=3)
            assert daemon.token not in "".join(output)
            assert "?1049l" in "".join(output), "alternate screen was not restored"
            print("PASS: ConPTY attach without worker launch, resize, detached follow-up, approval deny, clean terminal exit", flush=True)
        finally:
            if proc is not None:
                proc.close(force=True)
            daemon.stop()


if __name__ == "__main__":
    run()
