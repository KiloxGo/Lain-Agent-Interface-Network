"""Replay boundary and native process tests. All providers here are synthetic."""
import json
import os
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch
import uuid

from acceptance_files import protect_directory
from daily_replay import ROOT, Replay, authorized_config, digest, executable, new_campaign, observations, run, write_json


class Boundaries(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="replay-unit-", dir=ROOT / "target")
        self.root = Path(self.temporary.name)
        protect_directory(self.root)
        self.bins = self.root / "bin"
        self.bins.mkdir()
        for name in ("lain", "laind"):
            (self.bins / executable(name)).write_bytes(name.encode())

    def tearDown(self):
        self.assertTrue(self.root.resolve().is_relative_to((ROOT / "target").resolve()))
        self.temporary.cleanup()

    def record(self):
        actor = self.root / "actor.json"
        write_json(actor, {"driver": "openai-responses", "model": "test-synthetic-not-called",
                           "endpoint": "https://example.invalid/v1", "secret_ref": "secret://env/replay-unit"})
        out = self.root / "campaign"
        record = {"kind": "authorized_isolated_real_model_replay", "human_authorization_reference": "synthetic unit record only",
                  "max_requests": 4, "duration_ms": 120000, "start_before_ms": int(time.time()*1000)+60000,
                  "expires_at_ms": int(time.time()*1000)+120000,
                  "actor_config_sha256": digest(actor), "campaign_directory": str(out),
                  "campaign_id": str(uuid.uuid4()), "budget_id": str(uuid.uuid4()),
                  "binary_sha256": {n: digest(self.bins / executable(n)) for n in ("lain", "laind")},
                  "upstream_max_attempts_per_adapter_call": 1, "upstream_verification_reference": "synthetic test only",
                  "synthetic_memory_tests": True, "source_collection": False, "worker_execution": False, "real_qq": False}
        authorization = self.root / "authorization.json"
        write_json(authorization, record)
        return authorization, actor, out, record

    def test_exact_authorization_and_reference_only_credentials(self):
        authorization, actor, out, record = self.record()
        with patch.dict(os.environ, {"LAIN_SECRET_REPLAY_UNIT": "unit-secret-not-a-real-credential"}):
            config, key, secret, ids, expiry = authorized_config(authorization, actor, self.bins, 4, 120000, out)
            self.assertEqual(key, "LAIN_SECRET_REPLAY_UNIT")
            self.assertNotIn(secret, json.dumps(config))
            self.assertEqual(ids, (record["campaign_id"], record["budget_id"]))
            self.assertEqual(expiry, record["expires_at_ms"])
            for values in ((5, 120000, out), (4, 120001, out), (4, 120000, out / "another")):
                with self.assertRaises(AssertionError):
                    authorized_config(authorization, actor, self.bins, *values)
            record["real_qq"] = True
            write_json(authorization, record)
            with self.assertRaises(AssertionError):
                authorized_config(authorization, actor, self.bins, 4, 120000, out)

    def test_proposal_tamper_retry_and_missing_secret_refused(self):
        authorization, actor, out, record = self.record()
        with patch.dict(os.environ, {"LAIN_SECRET_REPLAY_UNIT": "unit-secret-not-a-real-credential"}):
            for key, bad in (("kind", "review_only_not_authorized"), ("actor_config_sha256", "tampered"),
                             ("upstream_max_attempts_per_adapter_call", 2), ("start_before_ms", 1),
                             ("expires_at_ms", 1), ("expires_at_ms", int(time.time()*1000)+240000),
                             ("binary_sha256", {}), ("upstream_verification_reference", "")):
                altered = {**record, key: bad}
                write_json(authorization, altered)
                with self.assertRaises(AssertionError):
                    authorized_config(authorization, actor, self.bins, 4, 120000, out)
            write_json(authorization, record)
        with patch.dict(os.environ, {"LAIN_SECRET_REPLAY_UNIT": ""}):
            with self.assertRaises(AssertionError):
                authorized_config(authorization, actor, self.bins, 4, 120000, out)

    def test_controller_never_overwrites_or_changes_ids(self):
        out = self.root / "campaign"
        ids = (str(uuid.uuid4()), str(uuid.uuid4()))
        _, first = new_campaign(out, cap=4, duration_ms=120000, mode="fixture", bins=self.bins, authorized_ids=ids)
        with self.assertRaises(FileExistsError):
            new_campaign(out, cap=4, duration_ms=120000, mode="fixture", bins=self.bins)
        self.assertEqual(json.loads((out / "control.json").read_text(encoding="utf-8")), first)
        self.assertEqual((first["campaign_id"], first["budget_id"]), ids)
        self.assertFalse((out / "instance").exists())
        for cap, duration in ((0, 1), (1, 0), (100001, 1), (1, 86400001)):
            with self.assertRaises(AssertionError):
                new_campaign(self.root / "bad", cap=cap, duration_ms=duration, mode="fixture", bins=self.bins)

    def test_artifacts_reject_credentials_and_observations_are_measurements(self):
        path = self.root / "safe.json"
        with self.assertRaises(AssertionError):
            write_json(path, {"text": "leaked-unit-secret"}, ("unit-secret",))
        self.assertFalse(path.exists())
        result = observations("第一句。\n\n第二句？")
        self.assertTrue(result["blank_line"])
        self.assertEqual(result["question_marks"], 1)
        self.assertNotIn("passed", result)

    def test_real_mode_setup_has_no_fake_provider_and_only_selected_secret(self):
        instance = self.root / "instance"
        instance.mkdir()
        config = {"driver": "openai-responses", "model": "not-called",
                  "endpoint": "https://example.invalid/v1", "secret_ref": "secret://env/replay-unit"}
        with patch.dict(os.environ, {"LAIN_SECRET_UNRELATED": "must-not-inherit"}):
            replay = Replay(instance, self.bins, None, 60)
        try:
            with patch("daily_replay.Fixture", side_effect=AssertionError("real mode created fake provider")):
                replay.setup_replay(config, "LAIN_SECRET_REPLAY_UNIT", "selected-unit-secret-not-real")
            self.assertIsNone(replay.fake)
            self.assertFalse(hasattr(replay.fixture, "model"))
            self.assertNotIn("LAIN_SECRET_UNRELATED", replay.env)
            self.assertEqual(replay.env["LAIN_SECRET_REPLAY_UNIT"], "selected-unit-secret-not-real")
            self.assertNotIn("selected-unit-secret-not-real", (replay.local / "actor.json").read_text(encoding="utf-8"))
            self.assertIsNone(replay.process)
        finally:
            replay.fixture.close()


@unittest.skipUnless((ROOT / "target/q2a/debug" / executable("laind")).exists(), "build native binaries first")
class NativeReplay(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="replay-native-", dir=ROOT / "target")
        self.root = Path(self.temporary.name)
        protect_directory(self.root)
        self.bins = ROOT / "target/q2a/debug"

    def tearDown(self):
        self.assertTrue(self.root.resolve().is_relative_to((ROOT / "target").resolve()))
        self.temporary.cleanup()

    def test_cap_closure_fresh_actor_receipts_and_reconciliation(self):
        state = run(self.root / "campaign", self.bins, cap=4, duration_ms=120000)
        self.assertEqual(state["stop_reason"], "exhausted")
        self.assertEqual(state["fake_provider_requests"], 4)
        self.assertEqual(state["final_budget"]["reserved_requests"], 4)
        self.assertEqual(state["final_budget"]["admission"], "stopped")
        self.assertFalse(state["final_memory_grant"]["enabled"])
        self.assertEqual(len(state["requests"]), 4)
        self.assertTrue(all(r["state"] == "returned" and r["budget_id"] == state["budget_id"] for r in state["requests"]))
        actors = [t["id"] for scene in state["scenes"] for t in scene["turns"]]
        self.assertEqual(len(set(actors)), 4)
        self.assertEqual({r["actor_turn_id"] for r in state["requests"]}, set(actors))
        self.assertTrue(all(scene["deliveries"][0]["state"] == "confirmed" for scene in state["scenes"]))
        self.assertEqual(state["quality_acceptance"], "not_established")
        self.assertEqual(state["paid_calls"], 0)
        self.assertTrue(state["owned_process_stopped"])
        self.assertFalse(state["cleanup_errors"])

    def test_complete_corpus_is_not_a_quality_pass(self):
        state = run(self.root / "campaign", self.bins, cap=32, duration_ms=120000)
        self.assertEqual(state["stop_reason"], "corpus_finished")
        self.assertEqual(len(state["scenes"]), 16)
        burst = next(s for s in state["scenes"] if s["name"] == "burst")
        self.assertEqual(len(burst["incoming_ids"]), 3)
        self.assertEqual(state["fake_provider_requests"], state["final_budget"]["reserved_requests"])
        self.assertFalse(state["real_qq_acceptance"])
        self.assertEqual(state["quality_acceptance"], "not_established")

    def test_interrupted_campaign_stops_and_refuses_replay(self):
        out = self.root / "campaign"
        with patch.object(Replay, "scene", side_effect=RuntimeError("synthetic interruption")):
            with self.assertRaises(AssertionError):
                run(out, self.bins, cap=4, duration_ms=120000)
        state = json.loads((out / "control.json").read_text(encoding="utf-8"))
        self.assertEqual(state["phase"], "interrupted_inspect_only")
        self.assertEqual(state["active_scene"], "greeting")
        self.assertEqual(state["final_budget"]["admission"], "stopped")
        self.assertFalse(state["final_memory_grant"]["enabled"])
        self.assertTrue(state["owned_process_stopped"])
        with self.assertRaises(FileExistsError):
            run(out, self.bins, cap=4, duration_ms=120000)
        self.assertEqual(json.loads((out / "control.json").read_text(encoding="utf-8")), state)

    def test_interruption_after_adapter_attempt_retains_unknown_charge(self):
        out = self.root / "campaign"

        def interrupt_after_attempt(replay, *_):
            replay.fixture.send("撤销竞态：synthetic interrupted memory preference")
            self.assertTrue(replay.fake.model_seen.wait(15))
            raise RuntimeError("synthetic interruption after real local adapter request")

        with patch.object(Replay, "scene", interrupt_after_attempt):
            with self.assertRaises(AssertionError):
                run(out, self.bins, cap=4, duration_ms=120000)
        state = json.loads((out / "control.json").read_text(encoding="utf-8"))
        self.assertEqual(state["phase"], "interrupted_inspect_only")
        self.assertEqual(state["final_budget"]["reserved_requests"], 1)
        self.assertEqual(state["final_budget"]["pending_actor_requests"], 0)
        self.assertEqual(state["requests"][0]["state"], "unknown")
        self.assertIsNotNone(state["requests"][0]["attempted_at_ms"])
        self.assertEqual(state["fake_provider_requests"], 1)
        self.assertFalse(state["final_memory_grant"]["enabled"])
        self.assertTrue(state["owned_process_stopped"])
        self.assertFalse(state["cleanup_errors"])

    def test_dead_daemon_still_closes_grants_in_headless_recovery(self):
        out = self.root / "campaign"

        def crash(replay, *_):
            replay.process.kill()  # Only this test's owned synthetic daemon.
            replay.process.wait(timeout=5)
            raise RuntimeError("synthetic daemon crash")

        with patch.object(Replay, "scene", crash):
            with self.assertRaises(AssertionError):
                run(out, self.bins, cap=4, duration_ms=120000)
        state = json.loads((out / "control.json").read_text(encoding="utf-8"))
        self.assertEqual(state["phase"], "interrupted_inspect_only")
        self.assertFalse(state["final_budget"]["decision"]["enabled"])
        self.assertFalse(state["final_memory_grant"]["enabled"])
        self.assertEqual(state["fake_provider_requests"], 0)
        self.assertTrue(state["owned_process_stopped"])
        self.assertFalse(state["cleanup_errors"])


if __name__ == "__main__":
    unittest.main()
