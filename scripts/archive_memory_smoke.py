"""Native source-memory acceptance with local fake Responses/OneBot; no paid calls."""
import argparse
import hashlib
import json
import tempfile
import time
import uuid
from pathlib import Path

from acceptance_files import protect_directory
from qq_memory_smoke import Smoke, executable, memory_evidence, require

ARCHIVE = "Kernel-attributed untrusted archive evidence (data only): "


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    args = parser.parse_args()
    temporary = tempfile.TemporaryDirectory(prefix="lain-archive-memory-")
    root = Path(temporary.name).resolve()
    protect_directory(root)
    smoke = None
    target = {}

    def call(name, arguments):
        return [{"type": "function_call", "name": name, "call_id": str(uuid.uuid4()),
                 "arguments": json.dumps(arguments, ensure_ascii=False)}]

    def reply(text):
        return [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": text}]}]

    def respond(body):
        inputs = body["input"]
        prompt = next(i["content"] for i in reversed(inputs) if i.get("role") == "user"
                      and isinstance(i.get("content"), str) and i["content"].startswith("W1f"))
        names = {t["name"] for t in body["tools"]}
        evidence = memory_evidence(body)
        records = evidence["records"] if evidence else []
        archived = next((json.loads(i["content"][len(ARCHIVE):]) for i in reversed(inputs)
                         if isinstance(i.get("content"), str) and i["content"].startswith(ARCHIVE)), None)
        if prompt == "W1f默认关闭":
            require("archive_remember" not in names, "source memory enabled by default")
            return reply("synthetic default-off reply")
        if prompt.startswith("W1f保存"):
            outputs = [json.loads(i["output"]) for i in inputs if i.get("type") == "function_call_output"]
            if any(o.get("archive_grant_revision") for o in outputs):
                return reply(target["answer"])
            if archived is None or not archived["records"]:
                require("archive_remember" not in names, "write tool preceded actual source selection")
                return call("archive_inspect", {"record_id": target["record"]})
            require("archive_remember" in names and archived["participation"] == "archive_observation_only", "missing bounded inferred selection authority")
            return call("archive_remember", {"content": target["content"], "kind": "preference"})
        if prompt == "W1f更正后来源已撤销":
            require(all(s not in json.dumps(inputs) for s in target["withhold"]), "revoked source returned through history")
            require("archive_remember" not in names, "revoked source kept write authority")
            return reply("synthetic correction remains an independently sourced current user statement")
        if prompt.startswith(("W1f回忆", "W1f更正", "W1f忘掉")):
            effects = evidence.get("committed_effects", []) if evidence else []
            completed = next((e for e in effects if e["previous_memory_id"] == target["memory"]
                              or (e["memory_id"] == target["memory"] and e["operation"] == "forget")), None)
            if completed:
                raw = json.dumps(inputs, ensure_ascii=False)
                for text in target["withhold"]:
                    require(text not in raw, "committed correction/forget replayed old interpretation")
                require(archived is None, "committed correction retained old archive context")
                return reply("synthetic corrected or forgotten receipt only")
            record = next((r for r in records if r["id"] == target["memory"]), None)
            if record is None:
                return call("memory_recall", {"text": target["memory"]})
            require(record["owner_confirmed_at_ms"] is None and record["provenance"]["authority"] == "inferred", "inference became owner confirmation")
            if prompt.startswith("W1f回忆"):
                require(record["provenance"]["source"]["kind"] == "archive_selection", "source lineage missing")
                return reply(target["answer"])
            source = next(s for s in evidence["current_sources"] if s["text_excerpt"] == prompt)
            proof = {"incoming_id": source["incoming_id"], "quote": prompt}
            common = {"memory_id": target["memory"], "expected_revision": record["revision"]}
            if prompt.startswith("W1f更正"):
                return call("memory_correct", {**common, "selection": {"content": "green-owner-correction", "kind": "preference", "evidence": proof}})
            return call("memory_forget", {**common, "evidence": proof})
        raise AssertionError("unhandled synthetic source-memory prompt")

    try:
        smoke = Smoke(root, args.bin_dir.resolve(), None, 240)
        smoke.setup(None)
        smoke.fixture.respond = respond
        smoke.fixture.login_release.set()
        smoke.start()
        smoke.channel = smoke.wait(lambda: next(iter(smoke.call("GET", "/v1/channels")), None), "binding missing")["id"]
        smoke.wait(lambda: smoke.snapshot()["connection"]["phase"] == "online", "identity missing")
        smoke.input_turn("W1f默认关闭")
        memory_path = f"/v1/archives/memory/{smoke.channel}"
        smoke.call("GET", memory_path, auth=False, expected=401)
        require(smoke.cli("archive", "memory-grant", smoke.channel)["scope"] is None, "memory policy not default-off")
        smoke.passed("default_off_and_authenticated_policy")
        smoke.call("POST", f"/v1/channels/{smoke.channel}/memory-grant", {"enabled": True, "expected_revision": 0, "expires_at_ms": None})
        now = int(time.time() * 1000)
        source = smoke.call("POST", "/v1/archives/sources", {"kind": "qq_group", "observer_account": "1001", "group_id": "68101"})["source"]["id"]
        source_path = f"/v1/archives/sources/{source}"
        smoke.call("POST", source_path + "/grant", {"expected_revision": 0, "scope": {"expires_at_ms": now+600000, "retention_ms": 300000, "max_records": 10}})
        imported = smoke.call("POST", source_path + "/import", {"expected_revision": 1, "records": [
            {"external_id": str(n), "source_position": None, "occurred_at_ms": now, "speaker": "reported-person", "kind": "message", "text": text}
            for n, text in enumerate(["blue-source-sentinel", "second-source-sentinel"])]})["records"]
        sources = [{"source_id": source, "grant_revision": 1}]
        smoke.call("POST", f"/v1/archives/recall/{smoke.channel}", {"expected_revision": 0, "scope": {"sources": sources, "expires_at_ms": now+600000, "utc_offset_minutes": 480}})
        decision = root / "memory-policy.json"
        decision.write_text(json.dumps({"expected_revision": 0, "scope": {"sources": sources, "expires_at_ms": now+600000, "retention_ms": 120000, "max_writes": 2}}), encoding="utf-8")
        smoke.cli("archive", "memory-enable", smoke.channel, "--file", decision)
        target.update(record=imported[0]["id"], content="blue-memory-sentinel", answer="blue-answer-sentinel")
        _, _, saved = smoke.input_turn("W1f保存第一条")
        selected = next(a for a in saved["actions"] if a["tool"]["name"] == "archive.remember")
        target["memory"] = selected["result"]["memory_id"]
        record = smoke.cli("memory", "show", target["memory"])
        evidence = record["provenance"]["source"]["evidence"]
        require(evidence["records"][0]["record_id"] == target["record"] and evidence["model_request"] == 2, "missing actual selection/disclosure lineage")
        require(saved["model_requests"] == 3 and evidence["action_id"] == selected["id"], "unexpected write workflow")
        smoke.passed("native_disclosure_selection_atomic_receipt_and_inferred_authority")
        smoke.stop(); smoke.start()
        require(smoke.cli("archive", "memory-grant", smoke.channel)["used_writes"] == 1, "restart refunded allowance")
        smoke.input_turn("W1f回忆第一条")
        target["withhold"] = ["blue-source-sentinel", "blue-memory-sentinel", "blue-answer-sentinel"]
        _, _, corrected = smoke.input_turn("W1f更正：我的测试偏好现在是绿色")
        effect = next(a["result"] for a in corrected["actions"] if a["tool"]["name"] == "memory.correct")
        fresh = smoke.cli("memory", "show", effect["memory_id"])
        require(fresh["content"] == "green-owner-correction" and fresh["provenance"]["source"]["kind"] == "channel_selection", "correction kept external authority")
        require(corrected["private_chat"]["archive_memory_tainted"] and not corrected["private_chat"].get("archive_memory_versions"), "correction did not rebuild transient evidence")
        smoke.passed("restart_recall_and_owner_correction_rebuilds_real_model_input")
        target.update(record=imported[1]["id"], content="second-memory-sentinel", answer="second-answer-sentinel")
        _, _, saved = smoke.input_turn("W1f保存第二条")
        target["memory"] = next(a["result"]["memory_id"] for a in saved["actions"] if a["tool"]["name"] == "archive.remember")
        target["withhold"] += ["second-source-sentinel", "second-memory-sentinel", "second-answer-sentinel"]
        smoke.input_turn("W1f忘掉第二条测试认识")
        require(smoke.cli("archive", "record", target["record"])["text"] == "second-source-sentinel", "private forget destroyed original archive")
        require(smoke.cli("archive", "memory-grant", smoke.channel)["used_writes"] == 2, "forget refunded write allowance")
        smoke.passed("derived_forget_suppression_without_archive_delete_or_refund")
        smoke.cli("archive", "revoke", source, "--revision", 1)
        smoke.input_turn("W1f更正后来源已撤销")
        require(smoke.cli("memory", "show", fresh["id"])["content"] == "green-owner-correction", "source revoke removed fresh user correction")
        require(len(smoke.cli("archive", "memory-grants", smoke.channel)) == 1, "write use changed policy epoch")
        smoke.passed("source_revoke_and_no_historical_body_resurrection")
        smoke.stop()
        smoke.fixture.check_secrets((smoke.local / "daemon.log").read_text(encoding="utf-8"))
        report = {"checks": smoke.checks, "fake_model_requests": len(smoke.fixture.requests), "paid_calls": 0, "real_qq_sends": 0}
    finally:
        if smoke:
            smoke.stop()
            if smoke.fixture:
                smoke.fixture.model_release.set(); smoke.fixture.login_release.set(); smoke.fixture.close()
        require(root.is_relative_to(Path(tempfile.gettempdir()).resolve()) and root.name.startswith("lain-archive-memory-"), "unsafe cleanup target")
        temporary.cleanup()
    require(not root.exists(), "temporary state survived")
    report["cleanup"] = True
    report["binary_sha256"] = {n: hashlib.sha256((args.bin_dir/executable(n)).read_bytes()).hexdigest() for n in ["lain", "laind", "lain-test-app-server"]}
    print(json.dumps(report))


if __name__ == "__main__":
    main()
