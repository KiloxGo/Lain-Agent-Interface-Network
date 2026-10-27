import hmac
import json
import os
import threading
import time
from collections import defaultdict, deque
from urllib.parse import parse_qs

from .store import Rejected, Store, encode, require
from .vision import Worker

MAX_BODY = 12 * 1024 * 1024


class Application:
    def __init__(self, store, tokens, worker=None):
        require(
            set(tokens) == {"admin", "ingest", "read"}
            and len(set(tokens.values())) == 3
            and all(isinstance(v, str) and len(v) >= 32 for v in tokens.values()),
            code="invalid_api_keys",
        )
        self.store, self.tokens, self.worker = store, tokens, worker
        self.lock, self.rates = threading.Lock(), defaultdict(deque)

    def __call__(self, environ, start_response):
        try:
            authorization = environ.get("HTTP_AUTHORIZATION", "")
            role = next(
                (
                    k
                    for k, v in self.tokens.items()
                    if hmac.compare_digest(
                        authorization.encode(), ("Bearer " + v).encode()
                    )
                ),
                None,
            )
            require(role is not None, 401, "unauthorized")
            with self.lock:
                entries = self.rates[role]
                stamp = time.monotonic()
                while entries and stamp - entries[0] >= 60:
                    entries.popleft()
                require(len(entries) < 120, 429, "rate_limited")
                entries.append(stamp)
            method, path = environ["REQUEST_METHOD"], environ.get("PATH_INFO", "")
            if method == "GET" and path != "/v1/status":
                require(role in ("admin", "read"), 403, "read_key_required")
            query = parse_qs(environ.get("QUERY_STRING", ""), max_num_fields=12)

            def arg(name, default=None):
                values = query.get(name, [default])
                require(len(values) == 1)
                return values[0]

            def number(name, default):
                value = arg(name, str(default))
                require(value.isdecimal() and len(value) < 12)
                return int(value)

            value = None
            if method in ("POST", "PUT"):
                raw_length = environ.get("CONTENT_LENGTH", "")
                require(raw_length.isdecimal(), 411, "content_length_required")
                size = int(raw_length)
                require(0 < size <= MAX_BODY, 413, "body_too_large")
                require(
                    environ.get("CONTENT_TYPE", "").split(";")[0] == "application/json",
                    415,
                    "json_required",
                )
                raw = environ["wsgi.input"].read(size)
                require(len(raw) == size)
                value = json.loads(raw)
                require(isinstance(value, dict))
            if method == "GET" and path == "/v1/status":
                result = self.store.status()
            elif method == "GET" and path == "/v1/messages":
                result = self.store.messages(
                    arg("source_id"),
                    arg("q", ""),
                    number("after", 0),
                    number("limit", 50),
                )
            elif method == "GET" and path == "/v1/changes":
                result = self.store.changes(number("after", 0), number("limit", 100), arg("source_id"))
            elif method == "GET" and path.startswith("/v1/sources/") and path.endswith("/changes"):
                source = path[len("/v1/sources/"):-len("/changes")]
                require(arg("source_id", source) == source)
                result = self.store.changes(number("after", 0), number("limit", 100), source)
            elif method == "GET" and path.startswith("/v1/images/"):
                iid = path.removeprefix("/v1/images/")
                require(len(iid) == 64 and all(c in "0123456789abcdef" for c in iid))
                mime, body = self.store.image(iid)
                start_response(
                    "200 OK",
                    [
                        ("Content-Type", mime),
                        ("Content-Length", str(len(body))),
                        ("Cache-Control", "no-store"),
                        ("X-Content-Type-Options", "nosniff"),
                    ],
                )
                return [body]
            elif method == "POST" and path == "/v1/ingest/onebot":
                require(role in ("admin", "ingest"), 403, "read_only_key")
                result = self.store.ingest(value)
            elif method == "POST" and path == "/v1/ingest/recall":
                require(role in ("admin", "ingest"), 403, "read_only_key")
                result = self.store.recall(
                    value.get("source_id"), value.get("external_id")
                )
            elif method == "POST" and path == "/v1/sources":
                require(role == "admin", 403, "admin_key_required")
                require(
                    not value.keys()
                    - {
                        "account",
                        "kind",
                        "peer",
                        "retention_ms",
                        "vision",
                        "enabled",
                        "expected_revision",
                    }
                )
                result = self.store.source(**value)
            elif (
                method == "POST"
                and path.startswith("/v1/messages/")
                and path.endswith("/forget")
            ):
                require(role == "admin", 403, "admin_key_required")
                mid = path.split("/")[-2]
                require(mid.isdecimal() and len(mid) < 12)
                result = self.store.forget(int(mid))
            elif method == "POST" and path == "/v1/ingest/coverage":
                require(role in ("admin", "ingest"), 403, "read_only_key")
                require(
                    value.get("reason")
                    in (
                        "connected_no_backfill",
                        "disconnected_gap",
                        "media_unavailable",
                        "outbox_delivery_failed",
                    )
                )
                with self.store.db() as db:
                    source = db.execute(
                        "SELECT enabled FROM sources WHERE id=?",
                        (value.get("source_id"),),
                    ).fetchone()
                    require(source and source["enabled"], 403, "source_not_authorized")
                    db.execute(
                        "INSERT INTO coverage(source_id,reason,observed_at_ms) VALUES (?,?,?)",
                        (value["source_id"], value["reason"], int(time.time() * 1000)),
                    )
                    db.execute(
                        "DELETE FROM coverage WHERE id NOT IN (SELECT id FROM coverage ORDER BY id DESC LIMIT 1000)"
                    )
                result = {"recorded": True}
            else:
                raise Rejected(404, "not_found")
            if self.worker and method == "POST":
                self.worker.wake()
            data = encode(result).encode()
            status = "200 OK"
        except Rejected as error:
            status = f"{error.status} Error"
            data = encode({"error": error.code}).encode()
        except (ValueError, TypeError, KeyError, UnicodeError, RecursionError):
            status = "400 Error"
            data = b'{"error":"invalid_request"}'
        except Exception:
            status = "503 Error"
            data = b'{"error":"service_unavailable"}'
        start_response(
            status,
            [
                ("Content-Type", "application/json; charset=utf-8"),
                ("Content-Length", str(len(data))),
                ("Cache-Control", "no-store"),
                ("X-Content-Type-Options", "nosniff"),
            ],
        )
        return [data]


def create():
    tokens = {
        role: os.environ["KNOWLEDGE_" + role.upper() + "_TOKEN"]
        for role in ("admin", "ingest", "read")
    }
    secret = os.environ.get("KNOWLEDGE_DEEPSEEK_KEY", "")
    store = Store(
        os.environ.get("KNOWLEDGE_DATABASE", "/data/knowledge.sqlite3"),
        [*tokens.values(), secret],
        vision_cap=int(os.environ.get("KNOWLEDGE_VISION_CAP", "10")),
    )
    worker = Worker(store, secret)
    app = Application(store, tokens, worker)
    worker.start()
    return app
