# ADR 028: Durable device commands, receipts and resynchronization

Date: 2026-11-02. Status: selected W2b contract; implemented and validated on native Windows; see [evidence](../DEVICE_COMMANDS.md).

## Scope and ownership

Continue W2a rather than introducing another agent runtime. The administrator can
queue a bounded Codex work instruction for an Active device's admitted WorkspaceId.
The instruction names a device-local profile and sandbox, never a path for the
server to open, provider configuration, secret or arbitrary shell command. Queueing
does not prove device availability, supported Codex or successful execution.
Reported capabilities remain claims; W3 must verify native backend/profile/path,
execution policy and actual Worker approval before launching any work.

W2b implements a durable server protocol and explicit native CLI reads/reports.
It starts no device execution loop, Worker, model call, archive reader or public
listener. Cloud TLS deployment, W3 native execution and W4 chat/dashboard routing
remain separate. Existing personal instances are not automatically migrated.

## Identity, delivery and observed execution

Each command has a Lain DeviceCommandId and immutable original administrator
RequestId/input. Repeating the same admission returns the same current command;
changed input under the same request conflicts. An expired/revoked identity cannot
admit new work or use an old receipt to regain device access.

Queued, Accepted, Running and terminal states are distinct. A device first records
its own immutable acceptance intent durably before sending Accepted. Only a later
Started observation means the device reports execution began. W2b does not claim
an independently verified remote WorkSession, native transcript or artifact.

Device events have a per-command contiguous sequence, stable RequestId, bounded
untrusted detail, server receive time and published command revision. Same identity
and exact payload returns the original receipt without another transition. Changed
IDs at an existing sequence, changed payload and sequence gaps conflict atomically.
Unknown HTTP must be resolved by receipt inspection or replaying the exact same
record; never create a replacement command or rerun an effect to recover an ACK.

The CLI journals a selected event before HTTP and its matching receipt afterward.
It refuses changed sequence contents and requires the preceding local receipt
before advancing. Retrying the selected immutable event can recover a missing
receipt. Fetch/inspect perform no execution. W3 still needs an effect-owning local
runtime journal and reconciliation; an HTTP acceptance alone cannot close the
receipt-to-process-start crash window.

## Cancellation, expiry, revocation and restart

Queued cancellation is definitively Cancelled because no device can subsequently
accept it. Accepted/Running cancellation sets a separate request flag. It does
not claim that a process stopped. The device may report Cancelled after observing
the stop, or a genuine completion if execution finished before cancellation.
Late Started/Progress facts remain recordable after cancellation: they are not
launch permission. A future executor must check fresh cancellation/authority at
its effect boundary; reporting an old event cannot authorize another start.

Before acceptance, deadline/device revocation becomes Expired/Revoked. After
acceptance it becomes Unknown, with a separate interruption reason, never a claim
that execution stopped. Device revocation updates its unfinished commands in the
same storage transaction. After a command deadline, late Started/Progress consume
their original sequence but retain Unknown, so a lost earlier receipt cannot
strand the final result. Actual final observations can resolve Unknown while
the original device credential is still Active and valid; this cannot authorize
a new start. Revoked/expired device credentials cannot report.

Kernel restart preserves Accepted/Running and event sequences. Server restart or
network disconnect is not proof that the device stopped. Never reset such work
to Queued, redeliver a new identity or automatically replay uncertain execution.

## Resynchronization and transport

Changes have a separate per-device monotonic server sequence. Bounded pages use
an explicit fixed high-water cursor; later events cannot move an existing page's
boundary. A change is metadata, not launch authority: inspect the current command
and record Accepted against current state. Cancellation/revocation can invalidate
a stale page. Owner/device views use the same durable ledger.

An optional one-request wait subscribes before the initial SQLite read. Command
changes/revocation/shutdown or a maximum 25-second timer wake it; the response
reconstructs current storage and rechecks device authority, including broadcast
lag. No idle polling loop or continuous inference is installed. There is no claim
of durable transport ACK/exactly-once execution or complete network coverage.

Administrator and device authentication stay separate. Each device operation
rechecks its credential, lifecycle and expiry under the Kernel storage lock.
Devices cannot queue/cancel administrator commands, read another device or access
Actor/Memory/Worker APIs. Structured input/output passes existing sensitive-value
guards. No protocol payload/credential is logged as an ordinary event.

## Bounds and migration

Task: 8192 UTF-8 bytes; device-local profile: 128 ASCII identifier bytes; command
expiry: at most 24 hours and no later than device authority. 128 retained commands
per device, 256 device events per command, with a reserved terminal slot and one
additional concrete final-result slot if that observation was Unknown. Original tasks, bounded event details, IDs and
receipts are retained as operational audit; capacity exhaustion is explicit and does not refund slots
by erasing deduplication history. Detail: 4096 bytes; change page: at most 32.

Migration 025 is additive. It enables no device, background process or new grant,
and changes no existing conversation/media/permission/provider selection. An old
binary must refuse the new schema. Restore a complete stopped backup to roll back;
never decrement user_version or overwrite an upgraded database with partial files.

## Evidence required

Domain/transaction tests: identity, changed replay, ordering, cursor high-water,
cross-device isolation, permission boundaries, cancellation races, deadlines,
revoke, restart and atomic rollback. HTTP tests include delayed/revoked requests
and bounded waits. Native CLI/process fixtures use independent principals and
temporary state, zero paid inference, zero actual Worker execution. Native
Windows evidence does not establish Linux/macOS or cross-device TLS deployment.
