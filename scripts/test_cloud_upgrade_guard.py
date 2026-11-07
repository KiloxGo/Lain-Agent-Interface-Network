"""Rollback predicates over isolated synthetic state, never a live database."""
import hashlib
from contextlib import closing
import os
from pathlib import Path
import shutil
import sqlite3
import tempfile
import unittest

from cloud_upgrade_guard import restoration_is_safe


class UpgradeGuardTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="lain-rollback-guard-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.baseline = self.root / "baseline32.db"
        self.current = self.root / "failed39.db"
        with closing(sqlite3.connect(self.baseline)) as db:
            db.executescript("""
                CREATE TABLE client_sync_state(singleton INTEGER PRIMARY KEY, instance_id TEXT, revision INTEGER);
                INSERT INTO client_sync_state VALUES(1,'3a0ded9b-356c-40a7-a58b-ce449fa1c86e',2553);
                CREATE TABLE facts(id TEXT PRIMARY KEY, body TEXT);
                INSERT INTO facts VALUES('a','synthetic original');
                CREATE TABLE bytes_data(id TEXT PRIMARY KEY, body BLOB) WITHOUT ROWID;
                INSERT INTO bytes_data VALUES('b',X'00FF');
                CREATE TABLE lain_conversations(id TEXT PRIMARY KEY);
                CREATE TABLE lain_actor_turns(id TEXT PRIMARY KEY);
                PRAGMA user_version=32;
            """)
        shutil.copyfile(self.baseline, self.current)
        migrations = Path(__file__).resolve().parents[1] / "crates/lain-kernel/migrations"
        with closing(sqlite3.connect(self.current)) as db:
            for number in range(33, 40):
                paths = list(migrations.glob(f"{number:03d}_*.sql"))
                self.assertEqual(len(paths), 1)
                db.executescript(paths[0].read_text(encoding="utf-8"))

    def edit(self, sql):
        with closing(sqlite3.connect(self.current)) as db:
            db.executescript(sql)

    def test_nonempty_nonzero_revision_baseline_and_empty_migration_are_safe(self):
        self.assertTrue(restoration_is_safe(self.baseline, self.current))
        shutil.copyfile(self.baseline, self.current)
        self.assertTrue(restoration_is_safe(self.baseline, self.current))

    def test_same_count_edit_without_counter_change_is_not_safe(self):
        self.edit("UPDATE facts SET body='synthetic new fact' WHERE id='a';")
        self.assertFalse(restoration_is_safe(self.baseline, self.current))

    def test_even_reverted_business_write_with_advanced_counter_is_not_safe(self):
        self.edit("UPDATE client_sync_state SET revision=revision+2;")
        self.assertFalse(restoration_is_safe(self.baseline, self.current))

    def test_empty_conversation_and_new_briefing_effect_are_not_safe(self):
        self.edit("INSERT INTO lain_conversations VALUES('new conversation');")
        self.assertFalse(restoration_is_safe(self.baseline, self.current))
        self.edit("DELETE FROM lain_conversations; INSERT INTO briefing_memory_suppressions VALUES('c','r');")
        self.assertFalse(restoration_is_safe(self.baseline, self.current))

    def test_same_count_binary_edit_is_not_safe(self):
        self.edit("UPDATE bytes_data SET body=X'00FE' WHERE id='b';")
        self.assertFalse(restoration_is_safe(self.baseline, self.current))

    def test_missing_invalid_or_changed_identity_counter_is_not_safe(self):
        for sql in [
            "DELETE FROM client_sync_state;",
            "UPDATE client_sync_state SET revision='invalid';",
            "UPDATE client_sync_state SET instance_id='1bba5930-30cb-44a0-8f66-27aac6338ad1';",
        ]:
            with self.subTest(sql=sql):
                shutil.copyfile(self.baseline, self.current)
                self.edit(sql)
                self.assertFalse(restoration_is_safe(self.baseline, self.current))

    def test_changed_schema_or_unexpected_empty_table_is_not_safe(self):
        self.edit("CREATE TABLE unexpected(value TEXT);")
        self.assertFalse(restoration_is_safe(self.baseline, self.current))
        self.edit("DROP TABLE unexpected; ALTER TABLE facts ADD COLUMN note TEXT;")
        self.assertFalse(restoration_is_safe(self.baseline, self.current))

    def test_unknown_or_corrupt_database_is_not_safe(self):
        self.edit("PRAGMA user_version=40;")
        self.assertFalse(restoration_is_safe(self.baseline, self.current))
        self.current.write_bytes(b"synthetic corrupt database")
        self.assertFalse(restoration_is_safe(self.baseline, self.current))
        self.assertFalse(restoration_is_safe(self.baseline, self.root / "missing.db"))
        self.assertFalse((self.root / "missing.db").exists())

    def test_predicate_preserves_both_database_files(self):
        before = [hashlib.sha256(p.read_bytes()).digest() for p in [self.baseline, self.current]]
        self.assertTrue(restoration_is_safe(self.baseline, self.current))
        self.assertEqual(before, [hashlib.sha256(p.read_bytes()).digest() for p in [self.baseline, self.current]])

    def test_same_file_directory_or_linked_baseline_is_not_safe(self):
        self.assertFalse(restoration_is_safe(self.baseline, self.baseline))
        self.assertFalse(restoration_is_safe(self.baseline, self.root))
        os.link(self.baseline, self.root / "linked.db")
        self.assertFalse(restoration_is_safe(self.baseline, self.current))

    def test_baseline_with_uncheckpointed_commits_is_not_a_standalone_restore_file(self):
        with closing(sqlite3.connect(self.baseline)) as writer:
            writer.execute("PRAGMA journal_mode=WAL;")
            writer.execute("INSERT INTO facts VALUES('wal','synthetic committed WAL');")
            writer.commit()
            with closing(sqlite3.connect(self.current)) as target:
                writer.backup(target)
            self.assertGreater(Path(str(self.baseline) + "-wal").stat().st_size, 0)
            self.assertFalse(restoration_is_safe(self.baseline, self.current))


if __name__ == "__main__":
    unittest.main()
