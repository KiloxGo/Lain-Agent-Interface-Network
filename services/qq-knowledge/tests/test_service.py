import base64
import io
import json
import tempfile
import threading
import time
import unittest
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from PIL import Image

from lain_knowledge.api import Application
from lain_knowledge.store import Rejected, Store, now
from lain_knowledge.vision import Worker


def picture():
    out = io.BytesIO()
    Image.new("RGB", (120, 80), "red").save(out, format="PNG")
    return out.getvalue()


def event(native=1, image=True, text="测试：周五去图书馆"):
    parts = [{"type": "text", "data": {"text": text}}]
    if image:
        parts.append(
            {
                "type": "image",
                "data": {
                    "file": "fixture.png",
                    "url": "http://169.254.169.254/private?token=secret",
                },
            }
        )
    return {
        "event": {
            "post_type": "message",
            "message_type": "private",
            "self_id": 1001,
            "user_id": 2002,
            "message_id": native,
            "time": int(time.time()),
            "message": parts,
        },
        "images": [
            {"file": "fixture.png", "base64": base64.b64encode(picture()).decode()}
        ]
        if image
        else [],
    }


class ServiceTests(unittest.TestCase):
    def test_scoped_changes_bind_persistent_store_source_and_current_tombstones(self):
        self.store.ingest(event(1, image=False))
        other = self.store.source("1001", "private", "3003")["id"]
        data = event(2, image=False, text="other private source")
        data["event"]["user_id"] = 3003
        self.store.ingest(data)
        page = self.store.changes(source=self.source, limit=1)
        self.assertEqual(len(page["items"]), 1)
        self.assertEqual(page["source"]["id"], self.source)
        self.assertNotIn(other, json.dumps(page["items"]))
        self.assertFalse(page["has_more"])
        self.assertEqual(page["through"], page["next_after"])
        self.assertEqual(page["store_id"], Store(self.path, vision_cap=2).status()["store_id"])
        status, _, body = self.call("GET", "/v1/changes", role="read", query=f"source_id={self.source}&after=0&limit=1")
        self.assertEqual(status, 200)
        self.assertEqual(json.loads(body)["store_id"], page["store_id"])
        status, _, body = self.call("GET", f"/v1/sources/{self.source}/changes", role="read", query="after=0&limit=1")
        self.assertEqual(status, 200)
        self.assertNotIn(other, json.dumps(json.loads(body)["items"]))
        self.assertEqual(self.call("GET", f"/v1/sources/{self.source}/changes", role="read", query=f"source_id={other}")[0],400)
        self.assertEqual(self.call("GET", "/v1/changes", role="read", query=f"source_id={self.source}&after=999")[0], 409)
        self.store.source("1001", "private", "2002", enabled=False, expected_revision=1)
        tombstone = self.store.changes(source=self.source, after=page["next_after"])
        self.assertFalse(tombstone["source"]["enabled"])
        self.assertEqual(tombstone["items"][0]["message"]["state"], "revoked")
        self.assertIsNone(tombstone["items"][0]["message"]["text"])

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.path = Path(self.tmp.name) / "state.db"
        self.store = Store(self.path, vision_cap=2, secrets=["private-secret-fixture"])
        self.source = self.store.source("1001", "private", "2002", vision=True)["id"]
        self.tokens = {
            k: k + "-test-key-" + "x" * 40 for k in ("admin", "read", "ingest")
        }
        self.app = Application(self.store, self.tokens)

    def tearDown(self):
        self.tmp.cleanup()

    def call(self, method, path, payload=None, role="admin", query=""):
        body = json.dumps(payload).encode() if payload is not None else b""
        env = {
            "REQUEST_METHOD": method,
            "PATH_INFO": path,
            "QUERY_STRING": query,
            "CONTENT_LENGTH": str(len(body)),
            "CONTENT_TYPE": "application/json",
            "wsgi.input": io.BytesIO(body),
            "HTTP_AUTHORIZATION": "Bearer " + self.tokens.get(role, "bad"),
        }
        headers = []
        result = b"".join(
            self.app(env, lambda status, values: headers.append((status, dict(values))))
        )
        return int(headers[0][0].split()[0]), headers[0][1], result

    def test_scope_normalization_image_provenance_search_and_url_is_not_retained(self):
        item = self.store.ingest(event())
        self.assertFalse(item["duplicate"])
        msg = self.store.messages()["items"][0]
        self.assertEqual(msg["source_id"], self.source)
        image = msg["images"][0]
        self.assertEqual(image["width"], 120)
        self.assertEqual(image["frames"], 1)
        self.assertEqual(self.store.image(image["id"])[1], picture())
        self.assertEqual(len(self.store.messages(text="图书馆")["items"]), 1)
        self.assertFalse(self.store.messages(text="%")["items"])
        with self.store.db() as db:
            snapshot = "".join(
                str(tuple(r)) for r in db.execute("SELECT * FROM messages")
            )
        self.assertNotIn("169.254", snapshot)
        self.assertNotIn("token=secret", snapshot)
        other = event(2)
        other["event"]["user_id"] = 3003
        with self.assertRaises(Rejected) as error:
            self.store.ingest(other)
        self.assertEqual(error.exception.status, 403)

    def test_parallel_duplicate_identity_conflict_and_tombstone_no_resurrection(self):
        small = Store(Path(self.tmp.name) / "small.db", storage_cap=len(picture()))
        small.source("1001", "private", "2002")
        small.ingest(event(41))
        # A cache hit at the exact byte ceiling consumes no additional storage.
        small.ingest(event(42))
        self.assertEqual(len(small.messages()["items"]), 2)
        payload = event()
        with ThreadPoolExecutor(max_workers=4) as executor:
            values = list(executor.map(self.store.ingest, [payload] * 8))
        self.assertEqual(sum(not v["duplicate"] for v in values), 1)
        conflict = event()
        conflict["event"]["message"][0]["data"]["text"] = "different"
        with self.assertRaises(Rejected):
            self.store.ingest(conflict)
        image = self.store.messages()["items"][0]["images"][0]["id"]
        self.store.forget(values[0]["id"])
        self.assertEqual(self.store.ingest(payload)["state"], "forgotten")
        self.assertFalse(self.store.messages()["items"])
        with self.assertRaises(Rejected):
            self.store.image(image)
        changes = self.store.changes()["items"]
        self.assertTrue(all(c["message"]["text"] is None for c in changes))

    def test_source_isolation_revoke_cas_and_same_image_different_source(self):
        payload = event()
        self.store.ingest(payload)
        self.store.source("1001", "private", "3003", vision=False)
        second = event(2)
        second["event"]["user_id"] = 3003
        self.store.ingest(second)
        rows = self.store.messages()["items"]
        self.assertNotEqual(rows[0]["images"][0]["id"], rows[1]["images"][0]["id"])
        with self.assertRaises(Rejected):
            self.store.source(
                "1001", "private", "2002", expected_revision=0, enabled=False
            )
        self.store.source("1001", "private", "2002", expected_revision=1, enabled=False)
        self.assertEqual(len(self.store.messages()["items"]), 1)
        self.store.source("1001", "private", "2002", expected_revision=2, vision=True)
        self.assertEqual(self.store.ingest(payload)["state"], "revoked")

    def test_auth_roles_limits_validation_and_known_credentials(self):
        self.assertEqual(self.call("GET", "/v1/status", role="unknown")[0], 401)
        self.assertEqual(
            self.call("POST", "/v1/ingest/onebot", event(), role="read")[0], 403
        )
        self.assertEqual(self.call("POST", "/v1/sources", {}, role="ingest")[0], 403)
        status, headers, _ = self.call(
            "POST",
            "/v1/ingest/onebot",
            event(image=False, text="private-secret-fixture sk-0123456789abcdef"),
            role="ingest",
        )
        self.assertEqual(status, 200)
        self.assertEqual(headers["Cache-Control"], "no-store")
        self.assertEqual(self.call("GET", "/v1/messages", role="ingest")[0], 403)
        body = self.call("GET", "/v1/messages", role="read")[2].decode()
        self.assertNotIn("private-secret-fixture", body)
        self.assertNotIn("sk-0123456789abcdef", body)
        self.assertEqual(self.call("GET", "/v1/messages", query="after=-1")[0], 400)
        bad = event()
        bad["images"][0]["base64"] = "not an image"
        self.assertEqual(self.call("POST", "/v1/ingest/onebot", bad)[0], 415)

    def test_vision_cache_durable_cap_restart_unknown_and_no_retry(self):
        self.store.ingest(event())
        calls = []

        def provider(raw, key):
            calls.append(raw)
            return "红色的测试图片", 11

        worker = Worker(self.store, "fixture-key", provider)
        self.assertTrue(worker.one())
        self.assertFalse(worker.one())
        self.store.ingest(event(2))
        self.assertFalse(worker.one())
        self.assertEqual(len(calls), 1)
        self.assertEqual(len(self.store.messages(text="红色")["items"]), 2)
        different = event(3)
        png = io.BytesIO()
        Image.new("RGB", (5, 6), "blue").save(png, format="PNG")
        different["images"][0]["base64"] = base64.b64encode(png.getvalue()).decode()
        self.store.ingest(different)

        def failed(raw, key):
            raise TimeoutError("provider may have received this")

        self.assertTrue(Worker(self.store, "fixture-key", failed).one())
        restarted = Store(self.path, vision_cap=2)
        self.assertFalse(Worker(restarted, "fixture-key", provider).one())
        self.assertEqual(restarted.status()["vision"]["reserved_requests"], 2)
        self.assertEqual(restarted.status()["vision"]["reported_usage_tokens"], 11)
        with self.assertRaises(Rejected):
            Store(self.path, vision_cap=3)

    def test_revoke_while_provider_runs_discards_description_but_counts_call(self):
        self.store.ingest(event())
        entered, release = threading.Event(), threading.Event()

        def provider(raw, key):
            entered.set()
            release.wait(5)
            return "must never persist after revocation", 7

        worker = Worker(self.store, "fixture-key", provider)
        thread = threading.Thread(target=worker.one)
        thread.start()
        self.assertTrue(entered.wait(5))
        self.store.source("1001", "private", "2002", enabled=False, expected_revision=1)
        release.set()
        thread.join(5)
        self.assertFalse(thread.is_alive())
        with self.store.db() as db:
            self.assertIsNone(
                db.execute("SELECT description FROM images").fetchone()[0]
            )
            self.assertIsNone(db.execute("SELECT body FROM images").fetchone()[0])
        self.assertEqual(self.store.status()["vision"]["reserved_requests"], 1)

    def test_expiry_purges_bytes_descriptions_and_projects_tombstones(self):
        self.store.ingest(event())
        Worker(self.store, "fixture", lambda raw, key: ("old description", 1)).one()
        with self.store.db() as db:
            db.execute("UPDATE messages SET expires_at_ms=?", (now() - 1,))
        self.assertFalse(self.store.messages()["items"])
        with self.store.db() as db:
            img = db.execute("SELECT body,description FROM images").fetchone()
            self.assertEqual(tuple(img), (None, None))
        self.assertTrue(
            all(
                c["message"]["state"] == "expired" and c["message"]["text"] is None
                for c in self.store.changes()["items"]
            )
        )

    def test_inflight_restart_is_unknown_without_provider_replay(self):
        self.store.ingest(event())
        with self.store.db() as db:
            iid = db.execute("SELECT id FROM images").fetchone()[0]
            db.execute("UPDATE images SET state='processing'")
            db.execute(
                "INSERT INTO vision_requests(id,image_id,source_revision,state,prepared_at_ms) VALUES ('interrupted',?,1,'attempting',?)",
                (iid, now()),
            )
        reopened = Store(self.path, vision_cap=2)
        self.assertFalse(
            Worker(reopened, "fixture", lambda *_: self.fail("must not replay")).one()
        )
        with reopened.db() as db:
            self.assertEqual(
                db.execute("SELECT state FROM vision_requests").fetchone()[0], "unknown"
            )


if __name__ == "__main__":
    unittest.main()
