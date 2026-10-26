import json
import tempfile
import threading
import time
import unittest
from unittest.mock import patch
from pathlib import Path

from lain_knowledge.connector import Observer, Outbox, public_image
from lain_knowledge.store import Rejected
from test_service import event, picture
from websockets.sync.server import serve


class ConnectorTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.config = {
            "self_id": "1001",
            "private_peers": ["2002"],
            "groups": [],
            "napcat_endpoint": "ws://127.0.0.1:3001",
            "api_url": "https://example.org/knowledge",
            "outbox_path": str(Path(self.tmp.name) / "outbox.sqlite3"),
        }
        self.creds = {
            "ingest_token": "fixture-ingest-" + "x" * 32,
            "napcat_token": "fixture-napcat-" + "x" * 32,
        }

    def tearDown(self):
        self.tmp.cleanup()

    def test_only_selected_messages_and_only_read_actions(self):
        obs = Observer(
            self.config, self.creds, cloud=object(), download=lambda _: picture()
        )
        calls = []

        def action(name, params):
            calls.append(name)
            return {"url": "https://gchat.qpic.cn/test"}

        obs.action = action
        payload = event()["event"]
        obs.capture(payload)
        row = obs.outbox.next()
        saved = json.loads(row["payload"])
        self.assertEqual(calls, ["get_image"])
        self.assertEqual(len(saved["images"]), 1)
        self.assertNotIn("url", saved["event"]["message"][1]["data"])
        obs.outbox.acknowledge(row["id"])
        self.assertIsNone(obs.outbox.next())
        payload["user_id"] = 3003
        obs.capture(payload)
        self.assertIsNone(obs.outbox.next())
        with self.assertRaises(Rejected):
            Observer(self.config, self.creds, cloud=object()).action(
                "send_private_msg", {}
            )

    def test_media_failure_still_preserves_original_message_and_gap(self):
        obs = Observer(self.config, self.creds, cloud=object())
        obs.action = lambda *_: (_ for _ in ()).throw(TimeoutError())
        obs.capture(event()["event"])
        gap = obs.outbox.next()
        self.assertEqual(json.loads(gap["payload"])["reason"], "media_unavailable")
        obs.outbox.acknowledge(gap["id"])
        message = obs.outbox.next()
        self.assertEqual(json.loads(message["payload"])["images"], [])
        self.assertEqual(
            json.loads(message["payload"])["event"]["message"][1]["type"], "image"
        )

    def test_durable_outbox_acknowledgement_and_revoke(self):
        box = Outbox(self.config["outbox_path"])
        box.add("source", "/v1/ingest/onebot", {"synthetic": True})
        again = Outbox(self.config["outbox_path"])
        self.assertEqual(json.loads(again.next()["payload"]), {"synthetic": True})
        again.revoke("source")
        self.assertIsNone(box.next())

    def test_unsafe_image_locations_and_nonlocal_plain_ws_rejected(self):
        for url in (
            "file:///etc/passwd",
            "http://example.org/test",
            "https://169.254.169.254/test",
            "https://gchat.qpic.cn.evil.example/a",
            "https://x:secret@gchat.qpic.cn/a",
        ):
            with self.assertRaises(Rejected):
                public_image(url)
        self.config["napcat_endpoint"] = "ws://192.168.1.2:3001"
        with self.assertRaises(Rejected):
            Observer(self.config, self.creds, cloud=object())
        with (
            patch(
                "socket.getaddrinfo", return_value=[(2, 1, 6, "", ("127.0.0.1", 443))]
            ),
            patch("socket.create_connection") as dial,
        ):
            with self.assertRaises(Rejected):
                public_image("https://gchat.qpic.cn/rebound")
            dial.assert_not_called()

    def test_real_websocket_identity_read_action_and_durable_cloud_delivery(self):
        delivered = threading.Event()
        calls = []
        received = []

        class Cloud:
            def call(self, method, path, value=None):
                if method == "GET":
                    return {"sources": [{"id": "qq:1001:private:2002", "enabled": 1}]}
                if path == "/v1/ingest/onebot":
                    received.append(value)
                    delivered.set()
                return {"ok": True}

        def onebot(ws):
            while True:
                try:
                    request = json.loads(ws.recv(timeout=5))
                except Exception:
                    break
                calls.append(request["action"])
                self.assertIn(request["action"], ("get_login_info", "get_image"))
                data = (
                    {"user_id": 1001}
                    if request["action"] == "get_login_info"
                    else {"url": "https://gchat.qpic.cn/fixture"}
                )
                ws.send(
                    json.dumps(
                        {
                            "status": "ok",
                            "retcode": 0,
                            "data": data,
                            "echo": request["echo"],
                        }
                    )
                )
                if request["action"] == "get_login_info":
                    time.sleep(0.1)
                    ws.send(json.dumps(event()["event"]))

        server = serve(onebot, "127.0.0.1", 0)
        server_thread = threading.Thread(target=server.serve_forever)
        server_thread.start()
        self.config["napcat_endpoint"] = (
            f"ws://127.0.0.1:{server.socket.getsockname()[1]}"
        )
        obs = Observer(
            self.config, self.creds, cloud=Cloud(), download=lambda _: picture()
        )
        thread = threading.Thread(target=obs.run)
        thread.start()
        try:
            self.assertTrue(delivered.wait(10))
            self.assertEqual(len(received[0]["images"]), 1)
            self.assertEqual(calls, ["get_login_info", "get_image"])
        finally:
            obs.closed.set()
            obs.changed.set()
            if obs.ws:
                obs.ws.close()
            thread.join(10)
            server.shutdown()
            server_thread.join(5)
        self.assertFalse(thread.is_alive())
        self.assertFalse(server_thread.is_alive())


if __name__ == "__main__":
    unittest.main()
