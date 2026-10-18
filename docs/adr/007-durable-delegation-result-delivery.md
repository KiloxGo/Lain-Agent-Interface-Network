# ADR 007: Durable delegation and conversation result notices

Date: 2026-10-18. Status: implemented; native macOS deterministic checks verified
in DEVLOG. Windows/ConPTY and Linux are unverified. Real-provider Phase 6.6
acceptance is opt-in and has not been run.

## Missing boundary

ADR 004 makes confirmed mutations durable, but its final Actor receipt proves
submission, not the later worker outcome. An ActorTurn can finish while its
worker continues. ActorTurn SSE cannot own later result delivery. Closing chat
must not cancel work, and reopening a completed ActorTurn to insert a result
would conflate model inference with worker execution.

## Decision

The Kernel owns Delegation correlation. Migration 006 adds indexed delegation
and conversation-notice tables alongside existing SQLite state/history. Each
confirmed work.create/work.send dispatch links its ConversationId, ActorTurnId
and ActionId to a WorkSessionId and the specific worker TurnId. Work.create
first commits its session, partial receipt and prepared link atomically before
startup. Allocating the worker Starting Turn commits its link and partial Actor
receipt before sending the exact task. Work.send uses the same Turn allocation
boundary in an existing session. An action cannot acquire another worker Turn by
retry. Original conversation messages and actual worker-bound instruction stay
separate; no compiler or rewriting is introduced.

Prepared/Submitting/Accepted/Rejected/Unknown describe submission evidence.
Worker execution remains authoritative in Supervisor/WorkerState; approval
remains authoritative in ActorAction. A link does not own another worker-state
machine. Exact instruction, canonical workspace, profile ID and dispatch receipt
are owner-inspectable; the session retains its frozen profile configuration.
Existing input validation/redaction excludes credentials.

Store projects normalized TurnStarted/TurnCompleted/TurnStateChanged events in
the same transaction that commits worker metadata and operational history.
Completed, Failed, Cancelled and Unknown observations produce immutable notices;
Unknown is uncertainty, not successful terminal completion. An action/outcome
unique key makes repeated transitions idempotent. Later reconciliation may add a
new terminal notice after Unknown; earlier observations retain their evidence and
timestamp. A created session whose Actor action becomes Unknown before allocating
a worker Turn produces SubmissionUnknown. Projection and recovery never replay a
mutation or call a model.

Notices have stable IDs, a database-wide increasing cursor, source history order,
source/target identities, timestamp, outcome, bounded factual evidence and owner
display acknowledgement. Cursor gaps between conversations are normal. Evidence
is restricted to the target session/Turn, through the establishing event: latest
completed assistant report (8 KiB), eight nonzero command excerpts (512 bytes each)
plus total count, sixteen changed paths (512 bytes each), four safe errors and
truncation metadata. Existing history sanitization precedes projection; legacy
excerpts are sanitized again. No prose summary or correctness conclusion is
invented. Completed does not imply all commands/tests succeeded. Later or
unattributed history stays inspectable without guessed correlation.

Conversation SSE reads committed notice pages by cursor. Existing worker and
Actor broadcasts are wakeups only; no additional event bus is introduced.
Worker subscription and initial snapshot are ordered under the Kernel lock;
Actor subscription precedes both. Pages recover after broadcast lag, stream loss,
restart, or death between commit and publication. Per-stream queues hold eight
pages of at most 100 notices; a blocked enqueue times out after ten seconds.
Reconnection never creates work. Disconnect does not change execution state.

## Display and acknowledgement contract

Delivery is at least once, with cursor deduplication within a CLI process and
durable per-notice owner display acknowledgement after output flush. SSE/HTTP
reads do not acknowledge. The authenticated acknowledgement API is idempotent
and conversation-scoped; it changes presentation metadata only, never approves
work, permissions or result correctness. All owner clients share acknowledgements.
Connected clients may each display a notice; a fresh chat skips acknowledged
notices. Explicit history inspection retains them all.

A crash after output but before acknowledgement can repeat a notice on a later
open. Failed or unqueued acknowledgements also remain pending. Exactly-once
terminal rendering and proof that a human read a report are not promised.
Committed but undisplayed notices remain recoverable. Existing Actor JSON and
worker-history contracts remain unchanged; notices are separate records, not
synthetic user messages or assistant model responses.

CLI observer and acknowledgement tasks never write terminal output. The active
chat/input owner coordinates previews, exact approval arguments, compact action
receipts and notices. During line input it clears wrapped input, renders the
notice, then restores the prompt and saved characters. Raw-mode restoration,
CRLF handling and worker independence remain intact.

## Limits

No optional model interpretation, scheduler, memory, Web changes, new backend,
prompt compiler or automatic permission approval. Phase 6.5 conversations remain
readable but historical action receipts are not guessed into new correlations.
Retention/pruning remains future explicit maintenance. Native Windows/ConPTY and
Linux acceptance requires actual hosts; macOS fixtures cannot establish it.
