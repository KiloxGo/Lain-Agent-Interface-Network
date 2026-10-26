"""Stage verified Windows binaries and a review-only daily acceptance proposal.

Never opens the proposed personal instance, loads credentials, starts services,
calls a provider or enables any permission. Outputs stay in this workspace's target/.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
from urllib.parse import urlsplit
import uuid

from acceptance_files import protect_directory

ROOT = Path(__file__).resolve().parent.parent


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def prepare(*, bins, evidence, out, target, self_id, user_id, urls, replay_cap, qq_cap, duration_ms):
    bins, evidence, out = (Path(p).resolve() for p in (bins, evidence, out))
    safe = (ROOT / "target").resolve()
    target = Path(target)
    if not target.is_absolute():
        raise ValueError("proposed instance path must be absolute")
    target = target.resolve()
    if not all(p.is_relative_to(safe) for p in (bins, evidence, out)) or out == safe:
        raise ValueError("release inputs/output must stay within this workspace target directory")
    if out.is_relative_to(target) or target.is_relative_to(out) or out.exists():
        raise ValueError("new package must not overlap or overwrite the proposed instance/existing package")
    if not self_id.isascii() or not user_id.isascii() or not self_id.isdecimal() or not user_id.isdecimal() or self_id == user_id:
        raise ValueError("distinct explicit QQ account identifiers required")
    if not 1 <= replay_cap <= 100000 or not 1 <= qq_cap <= 100000 or replay_cap+qq_cap > 100000:
        raise ValueError("explicit finite partition caps required")
    if not 1 <= duration_ms <= 31*86400000 or not 1 <= len(urls) <= 8:
        raise ValueError("explicit bounded duration/source proposal required")
    for url in urls:
        u = urlsplit(url)
        if (len(url) > 2048 or u.scheme != "https" or not u.hostname or u.username or u.password
                or u.query or u.fragment or u.port not in (None, 443) or any(ord(c) < 32 for c in url)):
            raise ValueError("proposal sources must be exact public HTTPS text URLs; Kernel still validates them")
    report = json.loads(evidence.read_text(encoding="utf-8"))
    if report.get("host") != "win32" or report.get("schema") != 16 or report.get("workspace_totals", [0, 1])[1] != 0:
        raise ValueError("successful native Windows schema-16 evidence required")
    if not all(report.get("validation", {}).get(k) is True for k in ("fmt", "clippy", "build")):
        raise ValueError("format/lint/build evidence missing")
    for key in ("process", "upgrade"):
        r = report[key]
        if r.get("paid_calls") != 0 or not r.get("cleanup") or not r.get("checks") or not all(r["checks"].values()):
            raise ValueError("successful isolated process/rollback evidence required")
    expected = report["binaries"]
    for name in ("lain.exe", "laind.exe"):
        source = (bins / name).resolve()
        if not source.is_relative_to(safe) or not source.is_file() or digest(source) != expected[name]:
            raise ValueError("binary does not match final acceptance evidence")
        for key in ("process", "upgrade"):
            if report[key]["binary_sha256"][name[:-4]] != expected[name]:
                raise ValueError("acceptance report binary mismatch")
    now = int(time.time()*1000)
    manifest = {"format": 1, "kind": "review_only_not_authorized", "schema": 16,
                "created_at_ms": now, "requires_new_live_authorization": True,
                "git_head_at_packaging": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                "git_dirty_at_packaging": bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True)),
                "binary_sha256": {name: expected[name] for name in ("lain.exe", "laind.exe")},
                 "replay_runner": {"location": "repository_scripts_not_copied",
                                   "sha256": {name: digest(ROOT / "scripts" / name) for name in
                                              ("daily_replay.py", "qq_memory_smoke.py", "acceptance_files.py")},
                                   "requires": "native Python with websockets sync API"},
                "target_instance": str(target), "target_instance_content_accessed": False,
                "rehearsed_upgrade_from_schema": report["upgrade"]["from_schema"],
                "qq_recipient_proposal": {"self_id": self_id, "user_id": user_id},
                "sources_proposal": urls, "request_partition_total": replay_cap+qq_cap,
                "duration_ms_after_authorized_start": duration_ms,
                "budget_ids": {"isolated_real_model_replay": str(uuid.uuid4()), "personal_qq": str(uuid.uuid4())},
                "request_partitions": {"isolated_real_model_replay": replay_cap, "personal_qq": qq_cap},
                "policy": {"reuse_secret_refs_only_after_authorization": True, "historical_source_grant": "off",
                           "memory_and_contact_require_separate_scope": True, "worker_auto_approval": False,
                           "restart_same_database_and_budget_ids": True, "no_automatic_quota_transfer_or_new_budget": True},
                "evidence_summary": {"workspace_totals": report["workspace_totals"],
                                     "process_checks": len(report["process"]["checks"]),
                                     "upgrade_checks": len(report["upgrade"]["checks"]), "paid_calls": 0},
                "remaining_before_live_run": ["authorized provider/profile access and source/retention/contact/memory settings",
                                              "verified upstream attempts, exact replay authorization record and concrete stop/upgrade authorization"]}
    out.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=".lain-release-", dir=out.parent)).resolve()
    if not staging.is_relative_to(safe) or staging == safe:
        raise ValueError("invalid staging location")
    try:
        protect_directory(staging)
        for name in ("lain.exe", "laind.exe"):
            shutil.copyfile(bins / name, staging / name)
            if digest(staging / name) != expected[name]:
                raise ValueError("staged binary digest mismatch")
        (staging / "proposal.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
        (staging / "README.md").write_text(
            "# Review-only daily acceptance package\n\nNo live action is authorized by this package.\n\n"
            "1. Review proposal.json, immutable binaries and the exact new live scope/budgets.\n"
            "2. After explicit authorization, inspect actual service state and stop/drain the old instance.\n"
            "3. Back up the complete stopped state with ACLs/credentials protected; preserve old binaries/entry point.\n"
            "4. Upgrade a copy first; bootstrap without model/QQ config, set a finite budget, then restart with approved config and run bounded checks.\n"
            "5. At the boundary close temporary grants/budgets; if needed restore the full matching backup/binary set.\n\n"
            "Never downgrade user_version, overwrite the personal database for synchronization, log credentials, "
            "transfer quota between partitions automatically, continue a paid campaign against a restored legacy daemon, "
            "or label simulated transport as real QQ.\n", encoding="utf-8")
        if os.name != "nt":
            for path in staging.iterdir():
                path.chmod(0o600)
        staging.rename(out)
    finally:
        if staging.exists():
            if not staging.is_relative_to(safe) or not staging.name.startswith(".lain-release-"):
                raise ValueError("refusing cleanup outside verified release staging")
            shutil.rmtree(staging)
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("bins", "evidence", "out", "target"):
        parser.add_argument("--"+name, type=Path, required=True)
    for name in ("self-id", "user-id"):
        parser.add_argument("--"+name, required=True)
    parser.add_argument("--url", action="append", required=True)
    for name in ("replay-cap", "qq-cap", "duration-ms"):
        parser.add_argument("--"+name, type=int, required=True)
    args = vars(parser.parse_args())
    args["urls"] = args.pop("url")
    print(prepare(**args))


if __name__ == "__main__":
    main()
