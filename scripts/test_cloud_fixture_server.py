"""Deterministic model fixture contracts; no listener, provider or credentials."""
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest

from cloud_fixture_server import Fixture


class FixtureModelTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        root = Path(self.directory.name)
        (root / "fixture.json").write_text(json.dumps({
            "owner_token": "synthetic-owner-proof",
            "model_token": "synthetic-model-proof",
        }))
        self.fixture = Fixture(root)
        self.fixture.kernel = SimpleNamespace(pid=1, poll=lambda: None)

    def respond(self, items):
        return self.fixture.respond({"store": False, "stream": True, "input": items})

    def test_old_tool_result_does_not_replace_a_new_parallel_chat_reply(self):
        output = self.respond([
            {"role": "user", "content": "cloud-delegate"},
            {"type": "function_call_output", "output": "recorded earlier"},
            {"role": "assistant", "content": "earlier acknowledgement"},
            {"role": "user", "content": "cloud parallel chat"},
        ])
        self.assertEqual(output[0]["content"][0]["text"],
                         "fixture reply: cloud parallel chat")

    def test_current_tool_result_is_acknowledged_without_dispatching_again(self):
        output = self.respond([
            {"role": "user", "content": "cloud-delegate"},
            {"type": "function_call_output", "output": "recorded now"},
        ])
        self.assertEqual(output[0]["content"][0]["text"],
                         "fixture acknowledged the recorded result")

    def test_kernel_evidence_frame_is_data_instead_of_the_fixture_user_command(self):
        output = self.respond([
            {"role": "user", "content": "cloud parallel chat"},
            {"role": "user", "content": "Kernel-attributed last-known device work reports: synthetic data"},
        ])
        self.assertEqual(output[0]["content"][0]["text"],
                         "fixture reply: cloud parallel chat")

    def test_memory_probe_uses_only_current_tool_results_as_reply_evidence(self):
        prompt = "cloud memory probe synthetic-request"
        self.fixture.control({"action": "memory_probe", "prompt": prompt, "steps": [
            {"name": "memory_inspect", "arguments": {"memory_id": "synthetic-id", "scope": {"kind": "global"}}},
        ]})
        history = [{"role": "user", "content": "cloud memory probe earlier-request"},
                   {"type": "function_call_output", "output": "old-private-marker"},
                   {"role": "user", "content": prompt}]
        first = self.respond(history)
        self.assertEqual(first[0]["name"], "memory_inspect")
        second = self.respond(history + [{"type": "function_call_output", "output": "current-evidence-marker"}])
        text = second[0]["content"][0]["text"]
        self.assertIn("current-evidence-marker", text)
        self.assertNotIn("old-private-marker", text)

    def test_memory_probe_cannot_invoke_other_tools_or_exceed_step_budget(self):
        for steps in [[{"name": "device_queue", "arguments": {}}],
                      [{"name": "memory_inspect", "arguments": {}}] * 3]:
            with self.assertRaises(ValueError):
                self.fixture.control({"action": "memory_probe", "prompt": "cloud memory probe rejected", "steps": steps})


if __name__ == "__main__":
    unittest.main()
