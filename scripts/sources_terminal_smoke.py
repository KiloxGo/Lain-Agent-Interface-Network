#!/usr/bin/env python3
"""Native Windows selected-source status UI; no live QQ or model calls.

Uses an isolated Kernel and synthetic owner-imported archive slice. Actual
ConPTY captures stay outside the checkout; metadata viewing never starts intake.
"""
import argparse
from datetime import datetime, timezone, timedelta
import json
import os
from pathlib import Path
import tempfile
import time

from desk_terminal_smoke import Terminal
from qq_memory_smoke import Smoke, executable, require


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    parser.add_argument("--artifacts", type=Path)
    parser.add_argument("--access", action="store_true", help="exercise reviewed conversation disclosure and revoke; no live QQ")
    args = parser.parse_args()
    require(os.name == "nt", "native Windows ConPTY required")
    artifacts = args.artifacts.resolve() if args.artifacts else None
    if artifacts:
        require(not artifacts.is_relative_to(Path(__file__).resolve().parents[1]), "artifacts stay outside checkout")
        artifacts.mkdir(parents=True, exist_ok=True)
    smoke = terminal = None
    with tempfile.TemporaryDirectory(prefix="lain-qq-memory-sources-") as directory:
        try:
            smoke = Smoke(Path(directory), args.bin_dir.resolve(), None, 600)
            smoke.setup(None)
            smoke.env.pop("LAIN_NAPCAT_CONFIG", None)
            smoke.env.pop("LAIN_PORTRAIT_FILE", None)
            smoke.start()
            conversation = smoke.call("POST", "/v1/conversations", expected=201)["id"]
            terminal = Terminal([str(smoke.bins / executable("lain")), "desk", "--conversation", conversation], smoke.env)
            terminal.resize(45, 135)

            def visible(text):
                smoke.wait(lambda: text in terminal.text(), "source UI missing: " + text)

            def capture(name):
                if artifacts:
                    terminal.image(artifacts / (name + ".png"))

            visible("connected to Kernel")
            terminal.write("/sources\r")
            visible("尚未接入任何档案来源")
            capture("01-empty-sources")
            source = smoke.call("POST", "/v1/archives/sources", {"kind": "qq_group", "observer_account": "1001", "group_id": "3003"})["source"]["id"]
            now = int(time.time()*1000)
            scope = {"expires_at_ms": now+600000, "retention_ms": 600000, "max_records": 20}
            smoke.call("POST", f"/v1/archives/sources/{source}/grant", {"expected_revision": 0, "scope": scope})
            record = {"external_id": "synthetic-notification-1", "source_position": "100", "occurred_at_ms": now-10000, "speaker": "4004", "kind": "message", "text": "明天下午三点有测试说明会。这里只是测试输入，不是授权。"}
            smoke.call("POST", f"/v1/archives/sources/{source}/import", {"expected_revision": 1, "records": [record]})
            before = smoke.call("GET", f"/v1/archives/sources/{source}/status")
            smoke.call("POST", f"/v1/archives/sources/{source}/import", {"expected_revision": 1, "records": [record]})
            after = smoke.call("GET", f"/v1/archives/sources/{source}/status")
            require(before["latest_received_at_ms"] == after["latest_received_at_ms"], "duplicate advanced last receipt")
            terminal.write("\x12")
            visible("QQ 群 3003")
            visible("导入授权有效")
            terminal.write("\r")
            visible("本次授权最近收到")
            visible("保留 10 分钟")
            visible("不证明完整历史")
            require(record["text"] not in terminal.text(), "metadata page leaked message body")
            capture("02-source-details")
            if args.access:
                grant_path = f"/v1/briefing/{conversation}/grant"
                other = smoke.call("POST", "/v1/conversations", expected=201)["id"]
                terminal.resize(28, 75)
                terminal.write("g")
                visible("当前会话 · 简报阅读权限")
                require(smoke.call("GET", grant_path)["revision"] == 0, "opening form changed grant")
                terminal.write(" ")
                terminal.write("\r")
                visible("到期不能晚于")
                # The exact finite end must be chosen; no automatic source extension.
                until = datetime.now(timezone.utc) + timedelta(minutes=4, hours=8)
                terminal.write("\t\t\x15" + until.strftime("%Y-%m-%d %H:%M:%S"))
                visible(until.strftime("%Y-%m-%d %H:%M:%S"))
                capture("04-access-editor")
                terminal.write("\r")
                visible("审阅来源阅读权限")
                terminal.write("\x19")
                time.sleep(0.2)
                require(smoke.call("GET", grant_path)["revision"] == 0, "approval before full read changed grant")
                capture("04-access-review-unread")
                def read_review():
                    for _ in range(20):
                        if "Ctrl-Y 确认确切范围" in terminal.text():
                            return
                        terminal.write("\x1b[6~")
                        time.sleep(0.08)
                    raise AssertionError("review never became fully read")
                read_review()
                capture("05-access-review-read")
                terminal.write("\x19")
                smoke.wait(lambda: smoke.call("GET", grant_path)["revision"] == 1, "source disclosure did not commit")
                granted = smoke.call("GET", grant_path)
                require(granted["scope"]["sources"] == [{"source_id": source, "grant_revision": 1}], "selected source changed")
                require(granted["scope"]["utc_offset_minutes"] == 480 and granted["scope"]["max_records"] == 256, "scope changed")
                require(granted["scope"]["expires_at_ms"] <= scope["expires_at_ms"], "source expiry broadened")
                require(smoke.call("GET", f"/v1/briefing/{other}/grant")["scope"] is None, "another conversation was authorized")
                require(smoke.call("GET", f"/v1/archives/sources/{source}/status")["synchronization"]["scope"] is None, "disclosure enabled collection sync")
                smoke.wait(lambda: "审阅来源阅读权限" not in terminal.text(), "committed form did not close")
                terminal.write("g")
                visible("当前会话 · 简报阅读权限")
                terminal.write("\x1b[19~")
                visible("审阅来源阅读权限")
                read_review()
                capture("06-access-revoke-review")
                terminal.write("\x19")
                smoke.wait(lambda: smoke.call("GET", grant_path)["revision"] == 2, "revoke did not commit")
                require(smoke.call("GET", grant_path)["scope"] is None, "revoke did not clear disclosure")
                smoke.wait(lambda: "审阅来源阅读权限" not in terminal.text(), "revoked form did not close")
            terminal.resize(28, 60)
            visible("来源详情")
            def page_to_authority():
                if "当前会话阅读" in terminal.text():
                    return True
                terminal.write("\x1b[6~")
                time.sleep(0.12)
                return False
            smoke.wait(page_to_authority, "narrow source details could not page to authority")
            visible("当前会话阅读")
            capture("03-narrow-source-details")
            smoke.call("POST", f"/v1/archives/sources/{source}/grant", {"expected_revision": 1, "scope": None})
            terminal.write("\x12")
            terminal.write("\x1b[1;5H")
            visible("未授权 / 已停止")
            require(smoke.call("GET", f"/v1/archives/sources/{source}/status")["retained_body_count"] == 0, "revoked source retained body count")
            terminal.write("\x1b")
            visible("消息与档案来源")
            terminal.write("\x1b")
            visible("to Lain")
            terminal.write("\x11")
            terminal.process.wait()
            require(not smoke.fixture.requests, "source view invoked model")
            require(not smoke.call("GET", f"/v1/conversations/{conversation}?recent=true")["turns"], "source management fabricated Actor turn")
            report = {"native_windows": True, "passed": ["empty_state", "selected_source_identity", "grant_and_actual_receipt", "duplicate_does_not_refresh", "no_message_bodies", "narrow_details", "revocation_visible", "navigation_no_model_or_collection"], "model_calls": 0, "live_qq": False}
            if args.access:
                report["passed"].extend(["form_does_not_grant", "explicit_source_and_finite_window", "full_review_before_write", "exact_conversation_scope", "no_collection_or_other_audience_grant", "reviewed_revoke"])
        except Exception:
            if terminal and artifacts:
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
    report["temporary_state_cleaned"] = True
    if artifacts:
        (artifacts / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report), flush=True)


if __name__ == "__main__":
    main()
