"""Explicit opt-in real Actor + Codex acceptance. Consumes model quota.

Run with an already configured daemon. No real calls occur without --run-real.
Owner reviews/approves the exact delegation interactively. No automatic approval.
Daemon restart is a separate manual final check; this script never stops a user's
service. Completion of this script alone does not claim restart acceptance.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-real", action="store_true")
    parser.add_argument("--workspace", type=Path)
    parser.add_argument("--lain", type=Path)
    args = parser.parse_args()
    if not args.run_real:
        parser.exit(message="Not run: --run-real is required; this test consumes Actor and Codex quota.\n")
    if args.workspace is None or not args.workspace.is_absolute():
        parser.error("--workspace must be an existing absolute path on the daemon host")
    repo = Path(__file__).resolve().parents[1]
    binary = args.lain or repo / "target/debug" / ("lain.exe" if os.name == "nt" else "lain")
    def cli(*words):
        output = subprocess.run([str(binary), "--json", *words], capture_output=True, text=True, encoding="utf-8", timeout=180)
        if output.returncode:
            raise RuntimeError("CLI failed; inspect lain doctor and the recorded conversation (raw provider output omitted)")
        return json.loads(output.stdout)
    doctor = cli("doctor")["diagnostics"]
    assert doctor["actor"]["status"] == "configured", "Configure real Actor credentials first"
    assert any(b["available"] for b in doctor["backends"]), "Supported native Codex required"
    hello = cli("ask", "Hello Lain. Reply briefly in your natural voice.")
    assert hello["state"] == "completed" and any(m["role"] == "assistant" for m in hello["messages"])
    conversation = hello["conversation_id"]
    print("Conversation:", conversation)
    inspected = cli("ask", "Use work.list to inspect existing WorkSessions, then explain the actual result briefly.", "--conversation", conversation)
    assert any(a["tool"]["name"] == "work.list" and a["state"] == "executed" for a in inspected["actions"])
    delegation = cli("ask", f"Propose one Codex delegation using work.create: workspace {json.dumps(str(args.workspace))}, profile codex/local, sandbox read_only, task: briefly inspect the available test configuration without editing files. Do not claim it has run; request confirmation.", "--conversation", conversation)
    assert delegation["state"] == "pending_confirmation"
    action = next(a for a in delegation["actions"] if a["state"] == "proposed")
    assert action["tool"]["name"] == "work.create"
    print(json.dumps(action["tool"], ensure_ascii=False, indent=2))
    approved = input("Approve this exact real Codex delegation and its quota use? [y/N] ").lower() == "y"
    outcome = cli("actor", "approve" if approved else "deny", delegation["id"], action["id"])
    if not approved:
        print("Declined. Real delegation acceptance is incomplete.")
        return
    assert outcome["state"] == "completed" and outcome["actions"][-1]["state"] == "executed"
    sid = outcome["actions"][-1]["result"]["outcome"]["work_session_id"]
    assert cli("work", "show", sid)["id"] == sid
    print("Real WorkSession:", sid)
    observed = cli("ask", f"Use work.inspect to inspect real WorkSession {sid}. Explain its actual state; accepted work is not proof of completion.", "--conversation", conversation)
    assert any(a["tool"]["name"] == "work.inspect" and a["state"] == "executed" for a in observed["actions"])
    print("Real model conversation, typed reads, confirmed delegation and client exit/resume passed.")
    print("Finish manual acceptance: observe this WorkSession independently with lain work watch; run lain chat --conversation", conversation)
    print("Exit chat, gracefully restart your daemon with the same database, then run lain conversation", conversation)
    print("Record worker execution and restart continuity separately; this script does not claim those checks passed.")


if __name__ == "__main__":
    main()
