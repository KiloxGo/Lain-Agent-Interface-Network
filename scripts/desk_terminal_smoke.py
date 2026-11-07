"""Native Windows ConPTY acceptance of the Lain desk; localhost fixtures only.

Build the workspace first. Requires pywinpty/pyte; Pillow only with --artifacts.
Images are renders of the actual ConPTY VT stream, not desktop screenshots.
No personal configuration, QQ account or paid provider is used.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile
import threading
import time
import uuid

from qq_memory_smoke import Smoke, executable, require

def screen_text(screen):
    """pyte leaves an empty wide-cell stub when ASCII replaces a CJK lead cell.

    Preserve its physical blank column rather than crashing or modifying the
    captured stream. Same wide-character skipping as Screen.display otherwise.
    """
    from wcwidth import wcwidth
    rows=[]
    for y in range(screen.lines):
        row=[]
        x=0
        while x<screen.columns:
            char=screen.buffer[y][x].data or " "
            row.append(char)
            x+=2 if wcwidth(char[0])==2 else 1
        rows.append("".join(row))
    return "\n".join(rows)


class Terminal:
    def __init__(self, args, env):
        import pyte
        from winpty import PtyProcess
        self.screen = pyte.Screen(120, 40)
        self.stream = pyte.Stream(self.screen)
        self.lock = threading.Lock()
        self.output = []
        self.process = PtyProcess.spawn(args, env=env, dimensions=(40, 120))
        self.reader = threading.Thread(target=self.drain, daemon=True)
        self.reader.start()

    def drain(self):
        try:
            while True:
                chunk = self.process.read(65536)
                with self.lock:
                    self.output.append(chunk)
                    self.stream.feed(chunk)
        except (EOFError, OSError):
            pass

    def text(self):
        with self.lock:
            return screen_text(self.screen)

    def write(self, text):
        self.process.write(text)

    def resize(self, rows, columns):
        with self.lock:
            self.screen.resize(rows, columns)
        self.process.setwinsize(rows, columns)

    def image(self, path):
        from PIL import Image, ImageDraw, ImageFont
        from wcwidth import wcwidth
        mono = ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 19)
        cjk = ImageFont.truetype("C:/Windows/Fonts/msyh.ttc", 17)
        braille = ImageFont.truetype("C:/Windows/Fonts/seguisym.ttf", 24)
        colors = {"default": "#ccd1da", "black": "#10151c", "cyan": "#79b9bd", "yellow": "#dfbc72",
                  "white": "#ccd1da", "brightblack": "#89919e"}
        def color(value, default):
            if len(value) == 6 and all(c in "0123456789abcdef" for c in value.lower()):
                return "#" + value
            return colors.get(value, default)
        with self.lock:
            image = Image.new("RGB", (self.screen.columns * 12 + 32, self.screen.lines * 25 + 52), "#10151c")
            draw = ImageDraw.Draw(image)
            draw.text((16, 7), "LAIN / Windows ConPTY / synthetic fixtures", font=mono, fill="#89919e")
            for y in range(self.screen.lines):
                for x in range(self.screen.columns):
                    cell = self.screen.buffer[y][x]
                    if not cell.data:
                        continue
                    left, top = x * 12 + 16, y * 25 + 35
                    if cell.bg != "default":
                        draw.rectangle((left, top, left+max(1, wcwidth(cell.data[0]))*12, top+25), fill=color(cell.bg, "#10151c"))
                    dots = all(0x2800 <= ord(c) <= 0x28ff for c in cell.data)
                    draw.text((left - (3 if dots else 0), top), cell.data,
                              font=braille if dots else cjk if any(ord(c)>127 for c in cell.data) else mono,
                              fill=color(cell.fg,"#ccd1da"))
            image.save(path)

    def close(self):
        self.process.close(force=True)
        self.reader.join(timeout=3)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    parser.add_argument("--artifacts", type=Path)
    parser.add_argument("--portrait-file", type=Path,
                        help="Explicit local character art; never copied into the test report")
    args = parser.parse_args()
    require(os.name == "nt", "this acceptance requires native Windows ConPTY")
    if args.artifacts:
        artifacts = args.artifacts.resolve()
        require(not artifacts.is_relative_to(Path(__file__).resolve().parents[1]), "artifacts must stay outside checkout")
        artifacts.mkdir(parents=True, exist_ok=True)
    smoke, terminal = None, None
    released, seen = threading.Event(), threading.Event()
    report = {}
    # Avoid a synthetic path containing the credential-shaped substring
    # "sk-terminal-..."; the existing output secret filter correctly rejects it.
    with tempfile.TemporaryDirectory(prefix="lain-ui-terminal-") as directory:
        try:
            smoke = Smoke(Path(directory), args.bin_dir.resolve(), None, 300)
            smoke.setup(None)
            # This check exercises only the HTTP Kernel and desktop client.
            smoke.env.pop("LAIN_NAPCAT_CONFIG", None)
            # A caller's presentation setting must not affect the fixture implicitly.
            smoke.env.pop("LAIN_PORTRAIT_FILE", None)
            if args.portrait_file:
                smoke.env["LAIN_PORTRAIT_FILE"] = str(args.portrait_file.resolve())
            def respond(body):
                prompt = next(i["content"] for i in reversed(body["input"]) if i.get("role") == "user")
                if prompt == "history-fixture":
                    # Markdown hard breaks keep this a 90-row scroll fixture;
                    # ordinary soft breaks now render as one paragraph.
                    return [{"type":"message","role":"assistant","content":[{"type":"output_text","text":"  \n".join(f"history-row-{n:03}" for n in range(90))}]}]
                if prompt == "desk-first":
                    seen.set()
                    require(released.wait(25), "test did not release first model request")
                if prompt == "desk-proposal":
                    return [{"type":"function_call","name":"work_create","call_id":str(uuid.uuid4()),"arguments":json.dumps({"workspace":str(smoke.workspace),"profile_id":"codex/local","sandbox":"read_only","task":"exact desk scope; do not launch without approval"})}]
                return [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "fixture reply: " + prompt}]}]
            smoke.fixture.respond = respond
            smoke.start()
            conversation = smoke.call("POST", "/v1/conversations", expected=201)["id"]
            terminal = Terminal([str(smoke.bins / executable("lain")), "desk", "--conversation", conversation], smoke.env)
            def wait(check, why):
                return smoke.wait(check, why)
            def visible(text):
                wait(lambda: text in terminal.text(), "terminal did not show " + text)
            def capture(name):
                if args.artifacts:
                    time.sleep(0.15)  # Allow ConPTY's asynchronous render flush.
                    terminal.image(artifacts / (name + ".png"))
            def passed(name):
                smoke.passed(name)
            visible("lain // wired")
            if args.portrait_file:
                art = args.portrait_file.read_text(encoding="utf-8").splitlines()
                require(len(art) == 18 and all(len(row) == 29 for row in art), "expected selected 29 x 18 portrait")
                visible(art[0])
                visible(art[-1])
                capture("portrait-wide-welcome")
                terminal.resize(24, 80)
                visible(art[0])
                visible(art[-1])
                capture("portrait-compact-welcome")
                terminal.resize(40, 120)
                visible(art[-1])
                passed("selected_portrait_preserved_in_wide_and_compact_welcome")
            require(not smoke.fixture.requests and not smoke.call("GET", "/v1/work-sessions"), "opening UI caused work/inference")
            terminal.write("multiline-first\x0amultiline-second")  # Ctrl-J / LF.
            visible("multiline-first")
            visible("multiline-second")
            require(not smoke.fixture.requests, "Ctrl-J submitted an idle draft")
            terminal.write("\x15")
            terminal.write("desk-first\r")
            wait(seen.is_set, "first request not submitted")
            terminal.write("next draft")
            visible("next draft")
            terminal.write("\x0astill-editing")
            visible("still-editing")
            require(len(smoke.fixture.requests) == 1, "Ctrl-J submitted a running draft")
            terminal.write("\x15next draft")
            visible("next draft")
            passed("ctrl_j_edits_multiline_draft_idle_and_running_without_submit")
            capture("chat-responding")
            terminal.write("\t")
            visible("Workers")
            terminal.write("\t")
            visible("Devices")
            terminal.write("\x1b")
            visible("next draft")
            require(len(smoke.fixture.requests) == 1, "navigation submitted a draft")
            passed("draft_editing_and_navigation_while_actor_runs")
            terminal.resize(10, 30)
            visible("Resize to")
            terminal.resize(24, 80)
            visible("next draft")
            if args.portrait_file:
                require(art[0] not in terminal.text(), "compact chat must yield portrait space to dialogue")
            capture("chat-compact")
            terminal.resize(40, 120)
            visible("next draft")
            released.set()
            visible("fixture reply: desk-first")
            visible("next draft")
            passed("native_resize_and_completed_reply_preserve_draft")

            # Rebuild a completed conversation after a real isolated Kernel restart.
            smoke.stop()
            smoke.start()
            terminal.write("\x12")  # Ctrl-R: read only.
            visible("next draft")
            terminal.write("\x15desk-second\r")  # Ctrl-U clears only the draft.
            visible("fixture reply: desk-second")
            require(len(smoke.fixture.requests) == 2, "reconnect replayed a model request")
            passed("kernel_restart_read_recovery_without_request_replay")

            worker = smoke.call("POST", "/v1/work-sessions", {"workspace": str(smoke.workspace)}, expected=201)
            base = "/v1/work-sessions/" + worker["id"]
            wait(lambda: smoke.call("GET", base)["state"] in {"idle", "ready"}, "worker not ready")
            smoke.call("POST", base + "/turns", {"text": "approve"}, expected=202)
            wait(lambda: any(a["status"] == "pending" for a in smoke.call("GET", base + "/approvals")), "worker did not request approval")
            before = smoke.call("GET", base + "/turns")
            terminal.write("retained draft\t")
            visible("codex")
            capture("work-dashboard")
            terminal.write("\r")
            visible("Timeline")
            require(smoke.call("GET", base + "/approvals")[0]["status"] == "pending", "inspection resolved an approval")
            terminal.write("\x11")  # Exit child Worker view, return to Lain.
            visible("lain // wired")
            visible("retained draft")
            require("Timeline" not in terminal.text(), "old Worker surface leaked into returned chat")
            passed("worker_inspection_returns_to_lain_without_permission_or_draft_change")
            terminal.write("\x15desk-third\r")
            visible("fixture reply: desk-third")
            require(smoke.call("GET", base + "/turns") == before, "parallel Lain conversation changed Worker state")
            require(len(smoke.fixture.requests) == 3, "unexpected extra model request")
            capture("chat-completed")
            terminal.write("desk-proposal\r")
            visible("action awaits review")
            terminal.write("/review\r")
            visible("exact desk scope")
            for _ in range(60):
                if "Ctrl-D deny" in terminal.text():
                    break
                terminal.write("\x1b[6~")
                time.sleep(0.04)
            visible("Ctrl-D deny")
            capture("action-review")
            terminal.write("\x04")  # Explicit denial of this exact Lain action.
            # Rebuilt checked history presents the durable action receipt; a
            # transient acknowledgement sentence need not survive a resnapshot.
            visible("declined; no operation executed")
            rejected = smoke.call("GET", f"/v1/conversations/{conversation}?recent=true")
            actions = [action for turn in rejected["turns"] for action in turn["actions"]]
            require(len(actions) == 1 and actions[0]["state"] == "rejected", "exact action rejection was not persisted")
            require(len(smoke.call("GET","/v1/work-sessions"))==1,"denied Lain proposal launched another Worker")
            require(len(smoke.fixture.requests)==4,"denial or review made another model call")
            passed("exact_lain_action_review_and_explicit_deny_without_worker_launch")

            device=Path(directory)/"device"
            smoke.cli("device","init","--directory",device,"--endpoint",f"http://127.0.0.1:{smoke.port}","--label","desk-device","--workspace",smoke.workspace,"--expires-at-ms",int(time.time()*1000)+600000)
            registration=json.loads((device/"registration.json").read_text(encoding="utf-8"))
            smoke.cli("device","pair","--file",device/"registration.json")
            smoke.cli("device","claim","--directory",device)
            did=registration["id"]
            queued=smoke.call("POST",f"/v1/devices/{did}/commands",{"request_id":str(uuid.uuid4()),"expected_device_revision":2,"workspace_id":registration["workspaces"][0]["id"],"profile_id":"codex/local","sandbox":"read_only","task":"synthetic remote task stays queued","expires_at_ms":int(time.time()*1000)+180000})
            terminal.write("device draft\t\t")
            visible("desk-device")
            terminal.write("\r")
            visible("Queued")
            capture("device-tasks")
            terminal.write("\r")
            visible("synthetic remote task stays queued")
            capture("device-task-detail")
            terminal.write("\x1b")
            visible("device draft")
            require(smoke.call("GET",f"/v1/devices/{did}/commands/{queued['id']}")==queued,"device observation mutated remote task")
            passed("enrolled_device_task_list_and_detail_are_read_only")
            terminal.write("\x15history-fixture\r")
            visible("history-row-089")
            require("history-row-010" not in terminal.text(), "fixture must overflow the viewport")
            before_wheel=len(smoke.fixture.requests)
            terminal.write("draft survives wheel")
            for _ in range(50):
                terminal.write("\x1b[<64;85;12M")
                time.sleep(0.06)
                if "history-row-010" in terminal.text():
                    break
            visible("history-row-010")
            require("history-row-089" not in terminal.text(), "wheel did not leave the latest messages")
            capture("chat-history-scrolled")
            terminal.write("\x1b[1;5F")  # Ctrl-End returns to follow mode.
            visible("history-row-089")
            visible("draft survives wheel")
            require(len(smoke.fixture.requests)==before_wheel,"history scroll called a model")
            passed("wheel_reads_history_and_ctrl_end_restores_follow_without_model_calls")
            if args.portrait_file:
                terminal.resize(55,150)
                visible("ID  ")  # Expanded sidebar proves the resize actually rendered.
                visible("本机")
                visible("Windows")
                visible("RAM")
                visible("进行中的任务")
                capture("sidebar-system-path-and-work")
                passed("wide_sidebar_shows_native_local_stats_and_kernel_work")
            terminal.write("\x11")
            wait(lambda: not terminal.process.isalive(), "terminal failed to exit")
            terminal.reader.join(timeout=3)
            require(smoke.call("GET", base + "/turns") == before, "exiting UI cancelled Worker")
            require(not any(c in "".join(terminal.output) for c in smoke.credentials), "credential in terminal stream")
            require("?1049l" in "".join(terminal.output), "alternate screen not restored")
            passed("parallel_chat_and_clean_exit_leave_worker_approval_pending")

            # The user-facing preview deliberately has no Actor. It must remain
            # a usable dashboard and preserve input instead of trapping an
            # unsubmitted message behind an uncertain-delivery prompt.
            terminal.close()
            smoke.stop()
            actor_config = smoke.env.pop("LAIN_ACTOR_CONFIG")
            smoke.start()
            conversation = smoke.call("POST", "/v1/conversations", expected=201)["id"]
            terminal = Terminal([str(smoke.bins / executable("lain")), "desk", "--conversation", conversation], smoke.env)
            visible("Chat model not configured")
            model_count = len(smoke.fixture.requests)
            turn_path = f"/v1/conversations/{conversation}?recent=true"
            empty = smoke.call("GET", turn_path)
            # Avoid the credential-shaped substring in a long "desk-..." marker.
            draft = "configuration_ready"
            terminal.write(draft + "\r\r")
            visible(draft)
            visible("Chat model not configured")
            require(smoke.call("GET", turn_path) == empty, "missing model created a turn")
            require(len(smoke.fixture.requests) == model_count, "readiness check called a model")
            capture("chat-not-configured-draft-retained")
            passed("known_missing_chat_model_preserves_input_without_submission_or_model_calls")

            smoke.stop()
            smoke.env["LAIN_ACTOR_CONFIG"] = actor_config
            smoke.start()
            terminal.write("\x12")
            visible("quiet · connected to Kernel")
            visible("Ready. /help for commands")
            require(smoke.call("GET", "/v1/diagnostics")["actor"]["status"] == "configured", "Kernel chat configuration did not recover")
            visible(draft)
            require(len(smoke.fixture.requests) == model_count, "configuration refresh automatically sent the draft")
            terminal.write("\r")
            visible("fixture reply: " + draft)
            require(len(smoke.fixture.requests) == model_count + 1, "explicit send was lost or repeated")
            capture("chat-ready-after-explicit-refresh")
            terminal.write("\x11")
            wait(lambda: not terminal.process.isalive(), "readiness client did not exit")
            passed("explicit_refresh_reads_new_configuration_without_automatic_draft_retry")
            report = {"checks": smoke.checks, "fake_model_calls": len(smoke.fixture.requests), "paid_calls": 0,
                      "platform": "native Windows ConPTY", "images": "rendered actual ConPTY VT stream; synthetic content"}
        except Exception:
            if terminal and args.artifacts:
                terminal.image(artifacts / "failure.png")
                (artifacts / "failure.txt").write_text(terminal.text(), encoding="utf-8")
                history=smoke.call("GET",f"/v1/conversations/{conversation}?recent=true")
                (artifacts / "failure-turns.json").write_text(json.dumps(history,ensure_ascii=False,indent=2),encoding="utf-8")
            raise
        finally:
            released.set()
            if terminal:
                terminal.close()
            if smoke:
                smoke.stop()
                if smoke.fixture:
                    smoke.fixture.close()
    report["temporary_state_cleaned"] = True
    report["binary_sha256"] = {name: hashlib.sha256((args.bin_dir / executable(name)).read_bytes()).hexdigest() for name in ["lain", "laind", "lain-test-app-server"]}
    if args.artifacts:
        (artifacts / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=2), flush=True)


if __name__ == "__main__":
    main()
