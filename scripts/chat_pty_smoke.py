"""Actual Unix terminal flow against the native fixture Kernel/Responses server.

Called by native_smoke.py on native macOS. This does not stand in for ConPTY.
"""
import fcntl
import os
import select
import signal
import subprocess
import termios
import time


class Chat:
    def __init__(self, binary, env, conversation):
        self.master, slave = os.openpty()
        self.before = termios.tcgetattr(self.master)
        def own_terminal():
            os.setsid()
            fcntl.ioctl(0, termios.TIOCSCTTY, 0)
        self.process = subprocess.Popen(
            [str(binary), "chat", "--timing", "--conversation", conversation],
            env=env, stdin=slave, stdout=slave, stderr=slave, preexec_fn=own_terminal)
        os.close(slave)
        self.output = bytearray()

    def drain(self, seconds=0.05):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            if select.select([self.master], [], [], min(0.02, max(0, deadline-time.monotonic())))[0]:
                try:
                    chunk = os.read(self.master, 65536)
                except OSError:
                    break
                if not chunk:
                    break
                self.output.extend(chunk)

    def until(self, text, after=0):
        deadline = time.monotonic() + 15
        while text.encode() not in self.output[after:]:
            assert time.monotonic() < deadline, f"PTY timeout waiting for {text!r}; exit={self.process.poll()}"
            self.drain()

    def send(self, data):
        mark = len(self.output)
        os.write(self.master, data)
        return mark

    def command(self, command, ending=b"\n"):
        prompts = self.output.count(b"You> ")
        mark = self.send(command.encode() + ending)
        self.until("You> ", mark)
        self.drain()
        assert self.output.count(b"You> ") == prompts + 1, "extra interactive prompt"
        assert b"You> You>" not in self.output
        return bytes(self.output[mark:])

    def finish(self, data=b"/exit\n"):
        self.send(data)
        assert self.process.wait(timeout=10) == 0
        self.drain()
        assert termios.tcgetattr(self.master) == self.before, "terminal modes not restored"

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait(timeout=5)
        os.close(self.master)


def run_chat_pty(binary, env, call, gate):
    def conversation():
        return call("POST", "/v1/conversations")["id"]
    def latest(cid):
        return call("GET", f"/v1/conversations/{cid}?recent=true")["turns"][-1]
    def completed(cid):
        deadline = time.monotonic() + 15
        while True:
            turn = latest(cid)
            if turn["state"] != "running":
                return turn
            assert time.monotonic() < deadline, "Kernel generation did not terminate"
            time.sleep(0.02)

    cid = conversation()
    chat = Chat(binary, env, cid)
    try:
        chat.until("You> ")
        # Empty input stays in the existing read. CRLF must not enqueue another
        # prompt, unlike the old canonical background read_line implementation.
        prompts = chat.output.count(b"You> ")
        chat.send(b"\n\r\n")
        chat.drain(0.1)
        assert chat.output.count(b"You> ") == prompts
        chat.command("/help", b"\r\n")
        chat.command("/status")
        chat.command("/history", b"\r\n")
        for ending in (b"\n", b"\r\n"):
            result = chat.command("hello", ending)
            assert result.count("你来了。".encode()) == 1, "final response displayed twice"
        result = chat.command("no-preview")
        assert result.count("完成而无预览。".encode()) == 1

        gate.clear()
        prompts = chat.output.count(b"You> ")
        mark = chat.send(b"stream-delayed\n")
        chat.until("STREAM-BEGIN 你好", mark)
        assert latest(cid)["state"] == "running", "text was buffered until completion"
        assert chat.output.count(b"You> ") == prompts, "input offered while still generating"
        # Independent conversation output while this client is busy does not
        # take over its prompt or cross conversation identities.
        other = subprocess.run([str(binary), "--json", "ask", "no-preview", "--timing"], env=env, capture_output=True, timeout=15)
        import json
        assert other.returncode == 0 and json.loads(other.stdout)["state"] == "completed"
        assert b"Timing>" not in other.stdout and b"cli_submission_ms" in other.stderr
        gate.set()
        chat.until("You> ", mark)
        result = bytes(chat.output[mark:])
        assert result.count(b"STREAM-BEGIN") == 1 and result.count(b"STREAM-END") == 1
        assert "第二行：".encode() in result and b"Timing>" in result
        assert latest(cid)["state"] == "completed"

        result = chat.command("partial-failure")
        assert b"PARTIAL-ONLY" in result and b"completed message not confirmed" in result
        assert latest(cid)["state"] == "unknown"
        assert not any(m["role"] == "assistant" for m in latest(cid)["messages"])
        result = chat.command("stream-secret")
        assert b"fixture-actor-credential" not in result
        assert latest(cid)["state"] == "unknown"
        assert not any(m["role"] == "assistant" for m in latest(cid)["messages"])

        # Enter is a real denial at [y/N], while empty input at You> is ignored.
        mark = chat.send(b"delegate\r\n")
        chat.until("Confirm this exact action?", mark)
        assert call("GET", "/v1/work-sessions") == []
        assert latest(cid)["state"] == "pending_confirmation", "CRLF must not answer confirmation"
        mark = chat.send(b"\n")
        chat.until("You> ", mark)
        assert latest(cid)["actions"][0]["state"] == "rejected"
        assert call("GET", "/v1/work-sessions") == []
        chat.finish()
    finally:
        gate.set()
        chat.close()

    # Reopen the persisted conversation without regenerating a response.
    prior = call("GET", f"/v1/conversations/{cid}?recent=true")
    chat = Chat(binary, env, cid)
    try:
        chat.until("You> ")
        assert call("GET", f"/v1/conversations/{cid}?recent=true") == prior
        assert chat.output.count(b"STREAM-BEGIN") == 1
        chat.finish(b"\x04")  # Native raw-mode EOF gesture.
    finally:
        chat.close()

    for gesture in (b"\x03", b"\x04"):
        pending_cid = conversation()
        pending_chat = Chat(binary, env, pending_cid)
        try:
            pending_chat.until("You> ")
            mark = pending_chat.send(b"delegate\n")
            pending_chat.until("Confirm this exact action?", mark)
            pending_chat.finish(gesture)
            assert latest(pending_cid)["state"] == "pending_confirmation"
            assert call("GET", "/v1/work-sessions") == []
            # Explicit fixture cleanup through the owner decision boundary.
            turn = latest(pending_cid)
            call("POST", f"/v1/actor-turns/{turn['id']}/actions/{turn['actions'][0]['id']}/decision", {"approve":False})
        finally:
            pending_chat.close()
    chat = Chat(binary, env, cid)
    try:
        chat.until("You> ")
        chat.finish(b"\x03")  # Actual Ctrl-C key while waiting for input.
    finally:
        chat.close()

    # Actual Ctrl-C in cooked mode interrupts inference; it cannot leave the
    # input reader alive or claim that the partial preview was committed.
    cid = conversation()
    chat = Chat(binary, env, cid)
    try:
        chat.until("You> ")
        gate.clear()
        mark = chat.send(b"stream-delayed\n")
        chat.until("STREAM-BEGIN 你好", mark)
        chat.send(b"\x03")
        chat.until("You> ", mark)
        assert completed(cid)["state"] == "unknown"
        chat.finish()
    finally:
        gate.set()
        chat.close()

    # Losing the client during generation leaves Kernel-owned inference alive.
    cid = conversation()
    chat = Chat(binary, env, cid)
    resumed = None
    try:
        chat.until("You> ")
        gate.clear()
        mark = chat.send(b"stream-delayed\n")
        chat.until("STREAM-BEGIN 你好", mark)
        tid = latest(cid)["id"]
        resumed = Chat(binary, env, cid)
        resumed.until("STREAM-BEGIN 你好")
        assert latest(cid)["id"] == tid and latest(cid)["state"] == "running"
        chat.process.kill()
        chat.process.wait(timeout=5)
        assert latest(cid)["state"] == "running"
        gate.set()
        turn = completed(cid)
        assert turn["state"] == "completed" and turn["id"] == tid
        resumed.until("You> ")
        assert resumed.output.count(b"STREAM-BEGIN") == 1 and resumed.output.count(b"STREAM-END") == 1
        resumed.finish()
    finally:
        gate.set()
        chat.close()
        if resumed:
            resumed.close()
    print("PASS native macOS PTY: LF/CRLF prompt ownership, live Unicode preview, no duplicate final, JSON/timing, denial, interruption, EOF, resume and detached inference")
