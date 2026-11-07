"""Read-only rollback predicate for the explicit cloud schema32 -> 39 upgrade.

The operator must stop the Kernel and hold its native database ownership lock.
Baseline is a verified standalone backup; inspect a private copy of failed state
including its committed WAL. This function never restores a file or replays work.
"""
from contextlib import ExitStack, closing
import hashlib
import json
from pathlib import Path
import sqlite3
import stat
import uuid


EXTRA_TABLES = {
    "briefing_grants", "briefing_manifests", "briefing_batches",
    "briefing_runs", "briefing_disclosures", "briefing_memory_suppressions",
    "conversation_memory_effects", "briefing_relations",
}


def _ordinary(path):
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or not 0 < info.st_size <= 1024**3:
        raise ValueError("invalid rollback database input")
    return info.st_dev, info.st_ino


def _identity(db):
    rows = db.execute("SELECT singleton,instance_id,revision FROM client_sync_state").fetchall()
    if len(rows) != 1:
        raise ValueError("invalid sync identity")
    singleton, instance, revision = rows[0]
    if (singleton != 1 or not isinstance(instance, str) or uuid.UUID(instance).int == 0
            or type(revision) is not int or not 0 <= revision < 9223372036854775807):
        raise ValueError("invalid sync identity")
    return instance, revision


def _objects(db):
    return {(kind, name): (table, sql)
            for kind, name, table, sql in db.execute(
                "SELECT type,name,tbl_name,sql FROM sqlite_master ORDER BY type,name")}


def _table_evidence(db, tables):
    without_rowid = {name: bool(wr) for schema, name, kind, columns, wr, strict
                     in db.execute("PRAGMA table_list") if schema == "main"}
    result = {}
    for name in sorted(tables):
        quoted = '"' + name.replace('"', '""') + '"'
        columns = "*" if without_rowid[name] else "rowid,*"
        hashes = []
        for row in db.execute("SELECT " + columns + " FROM " + quoted):
            encoded = json.dumps(list(row), ensure_ascii=True, allow_nan=False,
                                 default=lambda value: {"blob_hex": value.hex()}).encode()
            hashes.append(hashlib.sha256(encoded).digest())
        # Hash each complete row before sorting: no unbounded copied text report.
        result[name] = (len(hashes), hashlib.sha256(b"".join(sorted(hashes))).hexdigest())
    return result


def restoration_is_safe(baseline, current):
    """False on any changed data, authority, schema, counter or unreadable input."""
    try:
        baseline, current = Path(baseline), Path(current)
        if _ordinary(baseline) == _ordinary(current):
            return False
        for suffix in ("-wal", "-journal"):
            sidecar = Path(str(baseline) + suffix)
            if sidecar.is_symlink() or (sidecar.exists() and sidecar.stat().st_size != 0):
                return False
        with ExitStack() as stack:
            old = stack.enter_context(closing(sqlite3.connect(baseline.absolute().as_uri() + "?mode=ro", uri=True)))
            failed = stack.enter_context(closing(sqlite3.connect(current.absolute().as_uri() + "?mode=ro", uri=True)))
            for db in (old, failed):
                db.execute("BEGIN")
                if db.execute("PRAGMA quick_check").fetchone()[0] != "ok":
                    return False
            if old.execute("PRAGMA user_version").fetchone()[0] != 32:
                return False
            version = failed.execute("PRAGMA user_version").fetchone()[0]
            if version not in (32, 39) or _identity(old) != _identity(failed):
                return False
            before, after = _objects(old), _objects(failed)
            old_tables = {name for kind, name in before if kind == "table"}
            new_tables = {name for kind, name in after if kind == "table"}
            extra = EXTRA_TABLES if version == 39 else set()
            if new_tables != old_tables | extra or old_tables & extra:
                return False
            if any(after.get(key) != value for key, value in before.items()):
                return False
            if any(table not in extra for key, (table, sql) in after.items() if key not in before):
                return False
            for name in extra:
                if failed.execute('SELECT count(*) FROM "' + name + '"').fetchone()[0] != 0:
                    return False
            if _table_evidence(old, old_tables) != _table_evidence(failed, old_tables):
                return False
            return True
    except (sqlite3.Error, OSError, ValueError, TypeError, OverflowError, KeyError):
        return False
