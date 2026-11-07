# W3a execution evidence and explicit permission routing

## W4 device permission receipt and application evidence — 2026-11-03

The optional `execution.approval_reports` list adds cumulative, normalized device
observations to existing events. Empty lists are omitted, so historical evidence
and exact receipt JSON retain their original representation. No schema migration
or extra permission authority is introduced. Each report contains the approval ID,
the exact `DecideDeviceApproval` and `received` or `applied`.

`received` means the native service durably recorded that exact owner receipt in
its original response fence. It does not establish RPC submission or application.
`applied` additionally requires the original local Supervisor approval to be
Resolved with the same choice, scope, session and turn. For Codex this uses the
structured matching `serverRequest/resolved` observation after a recorded native
decision; it does not prove execution of the allowed command or user-goal success.
Pending, Responding, Expired and Unknown remain unconfirmed even if the turn ends.

Cloud validation requires each report to match an existing immutable decision for
this exact command. Known reports cannot disappear or change decision; Applied
cannot regress to Received. Devices report facts only and cannot create owner
decisions. Observation and final-result paths both include reports. A recovered
host retains already reported facts; no report authorizes replay of a response.
Rechecks of cloud/native scope precede creation of the response fence; there is
no extra pre-RPC cloud round trip added merely to display a transient stage.

At most 256 distinct reports fit the existing finite command-event capacity.
Protected native records are bounded at 128 KiB; device HTTP response reads at
256 KiB accommodate a command's current and last-event evidence. All other scope,
output and event limits remain. The terminal compares exact decision identities
and separately labels cloud recording, reported device receipt, and reported
Worker confirmation; missing evidence stays unconfirmed. Old devices can still
report legacy evidence until this command has reported the new cumulative facts.

Permission increment validation passed full368 Rust tests (zero failures, three
existing manual/live ignored), strict workspace checks/build and native16 host,
11 work and8 terminal cases. Exact allow/deny application and a completed turn
without permission resolution are covered; the latter stays Received. Actual VT
labels were also inspected. Later client cancellation validation and per-binary
evidence are recorded in [W4 acceptance](W4_ACCEPTANCE.md). This is not live-provider
or cross-device attestation.

W4b now offers terminal F4 review for the selected device task with full contiguous
scope, only offered choices, a fresh preflight read and preserved unknown RequestId.
Cloud receipt recovery is still not backend application. Contract and ongoing
cross-chain validation: [DEVICE_WORK.md](DEVICE_WORK.md). Historical W3a evidence
below remains separate; no additional permission or automatic approval is enabled.

Selected increment: 2026-11-02. Continues [W2b](DEVICE_COMMANDS.md).
Implementation: Kernel persistence/API and CLI protocol. Validation is recorded
below. **A native device host is not implemented by this protocol increment.** Manual or
synthetic device reports are not proof that Codex actually ran. W3 remains in
progress; W3b now supplies the explicitly configured [native execution service](DEVICE_HOST.md).
Its fixture/real-provider acceptance remains separate. See [ADR 029](adr/029-device-execution-evidence.md).

## Separate records and authority

The original `QueueDeviceWork` remains immutable. Optional `execution` evidence
on each device event now describes a Lain-owned execution UUID, local Lain
session/turn UUIDs, a phase, at most one pending approval and a bounded result.
These are Lain correlation identifiers, never Codex thread/RPC objects. After an
identity is bound it cannot be replaced or removed. Execution and turn IDs remain
exclusive across commands. Schema028's explicit
[continuation family](DEVICE_WORK.md#same-session-follow-up)
allows the original session only for a verified linear follow-up, with a distinct
execution/turn and immutable parent/root binding; it replaces the blanket session
uniqueness while preserving execution/turn uniqueness. No remote
`WorkSession` is fabricated in the cloud's local Supervisor.

The command state and reported phase are different: an expired command stays
Unknown while delayed observations consume their original sequence; reporting
WaitingForApproval in that state does not restore permission to act. Completed
evidence requires an identified turn and result. An Accepted report can only
describe Preparing, not a completed or approval-waiting turn. Existing W2b events
without evidence remain readable and replayable, but cannot route approvals.
Once evidence is present, later events must retain the binding.

`DeviceApprovalDecision` is an immutable administrator decision receipt, not an
execution receipt or a claim that a Worker accepted the decision. Recording it
increments the command revision and publishes a metadata change in the same
transaction. The latest device observation stays unchanged until the device
reports another fact. The existing change notification wakes consumers without
an additional model invocation or polling subsystem.

The owner can decide only a currently pending, exact fingerprint on a Running,
uncancelled, unexpired command belonging to an Active, unexpired device. A device
credential can report its own observations and read its own decision receipts;
it cannot decide or authenticate as the administrator. Revocation/expiry blocks
device receipt reads too. Owner audit reads remain available. Actor task text,
QQ messages, archives and model output have no permission-decision path here.

## Permission scope and retry rules

An approval contains its Lain UUID, kind (1–64 ASCII identifier bytes), description
(1–2048 UTF-8 bytes), truncation flag, 1–3 distinct explicit choices and a SHA-256
fingerprint. The description must convey the actual command/arguments, selected
workspace and any broader effect needed to make the choice meaningful; the cloud
cannot independently verify that a reporting device described these faithfully.
Truncated scope cannot offer `approve_once`; `deny`/`cancel` are still possible.
An old approval UUID cannot reappear with different contents, even after an
intervening report cleared it. A new request needs a new identity.

Fingerprint v1 is SHA-256 over ASCII `lain-device-approval-v1` followed by NUL,
then compact UTF-8 JSON encoding of this ordered array (no ASCII escaping):

```text
[execution_id, session_id, turn_id, approval_id, kind, description, truncated, choices]
```

UUID strings are lowercase canonical hyphenated values; absent IDs use JSON null.
Choice strings use snake_case. The fingerprint field itself is excluded. Rust
and Python share a fixed Unicode test vector. The digest binds what was shown;
it does not authenticate the report or evaluate whether its contents are safe.

Within a command, both approval identity and decision RequestId are single-use.
An exact retry returns the original decision receipt, including after resolution,
cancellation or expiry. A different choice/RequestId for the same approval, or a
reused RequestId for a different approval, conflicts. Recovering this immutable
receipt does **not** authorize replaying a Worker mutation. A future native host
must recheck the current command authority, local request identity, fingerprint,
choices and durable response state immediately before applying it. Responding or
Unknown backend decisions must be reconciled, never blindly replayed. A revoked,
cancelled or expired command cannot use an old receipt to resume effects.

## CLI and HTTP

Inspect `execution.pending_approvals` using the existing owner `show` command.
Write an explicit decision outside Git, using the fingerprint actually inspected:

```json
{
  "request_id": "<new UUID retained for exact retry>",
  "fingerprint": "<64 lowercase hex characters from the pending request>",
  "choice": "approve_once"
}
```

```text
lain device command show <device-id> <command-id>
lain device command decide <device-id> <command-id> <approval-id> --file <decision.json>
lain device command decisions <device-id> <command-id>
lain device command fetch-decisions --directory <protected-device-dir> <command-id>
```

The first three use the administrator client; the last loads only that device's
protected credential and requires no administrator setup. There is no default
approve choice, automatic retry with a new identity, or Worker RPC in these
commands. The W2b immutable event journal also retains full typed evidence;
malformed or secret-bearing evidence is rejected before local journaling.

| Principal | Route under its command prefix | Operation |
| --- | --- | --- |
| Administrator `/v1/devices/{device}/commands/{command}` | `GET /approvals` | Immutable decision receipts |
| Administrator | `POST /approvals/{approval}` | Explicit exact-scope decision |
| Device `/v1/device-agent/{device}/commands/{command}` | `GET /approvals` | Its own receipts only |

All use no-store responses and existing independent credentials. Device routes
never expose a POST decision handler. Bounded evidence is included through the
existing `POST /events`; the 256-event limit and reserved final/reconciliation
slots remain. Choices do not consume device event sequence numbers. HTTP
acknowledgement loss is handled through exact receipt recovery, not re-execution.

## Result facts and storage

`DeviceWorkResult` carries at most 2048 UTF-8 bytes of original output tail,
command failure count, at most 8 relative reported paths (512 bytes each),
withheld-path count and history-truncation flag. Paths cannot contain control
characters, absolute prefixes, drive/scheme colons, backslashes, empty/dot/parent
segments. They are references reported by the device, not verified artifacts.
This protocol opens no referenced file, uploads no bytes and computes no file
hash. A Worker can complete a turn containing failed commands: a Succeeded
command observation does not establish that the user's goal was accepted.

Credentials are rejected in descriptions, output, paths and event details.
Provider request objects and additional unknown fields are rejected. Future
device adapters must redact before constructing an immutable outbox record;
rejecting a secret at the server is not permission to store it locally first.

Migration 026 adds `device_executions` (unique per-device correlation) and
`device_approval_decisions` (unique command/approval and command/RequestId).
Legacy snapshots omit `execution` and deserialize as None. Old receipt JSON
retains its original representation and identity. Schema 25 → 26 and earlier
migrations are tested; there is no down-migration. Back up and restore the whole
protected runtime using the existing upgrade procedure, never copy a personal DB
to invent a cloud identity. Development does not upgrade the personal instance.

## Validation

This is historical W3a evidence before the native host increment. Current W3b
implementation and final validation are recorded in [DEVICE_HOST.md](DEVICE_HOST.md).

Final native Windows GNU validation on 2026-11-02:

| Check | Evidence |
| --- | --- |
| Rust workspace tests | 316 passed, 0 failed, 3 existing manual/real-service cases ignored |
| Formatting, all-target Clippy, workspace build | Passed with locked, offline dependencies |
| Native approval protocol | 6 checks; no Worker/model/real QQ effects |
| W2b command regression | 6 checks; no Worker/model/real QQ effects |
| QQ visual regression | 9 checks; 12 fake model requests and 8 fixture media reads |
| Real old-binary synthetic upgrade | 5 checks; schema24 → 26, old-binary refusal and complete stopped-state rollback; 3 fake requests |
| Python helper tests | 14 tests: 9 passed, 5 skipped |
| Documentation links | 17 changed/new documents, 283 relative links, none broken |

Reproduction commands:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo build --workspace --locked --offline
python scripts/device_approvals_smoke.py --bin-dir <native-binaries>
python scripts/device_commands_smoke.py --bin-dir <native-binaries>
python scripts/qq_visual_smoke.py --bin-dir <native-binaries>
python scripts/qq_memory_smoke.py --bin-dir <native-binaries> --upgrade-from <schema24-binaries>
```

The process rehearsal deliberately posts synthetic observations, not fake
completion of a real native Worker. It must produce zero model calls, Worker
executions and real QQ sends. Native Codex permissions/cancellation, host restart,
real cross-device TLS, Linux/macOS and user-facing chat/dashboard integration
remain unaccepted.

All four final process reports use the same binaries:

```text
lain.exe  e1de81d44543150eb042a1778a8afa3b7f9dc5bbb755598f9497263f141eb578
laind.exe 0341bf34b81a95cf5acf53b8d96f5daa00ac77a5dfafce408caf9cbe98dcf3b7
```

External local evidence (not public links), under
`%TEMP%/lain-q1-tools/`:
`w3a-workspace-tests.log`, `w3a-clippy.log`, `w3a-build.log`, `w3a-process.log`,
`w3a-command-regression.log`, `w3a-visual-regression.log`, `w3a-upgrade.log` and
`w3a-python-unit.log`. Temporary services/state were cleaned up. The same
elevation-specific fresh-path owner adapter described in
[W2b validation](DEVICE_COMMANDS.md) preserves actual user SID ownership and uses
`/COPY:DATSO` for synthetic backup/restore. The upgrade report's hardcoded
`COPY:DATS` label does not reflect that owner-preserving option. No existing user
path was repaired and no owner/credential check was weakened.

The protected personal runtime remains on `releases/visual-9b40de0`; it was not
migrated or restarted by these tests. No actual device installation, deployment,
paid model invocation, native MSVC acceptance or real artifact verification is
claimed. The migration unit test also covers schema25 receipts and bindings;
the old/new executable rehearsal specifically starts from schema24.

## W3b implementation handoff

The selected native path now implements this handoff; see [DEVICE_HOST.md](DEVICE_HOST.md)
for its separate deterministic/live scope and remaining gaps.

Implement one explicit native device host around the existing Supervisor and
structured Codex adapter, with protected local policy/journal, exact native
workspace/profile/version checks and separate device credentials. It owns only
Worker operational state, not another Actor/persona/identity. Persist launch and
submission fences before effects; reconcile uncertain provider-bound sessions,
never create a replacement thread/turn just because an acknowledgement was lost.
Enforce the decision replay rules above, confirm actual cancellation separately
from requests, and retain uncertainty on disconnect or unknown backend state.
Choose bounded network-loss/revocation behavior explicitly and test it using the
structured fixture before enabling real work. No Worker intelligence rewrite,
intent enhancement compiler, global backend config edit or automatic approval is
selected. W4 remains the terminal/chat/dashboard integration.
