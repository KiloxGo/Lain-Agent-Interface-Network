"""Native isolated QQ knowledge pull acceptance. Default: fake source, zero paid calls.

--cloud-client explicitly reads one selected existing source with its read key.
No cloud mutation, real model, personal database or real QQ send is performed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlsplit
from urllib.request import Request, build_opener, HTTPRedirectHandler
import uuid
from contextlib import closing

from acceptance_files import protect_directory
from qq_memory_smoke import Smoke, executable, require, location

class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, *_):
        return None


class Source:
    def __init__(self, token):
        self.token, self.store_id, self.calls = token, str(uuid.uuid4()), 0
        self.sequence, self.withdrawn, self.caption = 1, False, "synthetic red square"
        self.record_id = 1
        self.fail_status = None
        self.gate = threading.Event()
        self.gate.set()
        source = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_GET(self):
                parts = urlsplit(self.path)
                query = parse_qs(parts.query)
                if parts.path != "/v1/sources/qq:1001:private:2002/changes" or self.headers.get("Authorization") != "Bearer " + source.token or query.get("source_id") != ["qq:1001:private:2002"]:
                    self.send_error(403)
                    return
                source.calls += 1
                if not source.gate.wait(20):
                    self.send_error(503)
                    return
                if source.fail_status:
                    self.send_error(source.fail_status)
                    return
                after = int(query["after"][0])
                stamp = int(time.time() * 1000)
                msg = dict(id=source.record_id, source_id="qq:1001:private:2002", external_id=str(1233+source.record_id), sender="2002", occurred_at_ms=stamp-3000,
                           received_at_ms=stamp-2000, expires_at_ms=source.expires, state="recalled" if source.withdrawn else "active", revision=source.sequence,
                           text=None if source.withdrawn else "synthetic original message",
                           images=[] if source.withdrawn else [dict(id="synthetic-asset", description=source.caption, model="fixture-vision", prompt_revision="v1", state="completed")])
                value = dict(schema_version=1, store_id=source.store_id, source=dict(id=msg["source_id"], account="1001", kind="private", peer="2002", revision=1, enabled=True),
                             from_after=after, through=source.sequence, next_after=source.sequence, has_more=False,
                             items=[] if after == source.sequence else [dict(sequence=source.sequence, message=msg)])
                body = json.dumps(value).encode()
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                try:
                    self.wfile.write(body)
                except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
                    pass

        self.expires = int(time.time() * 1000)+300000
        self.http = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.thread = threading.Thread(target=self.http.serve_forever, name="knowledge-source-fixture")
        self.thread.start()
        self.endpoint = f"http://127.0.0.1:{self.http.server_port}"

    def close(self):
        self.gate.set()
        self.http.shutdown()
        self.http.server_close()
        self.thread.join(5)
        require(not self.thread.is_alive(), "source fixture leaked")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    parser.add_argument("--cloud-client", type=Path)
    parser.add_argument("--verify-sync", action="store_true", help="also authorize exactly two scheduled reads of the selected hosted source")
    parser.add_argument("--account", default="1001")
    parser.add_argument("--peer", default="2002")
    args = parser.parse_args()
    require(not args.verify_sync or args.cloud_client is not None,"--verify-sync requires an explicit cloud client")
    bins = args.bin_dir.resolve()
    for name in ("lain", "laind", "lain-test-app-server"):
        require((bins/executable(name)).is_file(), "build native binaries first")
    fixture, smoke, result = None, None, None
    before_threads = set(threading.enumerate())
    temporary = tempfile.TemporaryDirectory(prefix="lain-knowledge-bridge-")
    root = Path(temporary.name).resolve()
    protect_directory(root)
    try:
        smoke = Smoke(root, bins, None, 180)
        smoke.setup(None)
        smoke.fixture.login_release.set()
        smoke.stop()
        if args.cloud_client:
            client = json.loads(args.cloud_client.read_text(encoding="utf-8"))
            endpoint = client["base_url"]
            parsed=urlsplit(endpoint)
            require(parsed.scheme=="https" and parsed.hostname and not parsed.username and not parsed.password and not parsed.query and not parsed.fragment,"cloud endpoint must be exact HTTPS")
            token = client.get("read_api_key") or client.get("api_key") or client.get("tokens", {}).get("read")
            require(isinstance(token, str) and len(token)>=32, "selected read key missing")
            request = Request(endpoint.rstrip("/")+"/v1/status", headers={"Authorization": "Bearer "+token})
            with build_opener(NoRedirect()).open(request, timeout=15) as response:
                body=response.read(65537)
                require(len(body)<=65536,"cloud status too large")
                status = json.loads(body)
            store_id = status["store_id"]
            remote_source = f"qq:{args.account}:private:{args.peer}"
            require(any(s["id"]==remote_source and s["enabled"] for s in status["sources"]), "selected cloud source disabled")
        else:
            token = "knowledge-fixture-read-"+uuid.uuid4().hex
            fixture = Source(token)
            endpoint, store_id = fixture.endpoint, fixture.store_id
        config = smoke.local/"knowledge.json"
        config.write_text(json.dumps(dict(endpoint=endpoint, store_id=store_id, secret_ref="secret://env/knowledge-bridge")), encoding="utf-8")
        smoke.env.update(LAIN_KNOWLEDGE_CONFIG=str(config), LAIN_SECRET_KNOWLEDGE_BRIDGE=token)
        smoke.start()
        smoke.call("GET", "/v1/archives/sources", auth=False, expected=401)
        source = smoke.call("POST", "/v1/archives/sources", dict(kind="qq_private", observer_account=args.account, peer_id=args.peer))["source"]["id"]
        smoke.cli("archive", "enable", source, "--revision", 0, "--expires-at-ms", int(time.time()*1000)+120000, "--retention-ms", 60000, "--max-records", 100)
        require(smoke.cli("archive", "knowledge-state", source) is None, "source already linked")

        def pull(after=None):
            request_id = str(uuid.uuid4())
            argv = ["archive", "knowledge-pull", source, "--request-id", request_id, "--revision", 1, "--limit", 16]
            if after:
                argv += ["--after", after]
            smoke.cli(*argv)
            job = smoke.wait(lambda: (j if (j := smoke.cli("archive", "read-job", request_id))["state"] not in ("prepared", "reading") else None), "knowledge pull did not settle")
            if job["state"]!="completed":
                print(json.dumps({"knowledge_failure":{k:job.get(k) for k in ("state","failure","provider_error_code","created_at_ms","started_at_ms","finished_at_ms")}}),flush=True)
            require(job["state"]=="completed", "knowledge pull failed: "+str(job.get("failure")))
            return job, argv

        first, argv = pull()
        require(first["reader"]=="knowledge" and first["next"]==first["request"]["request_id"], "wrong source reader/continuation")
        query = root/"query.json"
        query.write_text(json.dumps(dict(expected_revision=1,cursor=None,limit=16)),encoding="utf-8")
        page = smoke.cli("archive", "query", source, "--file", query)
        require(all(r["text_origin"]=="knowledge_projection" for r in page["records"]), "observed projection lost origin")
        smoke.passed("native_scoped_http_pull_and_attributed_projection")
        old_calls = fixture.calls if fixture else None
        smoke.stop(); smoke.start()
        same = smoke.cli(*argv)
        require(same["state"]=="completed" and (fixture is None or fixture.calls==old_calls), "restart replayed an immutable read")
        second, _ = pull(first["request"]["request_id"])
        state = smoke.cli("archive", "knowledge-state", source)
        require(state["last_read"]==second["request"]["request_id"] and state["store_id"]==store_id, "committed source cursor lost")
        smoke.passed("native_restart_idempotency_and_owned_incremental_cursor")
        if fixture:
            record = first["record_ids"][0]
            initial = smoke.cli("archive", "record", record)
            require("external_model_inference_not_sender_words" in initial["text"] and fixture.caption in initial["text"], "image interpretation lost attribution")
            fixture.sequence=2;fixture.caption="synthetic changed caption"
            updated,_=pull(second["request"]["request_id"])
            require(updated["updated_records"]==1 and smoke.cli("archive","record",record)["revision"]==2, "caption update not versioned")
            fixture.sequence=3;fixture.withdrawn=True
            withdrawn,_=pull(updated["request"]["request_id"])
            require(withdrawn["withdrawn_records"]==1 and smoke.cli("archive","record",record)["text"] is None, "remote recall did not invalidate copy")
            fixture.sequence=4;fixture.withdrawn=False
            last,_=pull(withdrawn["request"]["request_id"])
            require(last["new_records"]==0 and smoke.cli("archive","record",record)["text"] is None,"withdrawn copy resurrected")
            smoke.passed("native_caption_update_recall_and_no_resurrection")
            # The administrator enables a different authority from archive retention.
            require(smoke.cli("archive","knowledge-sync",source)["scope"] is None,"sync was implicitly enabled")
            smoke.call("GET",f"/v1/archives/sources/{source}/knowledge/sync",auth=False,expected=401)
            smoke.call("POST",f"/v1/archives/sources/{source}/knowledge/sync",dict(expected_revision=0,scope=None),auth=False,expected=401)
            sync_file=root/"sync.json"
            def enable_sync(revision,maximum=3):
                sync_file.write_text(json.dumps(dict(expected_revision=revision,scope=dict(archive_revision=1,
                    expires_at_ms=int(time.time()*1000)+60000,max_requests=maximum,interval_ms=5000,limit=16))),encoding="utf-8")
                return smoke.cli("archive","knowledge-sync-enable",source,"--file",sync_file)
            def settled_sync(used):
                g=smoke.cli("archive","knowledge-sync",source)
                return g if g["used_requests"]==used and not g["pending"] else None
            fixture.record_id=2;fixture.sequence=5;fixture.caption="scheduled image observation"
            before=fixture.calls
            enable_sync(0)
            g=smoke.wait(lambda:settled_sync(1),"first scheduled read did not finish")
            first_sync=smoke.cli("archive","read-job",g["last_read"])
            scheduled_record=first_sync["record_ids"][0]
            require(first_sync["sync_revision"]==1 and fixture.calls==before+1,"unattributed or repeated scheduled GET")
            smoke.stop();smoke.start()
            require(smoke.cli("archive","knowledge-sync",source)["used_requests"]==1,"restart reset source budget")
            fixture.sequence=6;fixture.caption="scheduled changed caption"
            smoke.wait(lambda:settled_sync(2),"second scheduled read did not finish")
            record=smoke.cli("archive","record",scheduled_record)
            require(record["revision"]==2 and fixture.caption in record["text"],"scheduled caption update lost")
            fixture.sequence=7;fixture.withdrawn=True
            exhausted=smoke.wait(lambda:(g if (g:=settled_sync(3)) and g["stopped"]=="exhausted" else None),"source request budget did not close")
            require(fixture.calls==before+3 and smoke.cli("archive","record",scheduled_record)["text"] is None,"scheduled withdrawal or exact budget failed")
            time.sleep(5.3)
            require(fixture.calls==before+3,"exhausted sync issued another GET")
            smoke.passed("native_scheduled_updates_withdrawal_restart_and_exact_budget")
            fixture.gate.clear()
            enable_sync(exhausted["revision"],2)
            smoke.wait(lambda:fixture.calls==before+4,"revocation fixture did not enter HTTP")
            running=smoke.cli("archive","knowledge-sync",source)
            smoke.cli("archive","knowledge-sync-revoke",source,"--revision",running["revision"])
            cancelled=smoke.wait(lambda:(j if (j:=smoke.cli("archive","read-job",running["last_read"]))["state"]=="cancelled" else None),"revocation did not cancel scheduled GET")
            require(cancelled["sync_revision"]==running["revision"],"scheduled receipt lost grant epoch")
            fixture.gate.set()
            smoke.stop();smoke.start()
            require(smoke.cli("archive","knowledge-state",source)["last_read"]==exhausted["last_read"],"revoked read advanced committed cursor")
            require(smoke.cli("archive","knowledge-sync",source)["scope"] is None and fixture.calls==before+4,"restart replayed revoked synchronization")
            smoke.passed("native_sync_revoke_cancels_inflight_and_restart_never_replays")
            fixture.fail_status=503
            enable_sync(3,2)
            failed=smoke.wait(lambda:(g if (g:=smoke.cli("archive","knowledge-sync",source))["stopped"]=="read_failed" else None),"HTTP failure did not stop source sync")
            failure=smoke.cli("archive","read-job",failed["last_read"])
            require(failed["used_requests"]==1 and failure["state"]=="failed" and failure["provider_error_code"]==503,"failure lost reserved attempt or safe HTTP context")
            smoke.stop();smoke.start();time.sleep(5.3)
            require(fixture.calls==before+5 and smoke.cli("archive","knowledge-state",source)["last_read"]==exhausted["last_read"],"failed sync retried or advanced cursor")
            policies=smoke.cli("archive","knowledge-sync-grants",source)
            require(len(policies)==4 and sum(g["used_requests"] for g in policies)==5,"policy replacement erased earlier spent reservations")
            smoke.passed("native_sync_http_failure_preserves_attempt_and_stops_without_retry")
        elif args.verify_sync:
            sync_file=root/"hosted-sync.json"
            sync_file.write_text(json.dumps(dict(expected_revision=0,scope=dict(archive_revision=1,
                expires_at_ms=int(time.time()*1000)+60000,max_requests=2,interval_ms=5000,limit=16))),encoding="utf-8")
            smoke.cli("archive","knowledge-sync-enable",source,"--file",sync_file)
            g=smoke.wait(lambda:(g if (g:=smoke.cli("archive","knowledge-sync",source))["used_requests"]==1 and not g["pending"] else None),"first hosted scheduled read failed")
            require(smoke.cli("archive","read-job",g["last_read"])["state"]=="completed","first hosted scheduled read not completed")
            smoke.stop();smoke.start()
            g=smoke.wait(lambda:(g if (g:=smoke.cli("archive","knowledge-sync",source))["stopped"]=="exhausted" else None),"hosted sync did not exhaust exactly two reads")
            require(g["used_requests"]==2 and smoke.cli("archive","read-job",g["last_read"])["state"]=="completed","hosted sync counter or final receipt incorrect")
            smoke.passed("native_hosted_finite_sync_and_restart_preserves_budget")
        require(len(smoke.fixture.requests)==0 and not smoke.fixture.sent and not smoke.cli("work","list"), "pull initiated model QQ send or Work")
        smoke.fixture.healthy()
        smoke.stop()
        with closing(sqlite3.connect(smoke.local/"state.db")) as db:
            require(db.execute("PRAGMA user_version").fetchone()[0]==23,"wrong schema")
            reads=db.execute("SELECT count(*) FROM archive_reads").fetchone()[0]
            require(all(db.execute("SELECT count(*) FROM "+table).fetchone()[0]==0 for table in ("lain_actor_turns","inference_requests","lain_memories")),"pull created Actor inference or semantic memory")
        result=dict(mode="hosted_read_only" if args.cloud_client else "synthetic", checks=smoke.checks, read_jobs=reads,
                    source_http_requests=fixture.calls if fixture else None, model_requests=0, paid_calls=0, real_qq_sends=0,
                    copied_records=len(page["records"]), store_id=store_id)
    finally:
        if smoke:
            smoke.stop()
            if smoke.fixture:
                smoke.fixture.model_release.set();smoke.fixture.login_release.set();smoke.fixture.close()
        if fixture:
            fixture.close()
        require(root.is_relative_to(Path(tempfile.gettempdir()).resolve()) and root.name.startswith("lain-knowledge-bridge-"), "unsafe cleanup target")
        temporary.cleanup()
        for thread in set(threading.enumerate())-before_threads:
            thread.join(10)
            require(not thread.is_alive(), "fixture thread leaked")
    require(not root.exists(), "temporary source copy survived cleanup")
    result["cleanup"]=True
    result["binary_sha256"]={name:hashlib.sha256((bins/executable(name)).read_bytes()).hexdigest() for name in ("lain","laind","lain-test-app-server")}
    print(json.dumps(result,ensure_ascii=False))


if __name__=="__main__":
    try:
        main()
    except Exception as error:
        raise SystemExit("FAIL "+location(error))
