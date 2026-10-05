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

## 2026-10-05 — Phase 3: presence

Phase 3 is implemented in its required order: durable WorkSession and detachable
WorkerRuntime, persistent operational history, then an independent CLI/TUI.
Phase 4 has not begun. The architecture decision is recorded in ADR 001.

- Added separate Detached/Starting/Attached/Stopping/Lost runtime observations.
  An event-driven 300-second default idle timer releases only safe idle workers;
  active turns, connection-local approvals and serialized operations prevent it.
  Lazy prompt/resume uses the existing provider identity. Known idle boot does
  not spawn workers; uncertain work selectively reconciles. Planned idle/shutdown
  keeps Idle + Detached; unexpected idle loss keeps Idle + Lost; active unconfirmed
  loss/shutdown makes the session/turn Unknown. Processes remain Kernel-owned.
- Verified that an unused Codex 0.160.0 thread is not resumable by thread/start
  alone. A full read initializes this version's default history (including its
  -32601 list_turns limitation), and a persistent fork creates a resumable empty
  rollout. Bind only the successful fork at initial creation, never replace an
  existing binding, and never submit synthetic model work. This empirical,
  version-gated behavior and native seed-retention tradeoff are explicit in CODEX.md.
- Migration 003 stores normalized history in indexed rows separate from worker
  snapshots. Stable EventIds and global monotonic order unify live and stored
  events. State/history commit together before publication, including reconciliation
  and recovery turn changes. Only OutputProgress stays ephemeral. Byte-bounded
  completed output and safe metadata are retained indefinitely initially; raw
  prompts/protocol/reasoning and semantic memory remain excluded. Future pruning
  is explicit maintenance, not implicit summarization.
- Added authenticated forward/backward history pagination and snapshot watermark.
  Reconnect combines current snapshots, frozen history and live EventIds without
  an exactly-once promise. Historical output cannot roll newer projections back.
- Added `lain-client` with no Kernel/backend/SQLite dependency: HTTP/SSE only,
  environment URL/token, full control command surface and JSON output. The Ratatui
  0.30.2/Crossterm 0.29 TUI has list/detail views, current turn, recent/older history,
  visible offered approvals with scrollable details, UTF-8 prompt editing, scoped
  cancel/resume, resize and reconnect/resync. Terminal escape/bidi controls render
  as text; guards/panic hooks restore terminal modes. Its view window is capped
  at 1000 records, with CLI/API pagination for older history.

Validation: 26 deterministic Rust tests passed. Two native checks (Codex
no-inference persistence and terminal error/panic restoration) are ignored by
default and passed when explicitly run. Formatting and
Clippy with warnings denied passed. Tests cover idle/active/approval lifetimes,
lazy binding continuity, known-idle boot, active shutdown and idle loss, history
durability/order/isolation, state/history rollback without broadcast, UTF-8/output
bounds, whole-record redaction beyond truncation limits, API pagination/auth,
remote path preservation, offered approval decisions, fragmented SSE/reconnect,
snapshot ordering/dedup, deterministic resize/rendering and restoration commands.
No expensive CI or model automation was added.

Focused real Windows acceptance (`codex_smoke.py --run-real --phase3`) passed
unused-thread idle detach, CLI inspection without attachment, two short successful
model turns through lazy resume, stable provider identity, history surviving two
daemon restarts and Idle + Detached boot. Completed output/IDs/order were unchanged
after reboot and the daemon token was absent from captured output/logs.
Windows ConPTY acceptance with the deterministic subprocess passed attach without
worker launch, actual keyboard follow-up on detached work, resize, approval denial,
and alternate-screen restoration on exit. A native terminal guard test also
confirmed raw-mode restoration after an error return and a caught panic.
The helper dependency was installed only
in an ignored isolated test environment. No global worker config was changed.

Native Linux was not claimed: the available WSL Ubuntu has a C compiler but no
Rust toolchain. Unix signal/process-group and terminal acceptance remain host
validation work, and SIGKILL requires service-manager cgroup cleanup/terminal
reset. Other limits: exact Codex version gating and provider-owned rollout storage,
unknown/unbound submission outcomes, unknown/encoded secrets, pre-Phase-3 output
not backfilled, whole turn/approval snapshot write cost, bounded client response
and view sizes, indefinite total history growth until pruning, and loopback daemon
binding. Full profiles/SecretRefs, Web, additional backends, Lain memory/personality
and background cognition remain later phases.

## 2026-10-05 — Phase 4: Web control plane

Milestone: `window: "Present Day, Present Time"`. Phase 5 has not started.

- Added a fresh React 19 / TypeScript 5 / Vite 8 client in `web/`; no legacy
  frontend, Redux, WebSocket, browser orchestration or transcript database.
  Connection, list, New Work, detail, timeline, composer and approval cards use
  the same public HTTP/SSE API as the terminal. Backend model selection is now
  explicitly advertised for the already-supported session-options API.
- Chose explicit page-memory-only bearer entry and optional same-origin built
  assets from `LAIN_WEB_DIR`. All API routes retain authentication; public assets
  use first-party CSP, no-store, nosniff and no-referrer. Loopback binding remains.
  Refresh requires re-entry; only a public selected ID persists in the URL hash.
  ADR 002 records the interim choice and future TLS/session/Origin/CSRF boundary.
- Browser state is disposable. Fresh snapshots replace authoritative projections;
  paginated history merges only timeline entries by stable EventId/server order.
  Reconnect catches up from the last fully fetched history watermark, not the
  largest live event. Generation/abort guards reject stale history work. The
  recent 200-record view can page older within a 1000-record window.
- Logical state, runtime attachment and browser connectivity have separate labels.
  Unknown is not Failed; detached is healthy; cancellation acknowledgement is not
  confirmation. Creation/follow-up/cancel/reconcile and only actually offered
  approval decisions go through Kernel APIs. No concurrent prompt queue exists.
- Desktop sidebar/detail and phone list/detail layouts prioritize operational
  information. React text rendering, escaped terminal/bidi controls, bounded
  text/scroll containers and semantic focused controls protect worker output.

Validation: 28 deterministic Rust tests passed (2 optional checks ignored by
default), including new static/auth/config boundary tests. Clippy with warnings
denied passed. Ten frontend deterministic tests, TypeScript and production build
passed. Browser acceptance uses a private real daemon and deterministic worker:
creation, commands/files, model option, deny/approve-once, cancellation, two
observers, closure during active work, refresh, daemon restart, idle detach/lazy
follow-up, historical pagination, network loss/catch-up, Unknown/reconcile, safe
text and desktop/phone screenshots. Windows ConPTY and native terminal error/panic
guard regression passed. No real Codex/model call was made in Phase 4; no CI
workflow was added. Final audit: all three browser acceptance scenarios passed,
including independent unreachable/readiness/backend-availability UI states.
Formatting passed, and locked dependency metadata retains the Rust 1.89 floor.
Browser layouts were checked at desktop, laptop, tablet and phone widths without
horizontal overflow; desktop and phone screenshots were visually inspected.

The completion audit found a selection gap: a newly created or previously
unselected session could have durable activity after the initial stream snapshot.
Initial history now freezes the highest observed server order, retains live events
during selection, and advances coverage only after the frozen fetch completes.
Browser acceptance explicitly checks creation history and work while unselected.

The in-app Browser had no connected browser instance. Local Chromium/Playwright
provided opt-in acceptance and screenshots, which were visually inspected. An
existing user daemon locked the default Windows executable; validation used
`target/phase4` and private copies without stopping that service. Acceptance scripts
accept `LAIN_TEST_BIN_DIR` for this normal concurrent-development case.

Limitations: root-origin Web hosting, explicit credential re-entry, no public
Internet/TLS/cookie-session/CSRF design implemented, JavaScript safe-integer cursor
ceiling, and a bounded view rather than the full worker transcript. Native Linux
worker/terminal lifecycle remains unverified. Before Phase 5, settle profile
capability/resolved-view contracts and remote authentication/rotation/security if
needed. See `docs/WEB.md` for commands, recovery and actual deployment limits.

### 2026-10-05 — Web worker error explanations

A real Codex turn emitted retryable errors and subsequently completed, but the
Web parser discarded the Kernel's safe `worker error; backend will retry`
explanation. The timeline now preserves and displays normalized failure messages
for error observations, disconnects and turn failures; the latest turn's failure
also remains visible after snapshot reconstruction. Messages use the existing
bounded text renderer. Error observations do not alter session/turn state, and raw
provider diagnostics remain private. A focused frontend regression covers retry
semantics, snapshot failure parsing, malformed messages and safe text rendering.
Validation: all 11 frontend tests, TypeScript and the production build passed;
`git diff --check` passed. No Kernel or worker lifecycle change was required.
