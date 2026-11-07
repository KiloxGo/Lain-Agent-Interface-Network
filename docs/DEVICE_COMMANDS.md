# W2b device command protocol

W4b adds optional conversation-bound dispatch with atomic origin/Actor receipts
and current snapshot reads, without changing the original work input/RequestId or
device-principal ledger. Owner CLI adds `queue --conversation <id>`; existing
unlinked commands cannot be adopted afterwards. Current source schema is027;
historical validation below retains its original scope. See [DEVICE_WORK.md](DEVICE_WORK.md).

Status: implemented; native Windows deterministic acceptance passed, 2026-11-02.
Contract: [ADR 028](adr/028-durable-device-commands.md).
Prerequisite: [W2a enrollment](DEVICE_PAIRING_ACCEPTANCE.md).

Later W3a adds typed execution evidence and explicit owner decision receipts;
see [DEVICE_EXECUTION.md](DEVICE_EXECUTION.md). The schema25 binary hashes and
validation below are historical W2b evidence; current source uses schema26.
Neither protocol increment starts a native executor by itself. W3b now adds an
explicitly configured and separately started [native service](DEVICE_HOST.md).
When that service is running, queued work may launch its selected Worker.

This slice provides durable control-plane instructions and device-reported facts.
Without the separately enabled native service, these commands install/start no
executor. The W3b service can consume queued intents; explicit W3a owner decisions
can then reach its actual Worker. These effects require both admitted cloud scope
and protected local policy. The validation below records the earlier W2b slice.

## Administrator

W4a adds owner-only `GET /v1/devices/{id}/commands`, a bounded metadata list sorted
by descending creation time/ID. It projects current expiry/revocation, but returns
no task body, output or approval. The existing single-command GET remains the
detail source. Reading never starts an executor; device credentials cannot call
this owner route. See [terminal contract](TERMINAL_DESIGN.md).

Prepare a non-secret JSON outside Git:

```json
{
  "request_id": "00000000-0000-4000-8000-000000000001",
  "expected_device_revision": 2,
  "workspace_id": "00000000-0000-4000-8000-000000000002",
  "profile_id": "codex/local",
  "sandbox": "read_only",
  "task": "Read the selected project and report its structure.",
  "expires_at_ms": 0
}
```

Replace example UUIDs with a fresh stable request identity and the device's actual
admitted WorkspaceId. Set a real deadline within 24 hours/device authority. Paths
remain device-owned; no administrator API token is copied to the device.

```text
lain device command queue <device-id> --file <work.json>
lain device command show <device-id> <command-id>
lain device command changes <device-id> --after 0
lain device command cancel <device-id> <command-id> --revision <current-command-revision>
```

Queue retries must use exactly the original file/RequestId. Inspect current state
after uncertain HTTP. Cancellation of accepted work is a request until a device
observation arrives; neither CancelRequested nor Unknown proves a process stopped.

## Device principal

These commands load only the protected W2a device directory. They do not read the
administrator local setup, inherit administrator authority or launch work:

```text
lain device command fetch --directory <device-dir> --after 0 --wait-ms 25000
lain device command inspect --directory <device-dir> <command-id>
lain device command event --directory <device-dir> <command-id> --file <event.json>
lain device command receipt --directory <device-dir> <command-id> <request-id>
```

Fetch returns at most 32 metadata changes plus `next_after`, `through`, `has_more`.
While `has_more`, supply that exact `through` with the next `after`. Once caught
up, omit `through` to open the next page. Persist the consumer cursor only after
recording all returned changes. Resuming old pages is safe; do not execute from
an old Queued change. Inspect the fresh command first. No background fetch loop
is installed by these commands.

An explicit event file has this form:

```json
{
  "request_id": "00000000-0000-4000-8000-000000000003",
  "sequence": 1,
  "kind": "accepted",
  "detail": "Accepted into the device's durable local journal."
}
```

Subsequent kinds: `started`, `progress`, `succeeded`, `failed`, `cancelled`,
`rejected`, `unknown`. Only report observed facts. Sequence is contiguous per
command, independent of W2a inventory sequence and server change sequence.
The CLI persists `command-events/<command>-<sequence>.request.json` before HTTP,
then a matching `.receipt.json`. It refuses replacement of an existing sequence;
resume an uncertain post using the same file. Keep this private journal across
restart. HTTP receipt inspection is read-only; rerunning the exact event command
can recover and persist a response lost before the local receipt write.

## HTTP

Owner prefix `/v1/devices/{device}/commands`: POST queue, GET `/changes`,
GET `/{command}`, POST `/{command}/cancel`, GET `/{command}/receipts/{request}`.
Device prefix `/v1/device-agent/{device}/commands`: GET `/changes`, GET `/{command}`,
POST `/{command}/events`, GET `/{command}/receipts/{request}`.
Changes query: `after`, optional `through`, `limit` 1..32, `wait_ms` 0..25000.
Device key on an owner route and owner key on a device route are rejected.
Every management response is `Cache-Control: no-store`.

## Acceptance and remaining work

Native Windows x86_64 / PowerShell, Rust 1.99.1 GNU with GCC/MSVCRT and
`RUSTFLAGS=-C link-self-contained=no`:

| Gate | Evidence |
| --- | --- |
| Workspace Rust regression | 304 passed, 0 failed, 3 existing manual/interactive probes ignored |
| Format, strict workspace/all-target Clippy, locked build | passed |
| Device command process rehearsal | 6 checks; 0 Worker executions, 0 model calls, 0 real QQ sends |
| Existing visual process regression | 9 checks, 12 fake model requests, 8 fixture media reads |
| Actual old/new binary schema24→25 and full rollback | 5 checks, 3 fake model requests; synthetic state only |
| Existing Python unit scripts | 14 run: 9 passed, 5 platform/runtime opt-in checks skipped |

Command domain/API tests cover admission/receipt identity, changed replay and
sequence gaps, device isolation, cancellation facts, late deadline observations,
revocation and transaction failure, restart, stable page high-water, bounded
progress with a retained final-result slot, secrets/control characters and
byte-preserving schema24 migration. Two native CLI tests cover protected local
journals, immutable sequence reuse, missing-receipt barriers and secret rejection.
The authenticated wait tests verify publication wake-up and revoke rechecks.

Reproduce with a native toolchain and Python with websockets sync support:

```text
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --no-fail-fast
cargo build --locked --workspace
python scripts/device_commands_smoke.py --bin-dir <native-binaries>
python scripts/qq_visual_smoke.py --bin-dir <native-binaries>
python scripts/qq_memory_smoke.py --bin-dir <native-binaries> --upgrade-from <old-schema24-binaries>
```

The desktop's elevated Windows test process creates Administrators-owned paths
by default. A protected external harness assigned only newly created fixture
paths to the current SID and used `/COPY:DATSO` for synthetic backup/restore.
The existing upgrade report's hardcoded `COPY:DATS` label is not the actual
owner-preserving option used here. No credential/owner check was bypassed and no
pre-existing user path was repaired. Normal unelevated native operation does not
require this elevation-specific wrapper. The paused-clock identity-timeout test
also now completes real socket setup before pausing Tokio time; its timeout and
NotOnline assertions remain intact.

Final binaries agree across all three process reports:

```text
lain.exe  c6a504437bded9e4531054318fa0ba18b9c335dce3dd15b42c413a22ec557cec
laind.exe 8c084db84e01d39ed7f7791e065c632153179df0d1a1b06aaabe0eb2e6522ea9
```

Local evidence (not public links):
`%TEMP%/lain-q1-tools/w2b-workspace-tests.log`,
`w2b-clippy.log`, `w2b-build.log`, `w2b-process.log`,
`w2b-visual-regression.log`, `w2b-upgrade.log`, `w2b-python-unit.log` in the same
external directory. Fixtures and temporary services cleaned up. An uploaded
Succeeded observation does not establish real Codex execution. Native MSVC,
macOS/Linux, real cross-device TLS and actual remote Worker execution are not
accepted by this run. No cloud deployment, personal-instance migration, new paid
call or actual device installation occurred.

W3a now supplies typed execution correlation and exact approval decision routing.
W3b must add the device-owned execution journal, native profile/runtime checks,
actual process/session correlation, safe permission application/cancellation and
artifact evidence before QQ/Actor can delegate work to a real PC. W4 adds terminal
and dashboard integration. Existing chat and visual acceptance retain their scope.
