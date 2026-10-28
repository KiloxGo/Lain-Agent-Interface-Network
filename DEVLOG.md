# L.A.I.N. Development Log

## 2026-10-28 — Explicit private daily inference

The user requested local startup and then removed the proposed count/time limit
for the existing owner private conversation. Added an explicit binding-scoped
daily mode, retaining per-turn limits, durable usage, finite campaign history and
stop/restart gates. No model, memory, contact, Worker or source authority expansion.
This maintenance backport targets the installed schema16 release; validation and
installation evidence will be appended after execution. Native Windows/MSVC: 211
Rust tests passed, 3 manual ignored; format, strict Clippy and locked build passed.
The native daily-mode process smoke passed 4 checks / 4 fake model requests,
including auth, exhausted finite history, actual private replies, restart,
unbound denial and stop; cleanup succeeded. No paid test call or real QQ send.
See ADR 024.

This file records implementation history, milestone outcomes, important decisions, problems found during development, and the handoff state for the next coding session.

It is intentionally different from the architecture documents:

- `docs/*.md` describe what the system **should be**;
- `ROADMAP.md` describes what we **plan to build**;
- `DEVLOG.md` records what **actually happened**.

The Git history remains the authoritative record of exact commits.

## 2026-10-27 — W1e finite selected-source synchronization

Continued from pushed `666d1fc`. Added separately enabled source/archive-epoch
network scopes, retained revision/usage snapshots, atomic attempt/Prepared read
admission, persisted nearest-deadline scheduling and existing source cursor/update/
withdrawal handling. Restart preserves spent attempts; failed/unknown reads stop,
and revoke/expiry block late commits. Migration 021 enables nothing by default.
No generic scheduler, model calls or personal-instance upgrade is introduced.

Validation and current limits: [KNOWLEDGE_SYNC_ACCEPTANCE](docs/KNOWLEDGE_SYNC_ACCEPTANCE.md).
Durable derived continuity and the rest of the combined goal remain active work.

## 2026-10-27 — W1d explicit independent QQ source bridge

Continued from pushed `d954c80`. Added persistent independent store identity and
source-filtered current change export; deployed the additive protocol after
native Windows/server Linux service tests, with a stopped protected full backup
and compatible-container rollback that never rewinds spent vision reservations.

Kernel adds explicit private/group knowledge pulls through the existing durable
read receipt/cancel/Unknown lifecycle. Migration 020 stores source/event linkage,
atomic projections/updates/tombstones and short caught-up freshness leases. Raw
image bytes stay independent; original text and model captions retain separate
attribution. Exact same-perspective private suppression prevents a mirrored old
message from bypassing correction/forget. No automatic collection/model/Worker,
personal-instance upgrade or old budget reuse.

Evidence and remaining scope: [KNOWLEDGE_BRIDGE_ACCEPTANCE](docs/KNOWLEDGE_BRIDGE_ACCEPTANCE.md).
Continuous ingestion/finite standing network authority, durable derived memory,
real QQ images/long-term naturalness and W2–W4 remain active work.

## 2026-10-27 — W1c selected archive conversation recall

Continued after pushing W1b (`2f7a3eb`) to both development branch and main. Added
an independent default-off finite private-binding archive recall decision,
selected source/import epochs, typed archive search/inspect and fresh attributed
excerpts. Migration 019 enables nothing on existing instances. Source archive
storage is still independent of semantic memory, actual experiences and control.

Lookup selections/audit commit atomically; version checks cover source/recall
revoke/expiry, record correction/forget/retention and model adapter admission,
response, completion and delivery. Empty results retain authorization dependency.
New-input reconsideration clears selection without resetting budgets; later
model/source history cannot replay derived answers. Archive-influenced memory
proposal/remember/correct is blocked pending durable derived-memory provenance.

Seven new domain/Actor regressions and an extended native CLI/HTTP/model-SSE/
OneBot fixture exercise three-day recall, restart, correction, forget and revoke.
Final validation evidence/remaining limits are in
[ARCHIVE_RECALL_ACCEPTANCE](docs/ARCHIVE_RECALL_ACCEPTANCE.md). All this slice's
model responses are fixtures; no old paid budget or personal migration is reused.
Independent QQ ingestion into Kernel, persistent derived summaries/recognition,
actual source collection, long-term naturalness and W2–W4 remain active work.

## 2026-10-27 — W1b explicit native visible Worker archive reads

Added source/revision-scoped read receipts and explicit CLI/API operations for
local Codex visible history, separately verified for installed 0.160.1. Managed
Worker control remains gated to 0.160.0. Metadata/terminal-turn/item pagination
does not resume/control a source, run inference or create managed work. Private
native cursors continue through source/grant/version-bound Lain receipt IDs.
Reasoning/raw/unknown/diff bodies are omitted. Import/dedup and Completed cursor
commit atomically with adapter_observation origin; cancel/revoke/shutdown await
owned-reader closure, while boot recovers unfinished reads Unknown without replay.

Migration 018 appends one table/index. Existing owner imports and retention/
correction/forget remain intact. Native Windows workspace 223 passed/3 manual
ignored; strict Clippy, format and locked build passed. Six new contract/storage
tests cover pagination, gates, sensitive/mismatched sources, blocked-reader
cancellation/exit, receipt-write rollback and interrupted recovery. Fixed an old
30 ms expiry-test timing assumption using deterministic persisted expiry.
Fake process suite: 33 checks/76 fake calls; synthetic 17→18 full upgrade/old
refusal/rollback: 5 checks/3 fake calls; 0 paid and cleanup true.

An actual isolated native Codex turn used one local fake Responses request.
After restart, the final actual Lain CLI archived its user/assistant items through
read-only APIs; source rows remained unchanged, revoke withheld archived text,
and no new Worker/Lain model turn ran. Personal history/database were untouched;
schema16 personal release and hosted QQ store were not upgraded by this slice.
See ADR 020 and ARCHIVE_READ_ACCEPTANCE.md. W1 continuity/derived invalidation,
managed 0.160.1 control, W2–W4 and complete-stage/live quality remain open.

## 2026-10-27 — Independent QQ message/image knowledge service deployed

Final lifecycle follow-up: direct native-interpreter task ownership and an OS
outbox lock prevent duplicate observers. The original PowerShell wrapper left
Python children after task stop; only positively identified observers were
terminated. Actual Stop→zero processes→Start→one authenticated process passed.
The final independent service suite has 14 tests on native Windows and server
Linux, with the same 1/10 real vision reservation retained across deployments.

The user extended the active goal with decoupled QQ messages/images, DeepSeek
vision and a keyed hosted API for Utakera, explicitly authorizing server deployment.
Added a separately runnable Python source store and read-only NapCat observer:
source revisions/allowlists, ordered segments/media provenance, source-scoped image
hash/cache, bounded direct vision, durable conservative budget, retention/tombstones,
search/change feed, role-specific API keys and a durable bounded upload outbox.
No Kernel/persona/semantic-memory/Worker runtime dependency or historical DB upload.

Actual Ubuntu/Caddy/Docker deployment serves a loopback-only unprivileged container
through verified public HTTPS at hub.example.com/lain-knowledge. A real synthetic
image was stored/downloaded with equal SHA-256 and described by DeepSeek Flash:
one request, 322 reported tokens, independent lifetime cap 10. Duplicate event
delivery did not spend another call; synthetic source subsequently revoked.
No-auth 401/read 200/read-key mutation 403 passed. Three API roles/model key are
protected outside Git. Existing unrelated services were not restarted.

Native Windows and actual server Linux Python each passed 13 deterministic tests,
including an actual local WebSocket observer fixture. Ruff check/format passed.
The current-user hidden observer task is Running, actual NapCat identity verified,
outbox drained; only established bot/owner private pair enabled, groups empty.
MSIX AppData redirection initially broke scheduled-task paths; installation now
uses the physical resolved path and checks runtime health, not just registration.
Original Rust W1+D3 merge passes 217 tests/3 manual ignored and strict Clippy;
personal schema16 grants/budget remain closed. MaiBot mechanism research and
Utakera reference commits, API/operating limits and remaining actual-image/client/
Kernel-continuity evidence are recorded in ADR 019 and QQ_KNOWLEDGE_ACCEPTANCE.md.
The complete expanded goal remains unfinished; no multi-day or Worker acceptance.

## 2026-10-26 — W1a selected visible-source archive imports

Implemented the Kernel-owned archive foundation for declared QQ-group/Codex-session
sources: default-off revisioned import/disclosure scopes, bounded atomic owner
imports, exact per-source event identity, commit-order cursor windows, finite body
retention, correction/forget tombstones and content-free edit receipts. No source
import creates a conversation, model call, semantic memory, experience or managed
Worker. Owner declarations are explicitly not authenticated provider observation,
complete coverage or participation. Actual read adapters/derived continuity remain W1.

Schema 017 appends four tables/indexes and preserves prior grants/records/control.
Replays neither renew retention nor refund admission. Same-ID changed payloads
conflict atomically; different source accounts/events remain distinct. Corrections
label owner text; original-event replay cannot undo correction/forget. Grant expiry,
explicit revoke, body expiry and previous grant epochs have separate projections.
Startup/fresh-read checks and one nearest-deadline timer enforce expiry without
idle inference. CLI/API expose bounded selected JSON operations and audits.

Native Windows/MSVC: 215 Rust tests passed, 0 failed, 3 manual ignored; fmt/check,
strict workspace/all-target Clippy and locked native build passed. Eight archive
tests include credential-rotation response filtering and actual timer-driven body
clearing. Native fake HTTP/OneBot: 33 checks, 76 fake model calls, 0 paid calls;
archive cases add no inference. Whole-backup/schema16→17/old-version refusal/full
rollback: 5 checks, 3 fake calls, 0 paid, no personal state. Fourteen Python
replay/staging tests passed. Cleanup and binary hashes match; ARCHIVE_ACCEPTANCE.md
and ignored target/w1a/ retain evidence, including early compilation and stale
schema-expectation failures. No actual group or Worker history was collected.

The owner subsequently authorized the separate D3 live plan: a new fixed 48+12
request split and 6-hour deadline over the selected schema-16 release. Its protected
controller and exact source/binary snapshots are outside personal rollback state;
W1 is excluded from that installation/collection authority. Real D3 operations
and outcomes are tracked separately. The complete two-stage goal remains active.

## 2026-10-26 — D3 targeted real follow-up and reading receipt retention

Four fresh isolated memory scenes now commit/recall correction and forget using
the same original DB and budget: eight new calls, cumulative 23/48. The final
absence response remains verbose. The last allowed RFC acquisition completed with
nine form feeds and one bounded model summary (personal 1/12); quiet hours kept
sharing Prepared with zero QQ sends. All temporary scopes/budgets closed.
Post-revoke checks exposed a summary duplicate in the root Actor receipt. Cleanup
now removes that content too, including old invalidated snapshots, without losing
operation/usage facts. Native schema16 workspace 209 passed/3 manual ignored,
strict Clippy and locked build passed. Full-stage quality/live send remain open.
See docs/D3_LIVE_ACCEPTANCE.md; no personal data or credentials committed.

## 2026-10-26 — D3 live regression repair on schema 16

The new authorized campaign used 15/48 isolated real-model requests before a
private-memory correction failed safely: prefix-only automatic terms omitted
late topic words, generic global memory inspection could not inspect a private
ID, and correction lacked a valid inspection. No correction effect committed;
temporary scope/budget closed. Actual source acquisition failed UnsupportedContent,
with 0 personal model calls/0 sends. The personal target was backed up completely
and upgraded 11→16 headlessly before its finite 12-request budget was installed.

Repair keeps schema 16 and existing permission/revision guards. Private recall
supports exact receipt IDs within the active binding and bounded prefix/suffix
terms; generic inspect remains general-scope only. Bounded plain-text acquisition
retains benign ASCII form-feed page separators with JSON escaping while still
rejecting ESC/other forbidden controls. The first source failure's exact cause is
not asserted without the remaining actual acquisition. No extra URL or allowance.

Native Windows: 208 passed, 0 failed, 3 manual ignored; format/check, strict Clippy
and locked build passed. Native fake protocol: 28 checks/76 fake calls; synthetic
schema11→16 full rollback: 5 checks/3 fake calls; 0 paid/cleanup true. Real follow-up
must retain the existing DBs, budget IDs, caps and deadline, inspect failed effects
before fresh cases, and respect the now-active 23:00–09:00 contact quiet period.
Raw operational data/credentials remain outside Git. See D3_LIVE_ACCEPTANCE.md.

## 2026-10-26 — D3c isolated autonomous replay runner

Added an automatic local OneBot replay over 16 synthetic chat/continuity/memory
scenes. It records actual incoming/batch/Actor IDs, normalized model outputs,
memory effects, literal sends and simulated receipts rather than using user-sent
QQ messages. Mechanical format measurements do not certify naturalness. Real
mode creates no fake Responses server; free rehearsal is separately labeled.

The runner binds a new decision to exact binaries/config/SecretRef, immutable
campaign/budget IDs, output directory, request cap, duration and upstream-attempt
verification reference. It boots headlessly to install the budget before enabling
model/transport, expires/revokes synthetic memory independently, never approves
work, and leaves historical source collection off. Controller state is outside
the instance subtree; intent is flushed before input. Existing or interrupted
campaign output cannot be replayed. Exit closes grants/budget, stops the owned
daemon and reconciles the same database in a second headless boot; an attempted
but interrupted request remains spent and Unknown.

Ten native Windows Python boundary/process tests passed, including a complete
16-scene fake corpus, finite exhaustion, no-overwrite, tamper/proposal refusal,
secret filtering and interruption both before and after an actual local HTTP
attempt. Four release-staging tests passed; its manifest now fingerprints the
three repository runner dependencies without copying extra files/credentials.
All calls were fake/0 paid; native schema-16 Rust binaries are unchanged from D3b.
Details and retained early failures: DAILY_REPLAY_ACCEPTANCE.md, target/d3c/.

Fixed a Windows acceptance-directory ACL helper: verify the existing owner and
set only the DACL, avoiding an unnecessary WRITE_OWNER request that failed on the
workspace volume. Current-owner-only validation remains enforced. Early launcher
setup-in-Git and channel-list shape mistakes failed before model use and were
corrected in the runner, without weakening runtime credential location checks.

No personal credentials/state were read, no new real model/QQ calls or personal
upgrade occurred. D3 real-model quality, actual source→share→recall and multi-day
evidence remain pending; W1–W4 and the combined goal are not complete. Protected
credential-source access, actual upstream bridge inspection and one concrete
consolidated live decision still precede paid or personal operations.

## 2026-10-26 — D3b persistent global inference ceiling

Added a Kernel-wide optional Lain Actor inference limit over conversation,
private chat, reading, sharing, tool continuations and reconsideration. Immutable
budget IDs, revisioned enable/stop decisions and transactional Actor/request
reservations prevent turn/grant/restart refills. Adapter entry rechecks the limit;
returned normalized usage is recorded before later source/body rejection. Failure,
cancellation, timeout and restart preserve spent reservations and honest
Prepared/Attempting/Returned/NotAttempted/Unknown phases. No prompt, source body,
outbound words or credentials are stored in the ledger. Worker costs and actual
upstream invoices are explicitly outside its coverage.

Migration 016 preserves old limits until explicit opt-in and never invents paid
authority. After opt-in, stop/expiry/exhaustion stay closed until a finite owner
decision; reopening the same ID retains all usage. A budget cannot enable source,
memory, contact, Worker permissions or model access. CLI/API provide show/budgets/
requests/enable/stop without adding an Actor grant tool. Local reqwest 0.13.5
inspection revealed default protocol-safe retries; own model and text adapters now
explicitly disable retries as well as redirects.

Prepared a review-only release packager: exact evidence-matched binaries, non-secret
scope proposal and five-step full backup/upgrade/rollback note under owner-only
workspace storage. It never opens a target instance, loads credentials, starts a
service or authorizes live calls. Existing native ACL protection is shared with
the synthetic upgrade fixture. A real upgrade must bootstrap without model/QQ
configuration, install the finite campaign first, then restart approved settings.

Native Windows/MSVC (Rust/Cargo 1.98.1): 207 tests passed, 0 failed, 3 manual ignored;
format/check, strict workspace/all-target Clippy and locked build passed. Native
fake HTTP/OneBot: 28 checks, 76 fake requests, 0 paid calls. Full synthetic schema
15→16 and 11→16 backup/upgrade/refusal/rollback each passed 5 checks/3 fake calls;
no personal state access, cleanup passed. Four native Python package-staging tests
passed. Evidence and final hashes are in INFERENCE_BUDGET_ACCEPTANCE.md; local logs
and unapproved packages remain ignored under target/d3b.

No real provider, QQ contact or personal upgrade was performed. D3c still needs
the bounded autonomous real-model replay runner and concrete consolidated live
authorization, including source/retention/memory/contact/stop scope and verification
of any upstream bridge. Naturalness/multi-day evidence and W1–W4 remain open; the
combined goal is active and not complete.

## 2026-10-26 — D3a fresh sharing receipts in conversation

Connected private conversation to the actual D2 share outbox without pretending
its draft was user-visible speech. One recent content-free terminal receipt can
support follow-up; typed sharing search/inspect supplies freshly checked outbound
words and source evidence only while the original experience/disclosure remains
current. Confirmed/Unknown/NotSent/Rejected and user-read uncertainty stay distinct.
Source correction/withdrawal/revocation withholds content without denying an
actual earlier contact. Revoking contact does not erase its historical receipt.

Private ShareVersions participate in existing response/body/completion/Ready/
prewrite freshness guards and are reset on bounded reconsideration. Past share-
source-influenced assistant/tool copies do not reenter model history. Source
words cannot silently promote memory; automatic metadata does not block ordinary
reviewed/current-source memory. Migration 015 adds content-free transactional read
audits; CLI/API provide receipt recall/inspection/audit without new send authority.

Native Windows/MSVC (Rust/Cargo 1.98.1): 199 tests passed, 0 failed, 3 manual tests
ignored; formatting/check, strict workspace/all-target Clippy and locked build
passed. Final native HTTP + fake Responses/OneBot: 26 checks, 75 fake requests,
0 paid calls. Synthetic schema 14→15 backup/upgrade/refusal/full rollback: 5 checks,
3 fake requests, no personal-state access. Both report successful cleanup and
match final binary hashes in SHARING_RECALL_ACCEPTANCE.md. Actual acquisition for
that process case is seeded synthetic data, not a real public fetch.

Retained failures: omitted closed Wire ToolCall registration; invalid Running/
Proposed fixture state (replaced with actual reviewed-tool flow); unused fixture
variable; parallel global diagnostics warning capture (isolated before adapter
events). Topic search was tightened to permitted values rather than JSON schema
keys before final full-suite, lint/build and process reruns. Logs remain ignored
under target/d3, including all intermediate failures.

Personal installation/credentials/QQ remain untouched. This does not complete D3:
the concrete new live campaign budget/authorization/upgrade package and longer
real experience still need work. W1–W4 and the complete combined goal remain open.
See ADR 016 and SHARING_RECALL_ACCEPTANCE.md for exact limits and operating steps.

## 2026-10-26 — D2 independent proactive sharing

Implemented the next slice of the combined daily-companion/shared-work goal:
default-off per-binding ShareGrant, explicit revisioned attention/DND, fixed-offset
quiet windows, cooldown/day/grant limits, source-linked expiring candidates and
no-response control. New reading completions may generate a separate no-tool
Actor Share/Skip draft. Drafts, model reservations, attempts and confirmed/uncertain
QQ outcomes remain distinct; no user Batch or semantic memory is fabricated.

Migration 014 preserves old grants/records and starts contact disabled. Fresh
checks cover source/grant/experience/input/attention before disclosure and send.
Enforcement covers all historical attempts, even beyond the inspection page.
New user input takes priority; service-owned bounded send tasks keep receipt
waiting off chat inference. Restart expires unsent work, marks in-flight contact
Unknown and advances the boot watermark without catch-up or automatic retry.
CLI/API provide enable/revoke/show/consider and explicit available/DND controls.

Native Windows/MSVC (Rust/Cargo 1.98.1): 192 tests passed, 0 failed, 3 manual tests
ignored; formatting, strict workspace/all-target Clippy and locked build passed.
Actual native HTTP + fake Responses/OneBot suite: 25 checks, 69 fake requests,
0 paid calls, cleanup passed. Synthetic schema 11→14 backup/upgrade/refusal/full
rollback: 5 checks, 3 fake requests, personal state not accessed, cleanup passed.
The installed old executables were only copied as binary inputs to the fixture.
Schema 13→14 unit evidence also preserves original snapshot bytes/default-off state.

Failures retained: initial SQLite count return-type error; service timeout exposed
a missing Kernel wake-up for experience publication by another Actor; Clippy found
test-module ordering. Each was corrected before the final validation. Local logs
and hashes remain ignored under target/d2; reviewable evidence and operating limits
are in SHARING_ACCEPTANCE.md and ADR 015.

No real-model, public-source, QQ-contact or personal-upgrade operation was made.
Natural-language DND/calendar/DST, real timing/quality/multi-day evidence, complete
sharing-receipt recall integration (D3), and W1–W4 remain open. This milestone does
not complete either large stage or the active combined goal.

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

## 2026-10-24 — Q1 final-revision recheck and Windows fixture repairs

Rechecked the Rust tree at `560ddf9`, including the final outgoing numeric
OneBot `user_id` correction, on native x86_64 Windows/MSVC with Rust/Cargo 1.98.1.
Format, strict all-target workspace Clippy, locked workspace tests and workspace
build passed: **111 tests passed, zero failed, three opt-in tests ignored**.
The initial crates.io timeout was resolved using the existing Windows proxy only
for the Cargo retry; tests/build then ran offline. No lockfile, global toolchain,
proxy configuration, personal database or running QQ process was changed.

The additional native CLI smoke exposed fixture portability problems: inherited
PowerShell 7 modules, Windows canonical-path string comparison, a speed-dependent
streaming probe, a stale schema-7 assertion, and a SQLite inspection connection
left open during Windows temporary-directory cleanup. The fixture now uses
Windows PowerShell's bundled modules, file identity, an actual Running-snapshot
gate, schema 8 and an explicitly closed connection. A focused Windows check of
the actual SQLite inspection block passed immediate file deletion. The Windows
CLI subprocess bound is 90 seconds because repeated native ACL checks exceeded
the previous 30-second setup bound; other hosts retain 30 seconds.
The Windows launcher-readiness bound likewise rises from 20 to 90 seconds after
an env-file startup exceeded it; other hosts retain 20 seconds. This affects only
the fixture's waiting budget, not production startup, inference or shutdown policy.
Production Rust, channel and authorization behavior is unchanged.

The final `python -X utf8 scripts/native_smoke.py --platform windows` run exited
0/PASS, including setup/ACLs, signals/restart, doctor, streamed Actor, confirmation,
fake Worker, scoped memory/summary/Actor retrieval and persisted recovery, with
successful temporary-state cleanup and zero model quota. The embedded runner's
native interactive/ConPTY and real-provider acceptance flags remain incomplete;
this is deterministic native evidence only.

Added a Windows hands-on live acceptance walkthrough to `docs/QQ_CHANNEL.md`:
fresh owner-only local setup on port 18787, explicit Actor/NapCat files, hidden
credential entry, forward WS settings, receipt inspection, restart/reconnect and
revocation/re-grant. Its four PowerShell blocks passed native parser checks.
Official NapCat 4.18.41 Shell and Windows OneKey archives were prepared separately
outside the checkout, with hashes matched to GitHub release asset digests; they
were not launched or logged in by the coding agent. Real QQ/model/deployment,
ConPTY interaction, other native hosts and existing real-provider memory/result
delivery gaps remain unaccepted. See `docs/Q1_IMPLEMENTATION.md` for evidence.

## 2026-10-24–25 — Q1 diagnostics and Q2-A private-memory boundary

Recorded the user's manually confirmed real Chinese private QQ exchanges as
ordinary-chat evidence only. Full live reliability, memory experience and cloud
deployment, and historical Phase 6.6/7.1 real-provider failures remain open.

Added ephemeral Kernel-owned connection phase/failure observations to authenticated
channel API/CLI. Account verification gates Online; restart never restores an old
Online flag, and shutdown/task exit closes observations. Failures use safe closed
categories and repeated identical failures suppress duplicate warnings. Receive,
reply grants, send receipts and Unknown/no-replay rules remain unchanged.

Q2-A adds an explicit administrator-only, default-off, revisioned private-binding
memory grant with persistence, revocation and optional expiry. New authorization
captures an input sequence watermark; queued pre-grant sources cannot backfill.
Existing Actor turns gain bounded remember/recall/correct/forget capabilities only
while the grant is current, retaining all existing model/tool/time/token budgets.
No extraction loop, observer, scheduler or general Worker approval is introduced.

Schema 009 reuses Memory/FTS and adds grant revisions, effect receipts, source
suppression and inspected-target revisions. Autonomous selections retain actual
conversation/input/batch/Actor/action and grant provenance, Inferred authority and
no fabricated individual confirmation time. Old snapshots and confirmations keep
their exact meaning. Each request refreshes bounded relevant Active memory; target
correction/forget requires inspection and revision. Semantic effects, relations,
tombstones and receipts commit transactionally, with idempotent action IDs.
Dialogue, superseded versions, audit and backups remain separate from forgetting.

Added deterministic transport/API/CLI, grant/race/scope/expiry/migration/rollback
and actual MemoryId retrieval tests. The independent-memory Actor fixture removes
the original source from recent dialogue using temporary Kernel-only filler turns,
reopens storage and proves the selected record reached model context before
correction/forgetting. Design contracts, ADR 010, roadmap, operating paths and
staged manual acceptance are updated; see `docs/Q2_A_ACCEPTANCE.md` for final
commands/results and live gates.

All development uses isolated `target/q2a` outputs and temporary data. MSVC debug
symbols exhausted the available F: space; only that task-owned build cache was
cleaned, then dev/test symbols and incremental output were disabled for the local
validation commands. No running QQ executable, service, personal database, private
credential or real memory grant was changed. No paid call or real QQ send was made.

Final native Windows validation on 2026-10-25 passed format, strict all-target
workspace Clippy and workspace build. The final sequential workspace test run
passed **126 tests, zero failed, three manual tests ignored** across 15 suites.
The parallel Kernel executable passed
45 tests with two manual probes ignored. The native CLI smoke passed exit 0 with
clean temporary-state removal, including ACLs, process restart, fake-model/Worker
flows and existing memory/candidate/summary retrieval; no paid quota was used.
Windows PowerShell parsed all nine documented operating blocks without executing
them. A warning-capture fixture was isolated by unique ChannelId; a Cargo/native
smoke overlap was corrected after Windows locked the active test exe. Real
QQ/provider memory, ConPTY and native macOS/Linux remain unverified.

## 2026-10-25 — Q2-A process-level acceptance closeout

Reviewed current Q1/Q2-A transport/service/Kernel/Actor/persistence/API/CLI paths
against ADR 010 and the acceptance requirements. No additional product defect was
reproduced within that scope; this task makes no Rust product behavior, schema,
milestone or deployment change. Existing uncommitted work and historical failed
real-provider evidence remain intact; HEAD is still `560ddf9`.

Added the focused `scripts/qq_memory_smoke.py` entry point rather than extending
the broad older CLI/Worker smoke with a new optional dependency. It uses the
already-installed Python 3.13.5 / websockets 15.0.1, ephemeral loopback fake OneBot
and Responses servers, actual native laind/lain and the existing fake Codex process.
An owner-private temporary instance and generated credentials are independent of
all personal state. No package/framework, product dependency or CI was added.

The 12 normal checks cover identity-gated Online/reconnect, management auth,
default-off candidate review, sensitive-input rejection, source-linked MemoryId,
21 real fake-protocol exchanges evicting original dialogue, restart and exact
record retrieval in actual model HTTP input, correction/forget refresh and later
empty recall, old-source replay, in-flight revocation/expiry, unchanged Work review
and an actual pending fake Worker permission, and persisted receipts/tombstones/
semantic/FTS deletion/secret redaction. The provider uses actual source/record IDs
and revisions from request data. No memory is pre-seeded in SQLite.

All 12 passed, with 42 localhost fake model requests and zero paid calls. Separate
startup and provider-failure injections each returned expected exit 1 and cleaned
the daemon, fake services, background threads and temporary state. Operations and
cleanup are bounded; failure output reports a safe stage/assertion, not credentials
or provider bodies. The local ignored report is
`target/q2a/qq-memory-closeout.json`, including tested binary SHA-256 hashes.

The current native Windows rerun passed format, strict workspace/all-target Clippy,
workspace tests (**126 passed, zero failed, three manual tests ignored**, 15 suites,
46.98 seconds), workspace build and Python compilation. Cargo steps and process
acceptance ran sequentially. The previous broad native smoke remains baseline
evidence; it was not repeated without a relevant change. The operating guide now
has five manual upgrade/test stages, including consistent stopped-state backup,
schema-9 rollback requirements and explicit short-lived memory authorization.

This goal added the script and updated DEVLOG/Q2-A evidence only. No commit/push,
personal credentials/database access, real QQ/model call, grant enablement, service
restart or deployment occurred. Real memory quality, live reliability, ConPTY and
native macOS/Linux remain unverified.

## 2026-10-25 — Q2-B1: bounded private-history source lookup

Implemented the separately selected source-read slice on top of Q1 and Q2-A.
A default-off administrative SourceGrant fixes one private binding, a half-open
received-UTC window (at most 31 days), captured incoming high-water, CAS revision
and optional expiry. Reply/memory grants do not imply this permission. No future
collection, Worker approval, automatic memory restoration, experience summary,
proactive contact or deployment was added.

SourceRecord projects existing Incoming/Batch/ActorTurn/Delivery. Migration 010
adds decision/read ledgers and a receive-window index without copying raw chat
or changing old confirmation meaning. Search seeks the receipt-time/sequence
index directly, processes at most 512 candidate rows and returns at most 10 admin
or 3 Actor records / 8 KiB. Cursor scope, time bounds, ID route, high-water,
truncation, partial coverage, generation and actual delivery/Unknown stay explicit.
Every original input/reply is checked using current known credential filters
before excerpting, including opaque configured credentials crossing clip bounds.

The two typed readonly Actor capabilities use IDs-only receipts. All prior source
selections are freshly projected/fingerprinted before each further request and
after responses; revoke/narrow/expiry/suppression stops stale text/call dispatch.
Source-influenced streaming previews are held. Later model history excludes
assistant/action observations from source-reading turns while retaining original
user requests and operational snapshots. Q2-A suppressed inputs are excluded;
merged out-of-window inputs, changed/deleted inspected memory and source-derived
replies withhold their bodies. Audits say prepared, not comprehended/received.
Opaque provider continuation is retained only while every selection remains valid.
Existing request/tool/token/wall budgets and Work/Worker confirmations remain.

Added authenticated source grant/history/search/inspect/read-audit API and CLI,
ADR 011, the design/domain/runtime/CLI/QQ contracts and Q2_B1_ACCEPTANCE.md with
requirements-to-test mapping and five separately authorized manual upgrade/test
steps. Schema 010 rollback needs a consistent stopped pre-upgrade backup. Earlier
schema-9/Q2-A counts and real-provider failure records remain historical evidence.

Native Windows/MSVC verification passed format, strict all-target workspace
Clippy, offline locked workspace tests (138 passed, zero failed, three manual
ignored, 15 suites), and workspace build in target/q2a. New fixtures cover default
off/CAS/persistence, binding/time/ID/cursors, high-water with clock rollback,
index seek without temporary sort, count/byte/UTF-8 limits, suppression,
credential filtering, truthful generation/confirmed/Unknown sends, merged input
scope, stale inspected memory, atomic audit failure, no migration backfill,
actual IDs after eviction/restart, in-flight revoke/narrow/expiry/disable/forget,
preview privacy, source-derived history isolation and unchanged Work review.

Extended the existing qq_memory_smoke.py rather than adding a test framework.
All 18 actual process/HTTP/WS checks passed with 53 local fake model requests,
zero paid calls and complete cleanup. Source IDs come from real query results,
with original test inputs delivered over fake OneBot, not inserted into SQLite.
The normal provider timeout test requires Unknown, two attempted requests, no
further call or cached quote and successful restart. Startup/model/source failure
injections each returned expected exit 1 and cleaned services/temporary state.
The first timeout fixture wrongly expected Failed and did not handle Windows
connection abort; fixed the fixture, preserving the failed log without changing
Actor lifecycle semantics to satisfy it.

The initial broad native CLI regression was externally interrupted at six minutes
while still making progress through native configuration/ACL checks. Its owned
launcher/daemon were gracefully stopped via their console group and only that
verified temporary fixture was removed. Its safe daemon log is retained. The
full rerun passed in 496.34 seconds with native setup/reuse/ACLs, launcher/doctor/
shutdown/restart, streamed Actor, reviewed fake Worker delegation, independent
scoped memory/summary retrieval and restart persistence. Native terminal/ConPTY
and real-provider acceptance remain unverified.

Ignored evidence is under target/q2a/q2b1-*: gates/logs, successful process JSON
with binary SHA-256, failure results, and the earlier timeout assertion/wrapper
logs. Existing 45 uncommitted paths were preserved; the combined worktree now has
56 paths (40 modified, 16 untracked), including eight Q2-B1 files and three extra
tracked files touched for source preview/SSE snapshot sizing. HEAD remains
560ddf9. No commit/push, paid model, real QQ, personal credentials/database access,
F:\Lain-Q1-Acceptance migration/restart or real grant enablement occurred. Real
provider/QQ experience, ConPTY and native macOS/Linux for this slice remain
unverified. This stage stops before Q2 experiences or Q3.

## 2026-10-25 — Select C1 natural private conversation as next priority

The user clarified that the desired outcome is MaiBot-like lifelike conversation
and explicitly requires learning from its chat design. Recorded C1 as the next
selected priority in PRODUCT, ROADMAP and CHAT_DESIGN: natural everyday expression,
message/turn timing, graceful closure and naturally used correctable memory.
Q1/Q2-A/Q2-B1 infrastructure remains implemented, not proof of naturalness.

Rechecked the current local persona, channel dispatch and merge bound, and the
upstream official Bot configuration document. The current private channel still
submits merged input to Actor; full participation/wait/no-reply and draft
reconsideration remain unimplemented. Online upstream documentation is recorded
separately from historical source notes; Git remote lookup failed with connection
reset, so no fixed-source review or code reuse is claimed. Fixed-revision source
mapping remains a C1 implementation prerequisite. No new runtime behavior, goal,
model/QQ call, deployment or commit was made; this is a product-priority/doc change.

## 2026-10-25 — C1: bounded natural private conversation

Completed the selected C1 implementation and deterministic native Windows slice,
without claiming real-model naturalness. Git transport had failed earlier; the
official GitHub API and fixed files succeeded at MaiBot revision
062d45cf8bd4272ac57d7ec03fcf405e5a458211 (2026-10-23T17:46:39Z). Read the actual
debouncer, wait/stop, optional message steering, private/group gate, context and
personality/behavior/reply-style assembly. MAIBOT_REFERENCE.md records immutable
links and Lain's choices. No upstream source or prompt text was copied into product;
25 downloaded reference files, hashes and LICENSE remain in ignored target data.

Identity v3, always-appended Actor policy, private behavior and private expression
are assembled separately. Ordinary private recall need not display UUIDs in every
reply; exact IDs/revisions/provenance remain inspectable. Configured diagnostics
add private_chat_revision for this bundle. Stable personality, truthful experiences,
typed tool policy and explicit Work/Worker review remain enforced. No extra Planner,
Replyer, background extraction, random personality or paid evaluation was introduced.

The private channel now uses a trailing 750ms quiet window, capped at three seconds,
with the original 16-input/8KiB/128-queued bounds and ten-minute expiry. chat.wait is
once per turn for 1–5 seconds and wakes on new input/deadline; chat.finish records
an inferred social closure with no delivery. A conservative guard refuses silence
for protected requests; rejection removes finish for the remaining chain. Semantic
classification and naturalness still require real feedback, not fixture text.

In-flight supplements retain original IDs/text/times and associate atomically with
the current batch/Actor; root request and Batch.text remain unchanged. A returned
stale response is accounted but its text/calls are discarded. At most two
reconsiderations use the existing four-request/four-tool/reported-token/120-second
budgets without resetting counters. Over-cap input stays queued for a fresh turn,
with an honest capacity notice replacing the obsolete reply. Opaque continuation
is rebuilt across new input, with IDs-only receipts of already committed memory
effects. Private draft previews are held. No Worker is cancelled by reconsideration.

Final conversation text/new actions and private memory effects recheck current
input. Latest selected memory IDs/revisions and bounded source selections/hashes
are revalidated at commit/send boundaries. The final review found and fixed the
Ready historical-quote gap: revoke/expiry/suppression after generation now makes
that unsent quote ineligible, with prepared audit/fingerprints committed together.
The service also reapplies current credential filters immediately before writing
stored drafts. Ready ordinary drafts become Superseded; prewrite refusal is known
NotSent. Confirmed/Unknown outcomes and approval/failure notices retain their truth.
Later model history uses confirmed speech and withholds stale/uncertain drafts or
changed memory/source-derived replies. Migration 011 adds the batch-input index;
legacy snapshots are preserved without inventing old chat decisions or memory.

Native Windows gates passed format, strict all-target workspace Clippy, locked/
offline workspace tests (157 passed, zero failed, three manual ignored, 15 suites),
and workspace build. Nineteen added C1 tests cover quiet/cap/rollback, atomic input
adoption, bounded interruption/cumulative usage, protected requests, wait/deadline/
cancel/restart, proposal isolation, delivery truth, memory/source changes, private
preview/history, schema-10 compatibility and stored-credential prewrite filtering.
The original identity/dedup/receipt-loss fixtures remain; a separate post-confirmation
input now exercises the uncertain second send after an in-flight supplement is
merged into the first turn. Early old fixed-window/two-reply expectations and an
invalid approval fixture were corrected, not recorded as passes. One transient
Windows mapped-source formatting error was resolved by retrying without killing
user processes.

Extended the existing qq_memory_smoke.py: 22 actual native process/HTTP/WS checks
passed with 61 local fake model requests, zero paid calls and complete cleanup.
All original Q2-A/Q2-B1 scenes remain. Added C1 merge/original IDs, silent closure/
question protection, wait/new-input wake and in-flight negation/stale proposal
checks. Startup/model/source injections each returned expected exit 1 and cleaned
owned state/processes. Current binary SHA-256 values match the successful report.
Final broad native CLI regression passed in 237.12 seconds, including owner ACLs,
launcher/shutdown/restart, doctor, streamed Actor, reviewed fake Worker submission,
independent memory/summary retrieval and persistence. No concurrent Cargo build
ran against those process fixtures.

Updated ADR 012, product/chat/architecture/domain/runtime/memory/Actor/CLI/QQ docs,
README and ROADMAP. C1_ACCEPTANCE.md contains requirements-to-evidence mapping,
the repeatable offline command, twelve multi-turn experience scenes/rating criteria
and five separately authorized manual upgrade/test steps with schema-11 rollback.
Evidence is ignored under target/q2a/c1-*: gates, process/failure/native logs and
reports, reference hashes and preserved baseline fingerprints. Prior failed
real-provider records and historical Q2-B1 counts retain their original meaning.

Preserved all 57 pre-existing dirty paths; the combined tree has 69 paths (42
modified, 27 untracked), including the new C1 files and existing Q1/Q2 work. HEAD
remains 560ddf9. No commit/push/deploy, personal credentials/database read, real
QQ/model call, real grant enablement or F:\Lain-Q1-Acceptance upgrade/restart occurred.
The real instance still needs a separately authorized upgrade and experience review.
Naturalness, intent misclassification, real latency/cost, ConPTY and native
macOS/Linux remain unverified. Stop this bounded C1 here; no Q2 experiences/Q3 work.


## 2026-10-25 — C1.1: upgrade rehearsal, live acceptance pending

Prepared a concrete per-instance plan and a hash-checked launcher candidate under
ignored target/c1-1. Read only file/process metadata, startup AST and selected
non-secret settings. The existing entry selects target/debug; accepted C1 binaries
are in target/q2a/debug. No personal credentials, SQLite content or authenticated
personal API was accessed. No personal service was stopped or upgraded.

Extended the existing QQ process smoke script with an isolated --upgrade-from
mode. Native Windows passed five checks: old schema-8 state creation, stopped
whole-state backup with owner DACLs, migration to schema 11 preserving identity/
memory/receipts, old-binary refusal of the future schema, and complete rollback
with no input/delivery replay. Three local fake requests, zero paid calls and
complete cleanup. Earlier ACL failures remain in evidence; a DACL-only .NET
setter avoided the unavailable SeSecurityPrivilege while retaining owner checks.

Normal regression again passed all 22 process checks with 61 local fake requests,
zero paid calls and cleanup true. Python AST validation and positive/tampered
launcher inspection passed. Evidence is target/c1-1/upgrade-rehearsal.log and
normal-regression.log. No Rust/persona behavior changed; the earlier 157-test C1
result was not rerun or relabeled as C1.1 live evidence. Native macOS/Linux and
real naturalness, latency, cost and personal migration remain unverified.

Updated C1_1_ACCEPTANCE.md, QQ_CHANNEL.md schema documentation and ROADMAP.
Pending: consolidated authorization for stopped full backup/upgrade/rollback,
existing SecretRefs, a bounded model-call budget, the existing private binding,
and optional short-lived test-memory permission. Then user-led scene feedback
can support targeted fixes. No commit/push/deployment or Q2/Q3 expansion.


## 2026-10-25 — C1.1: authorized upgrade and first real expression feedback

The user explicitly authorized the prepared personal-instance operation, at most
60 real model requests, the existing private QQ binding, and optional one-hour
test memory permission with source access off. The user gracefully stopped the
old launcher; no Work sessions or unfinished channel batches were present.

Backed up all eleven stopped instance files with byte and DACL verification,
archived exact old binaries, staged only the accepted real CLI/daemon under a
versioned release, and retained the original entry/settings. Owner-plus-DACL
initialization of the backup parent failed before credential copy; a DACL-only
setter verified the existing owner and succeeded without elevation. Extended-path
spelling also required normalization in the path preflight. Personal migration
8 to 11 preserved all 38 old incoming/batch/delivery records in their old columns.
The new foreground entry is task-controlled; doctor paths/configuration agree,
QQ is identity-gated Online and memory/source grants remain off. Personal rollback
is prepared but not exercised.

Six actual user-sent greeting/joke/venting inputs used six requests and 21,516
reported tokens, with all turns completed and deliveries confirmed. User feedback:
content was generally acceptable but every short reply felt written, repeatedly
two sentences separated by a blank line with deliberate punctuation. Preserved
before-change evidence and feedback in the protected backup outside Git. Added
only short-message expression guidance; no output postprocessing, fixed responses
or permission/runtime behavior changed. Retest remains pending. Formatting and
strict workspace all-target Clippy passed; relevant Actor tests/build are underway.
The first-batch ledger leaves 54 requests. Read-only capture uses persisted Actor
counters, not the diagnostics endpoint literal model_calls zero. Worker version
compatibility is unavailable and outside this chat task. No auto commit/push.


## 2026-10-26 — C1.1: autonomous real-model repair and restored QQ service

After the user requested debugging without further manual message sending, paused
the owned personal foreground service and reused the existing native OneBot/process
fixture for isolated real-provider replay. Synthetic inputs were not inserted into
the personal conversation and no automated QQ probe was sent. Combined actual
request usage is 57/60: 28 user-led real QQ requests and 29 automatic real-model/
local-transport requests, reporting 255,754 aggregate tokens; monetary cost and
provider-resolved model revision remain unavailable. Automatic inference has stopped.

The first expression revision failed its actual QQ retest. Explicit user format
instructions worked, then firmer short-message defaults were tested independently.
The final cold-start sample, without personal history or style instruction, replied
in one short line without a final period. Some other replies still use periods or
formal explanation; broad naturalness is not declared solved.

Actual QQ remember/correct/forget effects succeeded, but the model falsely said
the preference had never been saved. Instruction clarification plus historical
IDs-only action projection alone also failed real replay. Added the binding latest
three committed MemoryEffect receipts to fresh, authorized memory context as Kernel
operation metadata. They contain no recalled text and grant no capabilities.
Historical private action observations omit old arguments/result bodies and retain
only correlated successful effect metadata/state. Failed/Unknown operations cannot
become committed truth; canonical audit stays unchanged. The identical real-model
post-forget question and a causal follow-up then correctly acknowledged save,
correction and deletion and corrected the earlier denial. Inferred provenance and
ordinary candidate review remain enforced; no memory/approval check was bypassed.

Real-model/local-transport timing: three inputs 101ms apart became one batch and
one request; successive in-flight corrections reached two reconsiderations in
three requests, then retained the excess final input for a fresh one-request turn.
The superseded turn emitted an honest capacity notice; obsolete drafts were not
sent. Closure followed by an explicit question answered normally. Existing actual
QQ screenshots and all failed replies remain separately preserved.

Validation passed formatting, strict all-target workspace Clippy, 77 Kernel unit
tests (two manual ignored), relevant Actor integration tests, workspace build and
22 cross-process checks with 61 fake requests/zero paid calls/cleanup true. New
context test covers operation truth without forgotten arguments; existing lifecycle
test now verifies three same-binding, content-free committed receipts after deletion.
Disk-full and later PDB LIMIT failures were preserved; only verified regenerable
q2a incremental/debug-symbol caches were cleared with compilers stopped, then gates
rerun successfully. A SQLite-handle temporary cleanup failure was repaired and
verified; a zero-call old-event-ID replay received no acceptance credit, and new
input/Actor checks precede the valid retest.

Final personal release c1-1-0c5048d155ee started at 00:00 Asia/Shanghai, using the
unchanged Actor config/identity/binding and schema 11. QQ Online, reply permission
retained, temporary memory grant revoked at revision 2, source grant never enabled.
The full stopped schema-8 backup/old binaries remain protected; personal rollback
was not needed. The new start-c1-qq-test.ps1 selects the verified release; original
entry and prior releases remain retained. No original dirty work was reset, and no
commit/push/cloud deployment/Q2/Q3 expansion occurred. C1_1_ACCEPTANCE.md has actual scene
outcomes, artifact paths, startup/recovery instructions and remaining quality limits.


## 2026-10-26 — D1a: bounded text acquisition in the combined companionship/work goal

The user selected both large stages together. Read the full goal and current
architecture/evidence; baseline main/origin main was clean at 3bfb855. Added an
explicit D1a–D3/W1–W4 delivery plan without declaring either complete. Corrected
PRODUCT current-state statements to reflect C1.1 real QQ/model replay and the
installed schema-11 release with temporary memory revoked and source access off.
Historical failed/live records retain their original meaning.

Implemented a separate default-off CAS ReadingGrant selecting exact HTTPS URLs,
expiry, retention and attempt/body limits. Migration 012 appends grant/activity/
result tables; no old authority is changed. One unfinished acquisition, durable
attempt charging, immutable request dedup, actual timestamps and safe closed
failures survive restart. Restart marks unfinished work Unknown without fetching;
cancel/revoke/expiry prevent late content commits. Completed content/state commit
atomically. Grant replacement removes old content without inheriting its authority.

The asynchronous reader pins validated public DNS addresses, disables proxies,
redirects and ambient auth, accepts only bounded complete UTF-8 plain text, checks
known credentials and has finite connect/overall deadlines. Stored text remains
untrusted administrator-inspectable data. AcquiredTextOnly never claims reading,
comprehension, summarization or verification. A next-retention-deadline timer is
rebuilt from storage and deletes bodies without model polling. Operation metadata,
WAL/backups and semantic erasure remain distinct. Added authenticated API and
Rust reading CLI; no Actor tool, model job, experience or contact is yet added.

Native Windows workspace run: 167 passed, zero failed, three manual ignored;
fmt, strict all-target Clippy and workspace build passed. Added one further atomic
rollback test afterward; focused reading suite passed all ten, including timer
expiry inspected directly through SQLite. Actual native QQ/Worker regression
passed 23 cross-process checks with 61 fake requests, zero paid and full cleanup.
Independent full backup/migration/rollback rehearsal passed five checks from
schema 8 to 12, three fake requests, no paid calls/personal data and full cleanup.
Schema 11 preservation is separately covered at the storage unit level.

Preserved failures: initial SQLite unsigned conversion/test-constructor compile
errors fixed; a later workspace compile exhausted disk and was rerun successfully
after cleaning only verified generated target artifacts with compilers stopped.
Process-scoped no-incremental/no-debug-symbol build settings limited disk use.
No global tool config, personal runtime, credential, database or grant was changed.
Evidence and binary hashes are under ignored target/d1a and READING_ACCEPTANCE.md.
No new real-model/public-source/QQ/device acceptance is claimed; old real-call
budget remains closed. D1b experiences/Actor processing, D2 contact, D3 full daily
chain and W1–W4 remain outstanding. This is a verified implementation milestone
for commit/push, not completion of the active combined goal.


## 2026-10-26 — D1b: source-linked reading experiences and private recall

Continued the combined goal from clean f4ec0d4. Added optional, default-off model
processing/disclosure scope to exact-URL reading grants: one private binding,
a cumulative prepared-request cap and an explicit fixed UTC offset. Schema 013
adds transactional reading jobs, model reservations, experiences, content-free
read/edit/source-decision audits and withdrawal/suppression records. Existing
schema-12 grants decode processing-off without rewriting their raw bytes/bodies.

A successful acquisition event or explicit process action reuses the existing
Actor in a Kernel-originated reading mode. It has no tools and cannot acquire
private contexts, memory writes or Worker actions. Source text enters only fresh
bounded untrusted inference context, not the immutable activity request. Actual
input/original sizes and truncation are recorded. Source/cancel/expiry/fingerprint
checks protect response and commit; returned usage is retained before discarding
a stale or malicious response. Completed Actor and source-linked summary publish
in one transaction. Idempotent/Unknown jobs never regenerate automatically, and
budget reservations survive failure, cancellation and restart.

Added typed private experience search/inspect with fresh evidence rather than
cached tool bodies. Revision/fingerprint and grant checks persist through model
response, action/final commit and send admission/prewrite. Correction is explicitly
owner-originated, not retroactively model interpretation or verified fact. Withdraw
suppresses the exact URL, removes its source bodies and invalidates derivatives;
explicit restore permits only new acquisitions and does not restore old jobs,
content or spent requests. Experience-influenced turns cannot silently promote
unlinked semantic memory; original ordinary memory and Worker review remain.

Closed the analogous C1.1 empty-lookup trap with at most three content-free committed
summary-operation receipts. No URL, forgotten original, old summary or correction
enters them. Operation-only reads also retain an admitted grant revision; revocation
before another request/response/commit/send cannot bypass policy just because the
selected body list is empty. A withdrawn/expired summary remains an actual past
operation, while active retrieval correctly returns no content. Real-model quality
is not inferred from the deterministic model outputs.

Added Rust CLI/API inspection, processing, date/topic search, correction/withdraw,
source restore, budget and audit. Updated architecture/domain/runtime/memory/chat/
Actor/CLI/QQ/product docs, ADR 014, combined plan and EXPERIENCE_ACCEPTANCE.md.
D1b is an implementation milestone; D2 sharing, D3 live daily-chain acceptance
and W1–W4 sources/devices/work/terminal remain outstanding.

Native Windows/MSVC Rust/Cargo 1.98.1 final gates: fmt, strict workspace/all-target
Clippy, build and full workspace tests passed (180 passed, zero failed, three
manual ignored). Reading tests total 22, including source extent, malicious tool
refusal, durable budget/restart, atomic terminal/experience failure, private scope,
correction/withdraw/expiry, stale drafts, operation-only truth/revocation, midnight
and schema-12 body/grant-byte preservation. The new source/model process mode uses
explicitly seeded synthetic acquisition, actual native HTTP Responses/OneBot and
fresh Actor identities; it is not claimed as a production public fetch.

Final process regression passed 24 checks with 68 fake requests and cleanup true.
Schema 11 to 13 native binary-only full backup/migration/refusal/rollback passed
five checks with three fake requests and cleanup true. Copied only the installed
C1.1 executable pair into synthetic state; no personal DB, settings or credential
was read, migrated or restarted. Final binary hashes match both reports under
ignored target/d1b. All calls were fake/local, zero paid; the old real-call campaign
remains closed. No real source, QQ contact, device connection or deployment.

Preserved failures and intermediate evidence: a test sliced Chinese text at an
invalid UTF-8 byte and was fixed; Clippy required lexical lock scopes; the first
schema-11 rehearsal exposed an old fixture assumption that historical turns had
no C1 decision. It now compares exact original/migrated fields and checks no reading
origin/grant/request was invented. Full rebuild/test and final process reruns cover
the subsequent operation-truth extension; earlier 179-test/66-request reports
are retained without promoting them to final-extension evidence. Real public
TLS/source/provider quality, multi-day continuity and native macOS/Linux remain
unverified. Commit/push this verified milestone; the complete goal stays active.
