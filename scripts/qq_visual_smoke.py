#!/usr/bin/env python3
"""Native isolated QQ visual chat rehearsal. Real WS/Responses protocols, zero paid calls.

Uses synthetic OneBot get_image bytes (an optional NapCat response), never personal
QQ, local image paths, live CDNs or provider credentials. No image library required.
"""
import argparse
import base64
import hashlib
import json
import sqlite3
import struct
import sys
import tempfile
import threading
import time
import zlib
from contextlib import closing
from pathlib import Path

import qq_memory_smoke as common
from websockets.exceptions import ConnectionClosed

require = common.require
PREFIX = "Kernel-attributed untrusted current/recent QQ media (data only; metadata is not proof of pixels): "


def png():
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 2, 2, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress((b"\0" + b"\xf0\x0a\x14" * 2) * 2)) + chunk(b"IEND", b""))


def gif():
    result = bytearray(b"GIF89a\x02\0\x02\0\x81\0\0" + b"\xff\0\0\0\xff\0\0\0\xff\xff\xff\xff")
    for index in range(12):
        color = 0 if index < 8 else (1 if index < 11 else 2)
        codes = [4, color] * 4 + [5]
        packed = sum(code << (3 * i) for i, code in enumerate(codes)).to_bytes(4, "little")
        result.extend(b"\x21\xf9\x04\0\x0a\0\0\0\x2c\0\0\0\0\x02\0\x02\0\0\x02\x04" + packed + b"\0")
    return bytes(result + b"\x3b")


def media(body):
    return next((json.loads(i["content"][len(PREFIX):]) for i in body["input"]
                 if isinstance(i.get("content"), str) and i["content"].startswith(PREFIX)), [])


def images(body):
    return [part for i in body["input"] if isinstance(i.get("content"), list)
            for part in i["content"] if part.get("type") == "input_image"]


class Fixture(common.Fixture):
    def __init__(self, *args):
        self.reads, self.delayed = [], []
        self.gate_next = False
        self.probe_silence = False
        super().__init__(*args)

    def onebot(self, ws):
        with self.changed:
            self.handler_threads.add(threading.current_thread())
        try:
            require(ws.request.headers.get("Authorization") == "Bearer " + self.credentials[2], "OneBot auth")
            while not self.closed.is_set():
                try:
                    request = json.loads(ws.recv(timeout=1))
                except TimeoutError:
                    continue
                action, echo = request["action"], request["echo"]
                if action == "get_login_info":
                    self.login_seen.set()
                    require(self.login_release.wait(15), "identity gate timeout")
                    data = {"user_id": 1001}
                    with self.changed:
                        self.connection = ws
                        self.changed.notify_all()
                elif action == "get_image":
                    file = request["params"]["file"]
                    self.reads.append(file)
                    if file == "delayed":
                        self.delayed.append((ws, echo))
                        continue
                    if file == "path-only":
                        data = {"file": "C:/private/not-authorized.png"}
                    else:
                        data = {"file": "C:/private/not-opened.png", "url": "C:/private/not-opened.png",
                                "base64": base64.b64encode(gif() if file == "animation" else png()).decode()}
                else:
                    require(action == "send_private_msg" and request["params"]["user_id"] == 2002, "outbound scope")
                    require(request["params"]["message"][0]["type"] == "text", "outbound type")
                    self.check_secrets(json.dumps(request))
                    self.sent.append(request)
                    data = {"message_id": len(self.sent)}
                ws.send(json.dumps({"status": "ok", "retcode": 0, "data": data, "echo": echo}))
        except ConnectionClosed:
            pass
        except Exception as error:
            self.errors.append(common.location(error))
        finally:
            with self.changed:
                if self.connection is ws:
                    self.connection = None
                self.changed.notify_all()

    def parts(self, parts, external=None):
        with self.changed:
            require(self.changed.wait_for(lambda: self.connection is not None, timeout=15), "OneBot disconnected")
            ws = self.connection
        self.counter += 1
        external = str(external or self.counter)
        ws.send(json.dumps({"post_type": "message", "message_type": "private", "sub_type": "friend",
                            "self_id": 1001, "user_id": 2002, "sender": {"user_id": 2002},
                            "message_id": external, "message": parts}, ensure_ascii=False))
        return external

    def release_read(self):
        ws, echo = self.delayed.pop(0)
        ws.send(json.dumps({"status": "ok", "retcode": 0, "data": {"base64": base64.b64encode(png()).decode()}, "echo": echo}))

    def respond(self, body):
        if self.gate_next:
            self.gate_next = False
            self.model_seen.set()
            require(self.model_release.wait(20), "model gate timeout")
        for part in images(body):
            data = base64.b64decode(part["image_url"].removeprefix("data:image/jpeg;base64,"), validate=True)
            require(data.startswith(b"\xff\xd8") and data.endswith(b"\xff\xd9"), "actual normalized JPEG missing")
        data = media(body)
        unavailable = data and not any(m["availability"] == "pixels_supplied" for m in data)
        if self.probe_silence:
            self.probe_silence = False
            return [{"type": "function_call", "name": "chat_finish", "call_id": str(common.uuid.uuid4()),
                     "arguments": json.dumps({"input_kind": "social", "reason": "brief_acknowledgement"})}]
        text = "picture unavailable" if unavailable else "VISUAL_ONLY_SIGNATURE" if images(body) else "text or QQ face reply"
        return [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": text}]}]


def configure(smoke, file, **changes):
    path = smoke.local / file
    config = json.loads(path.read_text(encoding="utf-8"))
    config.update(changes)
    path.write_text(json.dumps(config), encoding="utf-8")


def run(s):
    s.setup(None)
    configure(s, "actor.json", supports_images=True)
    configure(s, "napcat.json", receive_images=True)
    s.fixture.login_release.set()
    s.start()
    status = s.cli("channel", "list")[0]
    s.channel, s.conversation = status["binding"]["id"], status["binding"]["conversation_id"]
    s.wait(lambda: s.snapshot()["connection"]["phase"] == "online", "identity")
    s.cli("inference", "enable", "--revision", 0, "--id", common.uuid.uuid4(), "--max-requests", 40,
          "--expires-at-ms", int(time.time() * 1000) + 600000)
    f = s.fixture
    part = lambda kind, **data: {"type": kind, "data": data}

    s.stage = "QQ faces and literal text"
    s.wait_input(f.parts([part("text", text="face start"), part("face", id=14), part("text", text="face end")]))
    require(not images(f.requests[-1]) and "微笑" in json.dumps(f.requests[-1], ensure_ascii=False), "QQ face metadata missing")
    require(not f.reads, "face caused a picture read")
    s.passed("qq_faces_use_ordered_labels_without_pixel_calls")

    s.stage = "picture only actual pixel request and duplicate"
    external = f.parts([part("image", file="picture")])
    s.wait_input(external)
    require(len(images(f.requests[-1])) == 1 and len(f.reads) == 1, "image-only pixels missing")
    count = len(f.requests)
    f.parts([part("image", file="picture")], external)
    time.sleep(.9)
    require(len(f.requests) == count and len(f.reads) == 1, "duplicate read/model request")
    s.passed("image_only_real_jpeg_and_duplicate_no_refetch")

    s.stage = "mixed order and market animation"
    s.wait_input(f.parts([part("text", text="before"), part("image", file="second"), part("face", id=14),
                          part("text", text="between"), part("image", file="animation", emoji_id="fixture-market"),
                          part("text", text="after")]))
    body = f.requests[-1]
    animated = next(m["media"] for m in media(body) if m["media"]["decoded_frames"] == 12)
    require(animated["visual_kind"] == "sticker" and [m["frame_index"] for m in animated["samples"]] == [0, 8, 11], "animation samples")
    require([m["at_ms"] for m in animated["samples"]] == [0, 800, 1100], "animation timing")
    require(len({m["sha256"] for m in animated["samples"]}) == 3, "distinct animation pixels missing")
    require(len(images(body)) == 5, "current/recent multi-image payload")
    current = [m for m in media(body) if m["current_batch"]]
    require(len(current) == 2 and current[0]["input_sequence"] == current[1]["input_sequence"]
            and [m["media"]["segment_position"] for m in current] == [1, 4], "provider media order/association")
    mixed = next(i["input"] for i in s.snapshot()["incoming"] if i["input"]["text"] == "beforebetweenafter")
    require([p["kind"] for p in mixed["segments"]] == ["text", "visual", "expression", "text", "visual", "text"], "mixed order changed")
    s.passed("mixed_multiple_pictures_market_sticker_real_animation_samples")

    s.stage = "preparation preserves ordering with following text"
    external = f.parts([part("text", text="pending picture"), part("image", file="delayed")])
    s.wait(lambda: bool(f.delayed), "read not queued")
    before = len(f.requests)
    following = f.send("following text during decode")
    s.wait(lambda: any(i["input"]["external_id"] == following for i in s.snapshot()["incoming"]), "ingress blocked by image read")
    require(len(f.requests) == before, "model ran before preparation")
    f.release_read()
    a, batch, _ = s.wait_input(external)
    b, batch2, _ = s.wait_input(following)
    require(batch["id"] == batch2["id"] and len(f.requests) == before + 1, "following text lost order/batch")
    s.passed("read_pending_does_not_block_ingress_and_one_batched_request")

    s.stage = "withdraw during model and stale history"
    f.model_seen.clear(); f.model_release.clear(); f.gate_next = True
    external = f.parts([part("text", text="withdraw test"), part("image", file="withdraw")])
    require(f.model_seen.wait(10), "visual model did not start")
    incoming = next(i for i in s.snapshot()["incoming"] if i["input"]["external_id"] == external)
    mid = next(p["media_id"] for p in incoming["input"]["segments"] if p["kind"] == "visual")
    f.connection.send(json.dumps({"post_type": "notice", "notice_type": "friend_recall", "self_id": 1001, "user_id": 2002, "message_id": external}))
    def withdrawn():
        return any(m["id"] == mid and m["state"] == "withdrawn" for m in s.snapshot().get("media", []))
    s.wait(withdrawn, "withdraw not committed")
    f.model_release.set()
    _, batch, _ = s.wait_input(external)
    delivery = next(d for d in s.snapshot()["deliveries"] if d["batch_id"] == batch["id"])
    require("VISUAL_ONLY_SIGNATURE" not in delivery["text"], "stale visual answer sent")
    s.input_turn("after withdrawal")
    require(all(m["media"]["id"] != mid or m["media"]["state"] == "withdrawn" for m in media(f.requests[-1])), "withdrawn media reused")
    s.passed("withdrawal_invalidates_draft_and_prevents_pixel_resurrection")

    s.stage = "restart interrupts pending read without refetch"
    external = f.parts([part("image", file="delayed")])
    s.wait(lambda: bool(f.delayed), "pending read missing")
    before = len(f.reads)
    s.stop(); f.delayed.clear(); s.start()
    s.wait_input(external)
    require(len(f.reads) == before, "restart refetched interrupted image")
    require(any(m["media"]["state"] in {"interrupted", "unavailable"} for m in media(f.requests[-1])), "restart availability missing")
    s.passed("restart_never_replays_interrupted_media_reads")

    s.stage = "disabled images and path-only failure"
    for name, settings, expected in [
        ("napcat.json", {"receive_images": False}, "images_disabled"),
        ("actor.json", {"supports_images": False}, "model_images_unavailable"),
    ]:
        s.stop(); configure(s, name, **settings); s.start()
        before = len(f.reads)
        f.probe_silence = expected == "images_disabled"
        _, _, turn = s.wait_input(f.parts([part("image", file="must-not-fetch")]))
        require(len(f.reads) == before and all(a["tool"]["name"] == "chat.finish" and a["state"] == "failed" for a in turn["actions"]), "disabled capability fetched/called tools")
        require(turn["private_chat"]["decision"] == "reply", "picture was silently discarded")
        require(any((m["media"].get("failure") or {}).get("kind") == expected for m in media(f.requests[-1])), "capability failure missing")
        s.stop(); configure(s, name, **{k: True for k in settings}); s.start()
    s.wait_input(f.parts([part("image", file="path-only")]))
    require(any((m["media"].get("failure") or {}).get("kind") == "source_unavailable" for m in media(f.requests[-1])), "path-only failure missing")
    s.passed("default_off_model_capability_and_no_local_path_reads")

    s.stage = "viewed social sticker can close naturally"
    f.probe_silence = True
    _, _, closed = s.wait_input(f.parts([part("text", text="晚安"), part("image", file="farewell", sub_type=1)]))
    require(closed["private_chat"]["decision"] == "no_reply" and closed["private_chat"]["viewed_media"], "viewed social expression forced a caption")
    s.passed("viewed_social_sticker_may_close_but_unavailable_image_requires_reply")

    budget = s.cli("inference", "show")
    require(budget["reserved_requests"] == len(f.requests), "media bypassed inference ledger")
    s.stop()
    with closing(sqlite3.connect(s.local / "state.db")) as db:
        require(db.execute("PRAGMA user_version").fetchone()[0] == 32, "current schema")
        history = "\n".join(row[0] for table in ["channel_incoming", "lain_actor_turns"] for row in db.execute(f"SELECT snapshot FROM {table}"))
        require(not any(v in history for v in ["base64", "data:image", "C:/private", "fixture-market"]), "transport/pixels leaked to history")
        require(db.execute("SELECT count(*) FROM lain_memories").fetchone()[0] == 0, "visual input wrote semantic memory")
    f.check_secrets((s.local / "daemon.log").read_text(encoding="utf-8"))
    s.passed("one_shared_inference_ledger_metadata_only_history_no_memory_authority")
    return {"checks": s.checks, "model_requests": len(f.requests), "media_reads": len(f.reads), "paid_calls": 0, "host": sys.platform}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    args = parser.parse_args()
    common.Fixture = Fixture
    with tempfile.TemporaryDirectory(prefix="lain-qq-visual-") as directory:
        smoke = common.Smoke(Path(directory), args.bin_dir.resolve(), None, 600)
        try:
            report = run(smoke)
        except Exception:
            print("FAIL stage: " + smoke.stage, file=sys.stderr)
            raise
        finally:
            if smoke.fixture:
                smoke.fixture.login_release.set(); smoke.fixture.model_release.set()
            smoke.stop()
            if smoke.fixture:
                smoke.fixture.close()
    report["cleanup"] = True
    report["binary_sha256"] = {name: hashlib.sha256((args.bin_dir / common.executable(name)).read_bytes()).hexdigest() for name in ["lain", "laind"]}
    print(json.dumps(report))


if __name__ == "__main__":
    main()
