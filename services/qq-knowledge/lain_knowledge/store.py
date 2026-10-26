import base64
import hashlib
import io
import json
import re
import sqlite3
import time
from contextlib import contextmanager
from pathlib import Path

from PIL import Image, UnidentifiedImageError

MAX_IMAGE = 8 * 1024 * 1024
Image.MAX_IMAGE_PIXELS = 24_000_000
REMOVED = "[removed]"


class Rejected(Exception):
    def __init__(self, status, code):
        self.status, self.code = status, code


def require(condition, status=400, code="invalid_request"):
    if not condition:
        raise Rejected(status, code)


def now():
    return int(time.time() * 1000)


def encode(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def source_view(row):
    return dict(row) | {"enabled": bool(row["enabled"]), "vision": bool(row["vision"])}


def identifier(value):
    require(isinstance(value, (str, int)) and not isinstance(value, bool))
    value = str(value)
    require(re.fullmatch(r"[1-9][0-9]{0,19}", value) is not None)
    require(int(value) <= 2**64 - 1)
    return value


def external_id(value):
    require(isinstance(value, (str, int)) and not isinstance(value, bool))
    value = str(value)
    require(re.fullmatch(r"-?[0-9]{1,20}", value) is not None)
    return value


class Store:
    def __init__(self, path, secrets=(), vision_cap=10, storage_cap=2 * 1024**3):
        self.path, self.secrets, self.storage_cap = (
            Path(path),
            tuple(secrets),
            storage_cap,
        )
        require(0 <= vision_cap <= 1000, code="invalid_vision_cap")
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with self.db() as db:
            version = db.execute("PRAGMA user_version").fetchone()[0]
            require(version in (0, 1), 409, "unsupported_schema")
            db.executescript("""
                CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
                CREATE TABLE IF NOT EXISTS sources(id TEXT PRIMARY KEY, account TEXT NOT NULL,
                    kind TEXT NOT NULL, peer TEXT NOT NULL, revision INTEGER NOT NULL,
                    enabled INTEGER NOT NULL, retention_ms INTEGER NOT NULL, vision INTEGER NOT NULL);
                CREATE TABLE IF NOT EXISTS messages(id INTEGER PRIMARY KEY AUTOINCREMENT,
                    source_id TEXT NOT NULL REFERENCES sources(id), external_id TEXT NOT NULL,
                    fingerprint TEXT NOT NULL, sender TEXT NOT NULL, occurred_at_ms INTEGER NOT NULL,
                    received_at_ms INTEGER NOT NULL, expires_at_ms INTEGER NOT NULL,
                    state TEXT NOT NULL, text TEXT, segments TEXT, revision INTEGER NOT NULL,
                    UNIQUE(source_id,external_id));
                CREATE INDEX IF NOT EXISTS message_expiry ON messages(state,expires_at_ms);
                CREATE TABLE IF NOT EXISTS images(id TEXT PRIMARY KEY,source_id TEXT NOT NULL REFERENCES sources(id),
                    sha256 TEXT NOT NULL,mime TEXT NOT NULL,width INTEGER NOT NULL,height INTEGER NOT NULL,
                    frames INTEGER NOT NULL,body BLOB,description TEXT,state TEXT NOT NULL,
                    model TEXT,prompt_revision TEXT,UNIQUE(source_id,sha256));
                CREATE TABLE IF NOT EXISTS message_images(message_id INTEGER NOT NULL REFERENCES messages(id),
                    position INTEGER NOT NULL,image_id TEXT NOT NULL REFERENCES images(id),
                    PRIMARY KEY(message_id,position));
                CREATE INDEX IF NOT EXISTS image_message ON message_images(image_id);
                CREATE TABLE IF NOT EXISTS changes(sequence INTEGER PRIMARY KEY AUTOINCREMENT,
                    message_id INTEGER NOT NULL,source_id TEXT NOT NULL,recorded_at_ms INTEGER NOT NULL);
                CREATE TABLE IF NOT EXISTS vision_requests(id TEXT PRIMARY KEY,image_id TEXT NOT NULL,
                    source_revision INTEGER NOT NULL,state TEXT NOT NULL,prepared_at_ms INTEGER NOT NULL,
                    returned_at_ms INTEGER,usage_tokens INTEGER NOT NULL DEFAULT 0,error TEXT);
                CREATE TABLE IF NOT EXISTS coverage(id INTEGER PRIMARY KEY AUTOINCREMENT,source_id TEXT NOT NULL,
                    reason TEXT NOT NULL,observed_at_ms INTEGER NOT NULL);
                PRAGMA user_version=1;
            """)
            db.execute(
                "INSERT OR IGNORE INTO settings VALUES (?,?)",
                ("vision_cap", str(vision_cap)),
            )
            require(
                int(
                    db.execute(
                        "SELECT value FROM settings WHERE key=?", ("vision_cap",)
                    ).fetchone()[0]
                )
                == vision_cap,
                409,
                "budget_spec_changed",
            )
            db.execute(
                "UPDATE vision_requests SET state=? WHERE state=?",
                ("unknown", "attempting"),
            )
            db.execute(
                "UPDATE vision_requests SET state=? WHERE state=?",
                ("not_attempted", "prepared"),
            )
            db.execute(
                "UPDATE images SET state=? WHERE state=?", ("unknown", "processing")
            )
        self.prune()

    @contextmanager
    def db(self):
        db = sqlite3.connect(self.path, timeout=15)
        db.row_factory = sqlite3.Row
        db.execute("PRAGMA foreign_keys=ON")
        db.execute("PRAGMA journal_mode=WAL")
        db.execute("PRAGMA synchronous=FULL")
        try:
            db.execute("BEGIN IMMEDIATE")
            yield db
            db.commit()
        except BaseException:
            db.rollback()
            raise
        finally:
            db.close()

    def scrub(self, text):
        require(isinstance(text, str))
        for secret in self.secrets:
            if secret:
                text = text.replace(secret, "[credential removed]")
        return re.sub(
            r"(?i)(?:sk-[a-z0-9_-]{12,}|bearer\s+[a-z0-9._~+/-]{16,}|(?:api[_ -]?key|password|passwd|token|密钥|密码)\s*[:=：]\s*\S+)",
            "[credential removed]",
            text,
        )

    def source(
        self,
        account,
        kind,
        peer,
        *,
        retention_ms=30 * 86400000,
        vision=False,
        enabled=True,
        expected_revision=0,
    ):
        account, peer = identifier(account), identifier(peer)
        require(kind in ("private", "group"))
        require(
            isinstance(retention_ms, int) and 60000 <= retention_ms <= 31 * 86400000
        )
        require(type(enabled) is bool and type(vision) is bool)
        require(type(expected_revision) is int and expected_revision >= 0)
        key = f"qq:{account}:{kind}:{peer}"
        with self.db() as db:
            row = db.execute("SELECT * FROM sources WHERE id=?", (key,)).fetchone()
            require(
                (row["revision"] if row else 0) == expected_revision,
                409,
                "revision_conflict",
            )
            revision = expected_revision + 1
            db.execute(
                "INSERT INTO sources VALUES (?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,enabled=excluded.enabled,retention_ms=excluded.retention_ms,vision=excluded.vision",
                (key, account, kind, peer, revision, enabled, retention_ms, vision),
            )
            # Disabling is revocation, not just hiding a source. Tombstones survive re-enable.
            if not enabled:
                for m in db.execute(
                    "SELECT id FROM messages WHERE source_id=? AND state=?",
                    (key, "active"),
                ).fetchall():
                    self._remove(db, m["id"], "revoked")
            if not vision:
                db.execute(
                    "UPDATE images SET description=NULL,state='disabled' WHERE source_id=?",
                    (key,),
                )
                for m in db.execute(
                    "SELECT id FROM messages WHERE source_id=? AND state='active'",
                    (key,),
                ).fetchall():
                    self._change(db, m["id"], key)
            return source_view(
                db.execute("SELECT * FROM sources WHERE id=?", (key,)).fetchone()
            )

    @staticmethod
    def _change(db, message, source):
        db.execute(
            "INSERT INTO changes(message_id,source_id,recorded_at_ms) VALUES (?,?,?)",
            (message, source, now()),
        )

    def _remove(self, db, message, state):
        row = db.execute(
            "SELECT source_id FROM messages WHERE id=?", (message,)
        ).fetchone()
        db.execute(
            "UPDATE messages SET state=?,text=NULL,segments=NULL,revision=revision+1 WHERE id=?",
            (state, message),
        )
        db.execute("DELETE FROM message_images WHERE message_id=?", (message,))
        db.execute(
            "UPDATE images SET body=NULL,description=NULL,state='removed' WHERE NOT EXISTS(SELECT 1 FROM message_images l JOIN messages m ON m.id=l.message_id WHERE l.image_id=images.id AND m.state='active')"
        )
        self._change(db, message, row["source_id"])

    def prune(self):
        with self.db() as db:
            for row in db.execute(
                "SELECT id FROM messages WHERE state='active' AND expires_at_ms<=?",
                (now(),),
            ).fetchall():
                self._remove(db, row["id"], "expired")

    def ingest(self, payload):
        require(isinstance(payload, dict) and isinstance(payload.get("event"), dict))
        event = payload["event"]
        require(event.get("post_type") == "message")
        kind = event.get("message_type")
        require(kind in ("private", "group"))
        account = identifier(event.get("self_id"))
        peer = identifier(
            event.get("user_id") if kind == "private" else event.get("group_id")
        )
        source = f"qq:{account}:{kind}:{peer}"
        sender = identifier(event.get("user_id"))
        native = external_id(event.get("message_id"))
        occurred = event.get("time")
        require(
            isinstance(occurred, int)
            and not isinstance(occurred, bool)
            and 0 <= occurred <= now() // 1000 + 300
        )
        parts, supplied = event.get("message"), payload.get("images", [])
        require(
            isinstance(parts, list)
            and 1 <= len(parts) <= 128
            and isinstance(supplied, list)
            and len(supplied) <= 8
        )
        images = {}
        for item in supplied:
            require(
                isinstance(item, dict)
                and isinstance(item.get("file"), str)
                and len(item["file"]) <= 512
            )
            require(item["file"] not in images)
            data = item.get("base64")
            require(isinstance(data, str) and len(data) <= (MAX_IMAGE * 4 // 3 + 8))
            try:
                raw = base64.b64decode(data, validate=True)
                require(0 < len(raw) <= MAX_IMAGE, 413, "image_too_large")
                with Image.open(io.BytesIO(raw)) as image:
                    require(
                        image.format in ("JPEG", "PNG", "GIF", "WEBP"),
                        415,
                        "unsupported_image",
                    )
                    require(
                        image.width * image.height <= Image.MAX_IMAGE_PIXELS,
                        413,
                        "image_dimensions",
                    )
                    fmt, w, h, frames = (
                        image.format,
                        image.width,
                        image.height,
                        getattr(image, "n_frames", 1),
                    )
                    image.verify()
                images[item["file"]] = (raw, fmt, w, h, frames)
            except (
                ValueError,
                UnidentifiedImageError,
                OSError,
                Image.DecompressionBombError,
            ):
                raise Rejected(415, "invalid_image") from None
        segments, text, used = [], [], set()
        for part in parts:
            require(isinstance(part, dict) and isinstance(part.get("data"), dict))
            data, typ = part["data"], part.get("type")
            if typ == "text":
                body = data.get("text")
                require(isinstance(body, str) and len(body.encode()) <= 16384)
                body = self.scrub(body)
                segments.append({"type": "text", "text": body})
                text.append(body)
            elif typ == "image":
                file = data.get("file")
                require(isinstance(file, str) and len(file) <= 512)
                used.add(file)
                # URLs and local paths are never persisted or fetched by this server.
                segments.append(
                    {
                        "type": "image",
                        "file": self.scrub(file.split("/")[-1].split("\\")[-1])[:160],
                        "media_state": "supplied" if file in images else "unavailable",
                    }
                )
            elif typ in ("at", "reply"):
                value = data.get("qq") if typ == "at" else data.get("id")
                require(isinstance(value, (str, int)))
                segments.append({"type": typ, "reference": self.scrub(str(value))[:80]})
            else:
                require(isinstance(typ, str) and len(typ) <= 32)
                segments.append({"type": "unsupported", "native_type": typ})
        require(not images.keys() - used and len(used) <= 8)
        # Identity covers normalized content and media hashes; URL rotation is irrelevant.
        media_fingerprints = {
            key: hashlib.sha256(v[0]).hexdigest() for key, v in images.items()
        }
        fingerprint = hashlib.sha256(
            encode([sender, occurred, segments, media_fingerprints]).encode()
        ).hexdigest()
        self.prune()
        with self.db() as db:
            grant = db.execute("SELECT * FROM sources WHERE id=?", (source,)).fetchone()
            require(grant and grant["enabled"], 403, "source_not_authorized")
            old = db.execute(
                "SELECT * FROM messages WHERE source_id=? AND external_id=?",
                (source, native),
            ).fetchone()
            if old:
                require(
                    old["fingerprint"] == fingerprint, 409, "event_identity_conflict"
                )
                return {"id": old["id"], "duplicate": True, "state": old["state"]}
            total = db.execute(
                "SELECT COALESCE(sum(length(body)),0) FROM images"
            ).fetchone()[0]
            count = db.execute("SELECT count(*) FROM messages").fetchone()[0]
            extra = 0
            seen = set()
            for raw, *_ in images.values():
                digest = hashlib.sha256(raw).hexdigest()
                if digest in seen:
                    continue
                seen.add(digest)
                cached = db.execute(
                    "SELECT body IS NOT NULL FROM images WHERE source_id=? AND sha256=?",
                    (source, digest),
                ).fetchone()
                if cached is None or not cached[0]:
                    extra += len(raw)
            require(
                total + extra <= self.storage_cap and count < 100000,
                507,
                "storage_limit",
            )
            received = now()
            cursor = db.execute(
                "INSERT INTO messages(source_id,external_id,fingerprint,sender,occurred_at_ms,received_at_ms,expires_at_ms,state,text,segments,revision) VALUES (?,?,?,?,?,?,?,?,?,?,1)",
                (
                    source,
                    native,
                    fingerprint,
                    sender,
                    occurred * 1000,
                    received,
                    received + grant["retention_ms"],
                    "active",
                    "".join(text),
                    encode(segments),
                ),
            )
            mid = cursor.lastrowid
            for position, part in enumerate(parts):
                if part["type"] != "image" or part["data"]["file"] not in images:
                    continue
                raw, fmt, w, h, frames = images[part["data"]["file"]]
                sha = hashlib.sha256(raw).hexdigest()
                iid = hashlib.sha256((source + ":" + sha).encode()).hexdigest()
                state = "pending" if grant["vision"] else "disabled"
                db.execute(
                    "INSERT OR IGNORE INTO images VALUES (?,?,?,?,?,?,?,?,NULL,?,NULL,NULL)",
                    (iid, source, sha, Image.MIME[fmt], w, h, frames, raw, state),
                )
                # A removed asset can be re-observed in a genuinely new message.
                db.execute(
                    "UPDATE images SET body=?,state=? WHERE id=? AND body IS NULL",
                    (raw, state, iid),
                )
                db.execute(
                    "INSERT INTO message_images VALUES (?,?,?)", (mid, position, iid)
                )
            self._change(db, mid, source)
            return {"id": mid, "duplicate": False, "state": "active"}

    def recall(self, source, native):
        native = external_id(native)
        with self.db() as db:
            grant = db.execute(
                "SELECT enabled FROM sources WHERE id=?", (source,)
            ).fetchone()
            require(grant and grant["enabled"], 403, "source_not_authorized")
            row = db.execute(
                "SELECT * FROM messages WHERE source_id=? AND external_id=?",
                (source, native),
            ).fetchone()
            require(row is not None, 404, "message_not_found")
            if row["state"] == "active":
                self._remove(db, row["id"], "recalled")
            return {
                "id": row["id"],
                "state": "recalled" if row["state"] == "active" else row["state"],
            }

    def forget(self, message):
        with self.db() as db:
            row = db.execute(
                "SELECT state FROM messages WHERE id=?", (message,)
            ).fetchone()
            require(row is not None, 404, "message_not_found")
            if row["state"] == "active":
                self._remove(db, message, "forgotten")
            return {
                "id": message,
                "state": "forgotten" if row["state"] == "active" else row["state"],
            }

    def _message(self, db, row):
        value = {
            k: row[k]
            for k in (
                "id",
                "source_id",
                "external_id",
                "sender",
                "occurred_at_ms",
                "received_at_ms",
                "expires_at_ms",
                "state",
                "revision",
            )
        }
        active = row["state"] == "active" and row["expires_at_ms"] > now()

        def safe(value):
            if isinstance(value, str):
                return self.scrub(value)
            if isinstance(value, list):
                return [safe(item) for item in value]
            if isinstance(value, dict):
                return {key: safe(item) for key, item in value.items()}
            return value

        value.update(
            text=self.scrub(row["text"] or "") if active else None,
            segments=safe(json.loads(row["segments"])) if active else None,
            images=[],
        )
        if active:
            for img in db.execute(
                "SELECT i.*,l.position FROM images i JOIN message_images l ON l.image_id=i.id WHERE l.message_id=? ORDER BY l.position",
                (row["id"],),
            ):
                value["images"].append(
                    {
                        k: img[k]
                        for k in (
                            "id",
                            "sha256",
                            "mime",
                            "width",
                            "height",
                            "frames",
                            "state",
                            "model",
                            "prompt_revision",
                            "position",
                        )
                    }
                    | {
                        "description": self.scrub(img["description"])
                        if img["description"]
                        else None
                    }
                )
        return value

    def messages(self, source=None, text="", after=0, limit=50):
        self.prune()
        require(0 <= after and 1 <= limit <= 100 and len(text) <= 256)
        escaped = text.replace("\\", "\\\\").replace("%", "\\%").replace("_", "\\_")
        with self.db() as db:
            rows = db.execute(
                "SELECT m.* FROM messages m JOIN sources s ON s.id=m.source_id WHERE s.enabled=1 AND m.state='active' AND m.id>? AND (? IS NULL OR m.source_id=?) AND (?='' OR m.text LIKE ? ESCAPE '\\' OR EXISTS(SELECT 1 FROM message_images l JOIN images i ON i.id=l.image_id WHERE l.message_id=m.id AND i.description LIKE ? ESCAPE '\\')) ORDER BY m.id LIMIT ?",
                (
                    after,
                    source,
                    source,
                    text,
                    "%" + escaped + "%",
                    "%" + escaped + "%",
                    limit + 1,
                ),
            ).fetchall()
            return {
                "items": [self._message(db, r) for r in rows[:limit]],
                "has_more": len(rows) > limit,
                "next_after": rows[limit - 1]["id"]
                if len(rows) >= limit
                else (rows[-1]["id"] if rows else after),
                "coverage": "observed_only_no_history_backfill",
                "search": "literal_text_and_model_description",
            }

    def changes(self, after=0, limit=100):
        self.prune()
        require(after >= 0 and 1 <= limit <= 100)
        with self.db() as db:
            rows = db.execute(
                "SELECT * FROM changes WHERE sequence>? ORDER BY sequence LIMIT ?",
                (after, limit + 1),
            ).fetchall()
            items = []
            for row in rows[:limit]:
                message = db.execute(
                    "SELECT * FROM messages WHERE id=?", (row["message_id"],)
                ).fetchone()
                items.append(
                    {
                        "sequence": row["sequence"],
                        "observed_at_ms": row["recorded_at_ms"],
                        "message": self._message(db, message),
                    }
                )
            return {
                "items": items,
                "has_more": len(rows) > limit,
                "next_after": items[-1]["sequence"] if items else after,
            }

    def image(self, iid):
        self.prune()
        with self.db() as db:
            row = db.execute(
                "SELECT i.mime,i.body FROM images i JOIN sources s ON s.id=i.source_id WHERE i.id=? AND s.enabled=1 AND i.body IS NOT NULL AND EXISTS(SELECT 1 FROM message_images l JOIN messages m ON m.id=l.message_id WHERE l.image_id=i.id AND m.state='active' AND m.expires_at_ms>?)",
                (iid, now()),
            ).fetchone()
            require(row is not None, 404, "image_not_found")
            return row["mime"], row["body"]

    def status(self):
        self.prune()
        with self.db() as db:
            return {
                "schema_version": 1,
                "service": "lain-knowledge",
                "sources": [
                    source_view(r) for r in db.execute("SELECT * FROM sources")
                ],
                "vision": {
                    "max_requests": int(
                        db.execute(
                            "SELECT value FROM settings WHERE key='vision_cap'"
                        ).fetchone()[0]
                    ),
                    "reserved_requests": db.execute(
                        "SELECT count(*) FROM vision_requests"
                    ).fetchone()[0],
                    "reported_usage_tokens": db.execute(
                        "SELECT COALESCE(sum(usage_tokens),0) FROM vision_requests"
                    ).fetchone()[0],
                },
                "active_messages": db.execute(
                    "SELECT count(*) FROM messages WHERE state='active'"
                ).fetchone()[0],
                "image_states": {
                    r["state"]: r["n"]
                    for r in db.execute(
                        "SELECT state,count(*) n FROM images GROUP BY state"
                    )
                },
                "recent_coverage": [
                    dict(r)
                    for r in db.execute(
                        "SELECT source_id,reason,observed_at_ms FROM coverage ORDER BY id DESC LIMIT 20"
                    )
                ],
                "coverage": "only_selected_new_events; reconnects_have_gaps; no_hidden_reasoning",
            }
