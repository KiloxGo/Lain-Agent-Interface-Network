"""Native local release staging checks; no provider/QQ/personal-state access."""
import json
from pathlib import Path
import tempfile
import unittest

import prepare_daily_release as release


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        safe = release.ROOT / "target"
        safe.mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix="release-fixture-", dir=safe)
        self.root = Path(self.temp.name).resolve()
        self.assertTrue(self.root.is_relative_to(safe.resolve()))
        self.bins = self.root / "bins"
        self.bins.mkdir()
        for name in ("lain.exe", "laind.exe"):
            (self.bins / name).write_bytes(("synthetic-not-an-executable-"+name).encode())
        (self.bins / "credentials.env").write_text("must-not-copy-this", encoding="utf-8")
        self.hashes = {n: release.digest(self.bins / n) for n in ("lain.exe", "laind.exe")}
        report = {"checks": {"fixture": True}, "cleanup": True, "paid_calls": 0,
                  "from_schema": 11,
                  "binary_sha256": {n[:-4]: h for n, h in self.hashes.items()}}
        self.data = {"host": "win32", "schema": 16, "workspace_totals": [1, 0, 0],
                     "validation": dict(fmt=True, clippy=True, build=True), "binaries": self.hashes,
                     "process": report, "upgrade": report, "unrelated_sensitive_field": "must-not-copy-this"}
        self.evidence = self.root / "evidence.json"
        self.write_evidence()
        self.target = self.root / "never-open-personal-instance"
        self.args = dict(bins=self.bins, evidence=self.evidence, out=self.root / "package", target=self.target,
                         self_id="1001", user_id="2002", urls=["https://fixture.invalid/article.txt"],
                         replay_cap=48, qq_cap=12, duration_ms=21600000)

    def write_evidence(self):
        self.evidence.write_text(json.dumps(self.data), encoding="utf-8")

    def tearDown(self):
        self.assertTrue(self.root.is_relative_to((release.ROOT / "target").resolve()))
        self.temp.cleanup()

    def test_review_only_exact_binaries_no_extra_files_or_target_access(self):
        out = release.prepare(**self.args)
        self.assertFalse(self.target.exists())
        self.assertEqual({p.name for p in out.iterdir()}, {"lain.exe", "laind.exe", "proposal.json", "README.md"})
        plan = json.loads((out / "proposal.json").read_text(encoding="utf-8"))
        self.assertTrue(plan["requires_new_live_authorization"])
        self.assertEqual(plan["request_partition_total"], 60)
        self.assertEqual(plan["kind"], "review_only_not_authorized")
        self.assertFalse(plan["target_instance_content_accessed"])
        self.assertNotIn("must-not-copy-this", json.dumps(plan))
        for name, digest in self.hashes.items():
            self.assertEqual(release.digest(out / name), digest)

    def test_binary_tamper_or_failed_evidence_cannot_stage(self):
        (self.bins / "lain.exe").write_bytes(b"tampered")
        with self.assertRaises(ValueError):
            release.prepare(**self.args)
        self.assertFalse(self.args["out"].exists())
        self.data["process"]["cleanup"] = False
        self.write_evidence()
        with self.assertRaises(ValueError):
            release.prepare(**self.args)

    def test_existing_output_and_target_overlap_are_refused(self):
        self.args["out"].mkdir()
        with self.assertRaises(ValueError):
            release.prepare(**self.args)
        self.args["out"] = self.target / "package"
        with self.assertRaises(ValueError):
            release.prepare(**self.args)
        self.assertFalse(self.target.exists())

    def test_scope_is_explicit_and_rejects_credential_urls(self):
        for bad in ["https://fixture.invalid/a?token=not-permitted", "https://name:password@fixture.invalid/a", "http://fixture.invalid/a"]:
            with self.assertRaises(ValueError):
                release.prepare(**dict(self.args, urls=[bad]))
        with self.assertRaises(ValueError):
            release.prepare(**dict(self.args, qq_cap=0))
        with self.assertRaises(ValueError):
            release.prepare(**dict(self.args, self_id="2002"))


if __name__ == "__main__":
    unittest.main()
