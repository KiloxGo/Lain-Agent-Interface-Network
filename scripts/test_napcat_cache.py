"""Linux-native temporary-cache safety; synthetic files only."""
import os
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch

from napcat_cache import CacheOptions, prune


@unittest.skipUnless(os.name == "posix", "native Linux dir_fd semantics required")
class CacheTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="lain-napcat-cache-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.state = self.root / "qq"
        self.cache = self.state / "NapCat" / "temp"
        self.cache.mkdir(parents=True)
        self.options = CacheOptions(self.state, os.getuid())
        self.now = time.time() + 3 * 86400

    def file(self, name, body=b"synthetic", age=2 * 86400):
        path = self.cache / name
        path.write_bytes(body)
        os.utime(path, (self.now - age, self.now - age))
        return path

    def test_dry_run_is_read_only_and_apply_preserves_recent_and_session_data(self):
        old = self.file("old.png")
        recent = self.file("recent.png", age=60)
        database = self.file("session.db")
        outside = self.state / "login.db"
        outside.write_bytes(b"keep login")
        report = prune(self.options, now=self.now)
        self.assertEqual(report["planned_files"], 1)
        self.assertTrue(old.exists())
        self.assertFalse((self.cache / ".lain-media-prune").exists())
        result = prune(self.options, apply=True, now=self.now)
        self.assertEqual(result["deleted_files"], 1)
        self.assertFalse(old.exists())
        self.assertTrue(recent.exists() and database.exists())
        self.assertEqual(outside.read_bytes(), b"keep login")
        self.assertNotIn("old.png", str(result))

    def test_symlink_root_and_symlink_files_never_followed(self):
        target = self.root / "keep.png"
        target.write_bytes(b"unchanged")
        (self.cache / "link.png").symlink_to(target)
        prune(self.options, apply=True, now=self.now)
        self.assertEqual(target.read_bytes(), b"unchanged")
        linked = self.root / "alias" / "qq"
        linked.parent.mkdir()
        linked.symlink_to(self.state, target_is_directory=True)
        with self.assertRaises((ValueError, OSError)):
            prune(CacheOptions(linked, os.getuid()), apply=True, now=self.now)

    def test_hardlinks_and_unexpected_directory_owner_are_refused(self):
        target = self.root / "keep.bin"
        target.write_bytes(b"unchanged")
        link = self.cache / "linked.png"
        os.link(target, link)
        result = prune(self.options, apply=True, now=self.now)
        self.assertEqual(result["deleted_files"], 0)
        self.assertTrue(link.exists())
        with self.assertRaises(ValueError):
            prune(CacheOptions(self.state, os.getuid() + 1), apply=True, now=self.now)

    def test_budget_respects_fresh_files_and_explicitly_reports_unmet_limit(self):
        self.options.max_bytes = 4
        old = self.file("older.png", b"12345", age=7200)
        fresh = self.file("fresh.png", b"123456", age=60)
        report = prune(self.options, apply=True, now=self.now)
        self.assertFalse(old.exists())
        self.assertTrue(fresh.exists())
        self.assertTrue(report["over_budget"])

    def test_partial_scan_does_not_delete_and_reports_incomplete_inventory(self):
        self.options.max_files = 1
        first = self.file("one.png")
        second = self.file("two.png")
        result = prune(self.options, apply=True, now=self.now)
        self.assertTrue(result["scan_incomplete"] and result["over_budget"])
        self.assertEqual(result["deleted_files"], 0)
        self.assertTrue(first.exists() and second.exists())

    def test_delete_work_is_bounded(self):
        self.options.max_deletes = 1
        files = [self.file(f"item-{i}.png") for i in range(3)]
        result = prune(self.options, apply=True, now=self.now)
        self.assertEqual(result["deleted_files"], 1)
        self.assertEqual(sum(p.exists() for p in files), 2)

    def test_refreshed_replacement_is_restored_instead_of_deleted(self):
        path = self.file("replace.png")
        rename = os.rename
        def refreshed(source, destination, **kwargs):
            if source == "replace.png":
                path.unlink()
                self.file("replace.png", b"fresh replacement", age=0)
            return rename(source, destination, **kwargs)
        with patch("napcat_cache.os.rename", refreshed):
            result = prune(self.options, apply=True, now=self.now)
        self.assertEqual(result["deleted_files"], 0)
        self.assertEqual(path.read_bytes(), b"fresh replacement")

    def test_interrupted_quarantine_stops_without_removing_unknown_objects(self):
        self.file("old.png")
        quarantine = self.cache / ".lain-media-prune"
        quarantine.mkdir(mode=0o700)
        retained = quarantine / "retained"
        retained.write_bytes(b"preserve for recovery")
        with self.assertRaises(ValueError):
            prune(self.options, apply=True, now=self.now)
        self.assertEqual(retained.read_bytes(), b"preserve for recovery")


if __name__ == "__main__":
    unittest.main()
