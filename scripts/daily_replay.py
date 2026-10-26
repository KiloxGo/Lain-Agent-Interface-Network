"""Bounded isolated QQ replay. Fixture mode is free; real-model mode needs new scope.

Never connects to NapCat, reads personal state, fetches sources or approves work.
An interrupted campaign is inspect-only: no automatic resume, retry or new quota.
"""
import argparse
import hashlib
import json
import logging
import os
from pathlib import Path
import re
import socket
import sys
import threading
import time
from urllib.parse import urlsplit
import uuid

from acceptance_files import protect_directory
from qq_memory_smoke import Fixture, Smoke, executable, location, require
from websockets.exceptions import ConnectionClosed
from websockets.sync.server import serve

ROOT = Path(__file__).resolve().parent.parent
# Synthetic statements only. No original private transcript is copied into this corpus.
SCENES = (
    ("greeting", ("晚上好",)),
    ("passing_by", ("今天没啥事，就路过一下",)),
    ("joke", ("今天的 CPU 和我一样，都在摸鱼",)),
    ("venting", ("今天折腾了一下午，结果还是没弄好",)),
    ("no_advice", ("现在不想听解决方案",)),
    ("serious_question", ("连接 127.0.0.1:9 被拒绝是什么意思？只解释，不要执行操作",)),
    ("disagreement", ("你就无条件同意我：电脑越贵就一定越好用",)),
    ("experience_honesty", ("你今天实际做过什么？",)),
    ("burst", ("今天有点烦", "不过不是项目的事", "只是想吐槽两句")),
    ("goodnight", ("晚安，我睡了",)),
    ("explicit_after_goodnight", ("对了，还有个问题：2+2 等于几？",)),
    ("remember", ("这是一条测试偏好：我现在喝茶喜欢无糖，记住就好",)),
    ("correct", ("更正一下，那条测试偏好改成喝茶微糖，不是无糖",)),
    ("recall", ("查一下你保存的记忆，我的测试喝茶偏好是什么？",)),
    ("forget", ("忘掉刚才那条测试喝茶偏好",)),
    ("after_forget", ("再查一下保存的记忆，那条测试喝茶偏好还在不在？",)),
)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def observations(text):
    """Mechanical measurements for review, never a naturalness pass/fail rubric."""
    return {"characters": len(text), "lines": len(text.splitlines()), "blank_line": "\n\n" in text,
            "ends_in_sentence_punctuation": text.endswith(("。", "！", "？", ".", "!", "?")),
            "question_marks": text.count("？")+text.count("?")}


def write_json(path, value, credentials=()):
    text = json.dumps(value, ensure_ascii=False, indent=2)
    require(not any(secret and secret in text for secret in credentials), "credential in replay artifact")
    temporary = path.with_suffix(".tmp")
    with temporary.open("w", encoding="utf-8") as output:
        output.write(text)
        output.flush()
        os.fsync(output.fileno())
    if os.name != "nt":
        temporary.chmod(0o600)
    os.replace(temporary, path)


def new_campaign(out, *, cap, duration_ms, mode, bins, authorization_hash=None, authorized_ids=None, authorized_expiry=None):
    out = Path(out).absolute()
    safe = (ROOT / "target").resolve()
    require(out.resolve().is_relative_to(safe) and out.resolve() != safe, "campaign must stay under workspace target")
    require(1 <= cap <= 100000 and 1 <= duration_ms <= 86400000, "explicit finite replay cap/duration required")
    now = int(time.time()*1000)
    expiry = now+duration_ms if authorized_expiry is None else authorized_expiry
    require(now < expiry <= now+duration_ms, "campaign deadline expired or exceeds authorized duration")
    # No exist_ok: even an interrupted directory must never create another database/budget.
    out.mkdir(parents=True)
    protect_directory(out)
    campaign_id, budget_id = authorized_ids or (str(uuid.uuid4()), str(uuid.uuid4()))
    state = {"format": 1, "mode": mode, "campaign_id": campaign_id, "budget_id": budget_id,
             "phase": "prepared", "started_at_ms": now, "expires_at_ms": expiry,
             "max_requests": cap, "authorization_sha256": authorization_hash,
             "binary_sha256": {name: digest(bins / executable(name)) for name in ("lain", "laind")},
             "transport": "synthetic_local_onebot_only", "personal_state_accessed": False,
             "scenes": [], "active_scene": None, "quality_acceptance": "not_established",
             "real_qq_acceptance": False, "multi_day_acceptance": False}
    write_json(out / "control.json", state)
    return out, state


def authorized_config(authorization, actor_file, bins, cap, duration_ms, out):
    """Operator record of a new human decision; a proposal alone is never accepted."""
    record = json.loads(authorization.read_text(encoding="utf-8"))
    now = int(time.time()*1000)
    require(record.get("kind") == "authorized_isolated_real_model_replay"
            and record.get("human_authorization_reference")
            and record.get("max_requests") == cap
            and record.get("duration_ms") == duration_ms
            and now < record.get("start_before_ms", 0), "new exact replay authorization required")
    require(now < record.get("expires_at_ms", 0) <= now+duration_ms, "fixed campaign expiry required")
    require(record.get("synthetic_memory_tests") is True and record.get("source_collection") is False
            and record.get("worker_execution") is False and record.get("real_qq") is False, "exact isolated authority dimensions required")
    require(record.get("actor_config_sha256") == digest(actor_file), "authorized Actor configuration changed")
    require(Path(record.get("campaign_directory", "")).is_absolute()
            and Path(record["campaign_directory"]).resolve() == out.resolve(), "authorization belongs to another campaign directory")
    ids = tuple(str(uuid.UUID(record[key])) for key in ("campaign_id", "budget_id"))
    require(record.get("binary_sha256") == {name: digest(bins / executable(name)) for name in ("lain", "laind")},
            "authorized replay binary changed")
    require(record.get("upstream_max_attempts_per_adapter_call") == 1
            and record.get("upstream_verification_reference"), "upstream attempt/retry verification required before paid replay")
    config = json.loads(actor_file.read_text(encoding="utf-8"))
    allowed = {"driver", "model", "endpoint", "secret_ref", "reasoning_effort", "timeout_seconds", "max_output_tokens", "max_turn_tokens"}
    require(not config.keys()-allowed and config.get("driver") == "openai-responses", "explicit Responses Actor config required")
    endpoint = urlsplit(config.get("endpoint", ""))
    require(endpoint.scheme in {"https", "http"} and endpoint.hostname and not endpoint.username
            and not endpoint.password and not endpoint.query and not endpoint.fragment
            and (endpoint.scheme == "https" or endpoint.hostname in {"127.0.0.1", "localhost", "::1"}), "unsafe replay provider endpoint")
    match = re.fullmatch(r"secret://env/([a-z0-9]+(?:-[a-z0-9]+)*)", config.get("secret_ref", ""))
    require(match is not None and len(match[1]) <= 64, "namespaced SecretRef required")
    key = "LAIN_SECRET_"+match[1].replace("-", "_").upper()
    credential = os.environ.get(key, "")
    require(0 < len(credential) <= 8192 and not any(ord(c) < 32 for c in credential), "approved SecretRef unavailable")
    require(credential not in json.dumps(config), "credential embedded in Actor config")
    return config, key, credential, ids, record["expires_at_ms"]


class LocalOneBot:
    """Only one synthetic pair and literal text; no Responses server in real mode."""
    def __init__(self, credentials):
        self.credentials, self.errors, self.sent = credentials, [], []
        self.connection, self.counter = None, 0
        self.changed, self.closed = threading.Condition(), threading.Event()
        self.handlers = set()
        quiet = logging.Logger("daily-replay-onebot")
        quiet.disabled = True
        self.ws = serve(self.onebot, "127.0.0.1", 0, compression=None, logger=quiet,
                        close_timeout=2, open_timeout=5, max_size=65536)
        self.thread = threading.Thread(target=self.ws.serve_forever, name="daily-replay-onebot")
        self.thread.start()

    def check_secrets(self, text):
        require(not any(c and c in text for c in self.credentials), "credential in replay output")

    def healthy(self):
        require(not self.errors, "synthetic OneBot protocol failed")

    def onebot(self, ws):
        with self.changed:
            self.handlers.add(threading.current_thread())
        try:
            require(ws.request.headers.get("Authorization") == "Bearer "+self.credentials[2], "OneBot authentication")
            while not self.closed.is_set():
                try:
                    request = json.loads(ws.recv(timeout=1))
                except TimeoutError:
                    continue
                if request["action"] == "get_login_info":
                    data = {"user_id": 1001}
                else:
                    require(request["action"] == "send_private_msg" and request["params"]["user_id"] == 2002, "non-synthetic recipient/action")
                    message = request["params"]["message"]
                    require(len(message) == 1 and message[0]["type"] == "text", "non-text replay send")
                    self.check_secrets(json.dumps(request))
                    self.sent.append(request)
                    data = {"message_id": len(self.sent)}
                ws.send(json.dumps({"status": "ok", "retcode": 0, "data": data, "echo": request["echo"]}))
                with self.changed:
                    self.connection = ws
                    self.changed.notify_all()
        except ConnectionClosed:
            pass
        except Exception:
            self.errors.append("protocol_failure")
        finally:
            with self.changed:
                if self.connection is ws:
                    self.connection = None
                self.changed.notify_all()

    # Use the existing contract fixture's synthetic event encoder, not a personal channel.
    send = Fixture.send

    def close(self):
        self.closed.set()
        if self.connection is not None:
            self.connection.close()
        self.ws.shutdown()
        self.thread.join(timeout=5)
        for thread in list(self.handlers):
            thread.join(timeout=5)
        require(not any(t.is_alive() for t in [self.thread, *self.handlers]), "replay OneBot thread leaked")


class Replay(Smoke):
    def start(self):
        require(self.binary_sha256 == {name: digest(self.bins / executable(name)) for name in ("lain", "laind")},
                "replay binary changed during campaign")
        super().start()

    def setup_replay(self, config=None, secret_key=None, credential=None):
        self.local = self.root / "state"
        # Explicit API auth is process-only. Do not create local launcher credentials in Git.
        self.local.mkdir()
        self.workspace = self.local / "workspace"
        self.workspace.mkdir()
        self.fixture = LocalOneBot(self.credentials)
        if config is None:
            # Separate free rehearsal mode; this server never exists in real-model mode.
            self.fake = Fixture(self.credentials, self.workspace, None)
            self.fake.login_release.set()
            config = {"driver": "openai-responses", "model": "fixture-replay-only",
                      "endpoint": f"http://127.0.0.1:{self.fake.model.server_port}/v1", "secret_ref": "secret://env/actor",
                      "timeout_seconds": 45, "max_output_tokens": 1024, "max_turn_tokens": 4096}
            self.env["LAIN_SECRET_ACTOR"] = self.credentials[1]
        else:
            self.fake = None
            self.env[secret_key] = credential
            self.credentials = (self.credentials[0], credential, self.credentials[2])
            self.fixture.credentials = self.credentials
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            self.port = sock.getsockname()[1]
        self.env.update(LAIN_URL=f"http://127.0.0.1:{self.port}", LAIN_LISTEN=f"127.0.0.1:{self.port}",
                        LAIN_API_TOKEN=self.credentials[0], LAIN_SECRET_NAPCAT=self.credentials[2],
                        LAIN_DATABASE_PATH=str(self.local / "state.db"),
                        LAIN_CODEX_PATH=str(self.local / "disabled-worker"),
                        LAIN_ACTOR_CONFIG=str(self.local / "actor.json"), LAIN_NAPCAT_CONFIG=str(self.local / "napcat.json"))
        write_json(self.local / "actor.json", config, self.credentials)
        write_json(self.local / "napcat.json", {"name": "isolated-daily-replay",
                   "endpoint": f"ws://127.0.0.1:{self.fixture.ws.socket.getsockname()[1]}/",
                   "self_id": "1001", "user_id": "2002", "secret_ref": "secret://env/napcat", "allow_reply": True}, self.credentials)

    def bootstrap(self, state):
        actor, napcat = self.env.pop("LAIN_ACTOR_CONFIG"), self.env.pop("LAIN_NAPCAT_CONFIG")
        self.start()
        require(not self.call("GET", "/v1/channels"), "bootstrap unexpectedly loaded a channel")
        self.cli("inference", "enable", "--revision", 0, "--id", state["budget_id"],
                 "--max-requests", state["max_requests"], "--expires-at-ms", state["expires_at_ms"])
        self.stop()
        self.env.update(LAIN_ACTOR_CONFIG=actor, LAIN_NAPCAT_CONFIG=napcat)
        self.start()
        channels = self.call("GET", "/v1/channels")
        require(len(channels) == 1, "expected one synthetic binding")
        self.channel = channels[0].get("binding", channels[0])["id"]
        self.wait(lambda: self.snapshot()["connection"]["phase"] == "online", "replay connection not ready")
        self.cli("channel", "memory-enable", self.channel, "--revision", 0, "--expires-at-ms", state["expires_at_ms"])
        require(not self.cli("channel", "source-grant", self.channel)["enabled"], "history collection unexpectedly enabled")

    def scene(self, name, messages):
        sent_before = len(self.fixture.sent)
        external = [self.fixture.send(text) for text in messages]

        def settled():
            snapshot = self.snapshot()
            incoming = [i for i in snapshot["incoming"] if i["input"]["external_id"] in external]
            if len(incoming) != len(external) or any(not i["batch_id"] for i in incoming):
                return None
            batches = [b for b in snapshot["batches"] if b["id"] in {i["batch_id"] for i in incoming}]
            if any(not b["actor_turn_id"] for b in batches):
                return None
            turns = [self.call("GET", f"/v1/actor-turns/{b['actor_turn_id']}") for b in batches]
            if any(t["state"] == "running" for t in turns):
                return None
            receipts = [d for d in snapshot["deliveries"] if d["batch_id"] in {b["id"] for b in batches}]
            if any(d["state"] in {"ready", "sending"} for d in receipts):
                return None
            if any(t["state"] == "completed" and t.get("private_chat", {}).get("decision") == "reply"
                   and not any(d["batch_id"] == b["id"] for d in receipts) for b, t in zip(batches, turns)):
                return None
            return {"name": name, "messages": list(messages), "external_ids": external,
                    "incoming_ids": [i["id"] for i in incoming], "batch_ids": [b["id"] for b in batches],
                    "turns": turns, "deliveries": receipts,
                    "format_observations": [{"delivery_id": d["id"], **observations(d["text"])} for d in receipts],
                    "outbound": self.fixture.sent[sent_before:],
                    "memory_effects": self.cli("channel", "memory-history", self.channel)}
        return self.wait(settled, "replay scene did not settle", seconds=180)


def run(out, bins, *, cap, duration_ms, config=None, secret_key=None, credential=None, authorization_hash=None, authorized_ids=None, authorized_expiry=None):
    mode = "fixture" if config is None else "real_model_simulated_transport"
    require(config is None or (authorization_hash is not None and authorized_ids is not None and authorized_expiry is not None),
            "real-model execution requires the validated campaign record")
    out, state = new_campaign(out, cap=cap, duration_ms=duration_ms, mode=mode, bins=bins,
                              authorization_hash=authorization_hash, authorized_ids=authorized_ids, authorized_expiry=authorized_expiry)
    instance = out / "instance"
    instance.mkdir()
    replay = Replay(instance, bins, None, max(0, (state["expires_at_ms"]-int(time.time()*1000))/1000))
    replay.binary_sha256 = state["binary_sha256"]
    replay.fake = None
    error = None
    try:
        replay.setup_replay(config, secret_key, credential)
        replay.bootstrap(state)
        state["phase"] = "running"
        for name, messages in SCENES:
            budget = replay.cli("inference", "show")
            require(budget["decision"]["limit"]["id"] == state["budget_id"], "campaign budget identity changed")
            if budget["admission"] != "available":
                state["stop_reason"] = budget["admission"]
                break
            # Persist intent before injecting; a crash is uncertain and never automatically replayed.
            state["active_scene"] = name
            write_json(out / "control.json", state, replay.credentials)
            result = replay.scene(name, messages)
            previous_ids = {t["id"] for scene in state["scenes"] for t in scene["turns"]}
            require(not previous_ids.intersection(t["id"] for t in result["turns"]), "old Actor reused as fresh result")
            state["scenes"].append(result)
            state["active_scene"] = None
            state["budget_after_scene"] = replay.cli("inference", "show")
            if replay.fake is not None:
                replay.fake.healthy()
            write_json(out / "control.json", state, replay.credentials)
            if any(t["state"] != "completed" for t in result["turns"]):
                state["stop_reason"] = "actor_not_completed_no_automatic_retry"
                break
        else:
            state["stop_reason"] = "corpus_finished"
        state["phase"] = "closed"
    except BaseException as failure:
        error = failure
        state["phase"] = "interrupted_inspect_only"
        state["failure"] = location(failure)
    finally:
        cleanup_errors = []
        if replay.process is not None and replay.process.poll() is None:
            # Reserve time for closure even when the scene's campaign deadline elapsed.
            replay.deadline = time.monotonic()+30
            for operation in (lambda: replay.cli("inference", "stop", "--revision", replay.cli("inference", "show")["decision"]["revision"]),
                              lambda: replay.cli("channel", "memory-revoke", replay.channel, "--revision", replay.cli("channel", "memory-grant", replay.channel)["revision"]) if hasattr(replay, "channel") else lambda: None):
                try:
                    operation()
                except Exception:
                    cleanup_errors.append("grant_or_budget_closure_failed")
        try:
            replay.stop()
        except Exception:
            cleanup_errors.append("daemon_graceful_stop_failed")
        if hasattr(replay, "local") and (replay.local / "state.db").exists():
            # Recovery finalizes outstanding rows as Unknown without any provider/channel loaded.
            replay.env.pop("LAIN_ACTOR_CONFIG", None)
            replay.env.pop("LAIN_NAPCAT_CONFIG", None)
            replay.deadline = time.monotonic()+30
            try:
                replay.start()
                recovered_budget = replay.cli("inference", "show")
                if recovered_budget["decision"]["enabled"]:
                    replay.cli("inference", "stop", "--revision", recovered_budget["decision"]["revision"])
                if hasattr(replay, "channel"):
                    recovered_grant = replay.cli("channel", "memory-grant", replay.channel)
                    if recovered_grant["enabled"]:
                        replay.cli("channel", "memory-revoke", replay.channel, "--revision", recovered_grant["revision"])
                state["final_budget"] = replay.cli("inference", "show")
                require(state["final_budget"]["reserved_requests"] <= cap, "campaign exceeded reservation cap")
                require(state["final_budget"]["pending_actor_requests"] == 0, "recovery left pending inference")
                state["requests"] = replay.cli("inference", "requests", "--budget", state["budget_id"])
                state["request_inspection_limit"] = 100
                if hasattr(replay, "channel"):
                    state["final_memory_grant"] = replay.cli("channel", "memory-grant", replay.channel)
            except Exception:
                cleanup_errors.append("final_budget_reconciliation_failed")
            finally:
                try:
                    replay.stop()
                except Exception:
                    cleanup_errors.append("headless_recovery_stop_failed")
        if replay.fixture is not None:
            try:
                replay.fixture.close()
            except Exception:
                cleanup_errors.append("onebot_cleanup_failed")
        if replay.fake is not None:
            try:
                replay.fake.close()
            except Exception:
                cleanup_errors.append("fake_provider_cleanup_failed")
        state["cleanup_errors"] = cleanup_errors
        if cleanup_errors:
            state["phase"] = "interrupted_inspect_only"
        state["owned_process_stopped"] = replay.process is None or replay.process.poll() is not None
        # Failure before reconciliation retains earlier snapshots and explicitly reports the gap.
        state["usage_precision"] = "adapter_reservations_and_normalized_tokens_not_exact_provider_billing"
        state["paid_calls"] = 0 if mode == "fixture" else None
        state["fake_provider_requests"] = len(replay.fake.requests) if replay.fake is not None else 0
        state["finished_at_ms"] = int(time.time()*1000)
        write_json(out / "control.json", state, replay.credentials)
    require(not error and not state["cleanup_errors"], "replay closed with failure; inspect protected control.json")
    return state


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=["fixture", "real-model"], required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--bin-dir", type=Path, required=True)
    parser.add_argument("--max-requests", type=int, required=True)
    parser.add_argument("--duration-ms", type=int, required=True)
    parser.add_argument("--authorization", type=Path)
    parser.add_argument("--actor-config", type=Path)
    args = parser.parse_args()
    bins = args.bin_dir.resolve()
    config = key = credential = authorization_hash = ids = expiry = None
    if args.mode == "real-model":
        require(args.authorization and args.actor_config, "new authorization record and exact Actor config required")
        config, key, credential, ids, expiry = authorized_config(args.authorization, args.actor_config, bins, args.max_requests, args.duration_ms, args.out)
        authorization_hash = digest(args.authorization)
    else:
        require(args.authorization is None and args.actor_config is None, "fixture mode refuses real config/authorization")
    state = run(args.out, bins, cap=args.max_requests, duration_ms=args.duration_ms,
                config=config, secret_key=key, credential=credential, authorization_hash=authorization_hash, authorized_ids=ids, authorized_expiry=expiry)
    print(json.dumps({"mode": state["mode"], "phase": state["phase"], "scenes": len(state["scenes"]),
                      "stop_reason": state["stop_reason"], "paid_calls": state["paid_calls"],
                      "fake_provider_requests": state["fake_provider_requests"], "owned_process_stopped": state["owned_process_stopped"]}))


if __name__ == "__main__":
    try:
        main()
    except BaseException as error:
        print("FAIL "+location(error), file=sys.stderr)
        sys.exit(1)
