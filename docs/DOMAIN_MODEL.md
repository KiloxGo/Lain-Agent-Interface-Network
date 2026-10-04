# Domain Model

**Status:** normative foundation  
**Last reviewed:** 2026-10-04

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
