# ADR 030: Native device Worker service with durable effect fences

Date: 2026-11-02. Status: selected implementation; validation in
[DEVICE_HOST.md](../DEVICE_HOST.md). Continues W2b and W3a.

## Decision

Add `lain-device-agent`, an explicit native service using the existing Rust
Supervisor, Codex structured adapter and unchanged managed version gate. It has
no HTTP administrator API, Actor, channel adapter or conversational persona.
Cloud/device credentials remain separate. A device-local operational Kernel
database holds owned Worker sessions; it is never merged with the cloud's Lain
identity. No coding-agent planning loop or provider intelligence is added.

The first supported path is a serial `codex/local` queue. An explicit protected
host policy selects the canonical native executable, read-only or optionally
workspace-write authority and 0–60 second network-loss grace. No execution is
enabled by pairing, design documentation or a cloud command alone. Changes to
the local policy/manifest or canonical workspace stop new effects. The utility
crate `lain-local` reuses the CLI's native Windows SID ACL/Unix private-file
implementation; it does not own a second configuration or credential registry.

## Effects and recovery

Before acceptance, persist the immutable task, manifest/policy revisions and
deterministic Lain execution/session/turn identities in the protected device
journal. Claim launch before starting a Worker; claim submission before sending
the task. Journal records are completely written/synced to private pending files
and atomically published under the exclusive host lock; interrupted staging is
not a logical fence or receipt. Pure session preparation has no external effect and can safely find
the same metadata again. A stable internal turn identifier prevents duplicate
submission inside the existing Supervisor. HTTP event posts retain their exact
request identity and receipts separately from Worker effects.

Restart reads the device journal and operational database before choosing any
action. A claimed launch without a provider binding stays Unknown. A bound
session without a claimed submission may resume and submit once after fresh
authority checks. A submitted, mapped turn may be resumed/read for reconciliation;
an unmapped accepted turn is never guessed or submitted again. An uncertain
mutation does not cause a replacement thread. A pending cloud receipt can be
recovered without treating it as permission to repeat native work.

The cloud change ledger is replayed from zero on service startup; original
intent/fences and current command reads make that safe. Commands accepted without
this device journal are not adopted. One host lock prevents competing owners of
the same local directory. Closing another chat/work frontend does not stop the
service. Orderly service shutdown drains owned Supervisor resources and attempts
a factual final/Unknown report; process crashes retain uncertainty for restart.

## Authority, permission and cancellation

Immediately before launch, submission, resume and a permission response, check
current cloud scope and the protected local policy. A normalized pending approval
must match the W3a receipt's exact fingerprint and available choice. Persist a
response fence before applying the explicit decision. A claimed response is never
replayed, including a crash between the fence and RPC. The existing Supervisor
also records Responding before the provider response. No task, Actor output,
standing social grant or receipt recovery auto-approves a Worker request.

Redacted, truncated or incomplete descriptions cannot offer ApproveOnce. The
current adapter's file-change description does not contain a full change set,
so the remote host exposes deny/cancel only for that kind. Complete native
command/turn-permission requests can carry explicit approval. Expanding this
boundary requires meaningful additional scope evidence, not a prompt change.

Revocation, cancellation, deadline, local policy change or prolonged network loss
stop new effects and request owned-work cancellation once. A confirmed terminal
event can report Cancelled; interrupt acknowledgement, rejected interrupt or
process termination alone cannot. Bounded transport/RPC/cleanup time is part of
the loss-of-authority window; offline revocation is not physically instantaneous.
Only result/outbox facts are retried after reconnection, without restarting work.

## Results and limits

Original output tails, failure counts, bounded reported relative paths and
truncation/withheld counts are normalized from existing operational history.
Referenced files are not opened, hashed or uploaded. A completed turn can contain
denied permissions or failed commands and does not establish user-goal success.
Credentials are redacted before history, evidence or the local outbox is written.

W3b supplies an explicit foreground service; automatic installation/login,
background service controls, dashboard/chat dispatch, multi-worker scheduling,
new backend profiles and file-transfer APIs remain separate. Native Windows
fixture validation does not establish real paid-Codex, TLS, Linux/macOS or
production deployment acceptance. The personal QQ instance is not upgraded.
