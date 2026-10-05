"""Private, deterministic daemon for opt-in browser acceptance; no model calls.

The Playwright fixture supplies a temporary token through the process environment.
Only the port is printed. This helper never prints traffic or credentials.
"""
import json
import os
from pathlib import Path
import shutil
import sys
from codex_smoke import Daemon


def run():
    root = Path(os.environ["LAIN_WEB_TEST_ROOT"])
    repository = Path(__file__).resolve().parents[1]
    suffix = ".exe" if os.name == "nt" else ""
    binaries = Path(os.environ.get("LAIN_TEST_BIN_DIR", repository / "target/debug"))
    private = root / ("laind" + suffix)
    shutil.copy2(binaries / ("laind" + suffix), private)
    os.environ["LAIN_CODEX_PATH"] = str(binaries / ("lain-test-app-server" + suffix))
    os.environ["LAIN_WEB_DIR"] = str(repository / "web/dist")
    daemon = Daemon(root, private, idle_timeout=1)
    daemon.token = os.environ["LAIN_WEB_TEST_TOKEN"]
    try:
        daemon.start()
        print(json.dumps({"port": daemon.port}), flush=True)
        for line in sys.stdin:
            if line.strip() == "restart":
                daemon.stop()
                daemon.start()
                print("restarted", flush=True)
            elif line.strip() == "stop":
                break
    finally:
        daemon.stop()


if __name__ == "__main__":
    run()
