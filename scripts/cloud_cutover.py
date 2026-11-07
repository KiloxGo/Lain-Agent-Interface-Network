"""One-shot schema32->39 cloud cutover. Never restores a production database."""
import re


class MaintenanceCheckFailed(ValueError):
    """A trusted static check name, never an external error body or supplied value."""
    def __init__(self, check):
        self.check = re.sub("[^a-z0-9]+", "_", check.lower()).strip("_")
        if not 1 <= len(self.check) <= 80:
            raise ValueError("invalid maintenance check name")
        super().__init__(self.check)


def cutover(ops):
    # A repeated/lost-observer request must not touch its accepted operation.
    ops.reserve()
    stop_requested = selected = False
    preserve = True
    try:
        ops.preflight()
        ops.publish()
        ops.phase("stopping")
        stop_requested = True
        ops.stop()
        ops.phase("stopped")
        ops.backup_and_verify()
        ops.phase("selecting")
        selected = True
        ops.select_candidate()
        ops.phase("selected")
        ops.start()
        ops.verify()
        ops.phase("loopback_ready_pending_public_acceptance")
        preserve = False
        return True
    except Exception as error:
        metadata = {"error_type": type(error).__name__}
        if isinstance(error, MaintenanceCheckFailed):
            metadata["check"] = error.check
        ops.phase("failure_observed", **metadata)
        # A timeout is an observation, not permission to bounce a live service.
        try:
            state = ops.service_state()
        except Exception:
            state = "unknown"
        if stop_requested and not selected and state == "stopped":
            ops.start_previous()
            ops.verify_previous()
            ops.phase("aborted_previous_running")
            preserve = False
        elif selected and state == "stopped" and ops.previous_compatible_and_unchanged():
            ops.select_previous()
            ops.start_previous()
            ops.verify_previous()
            ops.phase("failed_previous_running_without_restore")
            preserve = False
        else:
            ops.phase("attention_preserved")
        return False
    finally:
        ops.finish(preserve=preserve)
