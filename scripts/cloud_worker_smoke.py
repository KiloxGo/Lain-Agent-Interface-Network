"""Native Windows -> real HTTPS Linux Kernel -> native structured Worker fixture.

Requires an explicitly provisioned disposable cloud fixture. URL/owner proof are
environment-only; no host, credential, production DB or real provider is embedded.
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
import urllib.error
import urllib.parse
import urllib.request
import uuid

def require(value, reason):
    if not value:
        raise AssertionError(reason)


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


class Suite:
    def __init__(self, args, root):
        self.args, self.root = args, root
        self.endpoint = os.environ["LAIN_CLOUD_FIXTURE_URL"].rstrip("/")
        parsed = urllib.parse.urlsplit(self.endpoint)
        require(parsed.scheme == "https" and parsed.hostname and not parsed.username
                and not parsed.password and not parsed.query and not parsed.fragment,
                "explicit verified HTTPS fixture required")
        require(parsed.path.startswith("/lain-acceptance-"), "refuse a production endpoint")
        self.owner = os.environ["LAIN_CLOUD_FIXTURE_OWNER_TOKEN"]
        self.secrets = [self.owner]
        self.opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
        self.env = {k: v for k, v in os.environ.items()
                    if not k.upper().startswith(("LAIN_", "OPENAI_", "DEEPSEEK_", "ANTHROPIC_"))
                    and k.upper() not in {"HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY"}}
        self.native_env = self.env | {"LAIN_LOCAL_DIR": str(root / "no-owner-config")}
        self.env.update(LAIN_URL=self.endpoint, LAIN_API_TOKEN=self.owner,
                        LAIN_LOCAL_DIR=str(root / "client"))
        if args.portrait_file:
            self.env["LAIN_PORTRAIT_FILE"] = str(args.portrait_file.resolve())
        self.checks = {}
        self.facts = {}
        self.host = self.terminal = None
        self.logs = []
        self.workspace = root / "workspace"
        self.workspace.mkdir()

    def call(self, method, path, body=None, status=200, credential=None):
        headers = {"Content-Type": "application/json"}
        if credential is not False:
            headers["Authorization"] = "Bearer " + (credential or self.owner)
        payload = None if body is None else json.dumps(body).encode()
        request = urllib.request.Request(self.endpoint + path, method=method, data=payload, headers=headers)
        try:
            with self.opener.open(request, timeout=20) as response:
                code, data = response.status, response.read(4 * 1024 * 1024 + 1)
        except urllib.error.HTTPError as error:
            code, data = error.code, error.read(65536)
        require(code == status, "unexpected HTTP status " + str(code) + " at " + path)
        require(len(data) <= 4 * 1024 * 1024 and not any(s.encode() in data for s in self.secrets),
                "response exceeded budget or disclosed credential")
        return json.loads(data) if data else None

    def control(self, action, **fields):
        return self.call("POST", "/__fixture/control", {"action": action, **fields})

    def cli(self, *items, native=False):
        result = subprocess.run([str(self.args.bin_dir / "lain.exe"), "--json", *map(str, items)],
                                env=self.native_env if native else self.env,
                                capture_output=True, text=True, encoding="utf-8", timeout=60)
        require(not any(s in result.stdout + result.stderr for s in self.secrets), "credential in native output")
        require(result.returncode == 0, "native CLI failed: " + " ".join(map(str, items[:2])))
        return json.loads(result.stdout)

    def wait(self, function, reason, seconds=60):
        deadline = time.monotonic() + seconds
        while True:
            value = function()
            if value:
                return value
            require(time.monotonic() < deadline, reason)
            time.sleep(.1)

    def passed(self, name):
        self.checks[name] = True
        print("PASS " + name, flush=True)

    def enroll(self, label):
        directory = self.root / label
        self.cli("device", "init", "--directory", directory, "--endpoint", self.endpoint,
                 "--workspace", self.workspace, "--label", label,
                 "--expires-at-ms", int(time.time() * 1000) + 1800000, native=True)
        registration = json.loads((directory / "registration.json").read_text())
        proof = json.loads((directory / "device-credential.json").read_text())["token"]
        self.secrets.append(proof)
        self.cli("device", "pair", "--file", directory / "registration.json")
        self.cli("device", "claim", "--directory", directory, native=True)
        return directory, registration, proof

    def start_host(self):
        require(self.host is None or self.host.poll() is not None, "host already running")
        output = (self.root / ("host-" + str(len(self.logs)) + ".log")).open("wb")
        self.logs.append(output)
        startup = subprocess.STARTUPINFO()
        startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        startup.wShowWindow = subprocess.SW_HIDE
        self.host = subprocess.Popen([str(self.args.bin_dir / "lain-device-agent.exe"), "--directory", str(self.device)],
                                     env=self.native_env, stdin=subprocess.DEVNULL, stdout=output,
                                     stderr=subprocess.STDOUT, creationflags=subprocess.CREATE_NEW_PROCESS_GROUP,
                                     startupinfo=startup)

    def stop_host(self, crash=False):
        if self.host and self.host.poll() is None:
            if crash:
                self.host.kill()
            else:
                self.host.send_signal(signal.CTRL_BREAK_EVENT)
            try:
                self.host.wait(timeout=30)
            except subprocess.TimeoutExpired:
                self.host.kill()
                self.host.wait(timeout=15)

    def current(self, command):
        return self.call("GET", "/v1/devices/" + self.did + "/commands/" + command)

    def state(self, command, states):
        return self.wait(lambda: c if (c := self.current(command))["state"] in states else None,
                         "device task did not reach " + str(states))

    def pending(self, command):
        return self.wait(lambda: c if (c := self.current(command)).get("execution", {}).get("pending_approvals") else None,
                         "native permission did not reach cloud")

    def queue(self, text, sandbox="read_only"):
        body = {"request_id": str(uuid.uuid4()), "expected_device_revision": 2,
                "workspace_id": self.registration["workspaces"][0]["id"], "profile_id": "codex/local",
                "sandbox": sandbox, "task": text, "expires_at_ms": int(time.time() * 1000) + 180000}
        path = "/v1/conversations/" + self.conversation + "/device-work/" + self.did
        return self.call("POST", path, body), body

    def decide(self, command, choice):
        pending = self.pending(command)
        approval = pending["execution"]["pending_approvals"][0]
        body = {"request_id": str(uuid.uuid4()), "fingerprint": approval["fingerprint"], "choice": choice}
        receipt = self.call("POST", "/v1/devices/" + self.did + "/commands/" + command + "/approvals/" + approval["id"], body)
        return receipt

    def native_counts(self):
        # The existing managed adapter starts an empty seed, then binds its
        # durable fork (backend/codex.rs). Count both provider threads, retain
        # the empty seed count separately, and count every accepted turn.
        records = [json.loads(path.read_text()) for path in self.workspace.glob("fixture-thread-*.json")]
        counts = {"new_provider_threads": len(records),
                  "empty_seed_threads": sum(not r["turns"] for r in records),
                  "accepted_provider_turns": sum(len(r["turns"]) for r in records)}
        self.facts["latest_observed_native_counts"] = counts
        return counts

    def native_permission_choices(self):
        # These bytes are written by the fixture after receiving the actual RPC
        # response, independently of cloud/local decision and Applied reporting.
        records = [json.loads(path.read_text()) for path in self.workspace.glob("fixture-thread-*.json")]
        choices = sorted(turn["fixture_approval_response"]["decision"]
                         for record in records for turn in record["turns"]
                         if "fixture_approval_response" in turn)
        self.facts["native_wire_permission_choices"] = choices
        return choices

    def prove_terminal_receives_new_server_fact(self, reason):
        # A previously visible connected label cannot prove reconnection. Add a
        # unique server-side turn after the fault and require it in the existing
        # ConPTY, without submitting any locally retained draft.
        marker = reason + "-" + uuid.uuid4().hex[:12]
        submitted = self.call("POST", "/v1/conversations/" + self.conversation + "/turns",
                              {"request_id": str(uuid.uuid4()), "text": "cloud parallel chat " + marker}, status=202)
        self.wait(lambda: self.call("GET", "/v1/actor-turns/" + submitted["id"])["state"] == "completed",
                  "server-side reconnect proof did not finish")
        self.conversation_visible(marker)
        self.facts.setdefault("terminal_post_fault_server_facts", []).append(marker)

    def local_worker(self, command):
        session = self.current(command).get("execution", {}).get("session_id")
        if not session:
            return None
        with closing(sqlite3.connect(self.device / "execution/worker.db", timeout=5)) as db:
            row = db.execute("SELECT snapshot FROM worker_state WHERE session_id=?", (session,)).fetchone()
        return json.loads(row[0]) if row else None

    def visible(self, text):
        self.wait(lambda: text in self.terminal.text(), "terminal did not show " + text, seconds=40)

    def conversation_visible(self, text):
        # Device report rows may push an already-fetched chat reply above the
        # current viewport. Exercise the ordinary read-only conversation scroll,
        # rather than mistaking the last page for the entire displayed history.
        deadline = time.monotonic() + 40
        while time.monotonic() < deadline:
            for _ in range(80):
                if text in self.terminal.text():
                    return
                self.terminal.write("\x1b[5~")
                time.sleep(.04)
            self.terminal.write("\x1b[1;5F")
        raise AssertionError("conversation did not display " + text)

    def open_terminal(self):
        from desk_terminal_smoke import Terminal
        self.terminal = Terminal([str(self.args.bin_dir / "lain.exe"), "desk", "--conversation", self.conversation], self.env)
        self.visible("lain // wired")
        self.visible("connected to Kernel")

    def close_terminal(self):
        if self.terminal:
            self.terminal.write("\x11")
            self.wait(lambda: not self.terminal.process.isalive(), "terminal did not exit", seconds=15)
            require(not any(s in "".join(self.terminal.output) for s in self.secrets), "credential in terminal stream")
            self.terminal.close()
            self.terminal = None

    def review_action(self, hint, key):
        for _ in range(80):
            if hint in self.terminal.text():
                break
            self.terminal.write("\x1b[6~")
            time.sleep(.04)
        self.visible(hint)
        self.terminal.write(key)

    def run(self):
        self.call("GET", "/health", status=401, credential=False)
        require(self.control("stats")["model_calls"] == 0, "fixture already has model activity")
        self.conversation = self.call("POST", "/v1/conversations", {}, status=201)["id"]
        self.device, self.registration, proof = self.enroll("cloud-device-a")
        other, registration_b, proof_b = self.enroll("cloud-device-b")
        self.did = self.registration["id"]
        require(self.registration["workspaces"][0]["path"] == registration_b["workspaces"][0]["path"], "path isolation fixture differs")
        self.call("GET", "/v1/device-agent/" + self.did, credential=proof)
        self.call("GET", "/v1/device-agent/" + self.did, status=401, credential=proof_b)
        self.call("GET", "/v1/devices", status=401, credential=proof)
        self.call("POST", "/__fixture/control", {"action": "stats"}, status=401, credential=proof)
        self.cli("device", "host-configure", "--directory", self.device, "--codex-path", self.args.bin_dir / "lain-test-app-server.exe",
                 "--offline-grace-seconds", 2, native=True)
        require(not (self.device / "execution").exists(), "pairing/configuration started a Worker")
        self.passed("native_https_enrollment_equal_windows_paths_separate_credentials_and_owner_device_isolation")
        self.start_host()
        denied, _ = self.queue("must not launch", sandbox="workspace_write")
        self.state(denied["id"], {"rejected"})
        require(self.native_counts() == {"new_provider_threads": 0, "empty_seed_threads": 0, "accepted_provider_turns": 0}, "local policy rejection launched work")
        self.passed("native_local_policy_rejects_broader_scope_without_worker_launch")

        self.control("dispatch", input={"device_id": self.did, "expected_device_revision": 2,
                     "workspace_id": self.registration["workspaces"][0]["id"], "profile_id": "codex/local",
                     "sandbox": "read_only", "task": "approve", "expires_at_ms": int(time.time() * 1000) + 180000})
        self.open_terminal()
        self.terminal.write("cloud-delegate\r")
        self.visible("action awaits review")
        require(len(self.call("GET", "/v1/devices/" + self.did + "/commands")) == 1, "proposal ran before explicit review")
        self.terminal.write("/review\r")
        self.visible("Outbound task: Changed")
        self.review_action("Ctrl-Y approve once", "\x19")
        def delegated():
            page = self.call("GET", "/v1/conversations/" + self.conversation + "/device-work")
            return next((row for row in page["work"] if row["origin"].get("action_id")), None)
        origin = self.wait(delegated, "reviewed Actor dispatch not recorded")
        command = origin["command"]["id"]
        self.pending(command)
        self.terminal.write("preserved chat draft\t")
        self.visible("device tasks")
        self.terminal.write("\r")
        self.visible("Device task")
        self.visible("No recorded decision")
        self.terminal.write("\x1bOS")
        self.visible("Remote Worker permission")
        self.review_action("Ctrl-D deny", "\x04")
        complete = self.state(command, {"succeeded"})
        decisions = self.call("GET", "/v1/devices/" + self.did + "/commands/" + command + "/approvals")
        require(len(decisions) == 1 and decisions[0]["decision"]["choice"] == "deny", "native UI denial not recorded")
        require(complete["execution"]["approval_reports"] == [{"approval_id": decisions[0]["approval_id"], "decision": decisions[0]["decision"], "stage": "applied"}], "denial application invented or not reported")
        require(self.native_permission_choices() == ["decline"], "native Worker did not actually receive denial")
        self.terminal.write("\x1b")
        self.visible("preserved chat draft")
        self.facts["after_first_native_permission"] = self.native_counts()
        require(self.facts["after_first_native_permission"] == {"new_provider_threads": 2, "empty_seed_threads": 1, "accepted_provider_turns": 1}, "first dispatch repeated native work")
        self.passed("real_https_actor_dispatch_and_native_terminal_exact_deny_report_applied_once")

        allowed, _ = self.queue("approve")
        receipt = self.decide(allowed["id"], "approve_once")
        complete = self.state(allowed["id"], {"succeeded"})
        require(complete["execution"]["approval_reports"] == [{"approval_id": receipt["approval_id"], "decision": receipt["decision"], "stage": "applied"}], "allow application missing")
        require(self.native_permission_choices() == ["accept", "decline"], "native Worker did not receive the exact permission choices")
        base = "/v1/conversations/" + self.conversation + "/device-work/" + self.did + "/" + allowed["id"] + "/followups"
        followup = {"request_id": str(uuid.uuid4()), "expected_device_revision": 2, "expected_parent_revision": complete["revision"],
                    "text": "cloud exact follow-up", "expires_at_ms": int(time.time() * 1000) + 180000}
        child = self.call("POST", base, followup)
        done = self.state(child["id"], {"succeeded"})
        require(done["execution"]["session_id"] == complete["execution"]["session_id"] and done["execution"]["turn_id"] != complete["execution"]["turn_id"], "follow-up changed native session or reused turn")
        retry = self.call("POST", base, followup)
        require(retry["id"] == child["id"], "exact cloud retry created another command")
        require(len(self.local_worker(child["id"])["turns"]) == 2, "follow-up repeated native submission")
        require(self.native_counts() == {"new_provider_threads": 4, "empty_seed_threads": 2, "accepted_provider_turns": 3}, "allow/follow-up counts differ")
        self.passed("real_https_allow_and_same_native_session_followup_exact_retry_without_duplicate_turns")

        waiting, request = self.queue("wait")
        active = self.state(waiting["id"], {"running"})
        self.terminal.write("\x15cloud parallel chat\r")
        self.conversation_visible("fixture reply: cloud parallel chat")
        require(self.current(waiting["id"])["state"] == "running", "chat cancelled independent work")
        model_count = self.control("stats")["model_calls"]
        self.close_terminal()
        require(self.current(waiting["id"])["state"] == "running", "closing client stopped native work")
        self.open_terminal()
        require(self.control("stats")["model_calls"] == model_count, "reopening caused inference")
        first_cursor = self.call("POST", "/v1/client-sync/bootstrap", {"protocol": 1, "conversation_id": self.conversation})["cursor"]
        self.control("restart_kernel")
        second_cursor = self.call("POST", "/v1/client-sync/bootstrap", {"protocol": 1, "conversation_id": self.conversation})["cursor"]
        require(first_cursor["instance_id"] == second_cursor["instance_id"] and first_cursor["boot_id"] != second_cursor["boot_id"], "Kernel restart changed identity or not exercised")
        self.visible("connected to Kernel")
        self.prove_terminal_receives_new_server_fact("after-kernel-restart")
        require(self.current(waiting["id"])["execution"]["session_id"] == active["execution"]["session_id"], "Kernel restart replaced native work")
        self.call("POST", "/v1/conversations/" + self.conversation + "/device-work/" + self.did, request)
        self.call("POST", "/v1/devices/" + self.did + "/commands/" + waiting["id"] + "/cancel", {"expected_revision": self.current(waiting["id"])["revision"]})
        self.state(waiting["id"], {"cancelled"})
        require(self.native_counts() == {"new_provider_threads": 6, "empty_seed_threads": 3, "accepted_provider_turns": 4}, "client/Kernel restart or exact queue retry repeated work")
        self.passed("parallel_cloud_chat_client_exit_restore_kernel_restart_and_explicit_native_cancellation")
        self.facts["native_execution_counts"] = self.native_counts()

        uncertain_followup = {"request_id": str(uuid.uuid4()), "expected_device_revision": 2,
                              "expected_parent_revision": self.current(child["id"])["revision"],
                              "text": "lost-prompt-ack", "expires_at_ms": int(time.time() * 1000) + 180000}
        next_base = "/v1/conversations/" + self.conversation + "/device-work/" + self.did + "/" + child["id"] + "/followups"
        lost_child = self.call("POST", next_base, uncertain_followup)
        self.state(lost_child["id"], {"unknown"})
        expected = {"new_provider_threads": 6, "empty_seed_threads": 3, "accepted_provider_turns": 5}
        require(self.native_counts() == expected, "uncertain follow-up created another thread or repeated submission")
        self.stop_host(crash=True)
        self.start_host()
        retried = self.call("POST", next_base, uncertain_followup)
        require(retried["id"] == lost_child["id"], "uncertain follow-up receipt changed identity")
        self.wait(lambda: self.host.poll() is None and self.current(lost_child["id"])["state"] == "unknown", "uncertain follow-up was revived")
        require(self.native_counts() == expected, "host restart replayed uncertain follow-up")
        self.passed("same_session_lost_native_followup_receipt_retains_unknown_without_replay_after_host_restart")

        unconfirmed, _ = self.queue("approve-unconfirmed")
        receipt = self.decide(unconfirmed["id"], "approve_once")
        complete = self.state(unconfirmed["id"], {"succeeded"})
        require(complete["execution"]["approval_reports"] == [{"approval_id": receipt["approval_id"], "decision": receipt["decision"], "stage": "received"}], "completion falsely manufactured Worker permission application")
        expected = {"new_provider_threads": 8, "empty_seed_threads": 4, "accepted_provider_turns": 6}
        require(self.native_counts() == expected, "unconfirmed permission repeated native work")
        self.passed("cloud_decision_and_device_receipt_do_not_fabricate_missing_native_application")

        recovery, _ = self.queue("wait")
        old = self.state(recovery["id"], {"running"})
        self.stop_host(crash=True)
        self.start_host()
        settled = self.state(recovery["id"], {"cancelled"})
        require(settled["execution"]["session_id"] == old["execution"]["session_id"], "host restart replaced original session")
        expected = {"new_provider_threads": 10, "empty_seed_threads": 5, "accepted_provider_turns": 7}
        require(self.native_counts() == expected, "crash recovery started replacement work")
        self.passed("actual_native_host_crash_recovers_bound_session_without_new_thread_or_prompt")

        lost, _ = self.queue("lost-prompt-ack")
        self.state(lost["id"], {"unknown"})
        self.stop_host(crash=True)
        self.start_host()
        sentinel, _ = self.queue("cloud-after-unknown")
        self.state(sentinel["id"], {"succeeded"})
        require(self.current(lost["id"])["state"] == "unknown", "lost native prompt outcome was invented")
        require(self.local_worker(lost["id"])["turns"][0]["provider_turn_id"] is None, "missing provider correlation invented")
        expected = {"new_provider_threads": 14, "empty_seed_threads": 7, "accepted_provider_turns": 9}
        require(self.native_counts() == expected, "lost native prompt was replayed after restart")
        self.passed("lost_native_prompt_receipt_stays_unknown_and_new_work_can_continue")

        self.control("lose_receipt", kind="started")
        lost_event, _ = self.queue("wait")
        self.state(lost_event["id"], {"cancelled", "unknown"})
        faults = self.control("stats")["dropped_receipts"]
        require(len(faults) == 1 and faults[0]["kind"] == "started", "successful committed HTTP receipt loss not exercised")
        require(len(self.local_worker(lost_event["id"])["turns"]) == 1, "lost cloud receipt replayed native turn")
        expected = {"new_provider_threads": 16, "empty_seed_threads": 8, "accepted_provider_turns": 10}
        require(self.native_counts() == expected, "lost HTTPS acknowledgement duplicated work")
        self.passed("committed_cloud_event_with_lost_https_receipt_recovers_without_duplicate_native_execution")

        partitioned, _ = self.queue("wait")
        partition_command = self.state(partitioned["id"], {"running"})
        self.partition_session = partition_command["execution"]["session_id"]
        self.terminal.write("draft survives network loss")
        self.visible("draft survives network loss")
        self.control("block", value=True)
        try:
            self.wait(lambda: state if (state := self.local_worker_without_cloud(partitioned["id"]))
                      and state["turns"][0]["turn"]["state"] in {"cancelled", "unknown"} else None,
                      "native work did not stop after bounded partition", seconds=35)
            self.visible("draft survives network loss")
            self.visible("disconnected")
        finally:
            self.control("block", value=False)
        self.state(partitioned["id"], {"cancelled", "unknown"})
        self.visible("connected to Kernel")
        self.prove_terminal_receives_new_server_fact("after-partition")
        self.visible("draft survives network loss")
        require(not any(item.get("role") == "user" and item.get("content") == "draft survives network loss"
                        for request in self.control("model_inputs")["requests"] for item in request["input"]),
                "reconnect submitted the retained draft")
        expected = {"new_provider_threads": 18, "empty_seed_threads": 9, "accepted_provider_turns": 11}
        require(self.native_counts() == expected, "partition reconnect repeated native work")
        self.passed("actual_https_partition_stops_owned_work_preserves_cli_draft_and_reconnects_without_replay")

        revoked, _ = self.queue("wait")
        self.state(revoked["id"], {"running"})
        self.cli("device", "revoke", self.did, "--revision", 2)
        self.wait(lambda: self.host.poll() is not None, "revoked host did not exit", seconds=35)
        require(self.current(revoked["id"])["state"] == "unknown", "revocation manufactured a confirmed stop")
        expected = {"new_provider_threads": 20, "empty_seed_threads": 10, "accepted_provider_turns": 12}
        require(self.native_counts() == expected, "revocation repeated native work")
        self.facts["native_execution_counts"] = expected
        self.facts["http_receipt_faults"] = faults
        require(self.call("GET", "/v1/work-sessions") == [], "local device sessions became cloud-local Workers")
        with closing(sqlite3.connect(self.device / "execution/worker.db")) as db:
            require(db.execute("SELECT count(*) FROM lain_conversations").fetchone()[0] == 0
                    and db.execute("SELECT count(*) FROM channel_bindings").fetchone()[0] == 0,
                    "local executor created another persona")
        self.passed("revoked_device_closes_authority_without_fabricating_cancellation_or_local_persona")

    def local_worker_without_cloud(self, command):
        # Partition checks must not use a cloud read to discover local identity.
        # Only the exact session captured before this partition is inspected.
        with closing(sqlite3.connect(self.device / "execution/worker.db", timeout=5)) as db:
            row = db.execute("SELECT snapshot FROM worker_state WHERE session_id=?", (self.partition_session,)).fetchone()
        return json.loads(row[0]) if row else None

    def close(self):
        if self.terminal:
            self.terminal.close()
            self.terminal = None
        self.stop_host()
        for output in self.logs:
            output.close()
            require(not any(s.encode() in Path(output.name).read_bytes() for s in self.secrets), "credential in native log")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--portrait-file", type=Path)
    args = parser.parse_args()
    require(os.name == "nt", "native Windows is required")
    args.bin_dir = args.bin_dir.resolve()
    args.artifacts = args.artifacts.resolve()
    require(not args.artifacts.is_relative_to(Path(__file__).resolve().parents[1]), "artifacts must be outside checkout")
    args.artifacts.mkdir(parents=True, exist_ok=True)
    report = {}
    # Python's cleanup must also address files beyond MAX_PATH. The native
    # application has its own ordinary-path regression; extending this absolute
    # synthetic parent changes neither its depth nor any filesystem permissions.
    temporary_parent = str(args.artifacts)
    if not temporary_parent.startswith("\\\\?\\"):
        temporary_parent = ("\\\\?\\UNC\\" + temporary_parent[2:] if temporary_parent.startswith("\\\\")
                            else "\\\\?\\" + temporary_parent)
    with tempfile.TemporaryDirectory(prefix="lain-cloud-worker-", dir=temporary_parent) as raw:
        suite = Suite(args, Path(raw))
        try:
            suite.run()
            report = {"checks": suite.checks, "facts": suite.facts, "model": suite.control("stats"),
                      "actual_https_certificate_verified": True, "platform": "native Windows to actual Linux",
                      "worker_backend": "structured fixture", "paid_calls": 0, "qq_calls": 0}
        except Exception:
            if suite.terminal:
                suite.terminal.image(args.artifacts / "failure.png")
                (args.artifacts / "failure.txt").write_text(suite.terminal.text(), encoding="utf-8")
            for output in suite.logs:
                output.flush()
                text = Path(output.name).read_text(encoding="utf-8", errors="replace")
                for secret in suite.secrets:
                    text = text.replace(secret, "[fixture credential redacted]")
                (args.artifacts / ("redacted-" + Path(output.name).name)).write_text(text, encoding="utf-8")
            (args.artifacts / "partial-checks.json").write_text(json.dumps(suite.checks, indent=2))
            (args.artifacts / "partial-facts.json").write_text(json.dumps(suite.facts, indent=2))
            raise
        finally:
            suite.close()
    report["temporary_windows_state_cleaned"] = not Path(raw).exists()
    report["binary_sha256"] = {name: hashlib.sha256((args.bin_dir / (name + ".exe")).read_bytes()).hexdigest()
                               for name in ["lain", "lain-device-agent", "lain-test-app-server"]}
    (args.artifacts / "report.json").write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2), flush=True)


if __name__ == "__main__":
    main()
