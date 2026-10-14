"""Opt-in real Actor/Codex acceptance; consumes quota only with --run-real.

Requires an already configured daemon and explicit owner authorization for its
provider and workspace. Exact mutations require a separate interactive decision.
Never restarts a daemon, approves worker permissions, or replays uncertain work.
--check-recovery performs only reads after the owner's separate graceful restart.
Evidence stores IDs, hashes, measured durations and outcomes, never transcripts.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time
import uuid


class CheckFailed(Exception):
    pass


def require(condition, message):
    if not condition:
        raise CheckFailed(message)


def digest(value):
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True,
                                     separators=(",", ":")).encode()).hexdigest()


def terminal_safe(value):
    # JSON escapes terminal controls, Unicode bidi controls and non-ASCII alike.
    return json.dumps(value, ensure_ascii=True, indent=2)


class Acceptance:
    def __init__(self, binary, evidence, workspace=None, env=None, fixture=False, request_budget=36):
        self.binary = str(binary)
        self.evidence = evidence
        self.workspace = workspace
        self.env = env
        self.evidence_loaded = False
        self.request_budget = request_budget
        self.report = {"format": 1, "evidence_kind": "deterministic_fixture" if fixture else "real_provider", "checks": {}, "actor_turns": {},
                       "model_requests": 0, "request_budget": request_budget, "worker": None,
                       "daemon_restart": "unverified"}

    def save(self):
        # Only a file this invocation created may be replaced; never stdout raw
        # provider/CLI errors. CLI owns redaction; this summary stores no prose.
        self.report["total_model_requests"] = self.spent_requests()
        with self.evidence.open("w", encoding="utf-8") as out:
            json.dump(self.report, out, ensure_ascii=True, indent=2)
            out.write("\n")

    def command(self, *words, timeout=180, expect_failure=False):
        result = subprocess.run([self.binary, "--json", *map(str, words)],
                                capture_output=True, text=True, encoding="utf-8",
                                timeout=timeout, env=self.env)
        if expect_failure:
            require(result.returncode != 0 and "Kernel returned HTTP 409" in result.stderr,
                    "Duplicate decision was not demonstrably rejected with HTTP 409")
            return
        if result.returncode:
            # A failed decision can still have committed a partial receipt.
            raise CheckFailed("CLI failed; inspect the saved IDs with lain actor show/work show; no retry performed")
        try:
            return json.loads(result.stdout)
        except ValueError:
            raise CheckFailed("CLI returned invalid JSON") from None

    def check(self, name, status=True):
        self.report["checks"][name] = status
        self.save()
        print(("fixture " if self.report.get("evidence_kind") == "deterministic_fixture" else "") + name + ": " + ("passed" if status is True else str(status)), flush=True)

    def remember(self, turn):
        self.report["conversation_id"] = turn["conversation_id"]
        previous = self.report["actor_turns"].get(turn["id"], {})
        self.report["model_requests"] += turn["model_requests"] - previous.get("model_requests", 0)
        self.report["actor_turns"][turn["id"]] = {
            "digest": digest(turn), "state": turn["state"],
            "model_requests": turn["model_requests"],
            "message_ids": [m["id"] for m in turn["messages"]],
            "action_ids": [a["id"] for a in turn["actions"]],
        }
        self.save()

    def ask(self, text):
        # Nine bounded Actor turns at most; each Kernel turn permits <=4 calls.
        self.reserve_turn()
        words = ["ask", text]
        if self.report.get("conversation_id"):
            words += ["--conversation", self.report["conversation_id"]]
        turn = self.command(*words)
        self.remember(turn)
        return turn

    def reserve_turn(self):
        require(self.spent_requests() <= self.request_budget - 4,
                "Acceptance model-call budget exhausted")

    def spent_requests(self):
        extra = sum(record.get("model_requests", 0)
                    for record in self.report.get("supplemental_checks", {}).values()
                    if record["turn_id"] not in self.report["actor_turns"])
        return self.report["model_requests"] + extra

    def history(self):
        turns, cursor = [], 0
        for _ in range(10):
            page = self.command("conversation", self.report["conversation_id"], "--after", cursor)
            turns.extend(page["turns"])
            if not page["has_more"]:
                return turns
            require(page["next_cursor"] > cursor, "Conversation cursor did not advance")
            cursor = page["next_cursor"]
        raise CheckFailed("Acceptance conversation unexpectedly exceeds history bound")

    def continuity(self):
        actual = {t["id"]: t for t in self.history()}
        for tid, record in self.report["actor_turns"].items():
            require(tid in actual and digest(actual[tid]) == record["digest"],
                    "Actor IDs/content/actions changed during recovery")
        # An explicitly run isolated read probe may use another conversation.
        # Verify its committed snapshot too; never regenerate it on reconnect.
        for name, record in self.report.get("supplemental_checks", {}).items():
            require(name in {"work.list", "work.inspect", "work.history", "profile.list", "profile.inspect"},
                    "Unexpected supplemental acceptance check")
            turn = self.command("actor", "show", record["turn_id"])
            require(turn["conversation_id"] == record["conversation_id"]
                    and digest(turn) == record["digest"], "Supplemental read changed during recovery")

    def stream(self):
        self.reserve_turn()
        request = str(uuid.uuid4())
        chunks = queue.Queue(64)
        started = time.monotonic()
        process = subprocess.Popen([self.binary, "ask", "请用中文写一段约三百字的自我介绍，只对话，不调用工具。",
                                    "--conversation", self.report["conversation_id"],
                                    "--request-id", request, "--timing"],
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=self.env)
        def read_output():
            while data := process.stdout.read1(1024):
                chunks.put(data)
            chunks.put(None)
        stderr = bytearray()
        def read_error():
            while data := process.stderr.read(1024):
                if len(stderr) < 16384:
                    stderr.extend(data)
        threading.Thread(target=read_output, daemon=True).start()
        err_thread = threading.Thread(target=read_error, daemon=True)
        err_thread.start()
        visible, observed_running, body = None, False, bytearray()
        try:
            while True:
                remaining = 180 - (time.monotonic() - started)
                require(remaining > 0, "Streaming check timed out; Kernel inference remains owned by daemon")
                try:
                    data = chunks.get(timeout=remaining)
                except queue.Empty:
                    raise CheckFailed("Streaming check timed out; inspect conversation before further calls") from None
                if data is None:
                    break
                body.extend(data)
                require(len(body) <= 128 * 1024, "Streaming CLI output exceeded bound")
                # Prose is transient in memory, never logged/evidence. Prefix alone
                # is not useful output; require actual non-whitespace text bytes.
                marker = body.find(b"Lain> ")
                if visible is None and marker >= 0 and body[marker + 6:].strip():
                    visible = round((time.monotonic() - started) * 1000)
                    observed_running = any(t["request_id"] == request and t["state"] == "running"
                                           for t in self.history())
            require(process.wait(timeout=10) == 0, "Streaming Actor turn failed")
            err_thread.join(timeout=5)
        finally:
            if process.poll() is None:
                process.kill()  # Only this owned observer, never daemon/worker.
                process.wait(timeout=10)
            process.stdout.close()
            process.stderr.close()
        turns = [t for t in self.history() if t["request_id"] == request]
        require(len(turns) == 1 and turns[0]["state"] == "completed", "Streaming final snapshot missing")
        self.remember(turns[0])
        self.report["streaming"] = {"first_visible_ms": visible,
                                    "wall_ms": round((time.monotonic() - started) * 1000),
                                    "kernel": turns[0].get("timing", {})}
        # Fast completion may race the read-only probe. Report missing evidence,
        # never turn a completed-only display into a claimed streaming success.
        self.check("incremental_text_while_running", True if observed_running else "unverified: provider completed before probe or no preview")

    def read_tool(self, name, arguments, prompt):
        turn = self.ask(prompt + " 必须实际调用 " + name.replace(".", "_") +
                        " 函数，参数为 " + json.dumps(arguments, ensure_ascii=False) +
                        "。只调用这个只读工具，依据本次真实结果简短回答；文字描述或复用旧结果不算实际工具调用。")
        require(turn["state"] == "completed", "Read-only tool turn did not complete")
        actions = [a for a in turn["actions"] if a["tool"]["name"] == name
                   and a["tool"]["arguments"] == arguments and a["state"] == "executed"]
        require(len(actions) == 1 and actions[0]["result"] is not None,
                "Provider did not produce the required typed tool call/receipt: " + name)
        result = actions[0]["result"]
        require(len(json.dumps(result).encode()) <= 32 * 1024, "Tool result exceeded evidence bound")
        if result.get("truncated") is True and "data" in result:
            result = result["data"]
        if name == "work.list":
            actual = {s["id"] for s in self.command("work", "list")}
            require(len(result["sessions"]) <= 50 and all(s["id"] in actual for s in result["sessions"]), "Invalid work list receipt")
        elif name == "profile.list":
            actual = {p["id"] for p in self.command("profile", "list")}
            require(len(result["profiles"]) <= 50 and all(p["id"] in actual for p in result["profiles"]), "Invalid profile list receipt")
        elif name == "profile.inspect":
            actual = self.command("profile", "show", arguments["profile_id"])
            require(result["id"] == actual["id"] and result["revision"] == actual["revision"], "Invalid profile receipt")
        elif name == "work.inspect":
            require(result["id"] == arguments["session_id"], "Wrong worker receipt")
            require(self.command("work", "show", result["id"])["id"] == result["id"], "Worker missing")
        elif name == "work.history":
            actual = self.command("work", "history", arguments["session_id"], "--limit", 200)
            known = {e["id"]: digest(e) for e in actual["events"]}
            require(len(result["events"]) <= 20 and all(known.get(e["id"]) == digest(e) for e in result["events"]), "Invalid work history receipt")
        self.check(name)
        return turn

    def proposal(self, separate=False):
        prefix = "这是一次新的独立提案请求，上一项拒绝仍然有效，不能重放旧操作。" if separate else ""
        turn = self.ask(prefix + "请实际调用 work_create 函数提出一个 work.create；只用文字描述不算提交提案。workspace=" + json.dumps(str(self.workspace)) +
                        "，profile_id=codex/local，sandbox=read_only，task=只读检查这个项目的测试配置，简短报告，不编辑文件。等待主人确认，不能声称已经执行。")
        require(turn["state"] == "pending_confirmation", "Provider did not propose a mutation")
        actions = [a for a in turn["actions"] if a["state"] == "proposed"]
        require(len(actions) == 1, "Expected exactly one proposed action")
        action = actions[0]
        require(action["tool"]["name"] == "work.create", "Unexpected mutation proposed; left unapproved")
        arguments = action["tool"]["arguments"]
        require(arguments["workspace"] == str(self.workspace)
                and arguments["profile_id"] == "codex/local"
                and arguments["sandbox"] == "read_only", "Proposed scope differs from authorization; left unapproved")
        print("Actual proposed arguments (approval creates a worker and submits this task):", flush=True)
        print(terminal_safe(action["tool"]), flush=True)
        print("Actor turn:", turn["id"], "Action:", action["id"], flush=True)
        return turn, action

    def worker_terminal(self, sid, tid, timeout):
        # Use existing event observation; snapshots cover a turn that completed
        # before subscription, then events trigger reads. No model polling loop.
        process = subprocess.Popen([self.binary, "--json", "work", "watch", sid],
                                   stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, env=self.env)
        changes = queue.Queue(1)
        def wake():
            try:
                changes.put_nowait(True)
            except queue.Full:
                pass
        def read_events():
            for line in process.stdout:
                if len(line) <= 256 * 1024:
                    try:
                        json.loads(line)
                        wake()
                    except ValueError:
                        pass
            wake()
        threading.Thread(target=read_events, daemon=True).start()
        wake()
        deadline = time.monotonic() + timeout
        try:
            while time.monotonic() < deadline:
                try:
                    changes.get(timeout=max(.01, deadline - time.monotonic()))
                except queue.Empty:
                    break
                turns = self.command("work", "turns", sid)
                actual = next((t for t in turns if t["id"] == tid), None)
                require(actual is not None, "Submitted worker turn missing")
                if actual["state"] in {"completed", "failed", "cancelled"}:
                    self.report["worker"]["turn_state"] = actual["state"]
                    self.report["worker"]["turn_digest"] = digest(actual)
                    self.check("worker_terminal_observed")
                    self.check("worker_completed_successfully", actual["state"] == "completed")
                    return
                approvals = self.command("approval", "list", sid)
                if any(a["status"] == "pending" for a in approvals):
                    self.check("worker_terminal_observed", "pending: owner must review worker permissions separately")
                    return
                require(process.poll() is None, "Worker observer stopped; inspect work without replaying")
            self.check("worker_terminal_observed", "unverified: observation timeout; work remains independent")
        finally:
            if process.poll() is None:
                process.terminate()
            process.wait(timeout=10)
            process.stdout.close()

    def run(self, timeout):
        doctor = self.command("doctor")["diagnostics"]
        require(doctor["actor"]["status"] == "configured", "Configure Actor credentials first")
        require(any(b["available"] for b in doctor["backends"]), "Supported native Codex required")
        hello = self.ask("你好，Lain。请用中文简短介绍你自己，不调用工具。")
        require(hello["state"] == "completed" and any(m["role"] == "assistant" and any("\u4e00" <= c <= "\u9fff" for c in m["content"]) for m in hello["messages"]), "Chinese conversation failed")
        self.check("chinese_conversation")
        self.stream()
        self.read_tool("work.list", {}, "请调用 work.list 查看当前工作。")
        self.read_tool("profile.list", {}, "请调用 profile.list 查看配置。")
        self.read_tool("profile.inspect", {"profile_id": "codex/local"}, "请调用 profile.inspect 检查 codex/local。")
        before = {s["id"] for s in self.command("work", "list")}
        turn, action = self.proposal()
        require(input("Denial test: type DENY to reject these exact arguments: ").strip() == "DENY", "Owner did not confirm denial; proposal remains pending")
        denied = self.command("actor", "deny", turn["id"], action["id"])
        self.remember(denied)
        require(next(a for a in denied["actions"] if a["id"] == action["id"])["state"] == "rejected", "Denial not committed")
        require(before == {s["id"] for s in self.command("work", "list")}, "Work list changed during denial; cannot certify zero launches")
        self.check("denial_no_worker")
        turn, action = self.proposal(separate=True)
        require(input("Separate real delegation: type APPROVE " + action["id"] + " to launch Codex: ").strip() == "APPROVE " + action["id"], "No explicit approval; proposal remains pending")
        try:
            approved = self.command("actor", "approve", turn["id"], action["id"])
        finally:
            # Read back even on uncertain command outcome; never retry mutation.
            self.remember(self.command("actor", "show", turn["id"]))
        result = next(a for a in approved["actions"] if a["id"] == action["id"])
        require(result["state"] == "executed", "Delegation uncertain; inspect partial receipt without replay")
        receipt = result["result"]["outcome"]
        require(receipt["stage"] == "submitted" and receipt["worker_completion"] == "not_observed", "Missing actual submission receipt")
        sid, tid = receipt["work_session_id"], receipt["turn_id"]
        self.report["worker"] = {"session_id": sid, "turn_id": tid}
        self.check("delegation_submitted")
        self.command("actor", "approve", turn["id"], action["id"], expect_failure=True)
        require(digest(self.command("actor", "show", turn["id"])) == digest(approved), "Repeated approval changed committed outcome")
        self.check("duplicate_decision_rejected")
        self.worker_terminal(sid, tid, timeout)
        self.read_tool("work.inspect", {"session_id": sid}, "请调用 work.inspect 检查工作 " + sid + "。")
        self.read_tool("work.history", {"session_id": sid, "after": 0}, "请调用 work.history 检查工作 " + sid + "，after=0。")
        self.continuity()
        self.check("separate_cli_process_continuity")
        self.chat_reopen()
        self.check("full_real_acceptance", "incomplete: daemon restart still requires separate verification")
        print("Next: inspect active work, gracefully stop/restart your own daemon with the same database, then run --check-recovery with this evidence file.")

    def chat_reopen(self):
        if sys.platform != "darwin":
            self.check("interactive_chat_exit_reopen", "unverified: native terminal check required")
            return
        from chat_pty_smoke import Chat
        for _ in range(2):
            chat = Chat(self.binary, self.env or os.environ.copy(), self.report["conversation_id"])
            try:
                chat.until("You> ")
                chat.finish(b"/exit\r\n")
            finally:
                chat.close()
            self.continuity()
        self.check("interactive_chat_exit_reopen")

    def recovery(self):
        self.report = json.loads(self.evidence.read_text(encoding="utf-8"))
        require(self.report.get("format") == 1, "Unsupported acceptance evidence")
        self.evidence_loaded = True
        require(input("Type RESTARTED only after gracefully restarting your own daemon with the same database: ").strip() == "RESTARTED", "Daemon restart not attested")
        self.continuity()
        worker = self.report.get("worker")
        require(worker is not None and worker.get("turn_digest"), "Original terminal worker evidence missing")
        require(self.command("work", "show", worker["session_id"])["id"] == worker["session_id"], "WorkSession missing after restart")
        actual = next((t for t in self.command("work", "turns", worker["session_id"]) if t["id"] == worker["turn_id"]), None)
        require(actual and digest(actual) == worker["turn_digest"], "Worker turn changed after restart")
        self.report["daemon_restart"] = "owner_attested; retained records verified by read-only APIs"
        self.check("daemon_restart_continuity")
        self.chat_reopen()
        required = ["chinese_conversation", "incremental_text_while_running", "work.list",
                    "profile.list", "profile.inspect", "denial_no_worker", "delegation_submitted",
                    "duplicate_decision_rejected", "worker_terminal_observed", "worker_completed_successfully",
                    "work.inspect", "work.history", "separate_cli_process_continuity",
                    "interactive_chat_exit_reopen", "daemon_restart_continuity"]
        complete = all(self.report["checks"].get(name) is True for name in required)
        self.check("full_real_acceptance", True if complete else "incomplete: review individual checks")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--run-real", action="store_true")
    mode.add_argument("--check-recovery", action="store_true")
    parser.add_argument("--workspace", type=Path)
    parser.add_argument("--lain", type=Path)
    parser.add_argument("--evidence", type=Path)
    parser.add_argument("--worker-timeout", type=int, default=180)
    parser.add_argument("--max-model-requests", type=int, default=36,
                        help="Request budget for this run (4..36); subtract earlier attempts from owner-authorized quota")
    args = parser.parse_args()
    if not args.run_real and not args.check_recovery:
        parser.exit(message="Not run: --run-real requires explicit owner provider/workspace authorization; --check-recovery performs reads only.\n")
    if not args.evidence or not 1 <= args.worker_timeout <= 1800 or not 4 <= args.max_model_requests <= 36:
        parser.error("--evidence is required; --worker-timeout must be 1..1800 seconds; --max-model-requests must be 4..36")
    if args.run_real and (args.workspace is None or not args.workspace.is_absolute()):
        parser.error("--workspace must be an existing absolute path on the daemon host")
    if not sys.stdin.isatty():
        parser.error("Owner decisions require an interactive terminal")
    repo = Path(__file__).resolve().parents[1]
    binary = args.lain or repo / "target/debug" / ("lain.exe" if os.name == "nt" else "lain")
    acceptance = Acceptance(binary, args.evidence, args.workspace, request_budget=args.max_model_requests)
    if args.run_real:
        # No silent overwrite of existing evidence or other owner files.
        with args.evidence.open("x", encoding="utf-8") as out:
            out.write("{}\n")
    try:
        acceptance.recovery() if args.check_recovery else acceptance.run(args.worker_timeout)
    except (CheckFailed, OSError, ValueError, KeyError, StopIteration, subprocess.TimeoutExpired, EOFError) as error:
        if args.run_real:
            acceptance.report["checks"]["full_real_acceptance"] = "incomplete"
            acceptance.save()
        elif acceptance.evidence_loaded:
            acceptance.report["checks"]["daemon_restart_continuity"] = "unverified: current recovery check failed"
            acceptance.report["checks"]["full_real_acceptance"] = "incomplete"
            acceptance.save()
        print("Acceptance incomplete: " + (str(error) if isinstance(error, CheckFailed) else type(error).__name__) + ". No mutation retried; no worker stopped.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
