"""Acceptance-assertion regressions; no network, native process or credentials."""
import copy
import unittest
from unittest.mock import patch

from cloud_memory_smoke import MemorySuite


class AcceptanceGuardTests(unittest.TestCase):
    def setUp(self):
        self.suite = MemorySuite.__new__(MemorySuite)
        selection = {"device_id": "selected-device", "workspace_id": "selected-workspace", "project": None}
        self.suite.bootstrap = lambda _: {"memory_context": {"selection": selection}}
        scopes = [{"kind": "global"}, {"kind": "internal"},
                  {"kind": "device", "device_id": "selected-device"},
                  {"kind": "device_workspace", "device_id": "selected-device", "workspace_id": "selected-workspace"}]
        variants = [{"type": "object", "additionalProperties": False, "required": list(scope),
                     "properties": {k: {"type": "string", "enum": [v]} for k, v in scope.items()}} for scope in scopes]
        self.definitions = []
        for name in ("memory_search", "memory_list", "memory_inspect", "memory_propose"):
            scope = {"anyOf": copy.deepcopy(variants)}
            properties = {"draft": {"type": "object", "properties": {"scope": scope}, "required": ["scope"], "additionalProperties": False}} if name == "memory_propose" else {"scope": scope}
            self.definitions.append({"name": name, "strict": True, "parameters": {
                "type": "object", "properties": properties, "required": list(properties), "additionalProperties": False}})

    def schema(self, definition):
        p = definition["parameters"]["properties"]
        return p["draft"]["properties"]["scope"] if definition["name"] == "memory_propose" else p["scope"]

    def test_exact_schema_is_accepted(self):
        self.suite.strict_scopes("synthetic-conversation", self.definitions)

    def test_each_memory_tool_rejects_added_foreign_scope(self):
        for index in range(4):
            definitions = copy.deepcopy(self.definitions)
            foreign = {"type": "object", "additionalProperties": False, "required": ["kind", "device_id"],
                       "properties": {"kind": {"type": "string", "enum": ["device"]},
                                      "device_id": {"type": "string", "enum": ["unselected-device"]}}}
            self.schema(definitions[index])["anyOf"].append(foreign)
            with self.assertRaises(AssertionError):
                self.suite.strict_scopes("synthetic-conversation", definitions)

    def test_each_memory_tool_requires_strict_provider_mode(self):
        for index in range(4):
            definitions = copy.deepcopy(self.definitions)
            definitions[index]["strict"] = False
            with self.assertRaises(AssertionError):
                self.suite.strict_scopes("synthetic-conversation", definitions)

    def test_scope_required_and_additional_property_guards_are_observed(self):
        for mutation in ("required", "additionalProperties"):
            definitions = copy.deepcopy(self.definitions)
            branch = self.schema(definitions[0])["anyOf"][-1]
            branch[mutation] = [] if mutation == "required" else True
            with self.assertRaises(AssertionError):
                self.suite.strict_scopes("synthetic-conversation", definitions)

    def test_root_and_proposal_draft_must_be_closed_objects(self):
        for location, mutation in (("root", "type"), ("draft", "type"),
                                   ("draft", "required"), ("draft", "additionalProperties")):
            definitions = copy.deepcopy(self.definitions)
            parameters = definitions[-1]["parameters"]
            target = parameters if location == "root" else parameters["properties"]["draft"]
            target[mutation] = "string" if mutation == "type" else [] if mutation == "required" else True
            with self.assertRaises(AssertionError):
                self.suite.strict_scopes("synthetic-conversation", definitions)

    def test_page_sweep_detects_offscreen_withdrawn_text(self):
        class Terminal:
            index = 3

            def text(self):
                return ["top", "withdrawn-source-marker", "middle", "latest clean reply"][self.index]

            def write(self, key):
                self.index = max(0, self.index - 1) if key == "\x1b[5~" else 3

        self.suite.terminal = Terminal()
        with patch("cloud_memory_smoke.time.sleep"):
            with self.assertRaisesRegex(AssertionError, "withdrawn"):
                self.suite.transcript_excludes(["withdrawn-source-marker"])


if __name__ == "__main__":
    unittest.main()
