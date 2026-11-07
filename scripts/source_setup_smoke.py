"""Native source registration/import/sync UI with an isolated synthetic QQ source.

No real NapCat, social source, model request, personal state or deployment.
"""
import argparse
from datetime import datetime, timezone, timedelta
import json
import os
from pathlib import Path
import tempfile
import time
import uuid

from desk_terminal_smoke import Terminal
from knowledge_bridge_smoke import Source
from qq_memory_smoke import Smoke, executable, require


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    parser.add_argument("--artifacts", type=Path, required=True)
    args = parser.parse_args()
    require(os.name == "nt", "native Windows required")
    artifacts = args.artifacts.resolve()
    require(not artifacts.is_relative_to(Path(__file__).resolve().parents[1]), "artifacts must stay outside checkout")
    artifacts.mkdir(parents=True, exist_ok=True)
    terminal = smoke = upstream = None
    with tempfile.TemporaryDirectory(prefix="lain-source-setup-") as directory:
        try:
            smoke = Smoke(Path(directory), args.bin_dir.resolve(), None, 180)
            smoke.setup(None)
            smoke.env.pop("LAIN_NAPCAT_CONFIG", None)
            smoke.env.pop("LAIN_PORTRAIT_FILE", None)
            token = "source-setup-fixture-read-" + uuid.uuid4().hex
            upstream = Source(token)
            config = smoke.local / "knowledge.json"
            config.write_text(json.dumps({"endpoint": upstream.endpoint, "store_id": upstream.store_id, "secret_ref": "secret://env/source-setup-fixture"}), encoding="utf-8")
            smoke.env.update(LAIN_KNOWLEDGE_CONFIG=str(config), LAIN_SECRET_SOURCE_SETUP_FIXTURE=token)
            smoke.start()
            conversation = smoke.call("POST", "/v1/conversations", expected=201)["id"]
            terminal = Terminal([str(smoke.bins / executable("lain")), "desk", "--conversation", conversation], smoke.env)
            terminal.resize(32, 95)

            def visible(text):
                smoke.wait(lambda: text in terminal.text(), "source setup UI missing: " + text)

            def capture(name):
                terminal.image(artifacts / (name + ".png"))

            def read_review():
                visible("审阅来源配置")
                for _ in range(25):
                    if "Ctrl-Y 确认这一项变更" in terminal.text():
                        return
                    terminal.write("\x1b[6~")
                    time.sleep(0.08)
                raise AssertionError("source review did not become fully read")

            visible("connected to Kernel")
            terminal.write("/sources\r")
            visible("尚未接入任何档案来源")
            terminal.write("n")
            visible("登记 QQ 来源")
            terminal.write("\x1b[15~")  # F5: private peer, not group.
            visible("QQ 私聊")
            terminal.write("1001\t2002")
            visible("2002")
            capture("01-register-editor")
            require(not smoke.call("GET", "/v1/archives/sources"), "editing registered a source")
            terminal.write("\r")
            read_review()
            capture("02-register-review")
            terminal.write("\x19")
            rows = smoke.wait(lambda: smoke.call("GET", "/v1/archives/sources"), "registration did not commit")
            require(len(rows) == 1 and rows[0]["source"]["key"]["kind"] == "qq_private", "wrong registered identity")
            source = rows[0]["source"]["id"]
            require(rows[0]["grant"]["scope"] is None and upstream.calls == 0, "registration enabled import or HTTP reads")
            visible("来源详情")
            terminal.write("e")
            visible("Lain 保存范围")
            until = (datetime.now(timezone.utc) + timedelta(hours=8, minutes=5)).strftime("%Y-%m-%d %H:%M:%S")
            terminal.write("\x15" + until + "\t\t\x1510\t\x1520")
            visible("20")
            terminal.write("\r")
            read_review()
            capture("03-import-review")
            require(smoke.call("GET", f"/v1/archives/sources/{source}")["grant"]["scope"] is None, "review applied import")
            terminal.write("\x19")
            smoke.wait(lambda: smoke.call("GET", f"/v1/archives/sources/{source}")["grant"]["revision"] == 1, "import scope did not commit")
            imported = smoke.call("GET", f"/v1/archives/sources/{source}")["grant"]["scope"]
            require(imported["retention_ms"] == 600000 and imported["max_records"] == 20, "import editor fields changed")
            require(upstream.calls == 0, "import permission started sync")
            visible("来源详情")
            terminal.write("s")
            visible("有限来源同步")
            visible("配置与凭据可用")
            # Exactly two scheduled reads, at the minimum supported interval.
            terminal.write("\t\t\x155\t\x152\t\x1516")
            terminal.write("\r")
            read_review()
            capture("04-sync-review")
            require(upstream.calls == 0, "sync review started reads")
            terminal.write("\x19")
            sync_path = f"/v1/archives/sources/{source}/knowledge/sync"
            selected = smoke.wait(lambda: (value if (value := smoke.call("GET", sync_path))["scope"] is not None else None), "sync did not commit")
            require(selected["scope"]["max_requests"] == 2 and selected["scope"]["interval_ms"] == 5000 and selected["scope"]["limit"] == 16, "sync editor fields changed")
            smoke.wait(lambda: smoke.call("GET", sync_path)["used_requests"] == 2, "finite sync did not use two attempts")
            def settled():
                status = smoke.call("GET", f"/v1/archives/sources/{source}/status")
                return status if status["retained_body_count"] == 1 and status["latest_read"] and status["latest_read"]["state"] == "completed" else None
            received = smoke.wait(settled, "synthetic upstream message did not reach Kernel")
            require(received["latest_received_at_ms"] is not None, "missing actual receipt time")
            require(received["receiving_now"] is None, "import success falsely claimed collector presence")
            require(received["latest_read"]["new_records"] == 0 and received["latest_read"]["duplicate_records"] == 0, "second read did not resume the committed upstream cursor")
            require(upstream.calls == 2, "unexpected HTTP retries or extra reads")
            require(smoke.call("GET", f"/v1/briefing/{conversation}/grant")["scope"] is None, "sync enabled conversation disclosure")
            require(not smoke.call("GET", f"/v1/conversations/{conversation}?recent=true")["turns"], "setup fabricated Actor turn")
            terminal.write("\x12")
            visible("存有文本：1 条")
            require("synthetic original message" not in terminal.text(), "metadata view exposed message body")
            capture("05-actual-receipt-status")
            # Inspect stored normalized projection only through the explicit owner API.
            page = smoke.call("POST", f"/v1/archives/sources/{source}/query", {"expected_revision": 1, "cursor": None, "limit": 4})
            require(len(page["records"]) == 1 and "synthetic original message" in page["records"][0]["text"], "missing actual stored source projection")
            terminal.write("s")
            visible("有限来源同步")
            terminal.write("\x1b[19~")
            read_review()
            capture("06-sync-stop-review")
            terminal.write("\x19")
            smoke.wait(lambda: smoke.call("GET", sync_path)["scope"] is None, "sync did not stop")
            count = upstream.calls
            terminal.write("\x11")
            terminal.process.wait()
            terminal.close()
            terminal = None
            smoke.stop()
            smoke.start()
            require(smoke.call("GET", sync_path)["scope"] is None, "restart re-enabled stopped sync")
            require(upstream.calls == count == 2, "restart replayed source reads")
            require(not smoke.fixture.requests, "setup invoked model")
            report = {"native_windows": True, "live_qq": False, "model_calls": 0, "source": "isolated synthetic HTTP projection", "upstream_gets": count, "passed": ["reviewed_registration_only", "explicit_private_identity", "reviewed_import_scope_without_sync", "reviewed_finite_sync", "actual_kernel_receipt_and_projection", "duplicate_cursor_no_duplicate_body", "no_disclosure_or_memory_grant", "metadata_view_no_body", "reviewed_stop", "restart_without_replay"]}
        except Exception:
            if terminal:
                capture("failure")
                (artifacts / "failure.txt").write_text(terminal.text(), encoding="utf-8")
            raise
        finally:
            if terminal:
                terminal.close()
            if smoke:
                smoke.stop()
                if smoke.fixture:
                    smoke.fixture.close()
            if upstream:
                upstream.close()
    report["temporary_state_cleaned"] = True
    (artifacts / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report), flush=True)


if __name__ == "__main__":
    main()
