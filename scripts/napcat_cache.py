"""Bounded Linux cleanup for NapCat/temp; never traverses QQ account data.

Dry-run is the default. Only aged, ordinary, singly-linked image files owned by
the dedicated QQ account qualify. Reports intentionally omit file names.
"""
import argparse
from contextlib import ExitStack
from dataclasses import dataclass
import json
import os
from pathlib import Path
import stat
import time


@dataclass
class CacheOptions:
    state_root: Path
    expected_uid: int
    max_bytes: int = 512 * 1024**2
    ttl_seconds: int = 86400
    min_age_seconds: int = 3600
    max_files: int = 10000
    max_deletes: int = 256


IMAGE_SUFFIXES = {".jpg", ".jpeg", ".png", ".gif", ".webp", ".bmp", ".avif", ".heic"}
QUARANTINE = ".lain-media-prune"


def _directory(path, stack):
    """Open every absolute component without resolving or following links."""
    if not path.is_absolute() or ".." in path.parts:
        raise ValueError("absolute non-traversing state directory required")
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC
    fd = os.open("/", flags)
    stack.callback(os.close, fd)
    for component in path.parts[1:]:
        fd = os.open(component, flags, dir_fd=fd)
        stack.callback(os.close, fd)
    return fd


def _ordinary(value, uid):
    return (stat.S_ISREG(value.st_mode) and value.st_uid == uid
            and value.st_nlink == 1)


def _identity(value):
    return (value.st_dev, value.st_ino, value.st_uid, value.st_nlink,
            value.st_size, value.st_mtime_ns, value.st_ctime_ns)


def _same_after_rename(before, after):
    # rename changes ctime, but never the inode, owner, links, body size or mtime.
    return _identity(before)[:-1] == _identity(after)[:-1]


def prune(options, *, apply=False, now=None):
    if os.name != "posix" or options.expected_uid <= 0:
        raise ValueError("native Linux and a dedicated non-root QQ UID required")
    import fcntl

    if (options.max_bytes < 0 or options.min_age_seconds < 3600
            or options.ttl_seconds < options.min_age_seconds
            or not 0 < options.max_files <= 100000
            or not 0 < options.max_deletes <= 4096):
        raise ValueError("invalid temporary-cache work/retention bounds")
    now = time.time() if now is None else now
    report = dict(planned_files=0, deleted_files=0, deleted_bytes=0,
                  scanned_bytes=0, remaining_bytes=0, scan_incomplete=False,
                  over_budget=False, skipped_races=0, unmanaged_entries=0,
                  dry_run=not apply)
    with ExitStack() as stack:
        cache = _directory(Path(options.state_root) / "NapCat" / "temp", stack)
        if os.fstat(cache).st_uid != options.expected_uid:
            raise ValueError("unexpected temporary-directory owner")
        # Lock the stable cache inode, not a disposable lock-file path.
        fcntl.flock(cache, fcntl.LOCK_EX | fcntl.LOCK_NB)
        candidates = []
        with os.scandir(cache) as entries:
            for count, entry in enumerate(entries, 1):
                if count > options.max_files:
                    report.update(scan_incomplete=True, over_budget=True)
                    break
                if entry.name == QUARANTINE:
                    continue
                try:
                    value = os.stat(entry.name, dir_fd=cache, follow_symlinks=False)
                except FileNotFoundError:
                    continue
                if stat.S_ISREG(value.st_mode):
                    report["scanned_bytes"] += value.st_size
                if (_ordinary(value, options.expected_uid)
                        and Path(entry.name).suffix.lower() in IMAGE_SUFFIXES):
                    candidates.append((value.st_mtime, entry.name, value))
                else:
                    report["unmanaged_entries"] += 1
        report["remaining_bytes"] = report["scanned_bytes"]
        if report["scan_incomplete"]:
            return report  # Never prune from an incomplete inventory.
        selected = []
        projected = report["scanned_bytes"]
        for modified, name, value in sorted(candidates):
            age = now - modified
            if (age >= options.ttl_seconds
                    or (projected > options.max_bytes and age >= options.min_age_seconds)):
                if len(selected) == options.max_deletes:
                    break
                selected.append((name, value))
                projected -= value.st_size
        report["planned_files"] = len(selected)
        if not apply:
            report["over_budget"] = projected > options.max_bytes
            return report

        try:
            os.mkdir(QUARANTINE, mode=0o700, dir_fd=cache)
        except FileExistsError:
            pass
        quarantine = os.open(QUARANTINE, os.O_RDONLY | os.O_DIRECTORY
                             | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=cache)
        stack.callback(os.close, quarantine)
        qstat = os.fstat(quarantine)
        if qstat.st_uid != os.geteuid() or stat.S_IMODE(qstat.st_mode) != 0o700:
            raise ValueError("unexpected quarantine ownership or mode")
        if os.listdir(quarantine):
            raise ValueError("interrupted quarantine requires operator inspection")
        for name, before in selected:
            try:
                current = os.stat(name, dir_fd=cache, follow_symlinks=False)
                if _identity(current) != _identity(before):
                    report["skipped_races"] += 1
                    continue
                os.rename(name, "selected", src_dir_fd=cache, dst_dir_fd=quarantine)
            except FileNotFoundError:
                report["skipped_races"] += 1
                continue
            os.fsync(cache)
            os.fsync(quarantine)
            moved = os.stat("selected", dir_fd=quarantine, follow_symlinks=False)
            if not _same_after_rename(before, moved):
                # Never overwrite a new live file; retain the quarantined object
                # for manual recovery if the original name is already occupied.
                os.link("selected", name, src_dir_fd=quarantine, dst_dir_fd=cache,
                        follow_symlinks=False)
                os.unlink("selected", dir_fd=quarantine)
                os.fsync(cache)
                os.fsync(quarantine)
                report["skipped_races"] += 1
                continue
            os.unlink("selected", dir_fd=quarantine)
            os.fsync(quarantine)
            report["deleted_files"] += 1
            report["deleted_bytes"] += before.st_size
        # Only remove the exact empty quarantine we opened.
        final = os.stat(QUARANTINE, dir_fd=cache, follow_symlinks=False)
        if (final.st_dev, final.st_ino) != (qstat.st_dev, qstat.st_ino):
            raise ValueError("quarantine changed during maintenance")
        os.rmdir(QUARANTINE, dir_fd=cache)
        os.fsync(cache)
    report["remaining_bytes"] -= report["deleted_bytes"]
    report["over_budget"] = report["remaining_bytes"] > options.max_bytes
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--state-root", type=Path, default=Path("/var/lib/lain-napcat/qq"))
    parser.add_argument("--expected-uid", type=int, required=True)
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()
    try:
        report = prune(CacheOptions(args.state_root, args.expected_uid), apply=args.apply)
    except (OSError, ValueError):
        print(json.dumps({"failed_closed": True, "operator_inspection_required": True}))
        return 1
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
