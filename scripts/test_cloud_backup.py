"""Linux-native backup regressions; no live configuration or cloud credentials."""
import contextlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tarfile
import tempfile
import unittest

from cloud_backup import BackupOptions, consistent_snapshot, create_backup


@unittest.skipUnless(os.name == "posix", "native Linux ownership lock required")
class CloudBackupTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="lain-backup-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "source"
        self.source.mkdir(mode=0o700)
        self.addCleanup(lambda: self.source.chmod(0o700))
        self.db = self.source / "lain.sqlite3"
        self.lock = self.source / "lain.sqlite3.lock"
        self.lock.touch(mode=0o600)
        with contextlib.closing(sqlite3.connect(self.db)) as c:
            c.executescript("PRAGMA journal_mode=WAL;PRAGMA user_version=28;CREATE TABLE facts(value TEXT);INSERT INTO facts VALUES('baseline');")
        self.scratch = self.root / "scratch"
        self.scratch.mkdir(mode=0o700)
        self.output = self.scratch / "snapshot.sqlite3"

    def values(self, path):
        with contextlib.closing(sqlite3.connect(path)) as c:
            return c.execute("SELECT value FROM facts ORDER BY value").fetchall()

    def readonly(self):
        for path in self.source.iterdir():
            path.chmod(0o400)
        self.source.chmod(0o500)

    def test_closed_wal_database_in_readonly_directory_is_backed_up_without_source_writes(self):
        self.readonly()
        before = {p.name: p.read_bytes() for p in self.source.iterdir()}
        mode = consistent_snapshot(self.db, self.output, self.scratch, 1024**3)
        self.assertEqual(mode, "quiescent_locked_copy")
        self.assertEqual(self.values(self.output), [("baseline",)])
        self.assertEqual({p.name: p.read_bytes() for p in self.source.iterdir()}, before)

    def test_crashed_committed_wal_is_included_in_quiescent_copy(self):
        code = "import os,sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute(\"INSERT INTO facts VALUES('committed WAL')\"); c.commit(); os._exit(0)"
        subprocess.run([sys.executable, "-c", code, str(self.db)], check=True, timeout=10)
        self.assertGreater(Path(str(self.db) + "-wal").stat().st_size, 0)
        self.readonly()
        before = {p.name: p.read_bytes() for p in self.source.iterdir()}
        consistent_snapshot(self.db, self.output, self.scratch, 1024**3)
        self.assertEqual(self.values(self.output), [("baseline",), ("committed WAL",)])
        self.assertEqual({p.name: p.read_bytes() for p in self.source.iterdir()}, before)

    def test_owned_live_database_uses_sqlite_backup_and_keeps_committed_wal(self):
        import fcntl
        with self.lock.open("rb") as owner, contextlib.closing(sqlite3.connect(self.db)) as c:
            fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
            c.execute("INSERT INTO facts VALUES('live WAL')")
            c.commit()
            self.readonly()
            mode = consistent_snapshot(self.db, self.output, self.scratch, 1024**3)
        self.assertEqual(mode, "live_sqlite_backup")
        self.assertEqual(self.values(self.output), [("baseline",), ("live WAL",)])

    def test_linked_ownership_lock_is_refused(self):
        self.lock.unlink()
        target = self.root / "unrelated"
        target.write_bytes(b"do not change")
        self.lock.symlink_to(target)
        with self.assertRaises(ValueError):
            consistent_snapshot(self.db, self.output, self.scratch, 1024**3)
        self.assertEqual(target.read_bytes(), b"do not change")

    def options(self):
        backups = self.root / "backups"
        backups.mkdir(mode=0o700, exist_ok=True)
        config = self.root / "private.env"
        config.write_text("LAIN_API_TOKEN=synthetic-not-a-live-credential\n", encoding="utf-8")
        config.chmod(0o600)
        release = self.root / "release"
        release.mkdir(exist_ok=True)
        return BackupOptions(self.db, backups, config, release, max_count=2)

    def test_archive_is_private_restorable_and_reports_no_config_body(self):
        options = self.options()
        report = create_backup(options)
        archive = options.backup_dir / report["backup"]
        self.assertEqual(archive.stat().st_mode & 0o777, 0o600)
        with tarfile.open(archive, "r:gz") as t:
            self.assertEqual(set(t.getnames()), {"lain.sqlite3", "kernel.env", "manifest.json"})
            metadata = json.load(t.extractfile("manifest.json"))
            self.assertEqual(metadata["user_version"], 28)
        self.assertEqual(report["restore_check"], "passed")
        self.assertNotIn("synthetic-not-a-live-credential", json.dumps(report))
        self.assertFalse(any(p.name.startswith(".") for p in options.backup_dir.iterdir() if p.name != ".backup.lock"))

    def test_retention_keeps_new_verified_archive_with_finite_count_and_bytes(self):
        options = self.options()
        reports = [create_backup(options) for _ in range(3)]
        retained = list(options.backup_dir.glob("lain-backup-*.tar.gz"))
        self.assertEqual(len(retained), 2)
        self.assertTrue((options.backup_dir / reports[-1]["backup"]).is_file())
        self.assertEqual(reports[-1]["retained_bytes"], sum(p.stat().st_size for p in retained))

    def test_explicit_actor_configuration_is_archived_and_verified_without_reporting_body(self):
        options = self.options()
        actor = self.root / "actor.json"
        body = b'{"model":"synthetic-model","secret_ref":"secret://env/synthetic"}\n'
        actor.write_bytes(body)
        actor.chmod(0o600)
        options.actor_config = actor
        report = create_backup(options)
        with tarfile.open(options.backup_dir / report["backup"], "r:gz") as archive:
            self.assertEqual(set(archive.getnames()), {"lain.sqlite3", "kernel.env", "actor.json", "manifest.json"})
            self.assertEqual(archive.extractfile("actor.json").read(), body)
            metadata = json.load(archive.extractfile("manifest.json"))
            import hashlib
            self.assertEqual(metadata["actor_config_sha256"], hashlib.sha256(body).hexdigest())
        self.assertEqual(report["restore_check"], "passed")
        self.assertNotIn("synthetic-model", json.dumps(report))

    def test_linked_actor_configuration_is_refused_without_changing_existing_archive(self):
        options = self.options()
        valid = create_backup(options)
        original = options.backup_dir / valid["backup"]
        before = original.read_bytes()
        actor = self.root / "actor.json"
        unrelated = self.root / "unrelated.json"
        unrelated.write_text('{"synthetic":"preserve"}')
        actor.symlink_to(unrelated)
        options.actor_config = actor
        with self.assertRaises(ValueError):
            create_backup(options)
        self.assertEqual(original.read_bytes(), before)
        self.assertEqual(unrelated.read_text(), '{"synthetic":"preserve"}')

    def test_missing_or_oversized_actor_configuration_does_not_publish_or_prune(self):
        options = self.options()
        valid = create_backup(options)
        original = options.backup_dir / valid["backup"]
        before = original.read_bytes()
        actor = self.root / "actor.json"
        options.actor_config = actor
        with self.assertRaises(FileNotFoundError):
            create_backup(options)
        actor.write_bytes(b"x" * (16384 + 1))
        with self.assertRaises(ValueError):
            create_backup(options)
        self.assertEqual(original.read_bytes(), before)
        self.assertEqual(len(list(options.backup_dir.glob("lain-backup-*.tar.gz"))), 1)

    def test_oversize_failure_does_not_delete_the_existing_valid_backup(self):
        options = self.options()
        report = create_backup(options)
        original = options.backup_dir / report["backup"]
        before = original.read_bytes()
        options.max_bytes = 1
        with self.assertRaises(ValueError):
            create_backup(options)
        self.assertEqual(original.read_bytes(), before)

    def test_aggregate_archive_bytes_prune_even_below_the_count_limit(self):
        options = self.options()
        options.max_count = 7
        # Incompressible synthetic configuration makes each archive exceed the
        # small raw DB size, so the aggregate-byte limit can be crossed without
        # failing the independent input/single-archive budget check first.
        options.config.write_text(os.urandom(16000).hex(), encoding="ascii")
        first = create_backup(options)
        first_path = options.backup_dir / first["backup"]
        options.max_bytes = first_path.stat().st_size * 2 + 2048
        second = create_backup(options)
        latest = create_backup(options)
        self.assertLessEqual(latest["retained_bytes"], options.max_bytes)
        self.assertEqual(latest["retained_count"], 2)
        # Archive names have second resolution plus a random suffix; either
        # prior archive may sort first within the same second.
        self.assertEqual(sum(p.exists() for p in [first_path, options.backup_dir / second["backup"]]), 1)
        self.assertTrue((options.backup_dir / latest["backup"]).exists())

    def test_corrupt_source_fails_without_publishing_archive(self):
        options = self.options()
        self.db.write_bytes(b"invalid SQLite")
        with self.assertRaises(sqlite3.DatabaseError):
            create_backup(options)
        self.assertEqual(list(options.backup_dir.glob("lain-backup-*.tar.gz")), [])

    def test_explicit_channel_configuration_is_included_and_restore_verified(self):
        import hashlib
        options = self.options()
        channel = self.root / "napcat.json"
        body = b'{"name":"synthetic-private","secret_ref":"secret://env/napcat"}'
        channel.write_bytes(body)
        options.channel_config = channel
        report = create_backup(options)
        with tarfile.open(options.backup_dir / report["backup"]) as archive:
            self.assertEqual(set(archive.getnames()), {"lain.sqlite3", "kernel.env", "napcat.json", "manifest.json"})
            self.assertEqual(archive.extractfile("napcat.json").read(), body)
            manifest = json.load(archive.extractfile("manifest.json"))
            self.assertEqual(manifest["channel_config_sha256"], hashlib.sha256(body).hexdigest())
        self.assertTrue(report["channel_configuration_included"])
        self.assertEqual(report["restore_check"], "passed")
        self.assertNotIn("synthetic-private", json.dumps(report))

    def test_channel_restore_hash_mismatch_does_not_publish_or_prune(self):
        import io
        from unittest.mock import patch
        options = self.options()
        good = create_backup(options)
        original = options.backup_dir / good["backup"]
        before = original.read_bytes()
        channel = self.root / "napcat.json"
        channel.write_text('{"name":"synthetic"}')
        options.channel_config = channel
        extract = tarfile.TarFile.extractfile
        def corrupted(archive, member):
            if member == "napcat.json":
                return io.BytesIO(b"synthetic corrupted restore")
            return extract(archive, member)
        with patch.object(tarfile.TarFile, "extractfile", corrupted):
            with self.assertRaisesRegex(ValueError, "Restored channel configuration hash mismatch"):
                create_backup(options)
        self.assertEqual(original.read_bytes(), before)
        self.assertEqual(len(list(options.backup_dir.glob("lain-backup-*.tar.gz"))), 1)

    def test_linked_channel_configuration_is_refused_and_preserves_last_good_backup(self):
        options = self.options()
        good = create_backup(options)
        original = options.backup_dir / good["backup"]
        before = original.read_bytes()
        target = self.root / "unrelated.json"
        target.write_text('{"synthetic":"unchanged"}')
        channel = self.root / "napcat.json"
        channel.symlink_to(target)
        options.channel_config = channel
        with self.assertRaises(ValueError):
            create_backup(options)
        channel.unlink()
        os.link(target, channel)
        with self.assertRaises(ValueError):
            create_backup(options)
        self.assertEqual(original.read_bytes(), before)
        self.assertEqual(target.read_text(), '{"synthetic":"unchanged"}')

    def test_missing_or_oversized_channel_configuration_does_not_publish_or_prune(self):
        options = self.options()
        good = create_backup(options)
        original = options.backup_dir / good["backup"]
        before = original.read_bytes()
        channel = self.root / "napcat.json"
        options.channel_config = channel
        with self.assertRaises(FileNotFoundError):
            create_backup(options)
        channel.write_bytes(b"x" * (8192 + 1))
        with self.assertRaises(ValueError):
            create_backup(options)
        self.assertEqual(original.read_bytes(), before)
        self.assertEqual(len(list(options.backup_dir.glob("lain-backup-*.tar.gz"))), 1)

    def test_explicit_knowledge_reader_configuration_is_archived_without_reporting_body(self):
        import hashlib
        options = self.options()
        reader = self.root / "knowledge.json"
        body = b'{"endpoint":"https://fixture.invalid/archive","store_id":"synthetic-store","secret_ref":"secret://env/reader"}'
        reader.write_bytes(body)
        options.knowledge_config = reader
        report = create_backup(options)
        with tarfile.open(options.backup_dir / report["backup"]) as archive:
            self.assertEqual(set(archive.getnames()), {"lain.sqlite3", "kernel.env", "knowledge.json", "manifest.json"})
            self.assertEqual(archive.extractfile("knowledge.json").read(), body)
            metadata = json.load(archive.extractfile("manifest.json"))
            self.assertEqual(metadata["knowledge_config_sha256"], hashlib.sha256(body).hexdigest())
            self.assertEqual(metadata["scope"], "cloud_kernel_only_not_knowledge_or_pc")
        self.assertTrue(report["knowledge_configuration_included"])
        self.assertEqual(report["restore_check"], "passed")
        self.assertNotIn("fixture.invalid", json.dumps(report))
        self.assertNotIn("secret://env/reader", json.dumps(report))

    def test_cli_can_back_up_all_three_explicit_kernel_configurations(self):
        options = self.options()
        actor = self.root / "actor.json"
        channel = self.root / "napcat.json"
        reader = self.root / "knowledge.json"
        for path in [actor, channel, reader]:
            path.write_text('{"synthetic":"configuration-body"}')
        run = subprocess.run([
            sys.executable, str(Path(__file__).with_name("cloud_backup.py")),
            "--database", str(options.database), "--backup-dir", str(options.backup_dir),
            "--config", str(options.config), "--release", str(options.release),
            "--actor-config", str(actor), "--channel-config", str(channel),
            "--knowledge-config", str(reader),
        ], capture_output=True, text=True, timeout=10)
        self.assertEqual(run.returncode, 0, run.stderr)
        report = json.loads(run.stdout)
        self.assertTrue(report["knowledge_configuration_included"])
        self.assertTrue(report["actor_configuration_included"])
        self.assertTrue(report["channel_configuration_included"])
        with tarfile.open(options.backup_dir / report["backup"]) as archive:
            self.assertEqual(set(archive.getnames()), {"lain.sqlite3", "kernel.env", "actor.json", "napcat.json", "knowledge.json", "manifest.json"})
            self.assertEqual(archive.extractfile("knowledge.json").read(), reader.read_bytes())
        self.assertNotIn("configuration-body", run.stdout + run.stderr)

    def test_missing_oversized_or_linked_knowledge_config_keeps_last_good_backup(self):
        options = self.options()
        good = create_backup(options)
        original = options.backup_dir / good["backup"]
        before = original.read_bytes()
        reader = self.root / "knowledge.json"
        options.knowledge_config = reader
        with self.assertRaises(FileNotFoundError):
            create_backup(options)
        reader.write_bytes(b"x" * 16385)
        with self.assertRaises(ValueError):
            create_backup(options)
        reader.unlink()
        target = self.root / "unrelated-reader.json"
        target.write_text('{"synthetic":"untouched"}')
        reader.symlink_to(target)
        with self.assertRaises(ValueError):
            create_backup(options)
        reader.unlink()
        os.link(target, reader)
        with self.assertRaises(ValueError):
            create_backup(options)
        self.assertEqual(target.read_text(), '{"synthetic":"untouched"}')
        self.assertEqual(original.read_bytes(), before)
        self.assertEqual(len(list(options.backup_dir.glob("lain-backup-*.tar.gz"))), 1)

    def test_knowledge_restore_hash_mismatch_does_not_publish_or_prune(self):
        import io
        from unittest.mock import patch
        options = self.options()
        good = create_backup(options)
        original = options.backup_dir / good["backup"]
        before = original.read_bytes()
        reader = self.root / "knowledge.json"
        reader.write_text('{"secret_ref":"secret://env/synthetic-reader"}')
        options.knowledge_config = reader
        extract = tarfile.TarFile.extractfile
        def corrupted(archive, member):
            if member == "knowledge.json":
                return io.BytesIO(b"synthetic corrupt reader restore")
            return extract(archive, member)
        with patch.object(tarfile.TarFile, "extractfile", corrupted):
            with self.assertRaisesRegex(ValueError, "Restored knowledge configuration hash mismatch"):
                create_backup(options)
        self.assertEqual(original.read_bytes(), before)
        self.assertEqual(len(list(options.backup_dir.glob("lain-backup-*.tar.gz"))), 1)


if __name__ == "__main__":
    unittest.main()
