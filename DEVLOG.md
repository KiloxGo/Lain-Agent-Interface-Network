# L.A.I.N. Development Log

This file records implementation history, milestone outcomes, important decisions, problems found during development, and the handoff state for the next coding session.

It is intentionally different from the architecture documents:

- `docs/*.md` describe what the system **should be**;
- `ROADMAP.md` describes what we **plan to build**;
- `DEVLOG.md` records what **actually happened**.

The Git history remains the authoritative record of exact commits.

---

## Legacy note

The pre-reboot implementation is preserved on the `legacy/v0` branch.

The old repository also contained a historical `DEVLOG.md` describing the earlier Rust + React rewrite, WebSocket connectivity, and file-based memory experiments. That log recorded dates as 2025-04-15, while the corresponding Git history is from 2026. The legacy branch is preserved unchanged; this v1 log uses the current Git history and reboot timeline.

Important legacy milestones include:

- Python + Live2D / ASR / TTS prototype;
- first Rust + React rewrite;
- WebSocket chat;
- direct LLM adapter;
- file-based profile/history memory;
- topic extraction and consolidation.

Those experiments are useful historical evidence, but they are not architectural precedent for v1.

---

## 2026-10-04 — Architecture reboot

### Milestone

`reboot: "Reset the Wired"`

### Why the reboot happened

The previous implementation had begun to reproduce responsibilities already owned by modern agent runtimes:

- LLM client/runtime logic;
- conversation history;
- memory extraction;
- retrieval;
- topic consolidation;
- parts of a generic agent loop.

The project direction was changed from "build another agent runtime" to:

> Lain is a persistent resident agent and an agent control plane for existing worker agents.

### Completed

- archived the previous implementation to `legacy/v0`;
- removed the old Rust kernel, React frontend, memory implementation, and chat loop from active `main`;
- established documentation-first architecture;
- added:
  - `AGENTS.md`
  - `README.md`
  - `ROADMAP.md`
  - `docs/PRODUCT.md`
  - `docs/ARCHITECTURE.md`
  - `docs/DOMAIN_MODEL.md`
  - `docs/RUNTIME_MODEL.md`
  - `docs/AGENT_BACKENDS.md`
  - `docs/MEMORY.md`
  - `docs/CONFIGURATION.md`
  - `docs/DEVELOPMENT.md`

### Important decisions

- Kernel is the server-side source of truth.
- Web/TUI/future integrations are clients of the Kernel.
- Worker-agent intelligence stays inside external harnesses.
- Lain owns orchestration, continuity, memory, and her private world.
- Worker context/transcripts are not Lain memory.
- Provider-specific protocol objects must not leak into the core domain.
- Runtime profiles will replace global configuration mutation.
- 24-hour residency is event-driven, not continuous model inference.
- GitHub Actions should remain lightweight and deterministic.
- Milestone commits may keep the project's short atmospheric naming style.

### Next step at the time

Build Phase 1: a durable headless Kernel foundation before integrating any real worker.

---

## 2026-10-04 — Phase 1: Kernel foundation

### Milestone

`foundation: "A Place to Remain"`

### Goal

Create a durable headless `laind` process with provider-neutral domain state, persistence, recovery, a minimal authenticated API, and no worker-agent implementation.

### Completed

#### Rust workspace

Created a minimal workspace:

```text
Cargo.toml
Cargo.lock
crates/
  lain-kernel/
```

The initial implementation intentionally remains a single crate rather than splitting into speculative micro-crates.

#### Domain model

Implemented strong domain identifiers:

- `WorkSessionId`
- `TurnId`
- `EventId`
- `ApprovalId`
- `RuntimeProfileId`
- provider-neutral `BackendKind`

Implemented `WorkSession` and a concrete `WorkSessionState` transition matrix.

Important semantics:

- `Idle` means a reusable session with no active turn;
- `Completed` means a confirmed terminal session state;
- `Cancelled` means confirmed cancellation, not merely a cancellation request;
- `Disconnected` and `Unknown` do not imply failure.

#### Operational persistence

Added SQLite persistence with:

- transactional schema migration;
- committed `Cargo.lock`;
- WAL mode;
- durable WorkSession snapshots;
- restart recovery;
- database ownership lock;
- fail-closed behavior for corrupt/unsupported persisted state.

On restart, previously active states that cannot be confirmed without a live backend are reconciled to `Unknown` rather than guessed.

#### Kernel ownership

The Kernel owns:

- lifecycle state;
- committed in-memory WorkSession projection;
- storage mutation ordering;
- event publication ordering.

Session changes are persisted before the in-memory projection/event notification is committed.

#### Event foundation

Added a bounded Tokio broadcast channel.

Initial normalized events:

- `SessionCreated`
- `SessionStateChanged`

The channel is a live notification mechanism, not a durable audit log.

Slow subscribers receive lag and must resnapshot authoritative state.

#### Local API

Added an Axum HTTP API:

- `GET /health`
- `GET /ready`
- `GET /v1/work-sessions`
- `GET /v1/work-sessions/{id}`

Phase 1 intentionally keeps the API read-only.

All routes require a bearer token.

The daemon binds to loopback only in Phase 1.

#### Secret handling

`LAIN_API_TOKEN`:

- is required;
- is validated;
- is never serialized;
- is redacted from Debug/errors;
- is compared using constant-time byte comparison;
- is not stored in SQLite or Lain domain state.

#### Lifecycle

Implemented:

- boot;
- recovery;
- ready state;
- SIGINT/SIGTERM and Windows Ctrl-C/Ctrl-Break handling;
- bounded graceful HTTP shutdown;
- storage close/reopen behavior.

### Validation

The Phase 1 test suite covers:

- domain IDs;
- complete WorkSession transition matrix;
- configuration validation/redaction;
- workspace validation;
- persistence round trips;
- restart reconciliation;
- corrupt-state fail-closed behavior;
- database single ownership;
- commit-before-event ordering;
- storage failure without false event publication;
- subscriber lag behavior;
- API authentication/status/errors;
- real TCP shutdown and database reopen.

Documented validation commands:

```sh
cargo fmt --all --check
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
```

No GitHub Actions workflow was added.

### Known limitations / deferred work

- no worker runtime is started;
- no Codex protocol exists yet;
- no public session-create/prompt/cancel API exists yet;
- no Turn or Approval entity is implemented yet;
- no provider-native session identity is persisted yet;
- no durable event timeline exists;
- no live WebSocket/SSE client event API exists;
- no RuntimeProfile registry/SecretRef store exists yet;
- no Lain Actor, memory, World, scheduler, Web, or TUI exists yet;
- native Linux signal behavior still deserves direct Linux validation.

### Phase 1 review

The implementation matches the intended architecture well enough to proceed.

Notable good choices:

- conservative restart semantics instead of pretending worker state is known;
- no speculative backend framework;
- no fake worker implementation just to satisfy APIs;
- provider-neutral domain names;
- strict separation between operational state and future Lain memory;
- event publication only after durable mutation;
- explicit lag/resnapshot semantics.

### Handoff to Phase 2

Phase 2 should integrate **Codex App Server** as the first real worker.

Before locking the adapter contract, verify the installed/current Codex version and protocol behavior.

The next milestone must settle, with evidence:

- App Server process ownership and restart behavior;
- initialize/initialized handshake;
- thread creation and resume;
- turn lifecycle;
- provider thread/turn identity persistence;
- event normalization;
- cancellation acknowledgement;
- approval request/response mapping;
- disconnect/reconciliation behavior;
- multiple concurrent WorkSessions;
- how live worker events are exposed to future clients.

Do not begin Claude Code, DeepSeek Harness, Lain memory, or UI work as part of Phase 2.

---

## 2026-10-04 — Phase 2: Codex worker control

### Milestone

`contact: "Across the Wired"`

Lain now supervises real Codex work through authenticated domain APIs. Phase 3
has not started.

### Implemented behavior and decisions

- Verified installed `codex-cli 0.160.0`, its generated schemas, official App
  Server documentation/source and the current ACP adapter. Selected direct
  App Server: the ACP adapter itself starts App Server, so an extra translation
  does not help this first backend. Volatile facts are in `docs/backends/CODEX.md`.
- Kept one Rust crate. Added a small supervisor plus private Codex/JSONL/process
  modules; no generic worker framework, coding loop or mandatory ACP layer.
- One owned process/task per WorkSession isolates failures and cancellation.
  Processes have allowlisted environments, drained private stderr, bounded
  frames/queues/RPC waits, and owned process-tree cleanup. Windows Job Objects
  were exercised; Unix process-group cleanup still needs native validation.
- Added Turn, Approval and explicit cancellation evidence. A completed, failed
  or interrupted turn leaves the reusable WorkSession Idle. RPC acknowledgement
  never substitutes for observed terminal state; a completed turn can coexist
  with an acknowledged cancellation. Rejected cancellation can be retried.
- Migration 002 preserves opaque provider thread/turn identities, selected
  session options, turns and approvals separately from public Lain IDs. Atomic
  recovery and mutations cover both session and worker metadata. No prompts or
  worker transcript history are stored in Lain SQLite.
- SQLite/projection access from async paths uses blocking tasks, including reads
  that might wait behind a writer. The independent reader routes RPC responses
  without SQLite waits. Output fragments publish counts without durable writes;
  completed output is sanitized and bounded. Queue overflow loses certainty and
  stops the affected runtime instead of dropping lifecycle observations silently.
- Owner decisions support approve-once, deny and applicable cancel. Permission
  requests grant only the requested subset for one turn. Unknown requests are
  rejected; unresolved connection-local approvals expire/become Unknown on loss.
- Added create/prompt/follow-up/cancel/resume/approval control, backend reporting,
  turn/approval inspection and authenticated SSE. Snapshot and subscription are
  established atomically against durable mutations. Lag requires a new snapshot;
  EventId is not a replay cursor. Client disconnect does not stop the worker.
- Restart restores the same Lain/provider identities and reconciles provider
  history. Missing rollouts, lost submission identities and uncertain outcomes
  remain explicit; no replacement thread is created silently.

### Validation and problems resolved

Local validation: 14 deterministic tests passed; `cargo fmt --all --check`,
`cargo test --locked --workspace`, and
`cargo clippy --locked --workspace --all-targets -- -D warnings` passed.
Tests use a local Rust fake App Server; they do not require login or model tokens.
Coverage includes migration/recovery rollback, notification-before-response
ordering, numeric/string approval IDs, command/file/permission approvals, unknown
requests, cancellation rejection/retry and completion races, two sessions,
foreign-session events, secret fragments, malformed frames, bounded-queue overflow,
crash/restart, API controls and SSE snapshot/lag/detachment.

Real Windows acceptance used `scripts/codex_smoke.py --run-real`, the local
ChatGPT login and `gpt-6-astra/high`. A complete successful run demonstrated:

1. Two distinct real provider sessions in separate temporary workspaces.
2. Initial turns, normalized progress/completion and same-session follow-up.
3. Cancellation of one active session without stopping the other.
4. SSE client disconnect/reconnect with continued worker ownership and snapshots.
5. A real command approval routed through the owner API and resolved.
6. Forced termination of one direct child of the test daemon, followed by
   conservative disconnect and explicit resume of the same identity.
7. Graceful daemon shutdown/restart, both provider identities unchanged, followed
   by a new successful real turn. Shutdown exited 0; the API token was absent
   from captured daemon logs. Temporary workspaces/databases were cleaned up.

After moving asynchronous projection reads to blocking tasks, the final binary
also passed a real HTTP/SSE prompt check: exact expected assistant text, correctly
correlated Lain session/turn IDs, persisted Completed turn and Idle session.

Implementation checks found and fixed a resume race after publishing disconnect,
a snapshot/subscription stale-event window, non-atomic worker/session recovery,
and protocol replies that could be interrupted by command selection. Test cleanup
initially held its own SQLite connection open on Windows; the smoke now closes it
explicitly and runs a private executable copy to avoid Cargo file-lock contention.
One real run failed an immediate interrupt; active-work acceptance now waits for
actual progress. The adapter still reports rejection/uncertainty rather than
claiming cancellation succeeded.

### Limitations and Phase 3 handoff

- Supported protocol version is explicitly 0.160.0. Capability reporting reflects
  verified adapter support, not a general server method negotiation mechanism.
- An unused thread may have no resumable rollout. A turn whose native ID was lost
  cannot safely be matched by text/order. Approval request IDs never survive a
  process connection; external side effects may remain uncertain after loss.
- Windows is the tested host. Linux needs native lifecycle tests and service
  cgroup cleanup for hard daemon crashes; managed shutdown uses process groups.
- Completed output is live-only and bounded; reconnect recovers operational
  state, not a transcript. Known/pattern-based secret filtering cannot recognize
  arbitrary unknown or encoded secrets. Credential retrieval remains prohibited.
- Metadata uses whole-session snapshots; indexed turn/approval rows are deferred
  until history size warrants them. Full profiles/SecretRefs, additional backends,
  durable event replay, archive semantics, memory/world and UI remain deferred.
- Phase 3 clients must handle Unknown, cancellation acknowledgement separately
  from completion, approval expiry, SSE resync and refresh after BackendReconnected.
  Decide transcript retention and profile configuration explicitly when needed;
  do not turn these operational records into Lain memory.
