#!/usr/bin/env python3
"""Linux Kernel backup with a read-only source and bounded private archives.

Install as /srv/lain/ops/backup.py. Defaults match the documented cloud layout;
explicit paths support isolated native tests without reading personal state.
"""
import argparse
from contextlib import closing
from dataclasses import dataclass
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import sqlite3
import stat
import tarfile
import tempfile
import uuid


@dataclass
class BackupOptions:
    database: Path
    backup_dir: Path
    config: Path
    release: Path
    max_count: int = 7
    max_bytes: int = 1024**3
    actor_config: Path | None = None
    channel_config: Path | None = None
    knowledge_config: Path | None = None


def ordinary(path):
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
        raise ValueError("Backup input must be an ordinary unlinked file")
    return info


def digest(path):
    result = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            result.update(block)
    return result.hexdigest()


def consistent_snapshot(database, destination, scratch, max_bytes):
    """Keep live SQLite consistency; copy a quiescent WAL only under its owner lock."""
    import fcntl  # Linux service only; no simulated Windows lock acceptance.

    if ordinary(database).st_size > max_bytes or destination.exists():
        raise ValueError("Database exceeds budget or snapshot destination exists")
    database = database.resolve(strict=True)
    ownership = Path(str(database) + ".lock")
    ordinary(ownership)
    descriptor = os.open(ownership, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(descriptor, "rb") as owner:
        try:
            fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
            quiescent = True
        except BlockingIOError:
            quiescent = False
        if quiescent:
            # The same native ownership lock prevents a Kernel from starting
            # while copying. Include an uncheckpointed committed crash WAL;
            # immutable=1 would silently ignore that WAL and is never used.
            copied = scratch / "source-copy"
            copied.mkdir(mode=0o700)
            source_path = copied / "source.sqlite3"
            for suffix in ("", "-wal"):
                source = Path(str(database) + suffix)
                if not source.exists():
                    if suffix:
                        continue
                    raise ValueError("Database disappeared under ownership lock")
                if ordinary(source).st_size > max_bytes:
                    raise ValueError("Database or WAL exceeds backup size budget")
                target = Path(str(source_path) + suffix)
                shutil.copyfile(source, target)
                target.chmod(0o600)
            connection = sqlite3.connect(source_path, timeout=15)
            mode = "quiescent_locked_copy"
        else:
            connection = sqlite3.connect(database.as_uri() + "?mode=ro", uri=True, timeout=15)
            mode = "live_sqlite_backup"
        with closing(connection) as source, closing(sqlite3.connect(destination)) as output:
            destination.chmod(0o600)
            source.backup(output, pages=256, sleep=0.1)
            if output.execute("PRAGMA quick_check").fetchone()[0] != "ok":
                raise ValueError("Backup integrity check failed")
        if destination.stat().st_size > max_bytes:
            raise ValueError("Consistent snapshot exceeds backup size budget")
    return mode


def create_backup(options):
    import fcntl

    if not (1 <= options.max_count <= 31 and 1 <= options.max_bytes <= 1024**3):
        raise ValueError("Invalid finite backup budget")
    directory = options.backup_dir
    info = directory.lstat()
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.geteuid() or info.st_mode & 0o077:
        raise ValueError("Backup directory must be private and owned by this operator")
    if ordinary(options.config).st_size > 64 * 1024:
        raise ValueError("Private configuration exceeds backup size budget")
    if options.actor_config is not None:
        if ordinary(options.actor_config).st_size > 16 * 1024:
            raise ValueError("Actor configuration exceeds backup size budget")
    if options.channel_config is not None:
        if ordinary(options.channel_config).st_size > 8 * 1024:
            raise ValueError("Channel configuration exceeds backup size budget")
    if options.knowledge_config is not None:
        if ordinary(options.knowledge_config).st_size > 16 * 1024:
            raise ValueError("Knowledge configuration exceeds backup size budget")
    lock_path = directory / ".backup.lock"
    descriptor = os.open(lock_path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "a") as lock:
        ordinary(lock_path)
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        name = "lain-backup-" + datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8] + ".tar.gz"
        pending = directory / ("." + name + ".pending")
        published = directory / name
        try:
            with tempfile.TemporaryDirectory(prefix=".lain-backup-", dir=directory) as raw:
                work = Path(raw)
                mode = consistent_snapshot(options.database, work / "lain.sqlite3", work, options.max_bytes)
                with closing(sqlite3.connect(work / "lain.sqlite3")) as c:
                    version = c.execute("PRAGMA user_version").fetchone()[0]
                shutil.copyfile(options.config, work / "kernel.env")
                (work / "kernel.env").chmod(0o600)
                entries = ["lain.sqlite3", "kernel.env", "manifest.json"]
                metadata = {
                    "created_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
                    "database_sha256": digest(work / "lain.sqlite3"),
                    "user_version": version,
                    "release": str(options.release.resolve(strict=True)),
                    "scope": "cloud_kernel_only_not_knowledge_or_pc",
                    "source_mode": mode,
                }
                if options.actor_config is not None:
                    ordinary(options.actor_config)
                    shutil.copyfile(options.actor_config, work / "actor.json")
                    (work / "actor.json").chmod(0o600)
                    if (work / "actor.json").stat().st_size > 16 * 1024:
                        raise ValueError("Actor configuration exceeds backup size budget")
                    metadata["actor_config_sha256"] = digest(work / "actor.json")
                    entries.append("actor.json")
                if options.channel_config is not None:
                    ordinary(options.channel_config)
                    shutil.copyfile(options.channel_config, work / "napcat.json")
                    (work / "napcat.json").chmod(0o600)
                    if (work / "napcat.json").stat().st_size > 8 * 1024:
                        raise ValueError("Channel configuration exceeds backup size budget")
                    metadata["channel_config_sha256"] = digest(work / "napcat.json")
                    entries.append("napcat.json")
                if options.knowledge_config is not None:
                    ordinary(options.knowledge_config)
                    shutil.copyfile(options.knowledge_config, work / "knowledge.json")
                    (work / "knowledge.json").chmod(0o600)
                    if (work / "knowledge.json").stat().st_size > 16 * 1024:
                        raise ValueError("Knowledge configuration exceeds backup size budget")
                    metadata["knowledge_config_sha256"] = digest(work / "knowledge.json")
                    entries.append("knowledge.json")
                (work / "manifest.json").write_text(json.dumps(metadata), encoding="utf-8")
                with tarfile.open(pending, "x:gz") as archive:
                    for item in entries:
                        archive.add(work / item, arcname=item, recursive=False)
                pending.chmod(0o600)
                if pending.stat().st_size > options.max_bytes:
                    raise ValueError("Backup archive exceeds size budget")
                # Restore only the exact owned entries; never extract arbitrary
                # paths or include the private scratch WAL copy in the archive.
                with tempfile.TemporaryDirectory(prefix=".lain-restore-check-", dir=directory) as restored:
                    restored = Path(restored)
                    with tarfile.open(pending, "r:gz") as archive:
                        if len(archive.getnames()) != len(entries) or set(archive.getnames()) != set(entries) or not all(m.isfile() for m in archive.getmembers()):
                            raise ValueError("Unexpected backup archive entries")
                        with archive.extractfile("lain.sqlite3") as source, (restored / "lain.sqlite3").open("xb") as target:
                            shutil.copyfileobj(source, target)
                        if options.actor_config is not None:
                            with archive.extractfile("actor.json") as source, (restored / "actor.json").open("xb") as target:
                                shutil.copyfileobj(source, target)
                            if digest(restored / "actor.json") != metadata["actor_config_sha256"]:
                                raise ValueError("Restored Actor configuration hash mismatch")
                        if options.channel_config is not None:
                            with archive.extractfile("napcat.json") as source, (restored / "napcat.json").open("xb") as target:
                                shutil.copyfileobj(source, target)
                            if digest(restored / "napcat.json") != metadata["channel_config_sha256"]:
                                raise ValueError("Restored channel configuration hash mismatch")
                        if options.knowledge_config is not None:
                            with archive.extractfile("knowledge.json") as source, (restored / "knowledge.json").open("xb") as target:
                                shutil.copyfileobj(source, target)
                            if digest(restored / "knowledge.json") != metadata["knowledge_config_sha256"]:
                                raise ValueError("Restored knowledge configuration hash mismatch")
                    if digest(restored / "lain.sqlite3") != metadata["database_sha256"]:
                        raise ValueError("Restored backup hash mismatch")
                    with closing(sqlite3.connect(restored / "lain.sqlite3")) as c:
                        if c.execute("PRAGMA quick_check").fetchone()[0] != "ok":
                            raise ValueError("Restored database integrity check failed")
                with pending.open("rb") as synced:
                    os.fsync(synced.fileno())
                pending.replace(published)
            archives = sorted((p for p in directory.iterdir() if re.fullmatch(r"lain-backup-\d{8}T\d{6}Z-[0-9a-f]{8}\.tar\.gz", p.name) and p.is_file() and not p.is_symlink() and p.stat().st_uid == os.geteuid()), key=lambda p: p.name)
            total = sum(p.stat().st_size for p in archives)
            old = [p for p in archives if p != published]
            while len(archives) > options.max_count or total > options.max_bytes:
                if not old:
                    raise ValueError("Cannot retain a valid current backup within budget")
                expired = old.pop(0)
                total -= expired.stat().st_size
                expired.unlink()
                archives.remove(expired)
            directory_fd = os.open(directory, os.O_RDONLY)
            try:
                os.fsync(directory_fd)
            finally:
                os.close(directory_fd)
            return {"backup": name, "source_mode": mode, "restore_check": "passed", "retained_count": len(archives), "retained_bytes": total, "offsite_backup": False, "actor_configuration_included": options.actor_config is not None, "channel_configuration_included": options.channel_config is not None, "knowledge_configuration_included": options.knowledge_config is not None}
        finally:
            pending.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--database", type=Path, default=Path("/var/lib/lain/lain.sqlite3"))
    parser.add_argument("--backup-dir", type=Path, default=Path("/srv/lain/backups"))
    parser.add_argument("--config", type=Path, default=Path("/etc/lain/kernel.env"))
    parser.add_argument("--release", type=Path, default=Path("/srv/lain/current"))
    parser.add_argument("--actor-config", type=Path, help="Explicit model JSON to retain and verify with this backup; missing files fail closed")
    parser.add_argument("--channel-config", type=Path, help="Explicit NapCat adapter JSON to retain and verify; excludes QQ login/session data")
    parser.add_argument("--knowledge-config", type=Path, help="Explicit Kernel knowledge-reader JSON to retain and verify; excludes the independent knowledge database and media")
    args = parser.parse_args()
    os.umask(0o077)
    print(json.dumps(create_backup(BackupOptions(args.database, args.backup_dir, args.config, args.release, actor_config=args.actor_config, channel_config=args.channel_config, knowledge_config=args.knowledge_config))))


if __name__ == "__main__":
    main()
