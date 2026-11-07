"""Fault cases for the one-shot32->39 maintenance flow, no live services."""
import unittest

from cloud_cutover import cutover


class Operations:
    def __init__(self, failure=None, running_after_failure=False, compatible=False):
        self.events = []
        self.failure = failure
        self.running_after_failure = running_after_failure
        self.compatible = compatible
        self.state = "running"
        self.current = "previous"
        self.record_exists = False
        self.data = b"synthetic accepted user data"
        self.final_phase = None

    def action(self, name):
        self.events.append(name)
        if self.failure == name:
            if self.running_after_failure:
                self.state = "running"
            raise TimeoutError("synthetic observation or command failure")

    def reserve(self):
        if self.record_exists:
            raise FileExistsError("operation already accepted")
        self.record_exists = True
        self.action("reserve")

    def preflight(self): self.action("preflight")
    def publish(self): self.action("publish")
    def stop(self):
        self.action("stop")
        self.state = "stopped"
    def backup_and_verify(self): self.action("backup")
    def select_candidate(self):
        self.action("select_candidate")
        self.current = "candidate"
    def start(self):
        self.action("start")
        self.state = "running"
    def verify(self): self.action("verify")
    def service_state(self):
        self.action("observe")
        return self.state
    def previous_compatible_and_unchanged(self):
        self.action("unchanged_check")
        return self.compatible
    def select_previous(self):
        self.action("select_previous")
        self.current = "previous"
    def start_previous(self):
        self.action("start_previous")
        self.state = "running"
    def verify_previous(self): self.action("verify_previous")
    def phase(self, name, **metadata):
        self.events.append("phase:" + name)
        self.final_phase = name
    def finish(self, preserve):
        self.events.append("preserve" if preserve else "cleanup")


class CutoverTests(unittest.TestCase):
    def test_success_has_fresh_stopped_backup_before_select_and_start(self):
        ops = Operations()
        self.assertTrue(cutover(ops))
        relevant = [e for e in ops.events if not e.startswith("phase:")]
        self.assertEqual(relevant, ["reserve", "preflight", "publish", "stop", "backup",
                                   "select_candidate", "start", "verify", "cleanup"])
        self.assertEqual(ops.final_phase, "loopback_ready_pending_public_acceptance")

    def test_backup_failure_resumes_previous_without_selection_or_data_restore(self):
        ops = Operations(failure="backup")
        self.assertFalse(cutover(ops))
        self.assertEqual((ops.current, ops.state), ("previous", "running"))
        self.assertNotIn("select_candidate", ops.events)
        self.assertEqual(ops.data, b"synthetic accepted user data")

    def test_running_candidate_is_not_bounced_after_observation_timeout(self):
        ops = Operations(failure="verify")
        self.assertFalse(cutover(ops))
        self.assertEqual((ops.current, ops.state), ("candidate", "running"))
        self.assertEqual(ops.events.count("stop"), 1)
        self.assertEqual(ops.events.count("start"), 1)
        self.assertNotIn("select_previous", ops.events)
        self.assertEqual(ops.final_phase, "attention_preserved")
        self.assertEqual(ops.events[-1], "preserve")

    def test_dead_candidate_with_changed_or_new_schema_never_restores_data(self):
        ops = Operations(failure="start", compatible=False)
        self.assertFalse(cutover(ops))
        self.assertEqual(ops.current, "candidate")
        self.assertNotIn("select_previous", ops.events)
        self.assertEqual(ops.data, b"synthetic accepted user data")
        self.assertEqual(ops.final_phase, "attention_preserved")

    def test_dead_unmigrated_unchanged_candidate_can_select_previous_binary_only(self):
        ops = Operations(failure="start", compatible=True)
        self.assertFalse(cutover(ops))
        self.assertEqual((ops.current, ops.state), ("previous", "running"))
        self.assertEqual(ops.data, b"synthetic accepted user data")
        self.assertEqual(ops.final_phase, "failed_previous_running_without_restore")

    def test_preflight_failure_does_not_stop_or_start_a_service(self):
        ops = Operations(failure="preflight")
        self.assertFalse(cutover(ops))
        self.assertNotIn("stop", ops.events)
        self.assertNotIn("start", ops.events)
        self.assertNotIn("start_previous", ops.events)

    def test_lost_observer_does_not_authorize_replaying_operation(self):
        ops = Operations()
        self.assertTrue(cutover(ops))
        before = list(ops.events)
        with self.assertRaises(FileExistsError):
            cutover(ops)
        self.assertEqual(ops.events, before)


if __name__ == "__main__":
    unittest.main()
