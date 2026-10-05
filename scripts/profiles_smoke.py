"""Deterministic Phase 5 API/CLI/restart/credential-rotation acceptance. No model calls."""
from contextlib import closing
import http.client
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import time
from urllib.parse import quote
from codex_smoke import Daemon


def run():
    repository = Path(__file__).resolve().parents[1]
    binaries = Path(os.environ.get("LAIN_TEST_BIN_DIR", repository / "target/debug"))
    suffix = ".exe" if os.name == "nt" else ""
    with tempfile.TemporaryDirectory(prefix="lain-profiles-") as folder:
        root = Path(folder)
        binary = root / ("laind" + suffix)
        shutil.copy2(binaries / binary.name, binary)
        workspace = root / "rotation"
        workspace.mkdir()
        daemon = Daemon(root, binary, idle_timeout=1, extra_env={
            "LAIN_CODEX_PATH": str(binaries / ("lain-test-app-server" + suffix)),
            "LAIN_SECRET_ROTATION": "fixture-rotation-old",
        })
        streams = []
        forbidden = ["fixture-rotation-old", "fixture-rotation-new"]
        def rejected(method, path, body, status):
            conn = http.client.HTTPConnection("127.0.0.1", daemon.port, timeout=10)
            try:
                conn.request(method, path, json.dumps(body), {"Authorization": "Bearer " + daemon.token, "Content-Type": "application/json"})
                response = conn.getresponse()
                content = response.read().decode()
                assert response.status == status
                assert all(secret not in content for secret in forbidden + [daemon.token])
            finally:
                conn.close()
        def prompt(sid, generation):
            stream = daemon.stream()
            streams.append(stream)
            stream.wait(lambda kind, _: kind == "snapshot")
            turn = daemon.call("POST", f"/v1/work-sessions/{sid}/turns", {"text": "credential-generation"})
            event = stream.wait(lambda kind, e: kind == "event" and e.get("kind") == "turn_completed" and e["turn"]["id"] == turn["id"], seconds=10)
            assert event["turn"]["state"] == "completed"
            deadline = time.monotonic() + 10
            while daemon.call("GET", f"/v1/work-sessions/{sid}")["runtime"] != "detached":
                assert time.monotonic() < deadline
                time.sleep(0.05)
            history = daemon.call("GET", f"/v1/work-sessions/{sid}/history?limit=200")
            assert any(e.get("text") == "credential generation " + generation for e in history["events"])
            stream.close()
            streams.remove(stream)
        try:
            daemon.start()
            definition = {"id": "codex/rotation", "backend": "codex", "driver": "app-server", "enabled": True,
                          "model": "rotation-model", "provider": "fixture-gateway", "base_url": "https://rotation.example.com/v1",
                          "reasoning_effort": "high", "sandbox_default": "read_only", "approval_policy": "on_request",
                          "secret_ref": "secret://env/rotation"}
            assert daemon.call("POST", "/v1/runtime-profiles", definition)["revision"] == 1
            rejected("POST", "/v1/runtime-profiles", definition, 409)
            inspected = daemon.call("GET", "/v1/runtime-profiles/inspect?id=codex/rotation")
            assert inspected["health"] == "valid" and inspected["secret_status"] == "available"
            assert "secret_value" not in inspected
            for bad in [dict(definition, id="codex/bad", base_url="http://external.example.com/v1"),
                        dict(definition, id="codex/bad", model="fixture-rotation-old"),
                        dict(definition, id="codex/bad", secret_ref="secret://env/PATH"),
                        dict(definition, id="codex/bad", argv=["--anything"]),
                        dict(definition, id="codex/bad", sandbox_default="fixture-rotation-old")]:
                # Extractor failures use 422; value/unknown-field validation can use 400.
                conn = http.client.HTTPConnection("127.0.0.1", daemon.port, timeout=10)
                conn.request("POST", "/v1/runtime-profiles", json.dumps(bad), {"Authorization": "Bearer " + daemon.token, "Content-Type": "application/json"})
                response = conn.getresponse(); content = response.read().decode(); conn.close()
                assert response.status in (400, 422)
                assert all(s not in content for s in forbidden + [daemon.token])
            missing = dict(definition, id="codex/missing", secret_ref="secret://env/rotation-missing")
            daemon.call("POST", "/v1/runtime-profiles", missing)
            assert daemon.call("POST", "/v1/runtime-profiles/validate", {"id": "codex/missing"})["secret_status"] == "missing"
            rejected("POST", "/v1/work-sessions", {"workspace": str(workspace), "profile_id": "codex/missing"}, 503)
            assert daemon.call("GET", "/v1/work-sessions") == []
            policy = {"workspace": str(workspace), "default_profile": definition["id"], "allowed_profiles": [definition["id"]]}
            daemon.call("PUT", "/v1/workspace-profile-policy", policy)
            assert daemon.call("GET", "/v1/workspace-profile-policy?workspace=" + quote(str(workspace), safe=""))["default_profile"] == definition["id"]
            rejected("POST", "/v1/work-sessions", {"workspace": str(workspace), "profile_id": "codex/local"}, 403)
            session = daemon.call("POST", "/v1/work-sessions", {"workspace": str(workspace)})
            sid = session["id"]
            original = daemon.call("GET", f"/v1/work-sessions/{sid}/resolved-profile")
            identity = daemon.identities()[sid]
            prompt(sid, "old")
            revised = dict(definition, model="updated-model", base_url="https://updated.example.com/v1")
            assert daemon.call("PUT", "/v1/runtime-profiles", {"definition": revised, "expected_revision": 1})["revision"] == 2
            assert daemon.call("GET", f"/v1/work-sessions/{sid}/resolved-profile") == original
            prompt(sid, "old")
            assert daemon.identities()[sid] == identity
            env = dict(os.environ, LAIN_URL=f"http://127.0.0.1:{daemon.port}", LAIN_API_TOKEN=daemon.token)
            for args in [["profile", "list"], ["profile", "show", definition["id"]], ["profile", "validate", definition["id"]], ["work", "show", sid], ["work", "list"]]:
                result = subprocess.run([str(binaries / ("lain" + suffix)), "--json", *args], env=env, capture_output=True, check=True)
                json.loads(result.stdout)
                assert all(s.encode() not in result.stdout + result.stderr for s in forbidden + [daemon.token])
            daemon.stop()
            daemon.extra_env["LAIN_SECRET_ROTATION"] = "fixture-rotation-new"
            daemon.start()
            assert daemon.call("GET", f"/v1/work-sessions/{sid}/resolved-profile") == original
            assert daemon.call("GET", f"/v1/work-sessions/{sid}")["runtime"] == "detached"
            prompt(sid, "new")
            assert daemon.identities()[sid] == identity
            config = json.loads((workspace / "fixture-config.json").read_text())
            assert config["model"] == "rotation-model"
            assert any("rotation.example.com" in arg for arg in config["args"])
            assert not any("updated.example.com" in arg for arg in config["args"])
            with closing(sqlite3.connect(root / "state.sqlite3")) as db:
                persisted = "\n".join(db.iterdump())
                assert all(s not in persisted for s in forbidden + [daemon.token])
        finally:
            for stream in streams:
                stream.close()
            daemon.stop()
            assert all(s.encode() not in b"".join(daemon.log) for s in forbidden + [daemon.token])
    print("PASS: profile API/CLI, safe errors, policy, frozen configuration, restart and SecretRef rotation; zero model quota")


if __name__ == "__main__":
    run()
