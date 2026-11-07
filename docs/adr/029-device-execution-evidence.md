# ADR 029: Typed device execution evidence and exact-scope human decisions

Date: 2026-11-02. Status: implemented protocol; validation in
[DEVICE_EXECUTION.md](../DEVICE_EXECUTION.md). Completes W3a, not the W3 host.

## Decision

Continue W2b with optional normalized execution evidence and a separate immutable
administrator permission-decision ledger. A cloud command binds one device-local
Lain execution/session/turn identity; those bindings cannot be removed, changed
or shared with another command on that device. Codex protocol objects remain in
its adapter. A device assertion is still a reported observation, not independently
verified native execution. No cloud local Supervisor session is invented from it.

An explicit decision binds the execution/session/turn, approval identity, visible
kind/description, truncation flag and permitted choices through a versioned
fingerprint. Truncated scope cannot offer approval. Only the owner can decide;
an Active device can read only its own receipts. New decisions require a Running,
uncancelled, unexpired command and a current exact pending request. Approval
identity cannot be recycled with different contents. Task/model/archive content
does not confer permission.

Receipt insertion, command revision and change publication share one transaction.
Exact retries recover immutable receipts even after the request is resolved;
this is not new permission to apply or replay a backend mutation. The future host
must compare current native scope and durable response state before effects and
never replay Responding/Unknown decisions. Kernel restart preserves records and
does not start work or invent an application acknowledgement.

Bounded original Worker output, failure count, reported relative paths and
truncation/withheld counts are separate facts. Turn completion does not establish
user-goal success. References are not verified artifact bytes; the protocol
opens/uploads no arbitrary local files. Secrets are rejected in evidence before
local CLI journaling and server persistence. Legacy W2b JSON remains compatible.

## Scope and consequences

W3a delivers the cloud/CLI contract and synthetic native process evidence using
schema 026. It deliberately adds no new executable, shared utility crate, generic
agent planning loop or global Worker configuration. Reuse the existing native
Supervisor in W3b, with a protected device-owned policy/journal and durable effect
fences. Native host startup, actual approvals/cancellation/reconciliation and
artifact collection still need their own implementation and acceptance. W4
interaction remains later. No deployment, actual device installation, paid run
or personal database migration is authorized by this protocol change.
