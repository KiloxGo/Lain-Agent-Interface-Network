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
                    HTTP + WebSocket
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

WebSocket is appropriate for live events. HTTP is appropriate for request/response and state snapshots. Exact endpoints are deferred until the domain model stabilizes.

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

## 15. Implemented Phase 1 boundary

The initial workspace contains one crate, `crates/lain-kernel`, with a library
and `laind` binary. Modules preserve the boundaries without empty services or
one-implementation repository/backend traits:

| Module | Current responsibility |
|---|---|
| `domain` | Strong IDs, WorkSession, state transition rules, timestamps |
| `persistence` (private) | SQLite schema migration, snapshots, atomic recovery |
| `kernel` | Session ownership, committed in-memory projection, lifecycle |
| `events` | Normalized session event payloads |
| `config` | Daemon-only settings and redacted API credential |
| `api` | Authenticated HTTP queries and bounded graceful HTTP shutdown |
| `main` | Configuration, logging, signal registration, startup and teardown |

Future adapters will occupy their own modules/crates; none are instantiated yet.
The Kernel is the initial Session Supervisor boundary. Only its methods may
commit session changes. A single mutex orders storage writes, projection updates,
and publication. These mutation methods are synchronous; async callers must use
a blocking task once backend writes are introduced. Current HTTP queries read
the projection and do not execute SQL.

SQLite stores versioned WorkSession JSON snapshots keyed by Lain ID. The schema
is owned by persistence, not by the public API. Snapshot decoding and domain
validation happen before recovery commits. Snapshots intentionally avoid a full
relational schema before the first worker exists; add indexed columns through
migrations when queries require them. This is operational storage only.

The event bus uses a bounded Tokio broadcast channel (256 events). Initial kinds
are `SessionCreated` and `SessionStateChanged`. Publishing follows a successful
durable commit and projection update, under the same Kernel lock. No subscribers
is normal; a slow subscriber receives `Lagged` and must resnapshot. Subscribe
before taking a snapshot and tolerate already-reflected events. There is no
durable event replay, outbox, or exactly-once promise. A crash between commit and
publication is resolved by querying the authoritative session snapshot. Recovery
does not replay or publish historical changes.

The local API uses Axum, binds only to loopback in Phase 1, and requires a bearer
token even for probes. It does not accept session creation, prompts, approvals,
or cancellation commands. API authentication is distinct from future worker
approval policy. Remote exposure and TLS/auth deployment need a security design
before relaxing this binding restriction.
