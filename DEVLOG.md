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

## 2026-10-05 — Phase 5: Runtime profiles, SecretRefs and provider switching

Milestone: `identity: "Who Are You?"`. Phase 6 has not started.

- Added the Kernel-owned SQLite profile registry and canonical workspace default/
  allowlist policy. Migration 004 is transactional, seeds `codex/local`, and adds
  revision-0 compatibility snapshots to existing Codex workers without changing
  public IDs, native thread/turn identities, approvals or history. Invalid legacy
  worker metadata rolls back the migration. No profile deletion cascades exist.
- Definitions start at revision 1; owner replacements require the expected
  revision. Disabled profiles reject new creation while existing work continues
  with its persisted snapshot. Selection uses explicit ID, workspace default,
  then Kernel default; policy denial is explicit with no fallback. Policy is
  checked on future prompts and attachment, including already attached workers.
- Non-secret resolved configuration is committed atomically with session creation
  in `worker_state.resolved_profile`: profile/revision, backend/driver, model,
  provider/endpoint, effort, sandbox, approvals and SecretRef. Defaults precede
  profile values and narrow model/sandbox overrides. Initial structured responses
  capture unspecified local model/effort. Registry edits never rewrite snapshots,
  restart workers or alter history. Managed official endpoints resolve explicitly.
- The first Secret Store permits only `secret://env/<lowercase-hyphen-name>` to
  `LAIN_SECRET_<UPPER_UNDERSCORE_NAME>`. No arbitrary env/argv public surface,
  credential persistence or plaintext CLI argument exists. Missing credentials
  fail before session creation. Values live in non-serializable zeroizing launch
  and runtime redaction buffers; each cleared/allowlisted child receives only its
  required `LAIN_PROFILE_CREDENTIAL`. Ordinary Codex tool environments exclude it.
- Verified the existing exact Codex 0.160.0 gate with installed help, regenerated
  structured schemas and native probes. Adapter-owned launch specs use process
  `-c` provider/model/effort settings and thread start/fork/resume model, provider,
  sandbox and owner approval settings. Managed providers speak Responses over
  HTTP. Global Codex config/authentication is never rewritten. Existing local
  authentication and unmanaged tool/home/endpoint settings remain externally owned.
- Profile inspection/validation reports safe local prerequisites and credential
  status, consuming no provider/model quota. Owner CRUD, workspace policy and
  frozen-session inspection APIs distinguish current registry and actual session
  revision. Invalid extractor values return safe JSON without echoing secrets.
  CLI adds profile list/show/validate and work start --profile. Web selection uses
  Kernel profiles, policy and advertised overrides; detail shows the frozen
  revision. TUI still renders old sessions and safe reconstructed turn failures.
- Worker failures now distinguish reliable structured authentication, rate limit,
  permission, provider connection/server and execution evidence, with unknown as
  fallback. Malformed known variants also remain unknown. Optional retryable
  decodes absent fields in old records; intermediate observations never terminate
  a turn. Raw provider prose is discarded. Redaction includes known daemon secrets
  and the runtime credential before commands, outputs, approvals, history and SSE.

Validation: 34 deterministic Rust tests passed, including Phase 4 migration and
rollback fixtures, new revision versus frozen sessions, disable/resume, concurrent
profiles with distinct endpoints/models/secrets/effort/approvals, redaction and
safe error reconstruction. Workspace formatting and all-target Clippy with
warnings denied passed. TypeScript, 12 frontend tests and production build passed;
all four private-daemon browser acceptance scenarios passed. Desktop and phone
screenshots were inspected. Native Windows ConPTY/terminal restoration passed.
The deterministic API/CLI smoke passed owner CRUD, safe errors, workspace policy,
detach/lazy resume, daemon restart and credential rotation under the same handle,
checking SQLite, CLI/API output and logs for fixture-secret leakage.

Two opt-in native Codex checks passed without inference: simultaneous App Server
config/read isolation and unused durable thread fork/resume. No second real
gateway credential was supplied, so paid two-provider/model access is unverified
and unclaimed. Ordinary tests consume zero model quota; no CI workflow was added.
Validation used `target/phase4` and private executable copies because an existing
user daemon can lock the default Windows binary; that daemon was not stopped.

Limits: one implemented worker family; namespaced environment secrets require a
daemon restart for rotation; configuration isolation is not an OS secret vault.
Local Codex auth/tool/endpoint defaults are not a full copied/frozen Codex home.
No arbitrary headers/auth commands, Chat Completions provider, live session
reconfiguration or automatic provider selection. Before Phase 6, decide the Lain
Actor's model/auth/budget boundary independently from worker profiles; retain
explicit owner-only profile mutation and no raw credentials in Actor/memory.
Remote browser authentication/TLS remains the separate ADR 002 decision.
ADR 003 records mutable definitions, frozen resolved snapshots and runtime refs.

## 2026-10-12 — Phase 6: CLI-first Actor implementation

Priority changed: Web is retained and frozen. Rust Kernel/CLI and Lain's own
capabilities now lead development. Phase 7 has not begun. This is implementation
progress, **not a completed cross-platform milestone**: native Windows acceptance
and real-provider acceptance are still outstanding. A final atmospheric milestone
such as `voice: "I Am Here"` should wait for the required evidence.

### Implemented

- `lain setup --local` creates per-user settings and a separate 256-bit OS-random
  token, exclusively and outside Git. A setup lock serializes repeated setup;
  existing credentials/settings are validated/reused, never silently overwritten.
  Unix validates owner/single-link ordinary files and 0600/0700; Windows uses
  native PowerShell/.NET current-SID owner-only DACLs, rejecting other access and
  reparse/linked files. Environment token overrides and explicit remote client
  mode remain available; local credentials are not forwarded to another URL.
- Foreground `lain up` launches the existing native laind beside the CLI or from
  LAIN_DAEMON_PATH. It supplies saved local token/listener/absolute database and
  waits for native console/process-group graceful shutdown. Existing authenticated
  compatible daemon returns already_running; mismatched auth/version or unrelated
  port occupancy fails without killing any process. No embedded Kernel/service.
- `doctor` reports CLI/PATH/executables, authenticated readiness, SQLite quick_check,
  backend availability/version, Runtime Profiles, missing SecretRefs and independent
  Actor setup, without model/provider calls. Explicit commands and remote HTTP/SSE
  remain compatible. Added watch, chat/ask, conversations/conversation inspection,
  actor show/cancel/approve/deny; JSON result/error semantics are documented.
- Kernel `actor/` separates domain, direct Responses model adapter and closed typed
  tool registry. Config is explicitly independent of Codex worker profiles/login;
  SecretRef resolves credentials only for the provider header. Persona is the
  versioned identity/persona.md with optional owner startup override. No coding
  tools, shell/file/HTTP/SQL access, worker auto-approval or profile-mutation tool.
- Migration 005 transactionally adds indexed Lain conversation and ActorTurn
  records, with distinct conversation/turn/message/action UUIDs, timestamps,
  request deduplication, revisioned states and tool provenance. Accepted user text
  commits before inference. Same conversation/request ID/text returns the original
  turn; changed payload conflicts. Restart marks unfinished execution Unknown and
  never replays inference/mutations. Pending proposals remain reviewable.
- A deterministic recent context window bounds history to 48 KiB/the latest 20
  turns, plus stable persona and typed tools. The runtime bounds model requests,
  tools/errors, active time, output/request/context/result/frame sizes and reported
  usage. Genuine provider SSE is consumed, but only complete redacted committed
  text/actions become public snapshots. No raw deltas, hidden reasoning or secrets
  in storage/history/logs. Mid-generation continuation is not claimed.
- Read tools reuse Kernel state; every mutating tool persists Proposed and requires
  explicit owner confirmation. A duplicate decision conflicts. Existing Supervisor
  validates workspace/profile policy and drives delegation. Real WorkSession ID
  commits as a partial receipt before prompt submission; acceptance never means
  worker completion. Partial/uncertain outcomes retain provenance and cannot be
  replayed under the same action. Owner denial performs no mutation.
- Documentation and ADR 004 describe commands, model/secret/persona boundaries,
  storage/context/confirmation, native acceptance and deferred work. Web source
  is untouched; existing Rust browser/API compatibility tests remain enabled.

### Validation evidence

Native macOS (Darwin, Rust 1.92.0), `zsh scripts/check-native.zsh`:
locked workspace build, formatting, all-target Clippy with warnings denied,
49 deterministic Rust tests passed; 3 existing opt-in tests remained ignored.
Native CLI smoke passed local permissions/reuse, startup/already-running,
auth mismatch/unrelated port, no-call doctor/PATH diagnostics, real local fixture
Responses streaming, typed reads/proposals, restart-preserved confirmation,
confirmed real fake-Codex WorkSession, session/history continuity, duplicate request,
secret-free SQLite/logs and native graceful shutdown/reopen. macOS PTY acceptance
also exercised chat resumption, exact-action owner confirmation, exit and Ctrl-C
at an input prompt without hanging on stdin.

The real-provider smoke entrypoint was verified to make no calls without its
explicit `--run-real` flag. No real Actor credential was supplied/configured for
acceptance, so paid Actor/Codex acceptance was **not run and is unverified**.
`native_smoke.py --platform windows` correctly refuses a macOS host; this is an
anti-emulation check, not Windows acceptance.

| Platform / acceptance | Result |
|---|---|
| Native macOS build/test/CLI/PTY/permissions/signals | Passed, deterministic fixtures |
| Native Windows PowerShell build/test/CLI/ACLs/Ctrl-Break | Unverified; no Windows host available |
| Native Linux server build/test/process lifecycle | Unverified; no Linux host used |
| Real Actor model + real Codex delegation/chat/restart | Unverified; explicit paid acceptance not run |
| Cross-platform Phase 6 Definition of Done | Pending native Windows and real acceptance evidence |

### Handoff / limits before Phase 7

Run `powershell -NoProfile -File scripts/check-native.ps1` on Windows; do not
substitute cross-compilation/emulation. Linux can run the locked Cargo checks and
native_smoke.py --platform linux. Run the opt-in actor_smoke.py only with deliberate
quota authorization and real model/Codex setup, then complete its documented live
chat, independent worker execution and daemon-restart checks. No automatic CI or
paid test was introduced.

Complete-record streaming instead of token-by-token display, no true mid-inference
resumption, environment-secret restart rotation, indefinite conversation retention,
latest-100 conversation discovery, exact UUID work references and foreground-only
launcher are current limits. Existing project databases need an intentional saved
local database-path selection; setup does not silently move/import sessions.
No semantic memory, MemoryCandidates, SessionSummary extraction, World, scheduler,
automatic routing or other Phase 7 work was started. No milestone commit was made.

### Local Codex refresh and additional macOS validation — 2026-10-12

- Updated the owner's standalone Codex from 0.154.0 to the adapter's verified
  0.160.0 using curl and the official OpenAI GitHub macOS ARM package, without npm.
  Verified the package SHA-256 against release metadata before installing and
  atomically switching the existing standalone current symlink. Retained 0.154.0.
- Re-ran `zsh scripts/check-native.zsh` on native macOS with Rust 1.92.0:
  locked build, formatting, warnings-denied Clippy, all 49 deterministic Rust
  tests and native CLI/PTY smoke passed. Three opt-in tests remained ignored.
- Explicitly ran `probe_profile_launch_configuration -- --ignored`: the real
  Codex 0.160.0 App Server passed the two-profile configuration isolation check
  without inference. A fresh authenticated Kernel reports this backend available.
- Ran `python3 scripts/codex_smoke.py --run-real --phase3` with the owner's
  existing ChatGPT login: passed unused-thread idle detach, two short real model
  replies, same-session lazy follow-up, durable history, retained provider identity
  and two daemon restart/recovery cycles. This is Worker acceptance only, not
  combined Actor/Worker acceptance. The first run exposed a Python 3.14 SSE reader
  close race after each successful turn; fixed the test helper to join its reader
  after socket shutdown and before closing the HTTP connection. Verified the fix
  with 30 local HTTP/SSE cycles and 30 real Kernel SSE/repeated-close cycles,
  without reader failures or additional model calls.
- An already-running daemon retains its startup-resolved old executable and
  backend health until restarted; updating the standalone symlink alone does not
  refresh that daemon. No owner daemon was interrupted by these private tests.
- Real Actor credentials/configuration remain absent, so real Actor chat and
  confirmed Actor-to-Codex delegation remain unverified. Native Windows and Linux
  also remain unverified; cross-platform Phase 6 acceptance is still pending.

### Owner-reported real Actor conversation — 2026-10-13

The owner supplied a native macOS `lain chat` transcript after restarting the
Kernel with explicit Actor configuration. Doctor reports configured, and the
conversation diagnostics identify the `openai-responses` driver and
`mimo-v2.6-flash` model with an available namespaced credential. Two ordinary
Chinese messages received real assistant responses. This establishes a manual,
owner-reported ordinary Actor conversation check; the startup "You're here."
greeting itself is static CLI text and is not inference evidence. No credential
or raw provider traffic was requested or recorded, and the coding agent did not
repeat these paid calls.

The transcript also exercises `/help`, `/status`, `/conversations` and the empty
initial `/history`. `/conversation` is unsupported; `/exit` must occupy the
whole input line. It does not yet demonstrate typed model tool calls, confirmed
Actor-to-Codex delegation, independent worker completion, or conversation recovery
after client/Kernel restart. Combined real acceptance and native Windows/Linux
acceptance remain pending.

The owner reports slow replies. The adapter requests and consumes provider SSE
but currently exposes only complete, redacted response records to the CLI;
provider token deltas are discarded. This explains the absence of incremental
display, not the provider's total latency: no timing measurements were supplied.
The transcript also contains a repeated `You>` prompt; its precise trigger has
not been reproduced. Incremental display and input prompt handling remain UX
follow-up work, not completed fixes.

### Actor streaming foundation checkpoint — 2026-10-13

- Added the small shared `lain-wire` crate with typed Actor IDs, states, tool
  calls, snapshots and ephemeral preview events. Durable Actor records are
  validated before writes and after reads; legacy complete message bodies with
  active turn metadata are normalized without changing identity or content.
- The Responses adapter now parses multiline SSE data and fragmented UTF-8,
  forwarding assistant text through bounded incremental redaction. Ambiguous
  credential suffixes are withheld, including Unicode case-fold equivalents of
  sensitive patterns. Tool argument deltas and reasoning remain private.
- Actor event streams retain snapshot-first revision semantics and add ordered
  preview identities/checkpoints. Previews are transient and may be discarded;
  validated complete responses remain the source of durable messages and tools.
  Added measured queue/provider/commit timings where available.
- Pre-commit checks found and fixed a flattened preview DTO deserialization
  failure, with a regression test covering every preview variant.
- Native macOS `zsh scripts/check-native.zsh` passed: locked workspace build,
  formatting, warnings-denied Clippy, 52 deterministic Rust tests and the native
  CLI/PTY fixture smoke. Three opt-in tests remained ignored. No paid inference
  was run, and no owner daemon or ignored Actor configuration was changed.

This is a foundation checkpoint, not Phase 6.5 acceptance. The CLI still renders
committed snapshots; incremental terminal rendering, end-to-end display timing,
the repeated `You>` reproduction/fix, reusable local Actor configuration and the
expanded real delegation/restart checks remain pending. Native Windows and Linux
are unverified. Real provider preview behavior and combined Actor/worker
acceptance are unverified. Web remains frozen and Phase 7 remains deferred.

### Incremental CLI and prompt regression — 2026-10-13

The CLI now decodes validated typed Actor snapshots/previews, displays sanitized
text during inference, and reconciles checkpoints/finals by identity and prefix.
Duplicate/stale preview sequences are ignored; gaps/discards/failures are marked
unconfirmed. Terminal controls are escaped. JSON ask keeps final-only stdout;
optional `--timing` diagnostics use stderr and contain measured durations only.

A native controlling-PTY reproduction established one concrete `You> You>`
trigger: canonical terminal CRLF became two newlines, and the background stdin
thread queued the empty second line, causing two adjacent prompts. It does not
prove the owner's original trigger was identical. Removed that stdin thread in
favor of one serialized, scoped raw line read with terminal restoration. LF
(Ctrl-J in Crossterm raw mode) and CR are handled explicitly, and paired LF is
consumed rather than printing another prompt or answering a later confirmation.
Empty ordinary input retains the current prompt; empty confirmation denies.

Native macOS locked build, fmt, warnings-denied Clippy and 59 deterministic Rust
tests passed, with three opt-in tests ignored. Extended native fixture/PTY checks
passed, including gated Chinese preview visible while the turn remains Running,
long/multiline output, final deduplication, no-preview completion, partial failure,
LF/CRLF/slash/consecutive inputs, JSON/timing separation, denial, existing approval,
Ctrl-C/EOF at input and confirmation, generation cancellation, exact terminal
restoration, persisted conversation reopen, same-generation preview checkpoint
reconnect and continued Kernel inference after client loss. Parser tests cover
multiline/byte fragmentation/private argument deltas/failure/incomplete/empty
output; slow-subscriber and contradictory durable-state tests were added.

No paid provider was called and no owner daemon/configuration was changed.
Native Windows/ConPTY and Linux remain unverified. Real provider incremental
output, typed reads, denial/confirmed delegation and combined recovery acceptance
remain pending. Local configuration reuse is the next implementation item.
See ADR 005 for the preview contract and the actual redaction limitations.

### Reusable local Actor files and startup diagnostics — 2026-10-13

Added `config actor set --file`, `config actor env-file --file`, `config actor
[show]` and `clear`, plus one-launch `up --env-file` and matching doctor selection.
Saved paths live in existing private per-user settings and are replaced atomically
under the setup lock. Selected files, including the owner's ignored Actor JSON,
are never copied, overwritten or migrated. Explicit process environment overrides
selected literal env-file values, then saved non-secret settings, then defaults.
Only Actor/persona paths and namespaced secrets are accepted; no shell execution,
interpolation, arbitrary-key injection or workspace/cwd discovery is introduced.
Credential files and parents require owner-only native permissions/ACLs outside Git.

Doctor compares validated non-secret Actor JSON/persona startup revisions and
reports reliable mismatches, unobservable old daemons or excluded remote mode.
Credentials are not hashed or compared; key rotation requires explicit restart.
`up` reports mismatches/open work instead of interrupting an existing process.
The documented workflow inspects active workers, stops the owner's foreground
launcher gracefully and starts it again with the same database. No hot reload,
guessed-PID killing or remote daemon control was added.

Native macOS `zsh scripts/check-native.zsh` passed locked build, fmt,
warnings-denied Clippy, 62 deterministic Rust tests and the extended native
CLI/PTY fixture smoke; three opt-in tests stayed ignored. New evidence includes
export-free saved configuration, protected credential sources, spaces in paths,
literal LF/CRLF parsing, precedence, rejected unsupported keys without secret
echo/settings changes, changed-config detection/no interruption, explicit env-file
selection, restart application and retained Actor/WorkSession IDs without model
regeneration. Public fixture streaming also checks character-split known-secret
withholding and no sensitive durable assistant message. Windows-specific ACL tests
are implemented but Windows/ConPTY and Linux remain unverified. No paid provider
was called and no owner daemon/configuration was changed. See ADR 006.

Remaining Phase 6.5 work includes the final wire-state audit, expanded opt-in real
provider/delegation/worker-terminal/restart acceptance and a conditional Phase 7
handoff. Real acceptance still requires explicit owner provider/workspace approval;
no Phase 6 completion or cross-platform acceptance is claimed.

### Typed durable messages and reviewable real acceptance — 2026-10-13

Separated complete message-body status from ActorTurn state and typed all message
constructors. New private message-format metadata rejects contradictory current
records; versionless Phase 6 bodies preserve identity/content while normalizing
legacy turn-status metadata. Recovery revision increments are checked. Provider
completion timing is captured on the completed SSE event, before final validation.

Added read-only `lain work turns`, compact flushed JSONL for machine work watch,
and shared genuine Unix/Windows signal registration for launcher/chat/watch.
Extended the existing opt-in Actor runner with Chinese conversation, measured
preview-before-completion, all five read tools with receipts, owner denial,
separate exact approval, duplicate-decision rejection, actual worker terminal
observation, macOS chat exit/reopen and read-only post-restart hash/ID recovery.
Safe evidence contains IDs/hashes/states/durations, not prose or credentials.
Fixture execution is explicitly labelled and does not establish real acceptance.

Native macOS `zsh scripts/check-native.zsh` passed locked build, formatting,
warnings-denied Clippy, 63 deterministic Rust tests and all CLI/PTY/native fixture
checks, including the expanded runner itself; three opt-in tests remain ignored.
Default real-runner invocation makes no model calls. Native Windows/ConPTY/Linux
and combined real-provider acceptance remain unverified. No paid calls or owner
daemon/configuration changes occurred. The acceptance record and conditional
Phase 7 inputs are in `docs/PHASE_6_5_ACCEPTANCE.md`; memory remains unimplemented.

Additional cancellation/partial-creation/history/worker-invariant probes found
in the workspace are being reviewed before real acceptance. This checkpoint is
not final Phase 6.5 acceptance and does not close those review items.

### Cancellation ownership and atomic delegation receipts — 2026-10-13

Reproduced four concrete defects from the workspace's independent architecture
probes before fixing them. A caller dropped while storage was queued could leave
Running/Executing Actor state without an execution owner. Backend startup could
fail after durable session creation but before the Actor learned its ID. Bounded
Actor history could retain the original page cursor after dropping tail events.
Worker recovery accepted confirmed cancellation on a Completed turn or a resolved
approval without a valid owner decision.

Submission and confirmation now transfer to Kernel-owned tasks before storage
awaits; supervisor creation also owns its handoff. WorkSession, worker metadata,
creation event and the exact confirmed Action's partial receipt commit atomically
before backend startup. A receipt-commit failure rolls creation back; startup
failure keeps its ID. Bounded history advances only through returned records and
marks additional pages; worker validation rejects the contradictory states before
recovery writes. The original untracked review probes were preserved unchanged;
new permanent regressions assert the corrected behavior.

Native macOS `zsh scripts/check-native.zsh` passed locked build, fmt,
warnings-denied Clippy, 69 deterministic Rust tests and full CLI/PTY/native
fixtures, including the opt-in runner against fake providers. Three opt-in Rust
tests remain ignored. No paid model calls, owner daemon restart or owner
configuration edits occurred. Native Windows/ConPTY and Linux remain unverified;
real provider/tool/delegation/restart acceptance remains pending explicit owner
provider/workspace authorization and an available protected credential source.

### Final Actor completion commit boundary — 2026-10-13

A final timing audit found a second-write failure window: the complete body could
commit Completed before its measured timing metadata, leaving false terminal
success if that final write failed. The body now commits while Running; terminal
state and metadata commit together before publication. Injected final-write
failure preserves the complete body with an Unknown turn rather than publishing
premature completion. Reconnection therefore cannot observe the intermediate
write as an authoritative completed turn.

Native macOS `zsh scripts/check-native.zsh` passed again after this change: locked
build, fmt, warnings-denied Clippy, 70 deterministic Rust tests and all native
CLI/PTY fixtures. Three opt-in tests remain ignored. No paid calls were made.
Real acceptance is awaiting the requested owner provider/workspace authorization
and protected env-file path; Windows/ConPTY and Linux remain unverified.

### Authorized native macOS Actor/Codex evidence — 2026-10-14

The owner explicitly authorized up to 36 Actor requests plus one read-only Codex
task in this repository, then separately approved the exact real work.create
Action. Added a tracked credential-file placeholder example; created the actual
owner file outside Git with 0600 permissions in the existing 0700 Lain directory.
The owner filled it. Saved its explicit selection and the existing ignored Actor
JSON without rewriting either file. No credential value entered Git, summaries,
ordinary logs or model prompts.

MiMo `mimo-v2.6-flash` provided actual Chinese incremental text while Running,
real work.list/profile.list/profile.inspect/work.inspect receipts, and a real
work.history receipt in an isolated conversation. A first attempt described a
second proposal without calling its function. The main-conversation history
request also omitted the tool call; one explicit follow-up ended Unknown without
an action and another still returned prose. Those failures remain recorded. A
clear separate-proposal request succeeded; the isolated history probe succeeded.
These observations establish protocol compatibility with intermittent model/tool
selection limits, not one uninterrupted successful run or universally reliable
natural-language delegation. No prose was counted as execution.

Denial left zero workers. The separately reviewed approval created WorkSession
`e51a67d9-18aa-49d3-9a04-21fb4be8f4ec`, worker turn
`7592fbce-1b41-4788-bcc8-63e3932874e2`, which reached Completed with no terminal
failure. Duplicate approval returned HTTP 409. Codex 0.160.0 recorded four
retryable provider-connection errors before recovering; some inspection commands
returned nonzero status, so successful turn completion does not mean every
command succeeded. No worker permission was auto-approved and no mutation was
replayed. Total actual quota use: 27 Actor provider requests and one Codex task.

Measured first useful CLI text was 785 ms / 6760 ms in two requests; provider
first text was 763 ms / 6734 ms and full CLI wall time 4775 ms / 16522 ms. Streaming
worked in both; the larger delay preceded the provider's first useful text.
Safe summaries contain only IDs/hashes/states/durations in the private per-user
acceptance directory, outside Git. The original partial attempt, main evidence
and isolated history probe are retained and cross-referenced without double
counting quota. The runner now supports a smaller per-run request budget,
explicit function prompts and supplemental read-snapshot recovery checks.

Actual PTY chat exit/reopen preserved real conversation records. After verifying
no running turns/pending approvals, the task-owned debug daemon stopped gracefully.
Both local Rust binaries were updated with locked Cargo installation; the
installed release launcher/daemon restarted with saved files and the same DB.
Main/partial/supplemental Actor snapshots and completed worker digests retained
identity/content. Recovery queries made no inference; the new daemon's Actor
model-call count remained zero. The installed daemon is left ready for local use.

Final `zsh scripts/check-native.zsh` passed locked build, formatting,
warnings-denied Clippy, 70 deterministic Rust tests and full native CLI/PTY/runner
fixtures. Three opt-in tests remained ignored. Budget-boundary checks consumed no
quota; supplemental-digest contradiction/recovery was tested with fixtures.
Windows/ConPTY and Linux remain unavailable/unverified; cross-platform Phase 6
acceptance is not claimed. Web remains frozen, Phase 7 is unstarted, and its
conditional handoff stays in `docs/PHASE_6_5_ACCEPTANCE.md`. Ordinary commits are
used while native platform acceptance is incomplete.

### Owner chat walkthrough and delegated-result delivery gap — 2026-10-14

After the implementation and authorized real acceptance recorded above (latest
pushed checkpoint `e883425`), the owner supplied a normal `lain chat` walkthrough
asking Lain to read and introduce a separate local project, `sjtu-automata`.
This follow-up was explicitly scoped to explaining the observed behavior. No
runtime/CLI behavior was changed and no new implementation was authorized by
that discussion.

Observed and verified flow:

- Conversation `dc8294fc-bc24-4ddb-9be2-8ba65f068061` queried profile.list, then
  requested an explicit daemon-host workspace rather than inventing a path.
- A bare absolute path was rejected as an unknown slash command. Prefixing it
  with ordinary text passed it to the Actor. Source inspection confirmed the
  CLI's broad `starts_with('/')` fallback, which currently conflates path input
  and commands.
- The owner reviewed and confirmed an exact work.create proposal using
  `codex/local`, `read_only`, and a project-introduction task. The Kernel returned
  the actual WorkSession `db707cf0-17d3-4088-b3f4-8c91151d6132` and worker turn
  `997fe844-1490-4466-a316-5e5d807820db`.
- At initial inspection the worker was Running. Later read-only Kernel
  observation confirmed Completed and retrieved its actual completed report.
  The worker reported a Python CLI project overview and that it had neither
  modified files nor run the application. Its full report/project contents are
  not copied into this development log or promoted to semantic memory.
- No additional Actor inference, worker creation, permission approval, mutation
  replay or daemon restart was performed for this explanation.

The distinction behind the visible transcript is important: Action Executed
means the confirmed delegation operation ran; `stage: submitted` means a worker
turn was accepted; `worker_completion: not_observed` explicitly does not claim
that worker execution finished. The confirmation continues the same ActorTurn;
it does not itself require a new model turn. The CLI returns `You>` when that
ActorTurn finishes, while the independent worker can continue.

Current chat has no worker-terminal-result bridge back into the conversation.
Even after the worker completes, no automatic Actor follow-up or chat result is
produced. Asking Lain to inspect/summarize the result therefore normally creates
another ActorTurn/model request. Direct `work watch`, `work turns` and
`work history` read operational state without invoking the Actor. Retrieving or
summarizing a result is also distinct from verifying that the work is correct.

Three UX gaps were identified, not fixed:

1. Absolute paths can be misclassified as slash commands.
2. Human chat prints full JSON on successive tool-state changes, making the
   receipt noisy; exact mutation arguments must remain reviewable in any future
   presentation change.
3. A submitted task's completed result does not return to chat automatically.

The owner's feedback is that requiring a manual second message to obtain the
result is cumbersome. The desired follow-up experience is one request plus
explicit mutation confirmation, then automatic delivery of actual completion or
failure. Internal retrieval/summary may still need a separate bounded Actor turn;
the owner should not have to send another message just to trigger it. Event
ownership, durable correlation, reconnect/deduplication, output coordination and
any additional model-call budget remain design work, not an accepted or deployed
implementation. These are unchecked ROADMAP follow-ups. Existing confirmation,
worker independence and operational-history/semantic-memory boundaries still
apply. Phase 7 remains unstarted.

Validation for this recording: re-read the current CLI/Actor implementation,
checked the real WorkSession/turn and completed normalized report through read
APIs, and ran documentation diff/consistency checks. No code changed, so no new
build or model-based test was run. Earlier native/real acceptance evidence and
its platform/provider limitations remain as recorded above.

### 2026-10-14 — Future intent-aware subagent delegation proposal

Recorded an owner-requested **optional Delegation Compiler** for future Phase 10
orchestration in `docs/INTENT_AWARE_DELEGATION.md`. The goal is to improve handoffs
of vague user requests using authorized project context, confirmed preferences,
constraints and targeted clarifications, **without silently changing intent**.
Proposed modes: Off (default), ContextOnly, Clarify, Propose. The exact original
request stays separate from attributed contextual additions and inferred claims;
ordinary Lain chat and a worker's own planning are unaffected. Approvals, scope,
secrets and profile policy remain Kernel-owned. Updated Roadmap, Product,
Architecture, Memory and AGENTS.md guidance accordingly.

Documentation-only future proposal. No dispatch/runtime behavior, migrations,
model calls, implementation tests or Phase 7 work were introduced. Phase 6.x
completion/result delivery and native-platform acceptance remain the priority.

### 2026-10-14 — Clarified owner intent: faithful rewriting, fully transparent dispatch

The owner confirmed that the future compiler **may materially rewrite the task
instruction sent to a subagent**, provided it remains faithful to the user's
resolved intent. The owner must always know **whether the actual submitted text
was modified or unchanged**, and be able to inspect what was really sent. This
is not black-box optimization. Retain the verbatim original as a separate
historical record; track clarifications/resolved intent, final worker-bound text,
Changed/Unchanged classification and the persisted dispatch receipt. The
previous absolute-sounding prohibition of "rewriting" was corrected across the
design, Product, Architecture, Roadmap and AGENTS.md.

Mode names, Off default and any **additional** per-text-change approval flow
remain proposed interaction designs, not owner-finalized requirements. Existing
Kernel mutation/permission approvals continue. Documentation only: no runtime
or data migration, no inference or implementation tests, no Phase 7 start.

### 2026-10-18 — Phase 6.6 preparation checkpoint

The owner supplied the next goal, **The Thread Continues: Durable Delegation &
Result Delivery**. Its central requirement follows the chat walkthrough above:
one owner request and one explicit delegation confirmation must lead to automatic
delivery of the actual worker outcome in the originating conversation. The owner
should not need another message just to retrieve the result. Retrieving a report
still does not establish that its conclusions or every command/test succeeded.

Preparation completed so far:

- fetched origin and fast-forwarded local main from `0a30c1f` to `ee0c954`,
  retaining the existing untracked `docs/reviews/` material;
- read the supplied goal and the Product, Architecture, Domain Model and Runtime
  Model documents; re-read the updated repository instructions;
- recorded the scope and unchecked acceptance work in ROADMAP. Remaining required
  document/source inspection and the concrete design are still pending.

The known gap is between confirmed delegation and later worker-result delivery:
the Actor receipt identifies a submitted operation, while the independent worker
can finish after the original ActorTurn and its observation stream have ended.
`Action Executed`, a submitted worker Turn, worker completion and owner-visible
delivery are separate facts. The next implementation must establish durable
conversation/action/session/worker-Turn correlation and conversation-scoped
notices using authoritative normalized evidence. A completed ActorTurn must not
be reopened to inject a result. Exact schema, API and delivery/acknowledgement
semantics have not yet been selected.

Required follow-through includes persisted exact instructions/receipts; bounded,
sanitized useful result excerpts; restart/reconnect recovery, ordering and
deduplication; concurrent and later worker turns; honest failed/cancelled/unknown
outcomes; and no mutation replay or model call merely to announce completion.
The absolute-path parser, noisy human receipts and asynchronous terminal output
are part of the same scope. Exact approval arguments and machine JSON contracts
must remain inspectable, and notices must preserve typed input and raw-terminal
restoration. Optional model interpretation is separate from factual delivery.

No Phase 6.6 Rust code, migration, runtime configuration or installed binary has
changed at this checkpoint. No new provider calls, worker tasks or daemon restart
were performed. This recording makes no new build/test or acceptance claim;
previous Phase 6.5 evidence remains historical. Native Windows/ConPTY and Linux
acceptance remain unverified; Phase 6.6 native macOS checks are also pending.
New real-provider acceptance must be opt-in with workspace/action authorization;
the earlier completed acceptance budget is not reused. Web stays frozen and
Phase 7, Scheduler and Delegation Compiler remain deferred.

### 2026-10-18 — Durable delegation results reach the original conversation

Implemented Phase 6.6 against `ee0c954`, following the preparation checkpoint.
The Kernel now owns a durable link from conversation/ActorTurn/confirmed Action
to WorkSession and the specific worker Turn, exact submitted instruction,
workspace, profile and receipt. Migration 006 adds delegation and notice tables.
Work creation retains atomic session/receipt semantics before backend launch;
worker Turn allocation and its source link/partial receipt commit before prompt
RPC. Work.send correlates a new Turn in the same session. Exact confirmed task
text is forwarded without rewriting. Approval, submission, execution and display
state remain distinct; no second authoritative worker-state machine was added.

Store projects meaningful normalized worker transitions in the same transaction
as worker metadata/history. Completed, Failed, Cancelled and Unknown produce
separate immutable factual notices; pre-Turn partial startup retains the created
session with SubmissionUnknown. A lost prompt acknowledgement preserves its
Starting/Unknown Turn and blocks action replay. Repeated action/outcome events
deduplicate. Completion never reopens the ActorTurn or calls a model. Evidence
contains an available actual final report, nonzero-command counts/excerpts, safe
errors, changed paths and truncation with IDs/full-history guidance. Completed
does not mean every command/test passed. Excerpts are sanitized and bounded.

Authenticated conversation notice pages/SSE reuse existing Actor/worker broadcasts
as wakeups and SQLite cursors for durable catch-up. No event bus, Scheduler,
semantic Memory or model continuation was added. Notice creation survives death
after commit and before publication. Eight-page bounded SSE delivery disconnects
slow consumers after a ten-second enqueue timeout; reconnect reads committed
pages. Notice IDs/cursors remain stable across restart. Delivery is at least once:
CLI cursors suppress ordinary reconnect duplicates, and owner display acks are
persisted after output flush. A render-before-ack crash, failed or unqueued ack
can repeat a notice on reopening; exactly-once rendering is not claimed. Acks
change presentation metadata only and are shared by owner clients.

Chat now displays independent `Worker>` notices while retaining typed characters,
wide-character wrapping, CRLF behavior and guarded terminal restoration. Other
Actor previews continue across interleaved notices and reconcile once with final
messages. Renderer dedup/error/timing state resets between ActorTurns. Background
observer/ack tasks never write the terminal. Human tool receipts are compact;
full exact proposed arguments still appear before confirmation, while existing
machine Actor JSON and complete receipts remain inspectable. Added quota-free
`lain notices <conversation-id>` and `lain delegations <conversation-id>`.
Multi-component Unix paths, drive/UNC paths and spaces pass through as ordinary
text. Ambiguous one-component slash words retain command feedback; use a trailing
slash or `path: /name` for such a directory. No daemon-host path is resolved by
the client. Existing Phase 6.5 conversations remain readable, without guessing
historical receipts into new delegation links.

Validation: `zsh scripts/check-native.zsh` passed locked workspace build, fmt,
warnings-denied Clippy, **83 Rust tests and 3 explicitly ignored opt-in tests**,
and full native macOS CLI/controlling-PTY fixtures, consuming zero provider quota.
Coverage includes confirmed/denied/duplicate dispatch, same-session follow-ups,
concurrent/out-of-order completion and conversation isolation, command failure in
a Completed Turn, rejection/crash/cancellation/lost acknowledgement, bounded
redaction, transaction rollback, duplicate projection, commit-before-publication
recovery, cursor replay/ack loss, slow consumers and older database migration.
Native PTYs demonstrated automatic delivery without another owner message or
model call, partially typed wrapped Chinese input retained exactly, frontend exit
before completion, pending notice replay, acknowledged reopen, and stable notices
after actual daemon restart. The PTY fixture now drains output while waiting for
exit, avoiding a harness backpressure hang when a notice arrives after `You>`.

Updated architecture/domain/Actor/CLI/configuration/README, ROADMAP and
[ADR 007](docs/adr/007-durable-delegation-result-delivery.md). The new
[acceptance matrix and opt-in real procedure](docs/PHASE_6_6_ACCEPTANCE.md) separate
this deterministic evidence from historical Phase 6.5 real runs. Native
Windows/ConPTY and Linux remain unavailable/unverified. No new paid model calls,
real Codex work, owner daemon restart, installed binary/configuration changes or
owner database migration were performed. The existing untracked review material
was retained. No Web/Memory/World/Scheduler/compiler work was introduced. Further
native acceptance and any newly authorized real-provider result-delivery check
are the next validation steps; Phase 7 remains deferred.

### 2026-10-19 — Independent owner-reviewed memory and project knowledge

Fetched `origin/main` before editing: latest HEAD was still `17a676d`. The owner's
Phase 7.1 instruction explicitly authorized deterministic memory implementation
while preserving pending Phase 6.6 real-provider/native Windows/Linux acceptance.
The existing untracked `docs/reviews/` material was left untouched. No subagents,
real model calls, actual Codex work, installed binary changes, owner daemon restart
or owner database migration were performed.

Reused the Kernel mutex/Store, SQLite migrations, authenticated HTTP boundary,
persisted Actor action confirmation, typed wire crate, CLI and Phase 6.6 result
projection. No new crate, persistence architecture, vector database, worker harness
or generic retrieval framework. Lain's independent personal continuity and her
agent-control role remain equally important; persona stays separately versioned.

New typed IDs/records distinguish Memory, Candidate and SessionSummary. Episodic,
ProjectKnowledge, Preference and Reflection have explicit scope and lifecycle.
Direct exact owner text gets Explicit provenance; Actor suggestions retain
LainGenerated/Inferred conversation/turn/action provenance after confirmation;
summary selections retain SummaryId/UnverifiedSelection. Kind/scope and
source/authority contradictions are rejected. Acceptance never claims independent
technical verification. Known credentials/patterns and terminal controls are
rejected before persistence; unknown/encoded secrets retain existing detection
limits. No raw transcript is automatically promoted.

Migration 007 adds indexed bounded memory snapshots, transactional FTS5 trigram
search, request tombstones, independent review candidates and factual summaries.
Literal search supports Unicode/CJK substrings and short-query fallback; one
explicit scope per page, increasing sequence order, bounded records/bytes and
cursors over retained rows. Repeated request IDs deduplicate or conflict on changed
canonical drafts. Expected revisions protect enable/disable/supersede/delete.
Deletion removes semantic/FTS data and scrubs candidate copies; independent Actor,
worker, notice and summary audit remains, with no secure-erasure claim. Unix
storage creation now uses 0600 database/lock files and 0700 new directories;
Windows uses the existing protected per-user directory/owner-only inherited ACL
boundary and still needs native verification.

Actor search/list/inspect returns explicit bounded evidence without whole-store
prompt injection; proposals atomically persist with Proposed actions. Existing
owner decision is the sole review gate. Denial/cancellation rejects; approval
atomically accepts memory/candidate without inference. Generic candidate review
cannot bypass an Actor action. An Unknown receipt after a committed acceptance
requires inspection, never automatic replay. Native CLI supports memory CRUD,
status/supersession, candidates/review/summary selection and factual summary
create/show/list, including stable JSON and typed IDs, without browser/LLM access.

SessionSummary explicitly freezes one worker Turn's normalized evidence through a
history watermark, source update time and current outcome. Reuses sanitized
Phase 6.6 evidence limits, preserves failed commands under a Completed Turn and
labels worker reports unverified. A summary is never automatically Memory or a
Candidate; selected project knowledge needs a separate owner review in the same
workspace. Phase 6.6 result notices/receipts remain independent.

Validation results follow in the acceptance record below. World, journals,
automatic extraction/personality evolution, reflection, consolidation, Scheduler,
new backends, browser changes and Delegation Compiler remain unimplemented.
Suggested Phase 7.2 is a small owner-inspectable World/notes foundation with
explicit evidence links, requiring new scope; no automatic phase advance.

Phase 7.1 validation: `zsh scripts/check-native.zsh` passed on native macOS with
`cargo build --locked --workspace`, `cargo fmt --all --check`,
`cargo clippy --locked --workspace --all-targets -- -D warnings`,
`cargo test --locked --workspace`: **96 passed, 0 failed, 3 opt-in tests ignored**,
plus full native CLI/controlling-PTY smoke. The extension exercised authenticated
memory add/idempotent retry/list/search/show, project isolation, actual fixture
daemon restart then Responses-adapter retrieval using the saved MemoryId/content,
Actor candidate denial/approval, refusal of generic action bypass, status changes,
deletion and independent summary creation/selection/acceptance. Existing Phase
6.6 PTY notice delivery, wrapped Unicode input, acknowledgement and restart
continuity still pass. All inference/work used local fixtures; provider quota was
zero. Targeted review added corrupted authority/index rejection, candidate lifecycle
validation, rollback, byte-bounded pagination and deletion of now-sensitive content.

[Phase 7.1 acceptance](docs/PHASE_7_1_ACCEPTANCE.md) maps requirements to tests and
records open obligations. Native Windows/PowerShell/ACL/ConPTY and Linux hosts were
unavailable and remain unverified. No real-provider Phase 7.1 memory conversation
was run, so that definition-of-done acceptance item is still open pending explicit
provider/budget authorization. Phase 6.6 real-provider delivery also remains
pending. Deterministic implementation completion does not imply final cross-platform
or real-provider acceptance. Ordinary reviewable commits are used; no atmospheric
milestone/Phase 7.2/Phase 8 completion is claimed.


Phase 7.1 real-provider follow-up (separately owner-authorized): one Actor turn,
maximum four model requests, current `mimo-v2.6-flash` provider, non-secret Global
preference in a task-owned temporary database/daemon, with no Codex work or changes
to the owner's existing instance/database. Direct creation and exact record equality
after a native test-daemon restart passed. The Actor CLI returned nonzero; this
is **not** a successful real retrieval acceptance. The temporary wrapper wrongly
raised before retaining the CLI failed-turn snapshot. Its generic RuntimeError
alone cannot establish the actual model-request count, terminal state or tool
receipts. The test daemon stopped, temporary database was removed, and no second
real attempt was automatically made. Provider-specific cause remains unknown.

Safe private evidence is `acceptance/phase-7-1-memory-2026-10-19-7748512f.json` under
the per-user Lain directory, with model/scope/limits, successful persistence checks,
MemoryId/content digest and generic failure only. The wrapper was fixed and tested
without inference to retain structured failed-turn snapshots/counters even on a
nonzero CLI exit. A further real turn requires additional authorization because
unused allowance from the first run cannot be proven. The 96-test native macOS
deterministic result remains valid; only this temporary acceptance script changed.


A separately authorized additional real turn used the corrected wrapper under the
same isolated scope. It **did not pass**: `mimo-v2.6-flash` made **2 requests**,
reported **4,838 usage tokens**, and ended Unknown with no executed memory-tool
receipt and no actual MemoryId citation. Native save/restart/exact-record equality
passed; no WorkSession was created; inspection retained an identical Actor snapshot;
the task-owned daemon stopped. The owner instance/database and Codex remained
untouched. Private safe evidence:
`acceptance/phase-7-1-memory-2026-10-19-aa23914a.json`, ActorTurn
`df2e27eb-3c97-4f10-9c76-0001214c3a37`, Conversation
`cc04e3c9-3293-42b1-a406-e82952ad7379`, Memory
`ca698129-487f-4bc5-8620-b44f7d7a3e12`. The provider-specific cause is unverified;
no rejected-call text or raw provider frame was preserved. The first run's request
count is still unknown, at most four; the two-run upper bound is six, with usage
known only for the second. No third run was made or automatically authorized.

Phase 7.1 implementation and native macOS deterministic acceptance remain complete,
but **final Phase 7.1 acceptance remains incomplete** because a real Actor retrieval
conversation has not passed. Native Windows/Linux and Phase 6.6 real-provider
acceptance also remain pending. Do not advance phases or use a milestone completion
commit to conceal these gaps. Further provider/tool-call diagnosis should be based
on safe evidence and any new paid probe requires its own concrete authorization.

## 2026-10-23 — QQ-first cloud residency and equal companionship: documentation

The user revisited the product boundary in detail and explicitly prioritized
daily QQ companionship. Lain should have one cloud identity with shared memory
and real experiences, interact as an equal, read selected social sources,
autonomously remember and privately share within agreed limits, and connect
local PCs for Worker execution and history. The user rejected master/servant
framing. Worker archives should include independently created sessions; archive
access does not itself authorize control. Faithful reuse of successful report
methods and bounded follow-up supervision remain product directions, with exact
interaction and resource defaults unresolved.

Rewrote PRODUCT.md as the detailed Chinese product contract, explicitly separating
confirmed decisions, provisional directions, proposals, current implementation and
open questions. Added RESIDENT_DESIGN.md and CHAT_DESIGN.md for cloud/device state
ownership, dual-account QQ, scoped authorization, provenance, correction, outgoing
delivery uncertainty, conversational timing and acceptance scenarios.
MAIBOT_REFERENCE.md records the user-selected upstream reference, observed
documentation/source mechanisms, Lain-specific adaptations and unverified limits.
The research read public main/online pages without a pinned SHA; no upstream
runtime or paid model evaluation was performed.

ADR 009 records the replacement of CLI-first ordering and universal per-item
confirmation as future product policy. ADRs 004/008 retain their historical and
current-code meanings with explicit supersession notices. Updated README,
AGENTS, ROADMAP, architecture/domain/runtime/memory and related operating/design
guides so future development sees the new direction before older phase plans.
The roadmap proposes Q1–Q3 for QQ interaction, selected observations/continuity
and bounded daily activity, followed by connected devices and terminal work.
The stages are proposals, not implementation completion or automatic authority.

This task changes documentation only. No Rust, frontend, SQL migration, compiled
persona, runtime configuration, installed binary or user database changed.
Existing per-action confirmation and Phase 7.1 candidate review still apply in
code until a separately scoped grant model and migration replace them. The
compiled persona's historical owner language is explicitly recorded as pending
behavioral work. No QQ account login, real chat collection, external messages,
production deployment or new paid inference occurred. Native Windows/Linux and
Phase 6.6/7.1 real-provider gaps remain open.

Validation: checked all 21 changed/new Markdown documents, 124 relative file
links and balanced code fences; no broken relative file links or fence imbalance
were found. `git diff --check` passed after whitespace normalization. Reviewed
status/supersession wording and confirmed no runtime/configuration/source or
acceptance-record files changed. Rust/frontend tests were not run because this
change is documentation-only.

## 2026-10-23 — Q1 decoupled private conversation implementation

Implemented the selected QQ reply-only slice on the existing Rust Kernel and Actor.
The optional OneBot 11 forward-WebSocket adapter verifies the logged-in Lain
account and accepts only the bound user's friend-private text. Group, self,
wrong-account/peer and unsupported mixed-media events cannot become Actor input.
Native IDs and protocol objects remain in the adapter; the shared conversation
service has a fixture transport and no Utakera runtime/database dependency.
Read Utakera's Bridge/source drafts at `8e30fd2` as interoperability proposals,
without adopting them as an approved source schema or implementing an observer.

Schema 008 adds durable bindings, incoming messages, bounded batches and delivery
intents/receipts. A stable batch request recovers exactly the committed Actor turn.
Queued input survives generation; send results distinguish platform confirmation,
definite non-submission/rejection and uncertainty. Interrupted claimed sends are
never automatically replayed. Fixed merge, backlog, lifetime, write/receipt timeout
and send-spacing limits bound the live path without idle model polling.

Reply permission has a persisted revision and compare-and-swap administrator API.
Revocation invalidates old queued/generated work and does not silently reset on
restart. Authorization and expiry are checked again at Actor admission and send.
Existing tool mutation, candidate review and Worker permission checks remain.
QQ “同意” is not an approval; proposals direct the user to the authenticated local
review flow. Compiled persona v2 uses equal companionship, calm/offbeat expression
and truthful experience claims. The same Actor serves the terminal and channel.

Added configuration example, operation/deployment/recovery guide and a requirement
to evidence map in `docs/Q1_IMPLEMENTATION.md`. Updated current product, architecture,
domain, runtime, configuration, Actor and roadmap descriptions; older acceptance
records retain their historical meaning. No real QQ/model/deployment acceptance,
Q2/Q3 work or personal data migration is included. The user subsequently requested
committing and pushing this implementation, with further testing handled separately.

Validation is recorded in the Q1 acceptance document. The full native Windows
workspace run passed 111 tests with zero failures and three manual/live tests
ignored; format, strict workspace Clippy and workspace build also passed.
Failure testing found and fixed a shutdown
admission race and bounded previously unbounded socket writes. Migration assertions
were updated for schema 008. The memory boundary test correctly checks that a
candidate is not active memory; proposal persistence itself is existing behavior.
Small lint fixes included an equivalent Windows input match guard. Native CLI
testing also fixed inherited PowerShell 7 module paths breaking the Windows
PowerShell ACL helper and ownership of exclusively created paths under elevated
tokens. Existing directories remain check-only and are never taken over.
No CI workflow or remote model automation was added.

Subsequent protocol review corrected outgoing OneBot `user_id` to a JSON number
and updated its fixture assertion. At the user's request, no build or test was
rerun after that correction. The pushed revision therefore retains a final
verification gap despite the preceding successful full run.

The initial portable LLVM-MinGW build passed Kernel tests but its CLI executable
failed before test entry with native access violation `0xc0000005`. A separate
checksum-verified GCC 16.2.0 / MinGW-w64 14.0.0 MSVCRT toolchain completed the full
run with `RUSTFLAGS=-C link-self-contained=no`. Tools, build output and test logs stay in the task-owned external
directory `C:/Users/example/AppData/Local/Temp/lain-q1-tools/`; no system PATH or
user runtime configuration was changed. Native MSVC, Linux/macOS, interactive
terminal acceptance and existing Phase 6.6/7.1 real-provider gaps remain separate.
