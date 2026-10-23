# Domain Model

**Status:** proposed resident-domain extensions plus existing concrete domain contracts

**Last reviewed:** 2026-10-23

## Current direction and vocabulary

The user and Lain interact as equals. User, account identity and authorizer are
technical concepts, not a master/servant relationship. Historical owner wording
must not become persona language. See [PRODUCT.md](PRODUCT.md) and
[ADR 009](adr/009-cloud-resident-equal-companion.md).

The following are **target concepts**, not a list of exact Rust types or migration
names. The narrow Q1 implementation below realizes only part of binding/delivery
and reply authorization:

| Concept | Meaning | Distinct from |
| --- | --- | --- |
| UserIdentity / ChannelBinding | Verified link from one person to channel accounts | Nickname matching or a role in persona |
| SourceRecord | Authorized social/visible Worker record with source/time/gaps | Selected Memory or an instruction |
| SourceAsset | Attachment with origin and retention rules | Unbounded copy of the local filesystem |
| Experience | Evidence of something Lain actually read/did/observed | Imported work falsely claimed as participation |
| AttentionItem | A bounded unresolved concern or potential share | A standing authorization or guaranteed task |
| Activity | A reasoned reading/organizing/sharing job with budget/outcome | Worker Turn or generic agent planning loop |
| StandingGrant | Revocable authorization over sources/actions/targets/resources | Model assertion that permission exists |
| DeliveryIntent | A selected outgoing message with expiry and receipt | ConversationNotice display acknowledgement |
| Device | Paired execution/observation endpoint and native capabilities | An independent Lain identity |
| ExternalSessionArchive | Readable native history with source and completeness | Automatically controllable WorkSession |
| WorkRequest | User goal and agreed follow-up boundary | A single session or one provider Turn |
| Skill / TemplateRevision | Reusable method/resource and applicable version | Every remembered preference |

Current Conversation, ActorTurn, WorkSession, Turn, Approval, ConversationNotice,
MemoryCandidate and SessionSummary remain separate and retain existing meanings.
Do not rename or change their wire/schema contracts just because these target
concepts are described. Implementation may use fewer types if distinctions remain explicit.

### Cross-source identity and evidence

Source events need stable identities scoped by connector/account/session; platform
message IDs cannot be assumed globally unique. Occurrence time, receipt time and
source order are distinct. Observation of an external session does not create
control rights or prove current runtime status.

Current MemoryProvenance and user confirmation fields describe actual Phase 7.1
semantics. Future autonomous writes require a source/grant distinction and a real
migration, not fake confirmation timestamps. Temporary impressions retain evidence,
validity and uncertainty; correction must affect retrieval and derived records.

See [RESIDENT_DESIGN.md](RESIDENT_DESIGN.md) for lifecycles and ownership.

### Concrete Q1 channel records

Schema 008 adds `Binding`, `Incoming`, `Batch` and `Delivery` in `channels::domain`.
Their UUIDs belong to Lain. Opaque account/peer/external IDs are adapter-scoped
references, not QQ structs. A binding maps an immutable route to one durable
`ConversationId` and a revisioned reply grant. Revocation/re-grant invalidates work
from an older revision without editing conversation history.

An Incoming record is queued, batched, revoked or expired. A Batch owns the immutable
text actually submitted and its stable Actor request UUID; prepared/running records
recover the same request. Finished batches may produce one Delivery. Deliveries move
from ready to sending, then confirmed/not_sent/rejected/unknown, or are revoked/expired
before dispatch. A confirmed receipt proves platform acknowledgement, not human reading.
These records are operational conversation history, never automatically semantic memory.
See [QQ_CHANNEL.md](QQ_CHANNEL.md) for bounds and [Q1_IMPLEMENTATION.md](Q1_IMPLEMENTATION.md)
for the implementation/real-acceptance distinction.

## Historical model and concrete implemented semantics

## 1. Purpose

This document defines the vocabulary of Lain's core domain.

The goal is to prevent backend terminology from leaking into the system and to avoid multiple overlapping concepts such as task, job, thread, conversation, run, and session meaning different things in different modules.

The names in this document should be preferred in core code, API design, storage, and documentation.

## 2. Top-level model

~~~text
Lain
 |
 +-- Conversation
 |
 +-- Memory
 |
 +-- World
 |
 +-- Schedule
 |
 +-- WorkSession
       |
       +-- WorkerBackend
       +-- RuntimeProfile
       +-- Workspace
       +-- Turn
       |    `-- Events
       |
       +-- Approval
       +-- Artifact
       `-- Summary
~~~

## 3. LainActor

LainActor represents Lain as a resident personal agent.

It owns no worker runtime itself.

Responsibilities:

- converse with the owner;
- interpret intent;
- delegate work;
- inspect active work;
- propose memory updates;
- produce reflections/summaries;
- use bounded capabilities exposed by the Kernel.

The LainActor MUST NOT directly hold raw credentials.

## 4. Conversation

A Conversation is a user-facing conversational context with Lain.

It is not the same as a worker session.

A conversation may:

- remain pure chat;
- create one or more WorkSessions;
- discuss existing WorkSessions;
- survive across multiple frontend connections.

Do not model a worker session as merely a nested chat message.

## 5. WorkerBackend

A WorkerBackend identifies a family of external agent runtimes.

Examples:

~~~text
codex
claude-code
deepseek-harness
~~~

A backend describes how to communicate with a type of worker, not a running worker process and not a provider configuration.

Do not encode a specific API provider into BackendKind.

## 6. RuntimeProfile

A RuntimeProfile is a named configuration used when starting or resuming a worker runtime.

Examples:

~~~text
codex/official
codex/local
claude/official
claude/gateway-a
deepseek/official
~~~

A profile may contain:

- backend kind;
- model/provider;
- endpoint;
- reasoning/effort settings where supported;
- environment variables;
- sandbox defaults;
- approval defaults;
- secret references;
- backend-specific extension fields.

A RuntimeProfile is configuration. It is not a running process and is not a WorkSession.

## 7. WorkerRuntime

A WorkerRuntime is a live runtime/process/connection capable of hosting or controlling one or more WorkSessions.

Depending on the backend, one runtime may manage exactly one session, several sessions, or a durable remote session independent of the local process.

Examples include a Codex App Server process, an ACP adapter process, a Claude Agent SDK sidecar, or a DeepSeek Harness process.

Important invariant:

~~~text
WorkSession != WorkerRuntime
~~~

A process can die while a provider-native session remains resumable.

## 8. WorkSession

A WorkSession is Lain's normalized representation of one durable unit of delegated worker work.

It has a Lain-owned ID and may also store a provider-native session ID.

Provider IDs are adapter metadata and must not become the public identity of the WorkSession.

A WorkSession should minimally carry:

- Lain session ID;
- backend kind;
- runtime profile ID;
- workspace reference;
- provider session ID if available;
- normalized state;
- created/updated timestamps.

## 9. WorkSessionState

Core states should be intentionally small and provider-neutral.

Initial candidate:

~~~text
Created
Starting
Ready
Running
WaitingForApproval
Idle
Completed
Cancelled
Failed
Disconnected
Unknown
Archived
~~~

Important rules:

- Disconnected means the control connection is unavailable, not necessarily that provider work stopped.
- Unknown means the Kernel cannot currently prove the actual backend state.
- Completed should represent a terminal work state according to the backend/session contract.
- frontend disconnect MUST NOT change WorkSessionState.

Backend-specific sub-state belongs in metadata/debug fields.

## 10. Turn

A Turn is one unit of instruction/execution inside a WorkSession.

~~~text
user/Lain prompt
      |
      v
Turn
      |
      +-- worker output
      +-- tool activity
      +-- approvals
      `-- completion/failure
~~~

A follow-up prompt usually creates another Turn in the same WorkSession.

Do not equate Turn with process execution or the whole session.

Different backends may not expose a perfect turn model. The adapter may synthesize one when meaningful.

## 11. Event

An Event is a normalized observation that something happened.

Events should carry:

- a Lain event ID;
- timestamp;
- WorkSession ID;
- optional Turn ID;
- event kind;
- normalized payload;
- optional provider metadata/provenance.

Candidate event kinds:

~~~text
SessionStateChanged
TurnStarted
TurnCompleted
AssistantMessageDelta
AssistantMessageCompleted
ToolStarted
ToolUpdated
ToolCompleted
CommandStarted
CommandOutput
CommandCompleted
FileChanged
ApprovalRequested
ApprovalResolved
UsageUpdated
BackendDisconnected
BackendReconnected
ErrorObserved
ArtifactProduced
~~~

Events are operational truth, not memory.

## 12. Approval

An Approval represents an explicit decision requested by a worker runtime.

Examples:

- execute a command;
- write outside an allowed area;
- access network;
- use a sensitive tool.

An approval should preserve:

- approval ID;
- session/turn relation;
- approval kind;
- human-readable description;
- supported decisions;
- optional expiry.

Lain may explain an approval request, but the Kernel must preserve the original structured decision.

Never reduce approval to typing yes/no into a terminal.

## 13. Workspace

A Workspace is a filesystem/project scope in which a worker operates.

Examples:

~~~text
/home/user/codebase/Klog
/home/user/projects/lain
~~~

A WorkspaceRef should be normalized and validated server-side.

Workspace is different from Lain's private World.

~~~text
Workspace != World
~~~

Workers may receive access to a Workspace without receiving access to Lain's World.

Future workspace policy may include allowed backends, concurrency limits, read-only mode, branch/worktree isolation, and remote workspace mapping.

## 14. Artifact

An Artifact is a durable output worth referencing independently of event history.

Examples:

- generated report;
- patch/diff;
- test result bundle;
- image;
- exported file;
- worker-produced summary.

Artifacts have provenance: which session, which turn, which backend, where stored, and what content type.

An Artifact is not automatically Memory.

## 15. SessionSummary

A SessionSummary is a normalized, durable synopsis of a WorkSession.

It may include:

- objective;
- important actions;
- files changed;
- tests/results;
- unresolved issues;
- decisions;
- candidate knowledge worth remembering.

A summary is an intermediate knowledge product. It does not automatically become Lain memory.

## 16. Memory

A Memory is information intentionally retained as part of Lain's continuity.

Important distinction:

~~~text
EventLog        = what happened operationally
SessionSummary  = compressed understanding of worker work
Memory          = what Lain chooses or was instructed to retain
WorldFile       = an artifact in Lain's private space
~~~

These may reference each other but are not interchangeable.

## 17. Schedule

A Schedule describes a persistent future trigger.

Examples:

- daily reflection at a configured local time;
- run consolidation every week;
- user-created task;
- delayed follow-up.

A Schedule produces a scheduled job/trigger. It does not itself contain LLM logic.

## 18. WakeTrigger

A WakeTrigger is an event that may cause LainActor inference.

Candidate sources:

~~~text
UserMessage
WorkerMilestone
WorkerCompleted
ApprovalNeedsExplanation
ScheduledReflection
MemoryMaintenance
ExplicitAutomation
~~~

Most WorkSession events should not be WakeTriggers.

A wake policy decides whether cognition is justified.

## 19. SecretRef

A SecretRef is a handle to a credential.

Examples:

~~~text
secret://openai/main
secret://anthropic/gateway-a
secret://deepseek/main
~~~

SecretRef may be stored in ordinary configuration. The resolved secret value may not.

## 20. Identity rules

Lain-owned identifiers should be stable even if provider identities change.

Recommended strong IDs:

- ConversationId
- WorkSessionId
- TurnId
- EventId
- ApprovalId
- ArtifactId
- RuntimeProfileId
- ScheduleId
- MemoryId

Use strong newtypes in Rust rather than passing generic strings where practical.

## 21. Relationship diagram

~~~text
Conversation
    |
    | may create/reference
    v
WorkSession -------------------- RuntimeProfile
    |                                |
    | uses                           | selects
    v                                v
Workspace                       WorkerBackend
    |
    +-- Turn
    |    |
    |    +-- Event*
    |    +-- Approval*
    |    `-- Artifact*
    |
    `-- SessionSummary
             |
             | may produce candidates
             v
           Memory

LainActor
  |-- participates in Conversation
  |-- controls WorkSession through capabilities
  |-- reads/writes Memory through Memory Service
  |-- reads/writes private World through World Service
  `-- receives selected WakeTriggers from Scheduler/Event layer
~~~

## 22. Anti-patterns

Avoid:

~~~text
AgentSession = everything
Task = both schedule and worker session
Message = every possible event
Memory = all persisted text
Backend = API provider
ProcessId = session identity
frontend-local UUID = authoritative session identity
~~~

If a new concept cannot be explained relative to this model, update this document before spreading the new term across code.

## 23. Concrete session model

`WorkSessionId`, `TurnId`, `EventId`, and `ApprovalId` are distinct UUID newtypes
generated by Lain (UUID v4, serialized as UUID strings). `RuntimeProfileId` is a
validated name so names such as `worker/official` remain usable. `BackendKind`
is a validated opaque name, not an API-provider enum. Names are 1–128 ASCII
characters in nonempty slash-separated segments containing letters, digits,
underscore, hyphen, or period. Phase 5 adds a durable profile registry.

A WorkSession stores ID, backend, runtime profile ID, canonical absolute workspace
path, logical state, runtime attachment, and created/updated UTC Unix milliseconds. Creation requires an
existing directory with a UTF-8 path. Recovery validates the record without
requiring that the workspace still exists. Timestamps never move backward on
transitions, even if the host clock does. Provider-native session/turn identities
and selected session options are private operational metadata, never public IDs.
Profiles determine the backend. `codex/local` is the compatibility/default profile,
with explicit selection or a canonical workspace default available in Phase 5.

The candidate state names in section 9 are adopted. Exact allowed transitions:

| From | Allowed next states |
|---|---|
| Created | Starting, Cancelled, Failed |
| Starting | Ready, Cancelled, Failed, Disconnected, Unknown |
| Ready | Running, Idle, Cancelled, Failed, Disconnected, Unknown |
| Running | WaitingForApproval, Idle, Completed, Cancelled, Failed, Disconnected, Unknown |
| WaitingForApproval | Running, Idle, Cancelled, Failed, Disconnected, Unknown |
| Idle | Running, Completed, Cancelled, Failed, Disconnected, Unknown |
| Disconnected | Unknown, Ready, Running, WaitingForApproval, Idle, Completed, Cancelled, Failed |
| Unknown | Disconnected, Ready, Running, WaitingForApproval, Idle, Completed, Cancelled, Failed |
| Completed / Cancelled / Failed | Archived |
| Archived | none |

Same-state and unlisted transitions are rejected without writes/events. Terminal
sessions cannot be reopened. Disconnected/Unknown can return to an observed
state only with supervisor evidence; the transition function does not establish
that evidence. `Cancelled` records confirmed cancellation, not a request to
cancel. `Idle` represents a completed turn in a reusable session; `Completed`
requires a terminal session contract. The current adapter leaves reusable sessions
Idle after successful, failed or cancelled turns; it never treats one completed
turn as WorkSession completion. An approval can be cleared by turn termination,
hence WaitingForApproval can transition directly to Idle.

## 24. Implemented Turn and Approval semantics

`WorkSession != Turn`: a session owns many instruction/execution cycles. Turn
has a Lain TurnId, WorkSessionId, state, cancellation state, timestamps and optional
safe failure category/code. States are Starting, Running, Completed, Cancelled,
Failed and Unknown. Starting is persisted before submission. A transport failure
can leave submission uncertain; Unknown is not a failed or cancelled outcome.
Only backend evidence reconciles it. A pending turn without a provider identity
cannot safely be matched to provider history by guesswork and remains Unknown.

Cancellation records NotRequested, Requested, Rejected, Acknowledged or Confirmed.
Only observed interruption confirms cancellation. Acknowledgement can race normal
completion; Completed plus Acknowledged is valid. Rejected cancellation can be
retried. Disconnect retains request evidence but makes unfinished turns Unknown.

Approval has its own Lain ID, session, optional turn, kind, sanitized description,
available decisions, status, chosen decision and timestamps. Kinds are Command,
FileChange and Permissions. Decisions are ApproveOnce, Deny and (where supported)
Cancel. There is no blanket/session approval or automatic approval policy.
Pending becomes Responding before sending the decision; server resolution makes
it Resolved. Resolution means the request was handled, not that a tool succeeded.
Lost connections expire unanswered requests and mark possibly delivered decisions
Unknown. Connection-local request IDs are not replayed after restart.

Events carry EventId, UTC timestamp, WorkSessionId and optional TurnId. Durable
events additionally have history_order and bounded payload truncation metadata. Lifecycle
and approval events carry normalized domain values. Live observations include
OutputProgress (character counts), AssistantMessageCompleted, CommandStarted,
CommandCompleted, successful FileChanged, ToolActivity and ErrorObserved.
BackendDisconnected and BackendReconnected describe transport ownership. Raw
provider request objects and secret-bearing protocol diagnostics are excluded.

## 25. Runtime attachment and operational history

WorkSession is durable logical work; WorkerRuntime is temporary infrastructure.
Public `runtime` has Detached, Starting, Attached, Stopping and Lost observations,
independent of WorkSessionState. Process IDs are never public identity. Idle plus
Detached is healthy; a prompt/resume uses the same Lain ID and private provider
binding. Planned detach emits runtime changes rather than BackendDisconnected.
Unexpected idle loss can retain Idle plus Lost; active unconfirmed loss is Unknown.

Operational history is an indexed normalized timeline, not a worker transcript,
SessionSummary or Lain semantic memory. Stable EventIds are reused in SSE and
history. `history_order` is a database-wide increasing sequence, so a session's
orders can have gaps. Timestamp is display information, not the ordering clock.
Meaningful state/turn/cancellation/approval changes, completed assistant messages,
commands, successful file changes, tools and safe errors are durable. Unknown turn
and expired/uncertain approval recovery use TurnStateChanged/ApprovalStateChanged;
they do not fabricate completion. SessionCreated includes the initial Lain snapshot.
Only OutputProgress is ephemeral, with no history_order or replay guarantee.

State and associated events commit atomically before publication. Observation-only
events commit their own row before publication. The history remains useful after
worker detach and daemon restart. Retention is indefinite in this milestone with
bounded individual payloads; pruning is future explicit maintenance. See the
[query/bounds contract](DEVELOPMENT.md#11-operational-history).

## 26. Runtime profiles, frozen configuration and failures (Phase 5)

RuntimeProfile combines a ProfileDefinition and monotonic revision. Definition
fields are stable Lain ID, backend, driver, enabled, optional model/provider/base
URL/reasoning effort, sandbox default, approval policy and optional SecretRef.
Unsupported fields are rejected; no worker-native JSON, arbitrary argv/env or raw
credential belongs in the public model. Disabling affects creation only; sessions
retain snapshots, including after an edit. There is no cascade deletion.

ResolvedRuntimeProfile contains profile ID/revision, backend/driver, selected
model/provider/endpoint/effort, sandbox/approval policy and SecretRef. It lives in
private worker metadata, with a credential-free inspection API. Registry revision
and session revision are distinct. Revision 0 identifies migrated Phase 4 local
metadata. SecretRef resolves only `secret://env/<name>` through the restricted
environment store. Profile configuration freeze does not freeze the credential.

WorkerFailure retains safe kind/message/code and optional `retryable`; old records
decode with None. AuthenticationFailed, RateLimited, ProviderUnavailable,
PermissionDenied, SessionNotFound, SessionNotResumable and Unknown supplement the
existing categories. Adapters use structured evidence, never provider error prose.
Unavailable covers runtime transport loss; Protocol and Unsupported remain
distinct. Session-not-found/not-resumable are reserved for reliable evidence, not
guessed from a generic HTTP 404 or missing connection. ErrorObserved is an
observation and never changes a turn to Failed; terminal turn evidence is required.

## Phase 6 concrete conversations and actions

`ConversationId`, `ActorTurnId`, `MessageId`, `ActionId` are distinct Lain-owned
UUID newtypes. Conversation has creation/update timestamps. Indexed ActorTurn
stores a unique conversation/request-id pair, sequence/revision, state, messages,
actions/results, error and model-request/token counters. Message carries ID,
conversation, role/content/timestamp/status. These records are independent of
WorkSession and provider-native threads and are not semantic Memory.

Actor states are Running, PendingConfirmation, Completed, Failed, Cancelled,
Unknown. Action states are Proposed, Executing, Executed, Rejected, Failed,
Unknown. Same request ID/text returns the existing turn; changed text conflicts.
Owner confirmation is an explicit state transition, never inferred from model
prose. Executed means the typed service accepted the operation, not that worker
execution completed. Created WorkSession IDs survive partial delegation failure.
Restart marks unfinished execution Unknown and never replays a mutation. Pending
proposals survive for explicit owner review. See [LAIN_ACTOR.md](LAIN_ACTOR.md).

## Phase 6.6 concrete delegation and notices

Delegation is a Kernel-owned correlation record keyed by confirmed ActionId,
containing ConversationId, ActorTurnId, WorkSessionId, optional worker TurnId,
exact confirmed instruction, canonical workspace, profile ID, submission state
and actual receipt. The session retains frozen resolved profile configuration.
Work.create can be Prepared with no worker Turn while starting its runtime;
allocation makes it Submitting before RPC. Accepted/Rejected/Unknown describe
submission evidence, independently of Actor approval and authoritative worker
execution. Work.send links a later Turn in the same session. No one-session or
one-delegation assumption is made for a conversation.

ConversationNotice is an immutable observation, with typed NoticeId, source/target
IDs, ordered cursor, timestamp, optional source history order, outcome, bounded
ResultEvidence and mutable owner display acknowledgement. Worker outcomes are
Completed, Failed, Cancelled or Unknown; Unknown can later be followed by a
distinct reliable terminal observation. SubmissionUnknown covers a created
session without a reliably submitted Turn. Repeated action/outcome observations
produce one notice. Notices are neither Actor messages/turns nor semantic Memory.
Acknowledgement confirms display metadata only, never work correctness or policy
approval. See [ADR 007](adr/007-durable-delegation-result-delivery.md).

## Phase 7.1 concrete memory and summaries

Memory, Candidate and Summary use distinct Lain UUIDs (`MemoryId`, `CandidateId`,
`SummaryId`). The shared public value types live in `lain-wire::memory` and are
used by Kernel domain services; they contain no backend/transport protocol objects.
MemoryRecord has a typed kind/scope/source/authority/status, revision, selected
bounded content, timestamps, explicit owner confirmation and optional supersession.
ProjectKnowledge requires Workspace; Reflection requires Internal. Explicit,
Inferred and UnverifiedSelection authority are source-bound and cannot be chosen
by the model. Accepting a suggestion never changes its origin into an owner quote
or its worker evidence into independently verified truth.

Actor MemoryCandidates use their confirmed ActionId as a durable review identity
wrapped in CandidateId. Candidate/Accepted/Rejected is independent of
Active/Disabled/Superseded memory status. Direct owner-authored memory bypasses
candidate review because the authenticated command is itself the exact statement.
Source references are Kernel-owned request IDs, Actor conversation/turn/action IDs,
or SummaryIds. No provider thread object is a memory identity. See
[MEMORY.md](MEMORY.md) for creation/revision/retention invariants.

SessionSummary records one WorkSession and worker Turn, outcome, workspace,
creation time, source Turn update time and immutable normalized history cursor.
Its selected report, failed commands/errors and changed paths are observed evidence,
with limits and uncertainty. It is not a ConversationNotice, semantic Memory,
worker correctness certificate or persona update. Creation and inspection need no
model; owner-selected project facts can enter a separately reviewed candidate.
