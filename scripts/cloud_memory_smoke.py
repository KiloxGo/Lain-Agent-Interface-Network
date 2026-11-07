"""Actual HTTPS scoped-memory acceptance with a disposable Linux Kernel.

Native Windows CLI/ConPTY, synthetic records and bounded local model only.
Requires the separately provisioned candidate fixture; never migrates personal data.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile
import time
import urllib.parse
import urllib.request
import uuid

from cloud_worker_smoke import Suite, require


def identity():
    return str(uuid.uuid4())


class MemorySuite(Suite):
    def strict_scopes(self, conversation, definitions):
        context = self.bootstrap(conversation)["memory_context"]
        selection = context["selection"] if context else None
        scopes = [{"kind": "global"}, {"kind": "internal"}]
        require(selection is not None, "this remote-scope acceptance needs a selected device")
        scopes.extend([{"kind": "device", "device_id": selection["device_id"]},
                       {"kind": "device_workspace", "device_id": selection["device_id"], "workspace_id": selection["workspace_id"]}])
        if selection["project"]:
            scopes.append({"kind": "project", "project_id": selection["project"]["project_id"]})
        expected = {json.dumps(scope, sort_keys=True) for scope in scopes}
        for name in ("memory_search", "memory_list", "memory_inspect", "memory_propose"):
            tool = next(t for t in definitions if t["name"] == name)
            require(tool.get("strict") is True and tool["parameters"]["additionalProperties"] is False,
                    "memory capability lost strict object boundary")
            properties = tool["parameters"]["properties"]
            require(tool["parameters"]["type"] == "object" and set(tool["parameters"]["required"]) == set(properties),
                    "strict tool omitted object type or required arguments")
            if name == "memory_propose":
                draft = properties["draft"]
                require(draft["type"] == "object" and draft["additionalProperties"] is False
                        and set(draft["required"]) == set(draft["properties"]), "proposal draft is not a closed strict object")
            scope = properties["draft"]["properties"]["scope"] if name == "memory_propose" else properties["scope"]
            actual = []
            for variant in scope["anyOf"]:
                require(variant["type"] == "object" and variant["additionalProperties"] is False
                        and set(variant["required"]) == set(variant["properties"]), "invalid strict scope object")
                record = {}
                for key, value in variant["properties"].items():
                    require(value.get("type") == "string" and len(value.get("enum", [])) == 1,
                            "scope IDs/kind are not exact selected enums")
                    record[key] = value["enum"][0]
                actual.append(json.dumps(record, sort_keys=True))
            require(set(actual) == expected and len(actual) == len(expected),
                    "scope schema omitted selected IDs or offered foreign/path/channel branches")

    def transcript_excludes(self, markers):
        # Terminal.text() is one viewport. Sweep contiguous PgUp pages all the
        # way to a stable top, checking each; restore ordinary follow afterward.
        self.terminal.write("\x1b[1;5F")
        time.sleep(.15)
        stable = 0
        for _ in range(100):
            before = self.terminal.text()
            require(not any(marker in before for marker in markers), "withdrawn body retained in terminal scroll history")
            self.terminal.write("\x1b[5~")
            time.sleep(.15)
            after = self.terminal.text()
            require(not any(marker in after for marker in markers), "withdrawn body survived above the current viewport")
            stable = stable + 1 if before == after else 0
            if stable == 3:
                self.terminal.write("\x1b[1;5F")
                return
        raise AssertionError("did not reach transcript top within bounded page sweep")

    def check_cache(self, markers):
        cache = self.root / "client/client-sync-cache.json"
        require(cache.is_file() and cache.stat().st_size <= 16384, "private bounded metadata cache missing")
        require(not any(value in cache.read_text() for value in [*markers, *self.secrets]),
                "local metadata cache retained semantic bodies or secrets")
        self.facts["metadata_cache_bytes"] = cache.stat().st_size

    def conversation_new(self):
        return self.call("POST", "/v1/conversations", {}, status=201)["id"]

    def bootstrap(self, conversation):
        return self.call("POST", "/v1/client-sync/bootstrap", {"protocol": 1, "conversation_id": conversation})

    def select(self, conversation, device, project=None):
        path = "/v1/conversations/" + conversation + "/memory-context"
        previous = self.call("GET", path)
        selection = {"device_id": device["id"], "workspace_id": device["workspaces"][0]["id"],
                     "device_revision": 2, "project": project}
        return self.call("POST", path, {"request_id": identity(), "expected_revision": previous["revision"] if previous else 0,
                                        "selection": selection})

    def add(self, scope, marker):
        return self.call("POST", "/v1/memory", {"request_id": identity(), "draft": {
            "content": marker, "kind": "episodic", "scope": scope}})

    def turn(self, turn_id):
        return self.call("GET", "/v1/actor-turns/" + turn_id)

    def probe(self, conversation, record, expected=True, propose=None):
        prompt = "cloud memory probe " + identity()
        steps = [{"name": "memory_inspect", "arguments": {"memory_id": record["id"], "scope": record["scope"]}}]
        if propose:
            steps.append({"name": "memory_propose", "arguments": {"draft": propose}})
        before = self.control("stats")["model_calls"]
        self.control("memory_probe", prompt=prompt, steps=steps)
        submitted = self.call("POST", "/v1/conversations/" + conversation + "/turns",
                              {"request_id": identity(), "text": prompt}, status=202)
        current = self.wait(lambda: t if (t := self.turn(submitted["id"]))["state"] != "running" else None,
                            "memory probe did not settle")
        wanted = "pending_confirmation" if propose else "completed"
        require(current["state"] == wanted, "memory probe did not reach " + wanted)
        requests = self.control("model_inputs")["requests"][before:]
        require(len(requests) == 2, "one inspect must produce exactly two bounded model requests")
        self.strict_scopes(conversation, requests[0]["tools"])
        actions = [a for a in current["actions"] if a["tool"]["name"] == "memory.inspect"]
        require(len(actions) == 1, "missing exact typed memory inspect action")
        action = actions[0]
        require(action["tool"]["arguments"] == steps[0]["arguments"], "memory inspect target changed")
        encoded = json.dumps(requests)
        if expected:
            require(action["state"] == "executed", "allowed memory inspect was not executed")
            selected = action["result"]["record"]
            require((selected["id"], selected["revision"], selected["scope"]) ==
                    (record["id"], record["revision"], record["scope"]), "retrieved record identity/version/scope differ")
            require(record["content"] in json.dumps(requests[-1]["input"]), "selected memory never reached model context")
            self.facts.setdefault("verified_retrieval_receipts", []).append({"memory_id": selected["id"],
                "revision": selected["revision"], "scope": selected["scope"], "authority": selected["provenance"]["authority"]})
        else:
            require(action["state"] == "failed", "forbidden memory read was executed")
            require(record["content"] not in encoded and record["content"] not in json.dumps(current),
                    "forbidden body leaked into model input or checked Actor view")
        return current, requests

    def invalidation(self, snapshot):
        cursor = snapshot["cursor"]
        query = urllib.parse.urlencode({"instance_id": cursor["instance_id"], "boot_id": cursor["boot_id"],
                                        "after": cursor["revision"], "lease_until_ms": snapshot["valid_until_ms"]})
        request = urllib.request.Request(self.endpoint + "/v1/client-sync/events?" + query,
                                         headers={"Authorization": "Bearer " + self.owner})
        total = 0
        with self.opener.open(request, timeout=60) as response:
            require(response.headers.get("Content-Type", "").startswith("text/event-stream"), "not an SSE event")
            while True:
                line = response.readline()
                total += len(line)
                require(total <= 16384 and line, "missing bounded invalidation event")
                if line.startswith(b"data:"):
                    value = json.loads(line[5:])
                    require(set(value) == {"cursor", "reason"}, "invalidation carried source bodies")
                    return value

    def derive(self, conversation, record, marker):
        current, _ = self.probe(conversation, record, propose={"content": marker, "kind": "episodic", "scope": record["scope"]})
        action = next(a for a in current["actions"] if a["tool"]["name"] == "memory.propose")
        candidate = self.call("GET", "/v1/memory-candidates/" + action["id"])
        require(candidate["status"] == "candidate" and candidate["memory_id"] is None, "proposal activated before review")
        require(candidate["provenance"]["authority"] == "inferred", "derived memory claimed explicit authority")
        self.call("POST", "/v1/actor-turns/" + current["id"] + "/actions/" + action["id"] + "/decision", {"approve": True})
        self.wait(lambda: self.turn(current["id"])["state"] == "completed", "reviewed derived memory did not settle")
        accepted = self.call("GET", "/v1/memory-candidates/" + action["id"])
        require(accepted["status"] == "accepted", "owner decision did not accept exact candidate")
        derived = self.call("GET", "/v1/memory/" + accepted["memory_id"])
        require(derived["content"] == marker and derived["scope"] == record["scope"]
                and derived["provenance"]["authority"] == "inferred"
                and derived["origin"]["kind"] == "owner_selected_actor_context", "derived provenance/scope changed")
        return derived

    def run(self):
        stats = self.control("stats")
        require(stats["model_calls"] == 0 and stats["kernel_binary_sha256"] == os.environ["LAIN_CLOUD_FIXTURE_BINARY_SHA256"],
                "fresh tested candidate was not actually launched")
        a_dir, a, proof_a = self.enroll("memory-device-a")
        _, b, proof_b = self.enroll("memory-device-b")
        require(a["workspaces"][0]["path"] == b["workspaces"][0]["path"], "equal Windows path fixture differs")
        ca, cb = self.conversation_new(), self.conversation_new()
        self.select(ca, a)
        self.select(cb, b)
        self.call("POST", "/v1/client-sync/bootstrap", {"protocol": 1, "conversation_id": ca}, status=401, credential=proof_a)
        self.call("GET", "/v1/conversations/" + ca + "/memory-context", status=401, credential=proof_b)
        token = uuid.uuid4().hex[:12]
        local_scope = {"kind": "device_workspace", "device_id": a["id"], "workspace_id": a["workspaces"][0]["id"]}
        local = self.add(local_scope, "A-WORKSPACE-" + token)
        device = self.add({"kind": "device", "device_id": a["id"]}, "A-ENVIRONMENT-" + token)
        global_fact = self.add({"kind": "global"}, "GLOBAL-RELATION-" + token)
        private_channel = self.add({"kind": "channel", "channel_id": identity()}, "CHANNEL-PRIVATE-" + token)
        self.probe(ca, local)
        self.probe(cb, local, expected=False)
        self.probe(ca, device)
        self.probe(cb, device, expected=False)
        self.passed("equal_native_windows_paths_do_not_share_device_or_workspace_memory_through_owner_model_requests")

        project = self.call("POST", "/v1/projects", {"id": identity(), "request_id": identity(), "name": "shared synthetic project"})
        project_fact = self.add({"kind": "project", "project_id": project["id"]}, "PROJECT-SHARED-" + token)
        self.probe(cb, project_fact, expected=False)
        project_contexts = {}
        for registration, conversation in ((a, ca), (b, cb)):
            mapping = self.call("POST", "/v1/project-workspaces", {"request_id": identity(), "device_id": registration["id"],
                "workspace_id": registration["workspaces"][0]["id"], "expected_device_revision": 2, "expected_revision": 0,
                "project_id": project["id"], "expected_project_revision": project["revision"]})
            selected = {"project_id": project["id"], "project_revision": project["revision"], "mapping_revision": mapping["revision"]}
            self.select(conversation, registration, selected)
            project_contexts[registration["id"]] = selected
            self.probe(conversation, project_fact)
        self.probe(cb, local, expected=False)
        # Generic channel scope is rejected at the typed tool boundary. Forging
        # a permitted global scope around the exact private ID must also fail.
        forged = dict(private_channel, scope={"kind": "global"})
        self.probe(cb, forged, expected=False)
        self.probe(cb, global_fact)
        self.passed("explicit_project_mapping_shares_only_project_knowledge_and_never_promotes_device_or_channel_facts")

        self.select(ca, b, project_contexts[b["id"]])
        switched = self.bootstrap(ca)
        require(local["content"] not in json.dumps(switched) and device["content"] not in json.dumps(switched), "context switch restored private history")
        _, requests = self.probe(ca, global_fact)
        require(local["content"] not in json.dumps(requests) and device["content"] not in json.dumps(requests), "context switch reused private model history")
        self.passed("context_switch_removes_prior_device_history_from_bootstrap_and_later_model_input")

        self.conversation = self.conversation_new()
        self.select(self.conversation, a, project_contexts[a["id"]])
        derived = self.derive(self.conversation, local, "DERIVED-PRIVATE-" + token)
        visible, _ = self.probe(self.conversation, derived)
        self.open_terminal()
        self.conversation_visible(derived["content"])
        self.terminal.write("retained memory draft")
        self.visible("retained memory draft")
        snapshot = self.bootstrap(self.conversation)
        corrected = self.add(local_scope, "CORRECTED-PRIVATE-" + token)
        self.call("POST", "/v1/memory/" + local["id"] + "/status", {
            "expected_revision": local["revision"], "status": "superseded", "superseded_by": corrected["id"]})
        event = self.invalidation(snapshot)
        require(event["reason"] == "changed" and event["cursor"]["revision"] > snapshot["cursor"]["revision"], "snapshot/subscription gap lost correction")
        self.wait(lambda: derived["content"] not in self.terminal.text(), "terminal retained withdrawn derived output")
        self.visible("retained memory draft")
        checked = self.turn(visible["id"])
        require(checked.get("availability") == "source_unavailable" and not checked["messages"], "derived reply remained publishable")
        snapshot = self.bootstrap(self.conversation)
        require(derived["content"] not in json.dumps(snapshot) and local["content"] not in json.dumps(snapshot), "bootstrap revived superseded evidence")
        _, requests = self.probe(self.conversation, corrected)
        require(local["content"] not in json.dumps(requests) and derived["content"] not in json.dumps(requests), "corrected model request reused withdrawn source")
        self.conversation_visible(corrected["content"])
        self.visible("retained memory draft")
        self.transcript_excludes([local["content"], derived["content"]])
        self.check_cache([local["content"], derived["content"], corrected["content"]])
        self.close_terminal()
        self.open_terminal()
        self.conversation_visible(corrected["content"])
        self.transcript_excludes([local["content"], derived["content"]])
        self.passed("reviewed_inferred_memory_correction_invalidates_live_scrolled_cli_history_model_input_and_bounded_cache")

        forgotten = self.derive(self.conversation, corrected, "DERIVED-FORGET-" + token)
        forget_turn, _ = self.probe(self.conversation, forgotten)
        self.conversation_visible(forgotten["content"])
        self.terminal.write("preserved forget draft")
        self.visible("preserved forget draft")
        snapshot = self.bootstrap(self.conversation)
        require(forgotten["content"] in json.dumps(snapshot), "forget source was already unavailable before deletion")
        self.call("POST", "/v1/memory/" + corrected["id"] + "/delete", {"expected_revision": corrected["revision"]})
        require(self.invalidation(snapshot)["reason"] == "changed", "active-source deletion was not observed")
        self.wait(lambda: forgotten["content"] not in self.terminal.text(), "terminal retained deleted-source output")
        self.visible("preserved forget draft")
        require(self.turn(forget_turn["id"]).get("availability") == "source_unavailable", "delete did not invalidate active derived reply")
        require(forgotten["content"] not in json.dumps(self.bootstrap(self.conversation)), "delete revived derived startup content")
        _, requests = self.probe(self.conversation, global_fact)
        require(forgotten["content"] not in json.dumps(requests) and corrected["content"] not in json.dumps(requests), "post-delete model history resurrected source")
        self.conversation_visible(global_fact["content"])
        self.transcript_excludes([local["content"], derived["content"], corrected["content"], forgotten["content"]])
        self.check_cache([corrected["content"], forgotten["content"], global_fact["content"]])
        self.close_terminal()
        self.open_terminal()
        self.conversation_visible(global_fact["content"])
        self.transcript_excludes([corrected["content"], forgotten["content"]])
        self.close_terminal()
        self.call("GET", "/v1/memory/" + corrected["id"], status=404)
        self.call("GET", "/v1/memory/" + forgotten["id"], status=503)
        self.call("GET", "/v1/memory/" + derived["id"], status=503)
        self.passed("deleting_a_fresh_active_source_invalidates_derived_ui_bootstrap_model_cache_and_reopened_client")

        # A short-lived third device supplies a real deadline without sleeping
        # until the main devices' 30-minute authority ends.
        server_ms = self.bootstrap(None)["issued_at_ms"]
        expiry = server_ms + 45000
        directory = self.root / "memory-expiring-device"
        self.cli("device", "init", "--directory", directory, "--endpoint", self.endpoint, "--workspace", self.workspace,
                 "--label", "synthetic expiring device", "--expires-at-ms", expiry, native=True)
        expiring = json.loads((directory / "registration.json").read_text())
        self.secrets.append(json.loads((directory / "device-credential.json").read_text())["token"])
        self.cli("device", "pair", "--file", directory / "registration.json")
        self.cli("device", "claim", "--directory", directory, native=True)
        ce = self.conversation_new()
        self.select(ce, expiring)
        expiring_fact = self.add({"kind": "device", "device_id": expiring["id"]}, "EXPIRING-PRIVATE-" + token)
        expiring_turn, _ = self.probe(ce, expiring_fact)
        expiry_snapshot = self.bootstrap(ce)
        require(expiry_snapshot["valid_until_ms"] <= expiry, "bootstrap lease outlived source device authority")
        event = self.invalidation(expiry_snapshot)
        require(event["reason"] == "expired", "source deadline did not invalidate idle HTTPS subscription")
        expired = self.bootstrap(ce)
        require(expiring_fact["content"] not in json.dumps(expired)
                and self.turn(expiring_turn["id"]).get("availability") == "source_unavailable", "expiry revived private evidence")
        self.passed("real_device_expiry_bounds_bootstrap_lease_and_invalidates_idle_https_subscription_and_history")

        cr = self.conversation_new()
        self.select(cr, a, project_contexts[a["id"]])
        revoke_source = self.add(local_scope, "ACTIVE-REVOCATION-" + token)
        revoke_turn, _ = self.probe(cr, revoke_source)
        require(revoke_source["content"] in json.dumps(self.bootstrap(cr)), "revocation source was not available before revoke")
        self.cli("device", "revoke", a["id"], "--revision", 2)
        revoked = self.bootstrap(cr)
        require(revoke_source["content"] not in json.dumps(revoked)
                and self.turn(revoke_turn["id"]).get("availability") == "source_unavailable", "revoked device scope remained publishable")
        stats = self.control("stats")
        require(stats["model_calls"] < 64 and not stats["errors"], "fixture exceeded budget or recorded errors")
        require(self.call("GET", "/v1/work-sessions") == [] and not list(self.root.glob("*/execution")), "memory acceptance started a Worker/local persona")
        self.facts["candidate_binary_sha256"] = stats["kernel_binary_sha256"]
        self.facts["model_calls"] = stats["model_calls"]
        self.passed("device_revocation_withholds_history_without_worker_launch_or_paid_or_channel_effects")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--portrait-file", type=Path)
    args = parser.parse_args()
    require(os.name == "nt", "native Windows required")
    args.bin_dir, args.artifacts = args.bin_dir.resolve(), args.artifacts.resolve()
    require(not args.artifacts.is_relative_to(Path(__file__).resolve().parents[1]), "artifacts must be outside checkout")
    args.artifacts.mkdir(parents=True, exist_ok=False)
    parent = str(args.artifacts)
    if not parent.startswith("\\\\?\\"):
        parent = "\\\\?\\UNC\\" + parent[2:] if parent.startswith("\\\\") else "\\\\?\\" + parent
    with tempfile.TemporaryDirectory(prefix="lain-cloud-memory-", dir=parent) as raw:
        suite = MemorySuite(args, Path(raw))
        try:
            suite.run()
        finally:
            try:
                (args.artifacts / "partial-checks.json").write_text(json.dumps(suite.checks, indent=2))
                (args.artifacts / "facts.json").write_text(json.dumps(suite.facts, indent=2))
                (args.artifacts / "model-inputs.json").write_text(json.dumps(suite.control("model_inputs"), indent=2))
                if suite.terminal:
                    suite.terminal.image(args.artifacts / "last-terminal.png")
                    (args.artifacts / "last-terminal.txt").write_text(suite.terminal.text(), encoding="utf-8")
            finally:
                suite.close()
    report = {"checks": suite.checks, "facts": suite.facts, "temporary_windows_state_cleaned": not Path(raw).exists(),
              "actual_https_certificate_verified": True, "paid_calls": 0, "qq_calls": 0,
              "native_platform": "Windows client and actual Linux Kernel", "fixture_only": True,
              "client_binary_sha256": hashlib.sha256((args.bin_dir / "lain.exe").read_bytes()).hexdigest()}
    (args.artifacts / "report.json").write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2), flush=True)


if __name__ == "__main__":
    main()
