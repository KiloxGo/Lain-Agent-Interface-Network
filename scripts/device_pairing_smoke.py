"""Native device enrollment/report/revoke fixture; no real device install or model."""
import argparse
import hashlib
import http.client
import json
from contextlib import closing
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import time

from acceptance_files import protect_directory
from qq_memory_smoke import Smoke, executable, require


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    args = parser.parse_args()
    temporary = tempfile.TemporaryDirectory(prefix="lain-device-pairing-")
    root = Path(temporary.name).resolve()
    protect_directory(root)
    smoke = None
    device_key = None
    try:
        smoke = Smoke(root, args.bin_dir.resolve(), None, 240)
        smoke.setup(None)
        smoke.fixture.login_release.set()
        smoke.start()
        require(smoke.cli("device", "list") == [], "device exists without enrollment")
        directory, workspace = root / "device", root / "device-workspace"
        workspace.mkdir()
        agent_env = {k: v for k, v in smoke.env.items() if not k.startswith(("LAIN_", "OPENAI_", "DEEPSEEK_", "ANTHROPIC_"))}
        # An invalid owner directory proves agent operations do not load owner setup.
        agent_env["LAIN_LOCAL_DIR"] = str(root / "no-owner-credential-here")

        def agent(*arguments, expected=0):
            result = subprocess.run([str(args.bin_dir / executable("lain")), "--json", "device", *map(str, arguments)],
                                    env=agent_env, cwd=root, capture_output=True, text=True, encoding="utf-8", timeout=60)
            smoke.fixture.check_secrets(result.stdout + result.stderr)
            if device_key:
                require(device_key not in result.stdout + result.stderr, "device credential in ordinary CLI output")
            require(result.returncode == expected, "unexpected device CLI result")
            return json.loads(result.stdout) if expected == 0 else None

        prepared = agent("init", "--directory", directory, "--endpoint", f"http://127.0.0.1:{smoke.port}",
                         "--workspace", workspace, "--label", "native-fixture", "--expires-at-ms", int(time.time()*1000)+600000)
        credential = json.loads((directory / "device-credential.json").read_text())
        device_key = credential["token"]
        require(device_key not in json.dumps(prepared), "prepare printed device key")
        registration_path = directory / "registration.json"
        registration_bytes = registration_path.read_bytes()
        registration = json.loads(registration_bytes)
        require(device_key.encode() not in registration_bytes and smoke.credentials[0].encode() not in registration_bytes,
                "registration contains bearer material")
        require(not registration["capabilities"], "unprobed Worker capability invented")
        device = prepared["device_id"]
        agent("init", "--directory", directory, "--endpoint", f"http://127.0.0.1:{smoke.port}",
              "--workspace", workspace, "--label", "native-fixture", "--expires-at-ms", int(time.time()*1000)+600000, expected=1)
        require(json.loads((directory / "device-credential.json").read_text()) == credential, "repeat init overwrote key")
        smoke.passed("native_protected_key_public_registration_and_no_owner_key_in_agent")
        agent("claim", "--directory", directory, expected=1)
        pending = smoke.cli("device", "pair", "--file", registration_path)
        require(pending["device"]["state"] == "pending", "owner registration silently claimed device")
        changed = {**registration, "label": "changed-during-transfer"}
        registration_path.write_text(json.dumps(changed), encoding="utf-8")
        agent("claim", "--directory", directory, expected=1)
        require(smoke.cli("device", "show", device)["device"]["state"] == "pending", "manifest mismatch activated device")
        registration_path.write_bytes(registration_bytes)
        claimed = agent("claim", "--directory", directory)
        require(claimed["device"]["state"] == "active" and claimed["device"]["revision"] == 2, "device proof missing")
        require(agent("claim", "--directory", directory) == claimed, "claim retry changed lifecycle")
        smoke.passed("native_owner_admission_exact_manifest_and_idempotent_device_proof")

        def status_with_key(path, key):
            connection = http.client.HTTPConnection("127.0.0.1", smoke.port, timeout=5)
            try:
                connection.request("GET", path, headers={"Authorization": "Bearer "+key})
                response = connection.getresponse()
                require(response.getheader("Cache-Control") == "no-store", "device management is cacheable")
                payload = response.read(65537)
                require(len(payload) <= 65536 and device_key.encode() not in payload, "credential or unbounded response")
                return response.status
            finally:
                connection.close()

        require(status_with_key("/v1/devices", device_key) == 401, "device key authenticated as administrator")
        require(status_with_key(f"/v1/device-agent/{device}", smoke.credentials[0]) == 401, "owner bearer became device proof")
        require(status_with_key(f"/v1/device-agent/{device}", registration["credential_fingerprint"]) == 401, "public verifier became bearer proof")
        first = agent("report", "--directory", directory, "--sequence", 1)
        require(first["report"]["available_workspaces"] == [registration["workspaces"][0]["id"]] and not first["report"]["capabilities"],
                "native workspace report is fabricated")
        agent("report", "--directory", directory, "--sequence", 3, expected=1)
        require(agent("status", "--directory", directory)["latest_report"] == first, "sequence gap advanced receipt")
        smoke.passed("native_principal_isolation_and_monotonic_workspace_report")
        smoke.stop(); smoke.start()
        require(agent("status", "--directory", directory)["latest_report"] == first, "restart lost device sequence/receipt")
        second = agent("report", "--directory", directory, "--sequence", 2)
        require(second["device_revision"] == 2 and second["report"]["sequence"] == 2, "report changed lifecycle revision")
        require(workspace.resolve().is_relative_to(root), "unsafe fixture workspace target")
        workspace.rmdir()
        third = agent("report", "--directory", directory, "--sequence", 3)
        require(third["report"]["available_workspaces"] == [], "missing native workspace reported available")
        smoke.passed("native_restart_receipts_and_honest_workspace_availability")
        smoke.cli("device", "revoke", device, "--revision", 2)
        agent("status", "--directory", directory, expected=1)
        agent("report", "--directory", directory, "--sequence", 4, expected=1)
        agent("claim", "--directory", directory, expected=1)
        after = smoke.cli("device", "pair", "--file", registration_path)
        require(after["device"]["state"] == "revoked" and after["latest_report"] == third, "old registration revived revoked device")
        require(not smoke.fixture.requests and not smoke.fixture.sent and smoke.cli("work", "list") == [], "pairing started model QQ or Worker activity")
        smoke.passed("native_revoke_prevents_reclaim_report_and_worker_or_model_effects")
        smoke.stop()
        with closing(sqlite3.connect(smoke.local / "state.db")) as db:
            require(db.execute("PRAGMA user_version").fetchone()[0] == 23, "wrong device schema")
            dump = "\n".join(db.iterdump())
            smoke.fixture.check_secrets(dump)
            require(device_key not in dump and db.execute("SELECT count(*) FROM device_reports").fetchone()[0] == 3, "key stored or receipt missing")
        log = (smoke.local / "daemon.log").read_text(encoding="utf-8")
        smoke.fixture.check_secrets(log)
        require(device_key not in log, "device key logged")
        report = {"checks": smoke.checks, "schema": 23, "device_reports": 3, "model_requests": 0, "paid_calls": 0,
                  "real_qq_sends": 0, "real_device_installed": False, "agent_has_owner_key": False}
    finally:
        if smoke:
            smoke.stop()
            if smoke.fixture:
                smoke.fixture.model_release.set(); smoke.fixture.login_release.set(); smoke.fixture.close()
        require(root.is_relative_to(Path(tempfile.gettempdir()).resolve()) and root.name.startswith("lain-device-pairing-"), "unsafe cleanup target")
        temporary.cleanup()
    require(not root.exists(), "temporary device state survived cleanup")
    report["cleanup"] = True
    report["binary_sha256"] = {name: hashlib.sha256((args.bin_dir/executable(name)).read_bytes()).hexdigest() for name in ["lain", "laind", "lain-test-app-server"]}
    print(json.dumps(report))


if __name__ == "__main__":
    main()
