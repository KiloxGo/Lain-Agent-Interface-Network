# Architecture

**Status:** normative foundation  
**Last reviewed:** 2026-10-04

## 1. Architectural objective

Lain must support a persistent, single-user resident agent that remains available on a personal server while supervising multiple independent worker-agent sessions.

The architecture separates four concerns:

1. **interfaces** — Web, TUI/CLI, future external integrations;
2. **resident intelligence** — Lain's own conversational identity, memory, and scheduled cognition;
3. **control plane** — sessions, events, approvals, profiles, persistence, process supervision;
4. **worker runtimes** — Codex, Claude Code, DeepSeek Harness, and future agents.

The core rule is:

> Lain owns orchestration and continuity. Worker harnesses own their internal agent loops.

## 2. System context

```text
                           owner
                             |
             +---------------+---------------+
             |               |               |
            Web             TUI         future adapters
             |               |          (Telegram, etc.)
             +---------------+---------------+
                             |
                    Lain public API
                    HTTP + SSE
                             |
+----------------------------------------------------------------+
|                          Lain Kernel                           |
|                                                                |
|  +------------+   +--------------+   +----------------------+  |
|  | Lain Actor |   |  Scheduler   |   | Session Supervisor   |  |
|  +------+-----+   +------+-------+   +----------+-----------+  |
|         |                |                      |              |
|  +------v----------------v----------------------v-----------+  |
|  |                    Domain Services                       |  |
|  | Event Bus | Memory | World | Profiles | Policy | Storage |  |
|  +------------------------------+---------------------------+  |
|                                 |                              |
|                         Backend Manager                        |
+---------------------------------+------------------------------+
                                  |
                      adapters / protocol drivers
                                  |
                 +----------------+----------------+
                 |                |                |
               Codex          Claude Code     DeepSeek Harness
```

## 3. Process model

The intended server process is `laind`.

`laind` is long-lived and owns:

- public API listeners;
- the in-memory projection of active state;
- persistent storage handles;
- scheduler timers;
- worker process supervision;
- backend protocol connections;
- approval routing;
- Lain Actor execution.

Frontends MUST be restartable independently.

Worker processes MAY outlive individual frontend connections but are normally supervised by `laind`.

If a backend supports durable sessions that can be resumed in a fresh process, `laind` should persist enough identity to reconnect after its own restart.

## 4. Component responsibilities

### 4.1 Public API

The public API is the only supported frontend boundary.

Responsibilities:

- authentication/authorization for remote clients;
- session listing and inspection;
- prompt/steer commands;
- cancellation/close commands;
- approval responses;
- Lain chat;
- profile listing and selection;
- event streaming.

The API must expose Lain domain objects, not raw provider protocol objects.

HTTP handles request/response and state snapshots. The implemented live transport
is authenticated SSE with explicit resnapshot/history catch-up; no WebSocket is required.

### 4.2 Lain Actor

The Lain Actor represents Lain herself.

Responsibilities:

- direct conversation;
- deciding when a request should be delegated;
- calling control-plane capabilities;
- reading permitted memory/world context;
- writing proposed memory through the Memory Service;
- producing bounded reflections and summaries;
- explaining worker state to the user.

The Actor does not own the worker coding loop.

The Actor should interact with the control plane through typed capabilities such as:

```text
work.create
work.list
work.inspect
work.prompt
work.cancel
work.close
work.approve
profile.list
memory.search
memory.propose
world.read/write (restricted)
```

The Actor must not be given direct access to raw secrets.

### 4.3 Session Supervisor

The Session Supervisor owns Lain's view of worker sessions.

Responsibilities:

- create/resume/close worker sessions;
- track normalized session state;
- maintain provider-session identity mappings;
- dispatch user/Lain instructions;
- route cancellation;
- surface approvals;
- recover after backend disconnection;
- ensure frontend disconnect does not kill work;
- enforce workspace concurrency policy where configured.

The Supervisor is the owner of session state transitions.

### 4.4 Backend Manager

The Backend Manager owns backend implementations and runtime processes.

It selects an adapter from a `BackendKind` / profile and provides a normalized contract to the Supervisor.

A backend may use:

- a native structured API (for example Codex App Server);
- ACP;
- an SDK sidecar;
- another supported structured transport.

Terminal UI scraping is not an acceptable primary integration strategy when structured APIs exist.

### 4.5 Event Bus

The Event Bus distributes normalized domain events inside the Kernel.

Examples:

- session state changed;
- worker message delta/snapshot;
- tool activity;
- command activity;
- file change;
- approval requested;
- approval resolved;
- usage update;
- backend disconnected;
- session became idle;
- summary completed;
- memory candidate created.

The Event Bus is not itself durable history.

Events that are needed for reconnect/recovery must be persisted by the appropriate service.

### 4.6 Scheduler

The Scheduler is responsible for time-based triggers.

Examples:

- daily reflection;
- periodic memory consolidation;
- user-defined tasks;
- delayed follow-up;
- maintenance of finished-session summaries.

The Scheduler emits jobs/events. It must not contain LLM-specific reasoning logic.

### 4.7 Memory Service

The Memory Service owns Lain's durable semantic/episodic memory.

It is explicitly separate from:

- raw worker transcript storage;
- normalized operational event history;
- worker-native conversation context.

See `MEMORY.md`.

### 4.8 World Service / World Storage

Lain owns a private persistent world containing human-inspectable artifacts such as:

- journals;
- notes;
- project summaries;
- relationship notes;
- selected artifacts;
- scratch material.

The World Service controls file/path access to this space.

Worker sessions do not receive unrestricted access to Lain's private world by default.

### 4.9 Profile Registry

The Profile Registry resolves named runtime profiles.

A profile can select:

- backend;
- model;
- endpoint/provider;
- reasoning options;
- sandbox/approval defaults;
- environment overrides;
- secret references.

Resolution must be deterministic and inspectable.

See `CONFIGURATION.md`.

### 4.10 Secret Store

The Secret Store resolves secret references at the last responsible moment.

The rest of the domain should hold handles such as:

```text
secret://claude/gateway-a
```

rather than raw values.

The initial implementation may use a local protected secret backend, but the interface should not require storing values in ordinary config files.

### 4.11 Persistence

Persistence owns durable operational truth needed across Kernel restarts.

Initial likely technology:

- SQLite for structured operational state;
- filesystem for Lain's human-readable world and selected memory artifacts.

This is a starting direction, not a requirement that all semantic memory be stored as files.

The storage model must distinguish:

- operational state;
- event/audit history;
- Lain memory;
- files/artifacts;
- secrets.

## 5. Backend boundary

### 5.1 Domain contract

The core speaks a Lain-owned backend contract.

A conceptual interface:

```text
probe()
start_runtime(profile)
list_sessions()
create_session(workspace, options)
resume_session(provider_session_id, workspace, options)
send_prompt(session, content)
cancel(session)
close(session)
respond_to_approval(session, approval, decision)
set_runtime_option(session, option)   # optional capability
shutdown_runtime()
```

The exact Rust traits may differ. The semantic boundary should remain.

### 5.2 Capability-driven behavior

Backends do not have identical features.

The adapter must expose capabilities such as:

- durable resume;
- multiple simultaneous sessions;
- approval requests;
- model selection;
- reasoning-level selection;
- dynamic runtime options;
- usage reporting;
- structured file/tool events;
- background/sub-agent events.

The Kernel should degrade gracefully when a capability is absent.

### 5.3 ACP position

ACP is a preferred interoperability layer where mature support exists.

However:

- ACP types MUST stay inside protocol/adapter code;
- the core must not assume one ACP version;
- protocol negotiation/capabilities must be honored;
- native adapters are allowed where they are more reliable or expose necessary functionality.

As of 2026-10-04, ACP v2 is evolving. Treat it as an external protocol, not as Lain's domain schema.

### 5.4 Codex position

Codex is the recommended first worker integration for the rewrite.

Codex App Server provides a structured long-lived protocol with thread/turn/item events. The Codex adapter may initially use App Server directly and normalize its model into Lain events.

This should be implemented in a way that a later ACP-backed Codex driver can satisfy the same domain contract.

## 6. State ownership

Every state value should have exactly one authoritative owner.

| State | Owner |
|---|---|
| current worker session status | Session Supervisor |
| provider connection/process state | Backend Manager |
| client connection state | Public API layer |
| Lain memory | Memory Service |
| scheduled trigger definitions | Scheduler |
| runtime profile definitions | Profile Registry |
| raw credentials | Secret Store |
| Lain private files | World Service |
| Web/TUI transient rendering state | frontend |

Frontends may cache, but they are not authoritative.

## 7. Session/event flow

A typical work request:

```text
client
  |
  | request work
  v
Lain Actor or direct work API
  |
  v
Session Supervisor
  |
  | resolve profile + workspace policy
  v
Backend Manager
  |
  | create/resume provider session
  v
worker runtime
  |
  | structured updates
  v
adapter normalization
  |
  v
Session Supervisor / Event Bus
  |
  +--> persistence
  +--> Web/TUI stream
  `--> Lain Actor when a meaningful trigger requires cognition
```

A worker event should not automatically invoke Lain's model. Most events only update operational state and UI.

## 8. Lain wake flow

```text
trigger
  |-- user message
  |-- meaningful worker transition
  |-- scheduled reflection
  `-- explicit automation
        |
        v
wake policy / budget check
        |
        v
collect bounded context
        |
        v
Lain Actor inference
        |
        +--> response
        +--> control-plane action
        `--> memory proposal
```

This is different from a continuously running reasoning loop.

## 9. Restart and recovery

Kernel restart is a normal event.

On startup the Kernel should eventually:

1. open persistent state;
2. load profiles and schedules;
3. mark previously active runtime connections as unknown/disconnected;
4. probe configured backend runtimes;
5. resume/reconnect durable worker sessions where possible;
6. rebuild in-memory projections;
7. restart timers;
8. expose recovery state to clients.

Never silently claim a session is still running when the backend cannot confirm it.

## 10. Failure model

Expected failures include:

- worker process exit;
- malformed backend message;
- protocol version mismatch;
- lost network;
- expired credentials;
- backend rate limit;
- approval timeout;
- partial session recovery;
- filesystem/workspace disappearance;
- Kernel restart during active work.

Adapters translate backend failures into structured errors. Core logic decides user-visible state and retry policy.

Do not bury backend failures in an assistant-style prose string.

## 11. Deployment boundary

Primary deployment:

```text
personal Linux server
  |
  +-- laind
  |    |-- local durable storage
  |    |-- secret backend
  |    `-- worker processes/adapters
  |
  +-- reverse proxy / TLS
  |
  `-- remote Web/TUI clients
```

The Kernel should be headless. A graphical frontend is optional.

## 12. Security boundaries

Even in a single-user system:

- remote API requires authentication;
- frontend permissions do not imply worker shell permissions;
- approvals are explicit domain objects;
- raw secret values are never sent to clients unless a dedicated future flow requires it;
- logs must redact credentials;
- worker workspace access and Lain world access are separate scopes.

A dedicated security design document should be added before exposing Lain to the public Internet.

## 13. Architectural tests

The architecture is working if:

- adding a second frontend does not change worker integration code;
- adding a new backend does not change Web/TUI domain contracts;
- replacing ACP with a native driver does not change the core session model;
- a browser can disconnect while a worker continues;
- a Kernel restart can represent uncertainty and recover durable sessions;
- worker transcript retention can be changed without changing Lain memory semantics;
- provider endpoint changes can be represented by profiles without rewriting the backend core.

If those properties fail, a boundary is probably misplaced.

## 14. External protocol references

Implementation work should re-verify upstream behavior before relying on it.

Current references:

- Codex App Server: https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server
- ACP protocol: https://github.com/agentclientprotocol/agent-client-protocol
- ACP Rust SDK: https://github.com/agentclientprotocol/rust-sdk
- DeepSeek Harness ACP: https://github.com/deepseek-ai/deepseek-harness/tree/master/packages/acp/acp
- Claude Code LLM gateway configuration: https://docs.anthropic.com/en/docs/claude-code/llm-gateway

## 15. Implemented Phase 3 boundary

The workspace contains `crates/lain-kernel` (library and `laind`) and
`crates/lain-client` (`lain`, no Kernel dependency). Modules preserve boundaries without empty services or
one-implementation repository/backend traits:

| Module | Current responsibility |
|---|---|
| `domain` | Strong IDs, WorkSession, Turn, Approval, state and cancellation rules |
| `persistence` (private) | SQLite schema migration, snapshots, atomic recovery |
| `kernel` | Session ownership, committed in-memory projection, lifecycle |
| `events` | Normalized lifecycle, approval, output and tool events |
| `config` | Daemon-only settings and redacted API credential |
| `api` | Authenticated HTTP queries/commands, SSE and bounded HTTP shutdown |
| `worker` (private) | Durable opaque provider identities, frozen resolved profiles and operational records |
| `profiles` | Profile definitions/revisions, resolution, workspace policy and namespaced SecretRefs |
| `backend` | Per-session supervisor and command serialization |
| `backend::codex/rpc/process` (private) | Codex translation, JSON framing and process ownership |
| `main` | Configuration, logging, signal registration, startup and teardown |

The supervisor owns at most one temporary runtime task/subprocess per WorkSession.
RuntimeAttachment is separate from logical state: Detached, Starting, Attached,
Stopping or Lost. Idle work normally has no process after its idle timeout.
The first adapter uses direct Codex App Server; there is no generic agent loop,
plugin registry or backend trait with a single implementation. JSON-RPC objects
stay in the adapter. Only Kernel methods commit domain changes. A single mutex
orders storage writes, projection updates and durable event publication.
Supervisor mutations and async projection reads run in `spawn_blocking`; reads
can otherwise wait behind SQLite and occupy a runtime thread. No Kernel lock
crosses an await. HTTP queries read committed projections and do not execute SQL.

SQLite stores versioned WorkSession JSON snapshots keyed by Lain ID. The schema
is owned by persistence, not by the public API. Snapshot decoding and domain
validation happen before recovery commits. Migration 002 adds a private
`worker_state` snapshot per WorkSession containing provider bindings, selected
session options, turns and approvals. Session and worker updates share one
transaction, including recovery. Migration 003 adds indexed operational history;
it is separate from worker context, summaries and Lain memory. Raw prompts,
protocol frames and reasoning are not stored.
Full snapshot writes are deliberately simple: history size affects write cost;
introduce indexed turn/approval tables when measured history volume warrants it.
This is operational storage only, not Lain memory.

The event bus uses a bounded Tokio broadcast channel (256 events). Durable
lifecycle/approval publication follows commit and projection update under the
Kernel lock. Meaningful completed text/tool observations also commit before
publication; associated state and history share one transaction. OutputProgress
is intentionally live-only and does not write SQLite. The dedicated protocol reader routes RPC responses
independently of the supervisor/storage task and queues at most 512 notifications.
Overflow stops that owned runtime and requires reconciliation instead of silently
dropping lifecycle evidence. Message frames are bounded at 8 MiB.

SSE establishes its snapshot and subscription under the same Kernel lock. There
is no SSE replay, outbox, or exactly-once promise. A slow subscriber receives
`resync_required` and reconnects for a fresh snapshot; `Last-Event-ID` is not a
replay cursor. Snapshots include a history high-water cursor. Query indexed
history through that cursor, then merge overlapping live EventIds in server order.
A crash between commit and publication is repaired from history and snapshots.
On reconnect the authoritative snapshot replaces session/turn/approval views;
history contributes timeline entries and must not roll current state backward.
No subscribers is normal. See [the protocol](DEVELOPMENT.md#11-operational-history).

The local API uses Axum, binds only to loopback, and requires a bearer token even
for probes and SSE. It accepts session creation, initial/follow-up prompts,
explicit approval decisions, cancellation and resume. Client lifetime is unrelated
to worker lifetime. API authentication is distinct from worker approval policy.
Remote exposure and TLS/auth deployment need a security design before relaxing
this binding restriction. Memory, World and scheduling remain deferred; the Phase 6 Actor boundary is documented below.

The terminal client uses authenticated HTTP/SSE exclusively. It never opens
SQLite, reads daemon files or starts a worker. Its rendering/selection/history
window is disposable. Attach means observe logical work, and sending a prompt
asks the Kernel to lazily resume the same provider binding. Closing the client
only closes its own subscriptions. See [ADR 001](adr/001-durable-work-and-runtime-attachment.md).

## 16. Implemented Web boundary (Phase 4)

`web/` is a React/TypeScript/Vite client of the same authenticated public HTTP/SSE
API used by `lain`. Its session/turn/approval projection is disposable; normalized
operational history is fetched from Kernel SQLite through the paginated API, never
stored as an authoritative browser transcript. Historical records cannot roll live
state back. Closing or multiplying observers does not launch/stop worker runtimes.

`LAIN_WEB_DIR` optionally serves public built assets on the existing loopback
listener while all API routes retain bearer authentication. Without it the daemon
remains headless. The browser token is explicit page-memory-only input; production
cookie/session/TLS/CSRF design remains a separate remote-deployment decision.
Static serving uses Tower HTTP and first-party CSP/security headers. The current
Web build assumes origin-root hosting. See [WEB.md](WEB.md) and
[ADR 002](adr/002-web-origin-and-authentication.md).

## 17. Implemented profile boundary (Phase 5)

The Kernel's SQLite registry stores definitions/revisions and canonical workspace
policies. Owner APIs manage these explicitly; worker-editable files cannot modify
them. Creation resolves selection and allowed model/sandbox overrides and commits
a non-secret resolved snapshot with the session under the existing storage lock.
Attached, detached and historical work retain that snapshot after registry edits.
Workspace restrictions apply to later prompts/attachment, while history stays
readable. Profiles cannot supply arbitrary argv, executable paths or environment.

The adapter translates resolved configuration into a private CodexLaunchSpec,
keeps env_clear and injects only the referenced credential into the child. The
environment-backed Secret Store restricts handles to LAIN_SECRET names. Resolution
and redaction buffers are runtime-only; no values enter worker metadata. Migration
004 adds profiles/policies and legacy local snapshots without changing bindings.
Health APIs do no inference. See [ADR 003](adr/003-profiles-secrets-and-session-snapshots.md).

## Phase 6 implemented Actor and local launcher boundary

The shared Rust CLI may explicitly configure/launch the existing native laind in
local mode; it never embeds a Kernel or launches workers directly. Remote CLI
commands continue through HTTP/SSE, with no local filesystem assumptions. OS
credential permission/signal handling is confined to narrow native boundaries.

`actor/domain.rs` defines Lain-owned conversations/messages/actions/turns;
`actor/model.rs` is the bounded direct Responses adapter; `actor/tools.rs` is a
closed registry calling existing Kernel/Supervisor operations. `actor/mod.rs`
owns on-demand turn lifecycle, context, confirmation and limits. The existing
private Store applies migration 005 and transactionally persists conversation
records. The model sees typed definitions and redacted data, never secrets,
SQLite or generic coding tools. Model and worker profiles are independent.

Every mutation is persisted as Proposed; explicit owner decision claims Executing
before dispatch. Work creation persists the real session ID before task submission.
Uncertain inference/actions remain Unknown, with no replay. Complete redacted
records commit before revisioned SSE snapshots; disconnect does not cancel them.
The small `lain-wire` crate defines public Actor observation DTOs and typed IDs.
The Kernel separately validates durable snapshots and publishes bounded sanitized
preview events; raw provider fragments and tool arguments never enter that channel.
Preview checkpoints are transient, while committed messages retain stable identity.
CLI consumption of incremental previews remains pending.
Web is frozen without Actor UI. Memory/World/scheduler remain deferred. Normative
contracts and limits: [LAIN_ACTOR.md](LAIN_ACTOR.md), [ADR 004](adr/004-cli-first-actor-and-confirmed-tools.md).
