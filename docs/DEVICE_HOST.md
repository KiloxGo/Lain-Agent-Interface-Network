# W3b native device Worker service

Implementation and native Windows deterministic validation: 2026-11-02. Contract:
[ADR 030](adr/030-native-device-worker-service.md). Read
[W2a enrollment](DEVICE_PAIRING_ACCEPTANCE.md), [W2b commands](DEVICE_COMMANDS.md)
and [W3a evidence/decisions](DEVICE_EXECUTION.md) first.

## Selected command-output repair — 2026-11-05

The first user-started real Codex turn disconnected during a read command's output
burst. The [transport correction](backends/CODEX.md#command-output-burst-recovery--2026-11-05)
passes seven native Windows Worker integration tests, one notification-classifier
test and 17 native device-service scenarios, including final-summary reporting
after 12,000 command progress notifications. Strict Kernel all-target Clippy and
workspace build pass. These fixtures use zero paid providers; real-provider
success after the repair remains a separate check.

The selected PC now has a separately installed checked host binary and a stable
current-user-only entry:

```powershell
& "$env:LOCALAPPDATA\Lain-Cloud\Start-Codex-Host.cmd"
```

Stop the existing foreground host with Ctrl-C and wait for its prompt before
using this new entry. It verifies the private manifest and exact binary hash,
then starts the native host with the original device directory and read-only
policy. It neither imports owner credentials nor changes global settings. The
old in-use executable is retained, not overwritten or killed. Installation alone
does not change the running process; the user has been asked to restart normally.
Original uncertain work is retained and never automatically resubmitted.

## Selected Windows Codex preparation — 2026-11-05

The selected PC is now paired to the existing cloud Kernel for this repository,
with a seven-day finite registration and explicit Codex-work declaration. Its
protected host policy selects the tested native Codex 0.166.0-alpha.5.1, read-only
scope and 30-second offline grace. Native `--check` confirms canonical workspace,
supported executable and active cloud authority. No provider login is copied or
changed. The host is prepared, not running; no real Codex task is claimed.
See [selected Work installation](WORK_DESK.md#selected-installation-follow-up--2026-11-05)
for the foreground command and shutdown instructions. Work can queue while the
host is stopped; registration alone is never an online claim.

## W4 Windows security recovery — 2026-11-03

The 2026-11-05 real-HTTPS fixture exposed an additional native path boundary:
Rust could create a deeply nested execution directory while the raw Win32 ACL
open failed at MAX_PATH. The shared local-file layer now lexically absolutizes
paths (without resolving links) and supplies the long-path drive/UNC namespace
when needed. Handle-based reparse/type/link and owner-only ACL checks remain in
force. Native regression covers Unicode paths over 300 UTF-16 units, creation,
private-file replacement and existing unsafe ACL/hardlink rejection. This local
fix is not evidence that the remaining cross-machine Worker cases have passed.

A later native crash regression reproduced a log handle retained by an orphan
PowerShell ACL helper despite console isolation. The selected fixture holder was
identified by Restart Manager and its PID/parent/creation/ACL-helper identity
rechecked before cleanup. The shared `lain-local` implementation now uses direct
Win32 token, handle and security APIs, without any ACL subprocess. Existing paths
remain check-only; exclusively created objects receive the current SID and a
protected owner-only DACL. Null/empty/inherited/foreign ACLs, reparse points,
hard-linked files and wrong object types fail closed.

On this increment's binaries, 364 Rust tests passed (three existing manual/live
ignored), strict workspace Clippy/format/build passed, and native Windows
11-work/15-host/eight-terminal scenarios passed with normal fixture cleanup.
The host set includes protected device status with PowerShell absent from PATH.
Evidence: external `%TEMP%/lain-q1-tools/w4c-native-acl-*.log` and
`lain-w4c-2026-11-03/{native-acl-final,terminal-acl-final}` under the session's
external artifact directory. These precede the subsequent permission-report
increment and do not validate later edits or other operating systems.

## Ownership and selected path

`lain-device-agent` consumes commands using only the device's protected credential.
It reuses the existing native Supervisor/Codex structured protocol and supports
one active `codex/local` command at a time. It creates no Actor, channel, memory
service, public listener or alternate Lain persona. Its `execution/worker.db` is
operational Worker state, not the cloud identity database. Existing cloud QQ and
other clients continue while the separate device service owns a Worker.

The service is an explicit foreground process suitable for an operator-managed
native service. Pairing, reporting inventory or queuing a task does not install
or start it. W4a now provides the [terminal shell and read-only device views](TERMINAL_DESIGN.md);
service controls and remote dispatch/permission interaction remain W4 follow-up.
No actual installation, cloud deployment, paid run or personal-instance upgrade
is part of this historical increment. The current explicit managed Codex version
allowlist is recorded in AGENT_BACKENDS.md;
archive-reader compatibility is a separate capability. A native executable that
fails the gate stops startup before accepting commands. Real-provider acceptance
remains separate from structured fixture acceptance.

## Configure and run

Prepare/pair/claim a device as described in W2a. Then on that device:

```text
lain device host-configure --directory <absolute-protected-device-dir> --codex-path <absolute-native-codex-executable> --offline-grace-seconds 30
lain-device-agent --directory <absolute-protected-device-dir>
```

### Read-only native prerequisite check

```text
lain-device-agent --check --directory <absolute-protected-device-dir>
```

Prints JSON after checking the same protected credential/manifest/host policy used
by service startup, the exact native executable/version gate, canonical registered
workspace directories and the device-authenticated own-device cloud GET. It reads
no administrator credential, makes no claim/report/queue request and never opens
the operational Kernel/database, creates a lock/journal, starts an App Server or
contacts a paid provider. The selected executable is invoked only with `--version`
through the existing adapter's cleared environment and native process isolation.

`ready_for_explicit_start` means checked prerequisites, not an already running
host. `cloud_authority` distinguishes active registration, not-claimed, expired,
revoked, denied, mismatched manifest and unavailable transport where observable.
`last_inventory_received_at_ms` is historical inventory, never current presence.
`service_availability` remains `not_proven_by_prerequisite_check` and
`provider_access` remains `not_probed`, including while an actual host is running.
Failed backend or missing workspace makes readiness false. Invalid private files
fail closed without repairing/overwriting them. An unavailable cloud produces a
readable not-ready report. Start arguments are separate `start_argv` strings,
not a shell-escaped command to execute automatically.

To run, explicitly invoke the foreground command above and retain its owning
process/terminal. Ctrl-C (or Windows Ctrl-Break) requests owned-runtime shutdown;
wait for process exit before replacing policy or restarting. Do not infer a live
process from retained lock files. Existing command acceptance/execution receipts
are distinct last-known evidence, and unknown effects still follow journal
reconciliation. No service installer, automatic restart, background daemon
manager or implicit terminal startup is introduced by diagnostics.

Native Windows isolated acceptance (2026-11-03) checks that no execution directory
or Worker/model exists after a successful pre-start check; at a quiescent running
permission wait it compares all ordinary file bytes and the two exclusively held
lock-file identities before/after, after matching local receipt publication. A
stopped cloud must return not-ready/unavailable. These three check points passed
inside the seven-case [W4 native chain](DEVICE_WORK.md#verification-and-remaining-work),
with no paid/QQ effects. They do not establish a presence lease or paid-provider access.

### Explicit host configuration

`host-configure` is exclusive creation of `host-policy.json`; an existing policy is
never overwritten by that command. It records version 1, the canonical executable,
`workspace_write: false` and finite offline grace (0–60 seconds, default 30).
Add `--workspace-write` only when explicitly selecting that wider local scope.
This still does not approve a future Worker permission request. To replace an
existing policy, stop the owned service and explicitly replace the protected
file while preserving owner-only permissions, then restart. A running service
detects changes and stops effects rather than silently widening authority.

The executable must match the actual native OS and version gate. The current
native Codex installation supplies its own login/configuration; cloud commands
provide neither a provider key nor a global settings mutation. Only the exact
registered canonical workspace and `codex/local` profile are admitted. Unsupported
profiles, missing/redirected workspaces and workspace-write without local policy
are rejected. The service probes the selected executable, never logs into QQ,
Telegram or a model provider, and never copies an administrator credential.

All device files must remain outside Git, owner-only Windows SID ACLs or Unix
0700 directories/0600 files, with links/reparse points rejected. The shared
`lain-local` crate contains the pre-existing CLI implementation. Existing paths
are checked, not automatically repaired. Windows ACL helpers use the same
no-window console isolation as owned Workers so service Ctrl-Break does not
interrupt a helper or leave its inherited handles behind. `execution/host.lock` is held for the
service lifetime. A second service cannot own the same directory. Availability is established
by local runtime probing and command receipts; this service does not rewrite the
immutable W2a capability declaration or publish a permanent Online claim.

## Durable execution and crash windows

W4c continuation implementation (eleven native isolated chain cases passed) reuses the exact
original local/provider session and derives a new execution/turn from its child
command. Protected parent intent/submission, current manifest/policy and actual
latest idle operational turn must agree before any effect. Missing parent scope
rejects fresh work, never creates an alternate thread. Each child retains separate
fences. The original no-replay rules below remain; the only new pre-launch binding
case is an independently verified idle parent. See [ADR033](adr/033-same-device-session-continuation.md)
and [interface/recovery](DEVICE_WORK.md#same-session-follow-up).

Each `execution/<command-id>/` holds:

| Record | Meaning |
| --- | --- |
| `intent.json` | Immutable command, original input, manifest/policy revisions, deterministic execution/session/turn IDs |
| `launch.json` | Launch may have begun; not proof a provider thread was created |
| `submission.json` | Prompt may have been sent; absence/presence is never inferred from an HTTP acknowledgement |
| `event-N.json` / `receipt-N.json` | Exact outbox request and matching original server receipt |
| `decision-<approval-id>.json` | Explicit owner receipt claimed before a native permission mutation; never replayed |
| `cancel.json` / `stop.json` | Cancellation intent and stop fence; neither asserts confirmed cancellation |

Each record is fully written/synced to a unique private pending file, then
atomically renamed into its published name under the exclusive host lock.
Interrupted pending files are retained privately and never count as published
effect fences or receipts. Existing published records are never overwritten.
Private writes are synced before effects. A conflicting or partial
record fails closed and remains for inspection. Original task text is not
rewritten. Secrets are rejected before intent journaling; completed native output
is redacted before bounding or producing event records. Provider thread/RPC
objects remain inside the existing operational adapter state, not cloud DTOs.

| Restart observation | Allowed action |
| --- | --- |
| No launch claim | Prepare/find the stable local metadata, recheck authority, launch once |
| Launch claimed, provider binding missing | Unknown; no replacement thread |
| Bound session, submission not claimed | Resume the original session, recheck authority, claim/send once |
| Submission claimed, original turn mapped | Resume/read that original turn; never send the prompt again |
| Submission claimed, provider turn unmapped | Unknown; do not guess correlation or resubmit |
| Local terminal fact | Report that fact without native execution |
| Stop/response fence exists | No replay of uncertain effects; retain Unknown unless a concrete terminal fact exists |

Cloud change pages are metadata notifications. Startup replays them from zero
and reads the current command before any effect; the private journal prevents
re-execution. Accepted commands without a local intent journal are not adopted.
Server restart preserves state and receipts; device restart separately reconciles
its own operational state. If the very first acceptance acknowledgement is lost,
its immutable event can be recovered without launching work; restarting the
explicit host retries that prepared intent. This is a conservative stop-before-
effects path, not an automatic replacement task.

The existing event capacity remains finite. If another nonterminal observation
is needed after sequence 254, the host stops work and reserves space for an
explicit uncertain/terminal outcome rather than emitting unbounded progress.

## Decisions, cancellation and network loss

Inspect and decide using the W3a owner CLI. The device compares the current
pending native request and cloud snapshot with the exact stored fingerprint and
choice, persists the response fence, then invokes the existing Supervisor once.
Old, changed, resolved, cancelled, expired, redacted or uncertain requests do not
acquire new approval. The existing Supervisor's Responding state remains another
durable fence. A crash after claiming a decision but before the RPC may leave the
task Unknown; replay is intentionally forbidden.

Remote `approve_once` is removed when the visible scope is truncated, redacted,
missing command/directory information, or the current adapter supplies only an
incomplete file-change description. Deny/cancel remain available when supported
by the native request. Full command and turn-permission requests retain explicit
approval. No model chooses a permission response.

Active work renews authority with bounded authenticated current reads and
notification waits; idle service waits on the cloud ledger for up to 25 seconds.
No background LLM polling or planning loop is added. Fresh reads time out after
3 seconds; native launch/submission/decision waits use the smaller of the task's
remaining deadline and 10 seconds. Active notification waits use half the chosen
grace clamped to 1–10 seconds. Network errors can wait only the configured grace
since the last successful current read. Zero grace stops on the first failed
read. Already in-flight bounded I/O and native cleanup can outlast that grace;
this is not instantaneous offline revocation.

On cancellation, expiry, revocation, changed policy or network-loss cutoff, persist
stop/cancel intent, attempt one interrupt, wait boundedly for a real terminal
event, then stop owned resources. Cancellation RPC and terminal wait each have a
five-second bound; existing Supervisor process cleanup retains its own bounds.
Confirmed Cancelled, raced Completed, Failed and Unknown are distinct outcomes.
A rejected interrupt or forced process stop cannot manufacture Cancelled.
After reconnection, only pending outbox/result facts are retried. Revoked device
proof cannot upload even a final fact; the owner sees Unknown and local evidence
remains for inspection. Stopping the host with Ctrl-C/Ctrl-Break (Unix SIGINT/
SIGTERM) drains owned workers and attempts a final/Unknown report; abrupt death
relies on persisted fences and the existing process-owner cleanup on restart.

## Results and remaining scope

Feedback uses the latest 200 retained operational events for that session and
the selected turn. Resumed status does not reconstruct missing provider transcript
content; results after a backend resume mark history incomplete. More/truncated history is marked explicitly; observed failure
counts are not a claim of exhaustive history. Output tail is bounded to 2048
UTF-8 bytes, and at most eight relative reported paths are retained. Outside,
parent, malformed, sensitive or excess references are withheld and counted.
No referenced artifact is opened, hashed or uploaded. Completed means a Worker
turn completed; permissions may have been denied or commands failed, and user
goal acceptance is still separate.

The W3b service itself added no database migration beyond schema026. Current source
also applies schema027's conversation-link table; the device starts no Actor and
does not populate cloud conversation origins. W4b now adds explicit cloud dispatch
and terminal observation/review in [DEVICE_WORK.md](DEVICE_WORK.md). It does not copy the
personal DB, expose a local administrator HTTP API, start a chat frontend or
enable autonomous Worker permission decisions. Parallel Worker scheduling, additional
profiles/backends, verified artifact transfer and remaining W4 UX stay separate work.
Linux/macOS and actual cross-device TLS/paid-provider behavior require native
evidence; a Windows loopback fixture does not stand in for them.

## Validation

Native Windows x86_64 / PowerShell, Rust 1.99.1 GNU, GCC/MSVCRT and
`RUSTFLAGS=-C link-self-contained=no`:

| Check | Actual evidence |
| --- | --- |
| Workspace tests | 322 passed, zero failed, 3 existing manual/live tests ignored |
| Formatting, all-target Clippy, locked/offline build | Passed |
| Native device service | 14 scenarios passed, including actual Ctrl-Break shutdown, crash/restart, response loss, partition and final cleanup |
| W3a permission protocol | 6 checks; no Worker/model/real QQ effects |
| QQ visual regression | 9 checks; 12 fake Responses requests, 8 fixture media reads |
| Old/new executable upgrade | 5 checks; actual schema24 binary → schema26, old-binary refusal and full stopped-state rollback; 3 fake requests |
| Python helper suite | 14 tests: 9 passed, 5 skipped |
| Documentation | 20 changed/new documents; 313 relative links checked, none broken |

The host's fourteen checks use the structured fake App Server and one deliberate
fake Responses chat request while work runs. They cover protected explicit local
policy, sandbox denial and structured results, reported-only artifact bounds,
explicit approval/denial, incomplete scope, concurrent QQ chat, confirmed and
rejected cancellation, normal shutdown, original-session crash recovery, unmapped
lost prompt acknowledgement, immutable event acknowledgement recovery, bounded
network loss/outcome delivery, changed local policy, and revocation. No paid
provider or real QQ request was made. Local device storage had zero Lain
conversations/channel bindings and contained no tested raw credential values.

Final native binaries (all four reports agree on the CLI/daemon pair, and the
host report identifies the actual service):

```text
lain.exe              3aea1380915ab061ab91bb9e18c4d06aec0fe2bbd070547948d8611ca7e95834
laind.exe             70668db0b87cabee28fd23f96bc66de598269567c7eeb70114bb1a099d0cc067
lain-device-agent.exe 89e56b4ed16b78dfa82a8b59f61ddc4181fce81e3b5381119a41741c916d4753
```

The native host report records its actual service/fixture hash; the other reports
verify the common CLI/daemon pair. External local evidence (not public links),
under `%TEMP%/lain-q1-tools/`:
`w3b-workspace-tests.log`, `w3b-clippy.log`, `w3b-build.log`,
`w3b-core-tests.log`, `w3b-process.log`, `w3b-approval-regression.log`,
`w3b-visual-regression.log`, `w3b-upgrade.log`, `w3b-python-unit.log`.
`w3b-red.log` and `w3b-receipt-red.log` preserve the failing test evidence.
All final fixtures/services cleaned up, including the previously verified orphan
ACL helper from the failed rehearsal. Cleanup errors were fixed, not ignored.

The existing elevation-specific fresh-path owner adapter described in
[W2b validation](DEVICE_COMMANDS.md) was reused. Synthetic backup/restore used
`/COPY:DATSO`; the upgrade report's hardcoded `COPY:DATS` label is not its actual
owner-preserving option. No pre-existing user path was repaired, ACL check relaxed
or personal state opened for that upgrade. The personal runtime remains on
`releases/visual-9b40de0`. No actual device service installation, personal database
migration, production deployment, real paid-Codex run, cross-device TLS,
native MSVC/Linux/macOS or sudden power-loss acceptance is claimed.

Reproduction:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo build --workspace --locked --offline
python scripts/device_host_smoke.py --bin-dir <native-binaries>
python scripts/device_approvals_smoke.py --bin-dir <native-binaries>
python scripts/qq_visual_smoke.py --bin-dir <native-binaries>
```

The host process rehearsal uses the structured test App Server, isolated local
and cloud stores, separate device credentials and a loopback fault proxy. It must
exercise real native process lifecycle, explicit permission, chat concurrency,
cancel/rejected-cancel, bound resume, unmapped lost submission, lost event receipt,
network loss/reconnection, changed policy and revocation. Its Worker has no real
model or tools. The single deliberate parallel-chat scenario uses only the fake
Responses service and fake OneBot; no paid request or real QQ send is authorized.
