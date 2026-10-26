"""Read-only forward-OneBot observer with a durable cloud-delivery outbox."""

import base64
import http.client
import ipaddress
import json
import queue
import socket
import sqlite3
import ssl
import threading
import uuid
from concurrent.futures import Future
from contextlib import contextmanager
from pathlib import Path
from urllib.parse import urlsplit

from websockets.sync.client import connect

from .store import MAX_IMAGE, Rejected, Store, encode, identifier, now, require


def public_image(url):
    parsed = urlsplit(url)
    require(
        parsed.scheme in ("https", "http")
        and parsed.hostname
        and not parsed.username
        and not parsed.password
        and parsed.port in (None, 443)
        and not parsed.fragment,
        code="unsafe_image_url",
    )
    host = parsed.hostname.lower()
    require(
        host == "multimedia.nt.qq.com.cn"
        or any(
            host == suffix or host.endswith("." + suffix)
            for suffix in ("qpic.cn", "qlogo.cn")
        ),
        code="image_host_not_allowed",
    )
    # Legacy QQ image URLs use http; fetch that exact CDN path only via HTTPS.
    addresses = socket.getaddrinfo(host, 443, type=socket.SOCK_STREAM)
    require(
        addresses and all(ipaddress.ip_address(a[4][0]).is_global for a in addresses),
        code="non_public_image_address",
    )
    connection = http.client.HTTPSConnection(
        host, timeout=12, context=ssl.create_default_context()
    )
    try:
        # DNS result is pinned to the actual socket while certificate/SNI use the host.
        raw_socket = socket.create_connection((addresses[0][4][0], 443), timeout=12)
        try:
            connection.sock = ssl.create_default_context().wrap_socket(
                raw_socket, server_hostname=host
            )
        except BaseException:
            raw_socket.close()
            raise
        connection.request(
            "GET",
            parsed.path + ("?" + parsed.query if parsed.query else ""),
            headers={"Accept-Encoding": "identity"},
        )
        response = connection.getresponse()
        require(response.status == 200, code="image_download_failed")
        data = response.read(MAX_IMAGE + 1)
        require(0 < len(data) <= MAX_IMAGE, 413, "image_too_large")
        return data
    finally:
        connection.close()


class Cloud:
    def __init__(self, url, token):
        self.url, self.token = urlsplit(url.rstrip("/")), token
        require(
            self.url.scheme == "https"
            and self.url.hostname
            and not self.url.username
            and not self.url.password
            and not self.url.query
            and not self.url.fragment,
            code="https_api_required",
        )

    def call(self, method, path, value=None):
        connection = http.client.HTTPSConnection(
            self.url.hostname, self.url.port or 443, timeout=20
        )
        try:
            body = encode(value).encode() if value is not None else None
            connection.request(
                method,
                self.url.path + path,
                body,
                {
                    "Authorization": "Bearer " + self.token,
                    "Content-Type": "application/json",
                },
            )
            response = connection.getresponse()
            raw = response.read(12 * 1024 * 1024 + 1)
            require(response.status == 200, response.status, "cloud_rejected")
            require(len(raw) <= 12 * 1024 * 1024, code="cloud_response_too_large")
            return json.loads(raw)
        finally:
            connection.close()


class Outbox:
    def __init__(self, path):
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with self.db() as db:
            db.execute("PRAGMA journal_mode=WAL")
            db.execute(
                "CREATE TABLE IF NOT EXISTS outbox(id INTEGER PRIMARY KEY,source_id TEXT NOT NULL,path TEXT NOT NULL,payload TEXT NOT NULL,expires_at_ms INTEGER NOT NULL)"
            )
            db.execute(
                "CREATE TABLE IF NOT EXISTS health(key TEXT PRIMARY KEY,value TEXT NOT NULL)"
            )

    @contextmanager
    def db(self):
        db = sqlite3.connect(self.path, timeout=15)
        try:
            with db:
                yield db
        finally:
            db.close()

    def add(self, source, path, payload):
        with self.db() as db:
            db.execute("DELETE FROM outbox WHERE expires_at_ms<=?", (now(),))
            size = db.execute(
                "SELECT COALESCE(sum(length(payload)),0),count(*) FROM outbox"
            ).fetchone()
            require(
                size[0] + len(encode(payload)) <= 256 * 1024 * 1024 and size[1] < 10000,
                507,
                "outbox_full",
            )
            db.execute(
                "INSERT INTO outbox(source_id,path,payload,expires_at_ms) VALUES (?,?,?,?)",
                (source, path, encode(payload), now() + 86400000),
            )

    def next(self):
        with self.db() as db:
            db.execute("DELETE FROM outbox WHERE expires_at_ms<=?", (now(),))
            db.row_factory = sqlite3.Row
            row = db.execute("SELECT * FROM outbox ORDER BY id LIMIT 1").fetchone()
            return dict(row) if row else None

    def acknowledge(self, sequence):
        with self.db() as db:
            db.execute("DELETE FROM outbox WHERE id=?", (sequence,))

    def revoke(self, source):
        with self.db() as db:
            db.execute("DELETE FROM outbox WHERE source_id=?", (source,))

    def health(self, state):
        with self.db() as db:
            db.execute(
                "INSERT INTO health VALUES ('connection',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                (encode({"state": state, "observed_at_ms": now()}),),
            )


class Observer:
    def __init__(self, config, credentials, cloud=None, download=public_image):
        self.config, self.credentials = config, credentials
        self.account = identifier(config["self_id"])
        self.sources = {
            f"qq:{self.account}:private:{identifier(p)}"
            for p in config.get("private_peers", [])
        }
        self.sources |= {
            f"qq:{self.account}:group:{identifier(p)}" for p in config.get("groups", [])
        }
        require(
            self.sources and len(self.sources) <= 64, code="explicit_sources_required"
        )
        endpoint = urlsplit(config["napcat_endpoint"])
        require(
            (
                endpoint.scheme == "wss"
                or (
                    endpoint.scheme == "ws"
                    and endpoint.hostname in ("127.0.0.1", "localhost", "::1")
                )
            )
            and not endpoint.username
            and not endpoint.password
            and not endpoint.query
            and not endpoint.fragment,
            code="unsafe_napcat_endpoint",
        )
        self.cloud = cloud or Cloud(config["api_url"], credentials["ingest_token"])
        self.outbox = Outbox(config["outbox_path"])
        self.download = download
        self.events = queue.Queue(maxsize=64)
        self.pending = {}
        self.lock = threading.Lock()
        self.ready, self.changed, self.closed = (
            threading.Event(),
            threading.Event(),
            threading.Event(),
        )
        self.ws = None
        self.blocked = set()
        self.scrubber = Store.__new__(Store)
        self.scrubber.secrets = tuple(credentials.values())

    def source_for(self, event):
        if str(event.get("self_id")) != self.account:
            return None
        kind = event.get("message_type")
        if kind is None and event.get("notice_type") in (
            "friend_recall",
            "group_recall",
        ):
            kind = "group" if event["notice_type"] == "group_recall" else "private"
        if kind not in ("private", "group"):
            return None
        peer = event.get("group_id") if kind == "group" else event.get("user_id")
        source = f"qq:{self.account}:{kind}:{peer}"
        return source if source in self.sources and source not in self.blocked else None

    def action(self, name, params):
        require(name in ("get_login_info", "get_image"), 403, "read_only_observer")
        echo = str(uuid.uuid4())
        future = Future()
        with self.lock:
            self.pending[echo] = future
        try:
            require(self.ws is not None, 503, "not_connected")
            self.ws.send(encode({"action": name, "params": params, "echo": echo}))
            result = future.result(timeout=12)
            require(
                result.get("status") == "ok" and result.get("retcode") == 0,
                code="onebot_read_failed",
            )
            return result.get("data", {})
        finally:
            with self.lock:
                self.pending.pop(echo, None)

    def capture(self, event):
        source = self.source_for(event)
        if not source:
            return
        if event.get("post_type") == "notice":
            self.outbox.add(
                source,
                "/v1/ingest/recall",
                {"source_id": source, "external_id": str(event["message_id"])},
            )
            self.changed.set()
            return
        if event.get("post_type") != "message":
            return
        require(
            isinstance(event.get("message"), list)
            and 1 <= len(event["message"]) <= 128,
            code="segment_array_required",
        )
        require(
            sum(p.get("type") == "image" for p in event["message"]) <= 8,
            code="too_many_images",
        )
        images = []
        parts = []
        for part in event["message"]:
            typ, data = part.get("type"), part.get("data", {})
            if typ == "image":
                file = data.get("file")
                require(isinstance(file, str) and len(file) <= 512)
                parts.append(
                    {
                        "type": "image",
                        "data": {
                            "file": file.split("/")[-1].split("\\")[-1].split("?")[0]
                        },
                    }
                )
                if any(img["file"] == parts[-1]["data"]["file"] for img in images):
                    continue
                try:
                    info = self.action("get_image", {"file": file})
                    raw = self.download(info.get("url") or data.get("url") or "")
                    images.append(
                        {
                            "file": parts[-1]["data"]["file"],
                            "base64": base64.b64encode(raw).decode(),
                        }
                    )
                except Exception:
                    self.outbox.add(
                        source,
                        "/v1/ingest/coverage",
                        {"source_id": source, "reason": "media_unavailable"},
                    )
            elif typ == "text":
                parts.append(
                    {
                        "type": "text",
                        "data": {"text": self.scrubber.scrub(data.get("text", ""))},
                    }
                )
            elif typ in ("at", "reply"):
                key = "qq" if typ == "at" else "id"
                parts.append({"type": typ, "data": {key: str(data.get(key, ""))[:80]}})
            else:
                parts.append({"type": str(typ)[:32], "data": {}})
        normalized = {
            k: event[k]
            for k in (
                "post_type",
                "message_type",
                "self_id",
                "user_id",
                "message_id",
                "time",
            )
        }
        if "group_id" in event:
            normalized["group_id"] = event["group_id"]
        normalized["message"] = parts
        self.outbox.add(
            source, "/v1/ingest/onebot", {"event": normalized, "images": images}
        )
        self.changed.set()

    def deliver(self):
        backoff = 1
        while not self.closed.is_set():
            self.changed.clear()
            row = self.outbox.next()
            if row is None:
                self.changed.wait()
                continue
            try:
                self.cloud.call("POST", row["path"], json.loads(row["payload"]))
                self.outbox.acknowledge(row["id"])
                backoff = 1
            except Rejected as error:
                if error.status == 403:
                    self.blocked.add(row["source_id"])
                    self.outbox.revoke(row["source_id"])
                elif error.status in (400, 404, 409, 413, 415):
                    self.outbox.acknowledge(row["id"])
                    self.outbox.health("delivery_rejected_gap")
                else:
                    self.closed.wait(backoff)
                    backoff = min(60, backoff * 2)
            except Exception:
                self.closed.wait(backoff)
                backoff = min(60, backoff * 2)

    def consume(self):
        while not self.closed.is_set():
            try:
                event = self.events.get(timeout=1)
            except queue.Empty:
                continue
            try:
                self.capture(event)
            except Exception:
                self.outbox.health("capture_failed_gap")

    def read(self, ws):
        try:
            for raw in ws:
                event = json.loads(raw)
                if "echo" in event:
                    with self.lock:
                        future = self.pending.get(event["echo"])
                    if future and not future.done():
                        future.set_result(event)
                elif self.ready.is_set() and self.source_for(event):
                    try:
                        self.events.put_nowait(event)
                    except queue.Full:
                        self.outbox.health("capture_overflow_gap")
                        ws.close()
                        break
        except Exception:
            pass
        finally:
            self.ready.clear()
            with self.lock:
                for future in self.pending.values():
                    if not future.done():
                        future.set_exception(ConnectionError("observer_disconnected"))

    def run(self):
        # Fail closed before connecting if the server has no matching enabled sources.
        status = self.cloud.call("GET", "/v1/status")
        allowed = {s["id"] for s in status["sources"] if s["enabled"]}
        self.blocked = self.sources - allowed
        require(self.sources - self.blocked, 403, "no_enabled_sources")
        threads = [
            threading.Thread(target=self.deliver, daemon=True),
            threading.Thread(target=self.consume, daemon=True),
        ]
        for thread in threads:
            thread.start()
        backoff = 1
        try:
            while not self.closed.is_set():
                try:
                    with connect(
                        self.config["napcat_endpoint"],
                        additional_headers={
                            "Authorization": "Bearer "
                            + self.credentials["napcat_token"]
                        },
                        proxy=None,
                        max_size=12 * 1024 * 1024,
                        open_timeout=12,
                        close_timeout=3,
                    ) as ws:
                        self.ws = ws
                        reader = threading.Thread(
                            target=self.read, args=(ws,), daemon=True
                        )
                        reader.start()
                        identity = self.action("get_login_info", {})
                        require(
                            str(identity.get("user_id")) == self.account,
                            403,
                            "observer_identity_mismatch",
                        )
                        self.ready.set()
                        self.outbox.health("online_observed_only")
                        backoff = 1
                        for source in self.sources - self.blocked:
                            self.outbox.add(
                                source,
                                "/v1/ingest/coverage",
                                {
                                    "source_id": source,
                                    "reason": "connected_no_backfill",
                                },
                            )
                        self.changed.set()
                        while reader.is_alive() and not self.closed.wait(1):
                            pass
                        ws.close()
                        reader.join(5)
                except Exception:
                    self.outbox.health("disconnected_gap")
                finally:
                    self.ready.clear()
                    self.ws = None
                self.closed.wait(backoff)
                backoff = min(60, backoff * 2)
        finally:
            self.closed.set()
            self.changed.set()
            for thread in threads:
                thread.join(25)


def main():
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("--config", type=Path, required=True)
    args = parser.parse_args()
    config = json.loads(args.config.read_text(encoding="utf-8"))
    credentials = json.loads(
        Path(config["credentials_file"]).read_text(encoding="utf-8")
    )
    observer = Observer(config, credentials)
    try:
        observer.run()
    except KeyboardInterrupt:
        observer.closed.set()
        observer.changed.set()
    except Exception:
        raise SystemExit(
            "Observer stopped; inspect protected outbox health, no raw events logged"
        ) from None


if __name__ == "__main__":
    main()
