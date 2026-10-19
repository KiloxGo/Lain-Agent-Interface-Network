# Roadmap

**Status:** working plan  
**Last reviewed:** 2026-10-19

This roadmap defines implementation order for the Lain architecture reboot.

It is deliberately staged to prevent the project from rebuilding a generic agent framework before the control plane is proven.

## Phase 0 — Freeze the foundation

**Goal:** establish shared language before rewriting code.

Deliverables:

- [x] AGENTS.md
- [x] README.md
- [x] docs/PRODUCT.md
- [x] docs/ARCHITECTURE.md
- [x] docs/DOMAIN_MODEL.md
- [x] docs/RUNTIME_MODEL.md
- [x] docs/AGENT_BACKENDS.md
- [x] docs/MEMORY.md
- [x] docs/CONFIGURATION.md
- [x] archive the old prototype on `legacy/v0` and restart from a clean implementation tree
- [x] add initial ADRs for major architecture decisions
- [x] freeze initial Rust workspace/crate layout

Exit criteria:

- a fresh coding-agent session can explain Lain's product boundary without reading old source;
- domain terms are consistent;
- no implementation work requires inventing a second architecture.

## Phase 1 — Kernel skeleton

**Goal:** create a durable headless daemon with no dependence on the old chat loop.

Deliverables:

- [x] Rust workspace for the new Kernel
- [x] configuration loading
- [x] tracing/logging
- [x] SQLite persistence bootstrap/migrations
- [x] strongly typed domain IDs
- [x] basic WorkSession repository
- [x] in-process Event Bus
- [x] health/readiness endpoints
- [x] authenticated local API skeleton
- [x] graceful shutdown

Implemented as one `lain-kernel` library + `laind` binary, with Axum and
rusqlite. Phase 1 established read-only APIs and internal metadata operations;
Phase 2 extends these with real worker control. See `docs/DEVELOPMENT.md` for
current operation and checks.

Non-goals:

- Lain persona
- memory intelligence
- multiple worker backends
- elaborate Web UI

Exit criteria:

- laind can run continuously;
- restart preserves basic domain state;
- API/state boundaries follow the foundation docs.

## Phase 2 — Codex worker control

**Goal:** prove the control-plane architecture with one real backend.

Preferred initial integration:

- Codex App Server directly, unless current upstream testing shows ACP is the more reliable path.

Deliverables:

- [x] Codex executable/version probe
- [x] runtime process supervisor
- [x] Codex driver
- [x] create WorkSession
- [x] send prompt / create Turn
- [x] structured event normalization
- [x] cancellation
- [x] approval routing
- [x] provider session ID persistence
- [x] resume/recovery where supported
- [x] multiple simultaneous sessions
- [x] session inspection API
- [x] backend capability reporting
- [x] authenticated control API and snapshot-first SSE
- [x] deterministic fake-process contracts and opt-in real acceptance script

Verified on Windows with Codex 0.160.0 on 2026-10-04: two real sessions,
follow-up turns, scoped cancellation, explicit approval, frontend detach,
owned-process crash/resume and daemon restart with unchanged provider identities.
Uses direct App Server, one owned process per session and migration 002. See
`docs/backends/CODEX.md` for contract/limitations and `DEVLOG.md` for evidence.
Native Linux lifecycle validation remains outstanding. Phase 3 extends this
foundation with temporary runtimes and durable operational history.

Exit criteria:

- two Codex sessions can work concurrently in different workspaces;
- frontend disconnect does not stop either;
- Kernel can restart and represent/recover session state correctly;
- no terminal UI scraping is required.

## Phase 3 — Durable work, operational history, CLI and TUI

**Goal:** establish process-independent work and reconnectable history, then make
terminal users first-class before building a graphical frontend.

Ordered deliverables:

1. [x] WorkSession / temporary WorkerRuntime separation, public attachment state,
   event-driven idle detach, lazy identity-preserving resume, selective boot
   reconciliation and truthful graceful shutdown.
2. [x] Indexed migration-003 operational history, bounded sanitized meaningful
   observations, state/history atomic commits, stable EventId/order and paginated API.
3. [x] Separate API-only `lain` client, non-interactive JSON commands, session list
   and detail TUI, approvals, input editing, scrolling, resize, history/live dedup,
   reconnect/resync and terminal restoration.

Implemented CLI:

~~~text
lain status
lain work list
lain work start --workspace <server-path>
lain work show <id>
lain work attach <id>
lain work send <id>
lain work cancel <id>
lain work resume <id>
lain work history <id>
lain approval list/approve/deny/cancel <session> [approval]
lain --json work list
~~~

TUI:

- active session list;
- session timeline;
- approval view;
- follow-up input when Idle/Ready and explicit cancel;
- profile/backend information.

The TUI talks only to the public Kernel API.
Profile/backend labels are visible; Phase 5 adds registry inspection and selection.
Ratatui 0.30.2/Crossterm 0.29 implement the terminal. Windows ConPTY and real
Codex 0.160.0 acceptance cover detached follow-up, history/reboot continuity
and unchanged provider identity. See DEVLOG and the development guide for
validation and known Linux/native-host limits. Phase 4 adds a browser client.

Exit criteria:

- normal worker supervision is practical without a browser;
- no direct backend-specific logic exists in the TUI.

## Phase 4 — Web control plane

**Goal:** remote cross-device access.

Deliverables:

- [x] authenticated Web client (explicit page-memory token; refresh re-entry)
- [x] live WorkSession list with separate logical/runtime state
- [x] session detail and normalized durable timeline
- [x] offered approval decisions only
- [x] snapshot/history/SSE reconnect and resync
- [x] create/follow-up/cancel/reconcile through existing APIs
- [x] responsive mobile list/detail layout
- [x] optional same-origin built static assets and documented remote boundaries
- [x] deterministic frontend tests and opt-in fake-worker browser acceptance

React + TypeScript + Vite implement a disposable browser projection. Session
creation uses the existing supported model/sandbox options; profile selection
belongs to Phase 5, not this milestone. The daemon remains loopback-only and API
authentication remains required. Page-memory bearer authentication is an interim
flow, not public Internet readiness. See [WEB.md](docs/WEB.md) and ADR 002.
Phase 5 adds profile selection while preserving these observation/auth boundaries.

The initial visual identity uses restrained dark neutrals, readable timeline
details and explicit status labels. More expressive identity remains future work.

Exit criteria:

- open browser on another device;
- inspect server-side work;
- close browser;
- return later;
- observe continued work and current state.

## Phase 5 — Runtime profiles and provider switching

**Goal:** provide CC-Switch-like configurability at a higher abstraction level.

Deliverables:

- [x] durable owner-managed RuntimeProfile registry and revisions
- [x] namespaced environment-backed SecretRef integration and runtime redaction
- [x] at least two concurrent isolated Codex profiles
- [x] per-session Web/CLI profile selection
- [x] process-scoped Responses endpoint/provider/authentication override
- [x] frozen session configuration and redacted resolved-profile inspection
- [x] workspace default/allowed profile policy, checked on subsequent attachment
- [x] profile validation/health without paid inference
- [x] migration-004 local compatibility, safe failure categories and retry evidence

SQLite definitions remain mutable; WorkSessions retain non-secret resolved
snapshots and credential handles. Secret values resolve for each new runtime.
The exact Codex 0.160.0 gate remains. Native configuration checks use no inference;
different real provider accounts have not been tested. See ADR 003 and CODEX.md.
Phase 6 implementation follows; native/real-provider acceptance remains explicit.

Exit criteria:

- two Codex sessions can run simultaneously with different profiles without mutating global Codex configuration.

## Phase 6 — CLI-first Lain Actor

**Priority change:** Web development is paused. Keep `web/` and its existing API
contracts, while prioritizing the Rust Kernel, terminal workflows and Lain herself.
Windows PowerShell and macOS zsh are equal native targets; Linux stays compatible.

- [x] private per-user `setup --local`, token reuse and environment overrides
- [x] foreground native `up`, identity/auth/port checks and graceful shutdown
- [x] safe quota-free `doctor`, PATH/executable/profile/Actor diagnostics
- [x] CLI chat/ask, conversation inspection, explicit action decisions and watch
- [x] Kernel-owned bounded Actor, separate Responses model config and SecretRef
- [x] versioned persona with owner startup override
- [x] migration-005 durable Lain conversations and request deduplication
- [x] bounded context, strict typed reads/work tools using existing services
- [x] persisted owner confirmation, partial delegation receipts and no replay
- [x] committed SSE snapshots, observer disconnect independence and recovery
- [x] deterministic model/fake-worker/permissions/lifecycle tests without quota
- [x] native macOS build/test/CLI smoke, including PTY chat/confirmation/Ctrl-C
- [ ] native Windows build/test/CLI smoke (unavailable on implementation host)
- [ ] native Linux server lifecycle acceptance (unavailable)
- [x] authorized real macOS Actor/Codex evidence: tools, denial/approval, terminal worker and restart (2026-10-14; provider omissions/isolated history probe documented)

Implementation is available; **Phase 6 cross-platform acceptance is incomplete**.
Use `scripts/check-native.ps1` and `scripts/check-native.zsh` on their actual hosts.
No emulated platform results or fixture provider results count as real acceptance.
No milestone completion claim until the required native evidence exists.
See [CLI.md](docs/CLI.md), [LAIN_ACTOR.md](docs/LAIN_ACTOR.md) and ADR 004.

Phase 6.5 stabilization is in progress:

- [x] bounded sanitized Actor previews, typed wire events and final-message reconciliation
- [x] measured optional CLI/provider timing diagnostics without JSON stdout pollution
- [x] native macOS PTY reproduction of CRLF `You> You>` and serialized-input regression fix
- [x] deterministic gated streaming, failure, denial, detach and reconnect checks
- [x] saved local Actor settings/env-file workflow and observable startup mismatch (native macOS verified)
- [x] focused Actor message/turn status audit with versioned legacy-compatible storage
- [x] cancelled-caller ownership, atomic delegation receipt and bounded-history cursor regressions
- [x] expanded opt-in runner and deterministic fixture coverage for reads/denial/delegation/worker terminal/restart
- [x] authorized real provider reads/denial/confirmed delegation/worker completion/restart evidence on native macOS (2026-10-14)
- [ ] native Windows input/configuration/build/test checks
- [x] requirement evidence record and conditional Phase 7 handoff, without implementing memory
- [ ] final real-provider/platform acceptance audit

Owner chat walkthrough follow-ups (recorded 2026-10-14; implemented below in
Phase 6.6 on 2026-10-18, without upgrading prior acceptance evidence):

- [x] distinguish ordinary absolute-path input from REPL slash commands (single slash-word ambiguity documented)
- [x] shorten human tool/status receipts while preserving exact mutation review and machine JSON contracts
- [x] return delegated terminal results to chat without requiring another owner message; deterministic delivery and reconnect deduplication implemented, optional model interpretation deferred

**Phase 6.6 — The Thread Continues: Durable Delegation & Result Delivery**

Owner-defined scope recorded on 2026-10-18; implementation and native macOS
deterministic acceptance passed. Goal: one owner request, one explicit delegation
confirmation, then automatic delivery of the actual worker outcome in the same
conversation, without another owner message.

- [x] persist correlation from conversation/ActorTurn/confirmed action to WorkSession/worker Turn, exact submitted instruction and receipt
- [x] derive bounded, sanitized factual result notices from authoritative normalized worker evidence, without additional model inference
- [x] provide durable conversation observation with stable notice IDs/cursors, reconnect deduplication and crash recovery
- [x] keep completed ActorTurns unchanged and workers independent of connected clients; distinguish submission, execution and delivery
- [x] fix ordinary absolute-path input, compact human receipts while preserving exact approval arguments and machine JSON, and display notices without disrupting typed input
- [x] cover isolation, concurrent tasks, later turns in the same session, failure/unknown/cancellation, duplicate events/confirmations, slow consumers and legacy persistence with deterministic fixtures
- [x] run native macOS locked build/fmt/Clippy/tests and CLI/PTY checks (83 passed, 3 ignored; zero quota)
- [ ] native Windows/ConPTY checks (host unavailable; unverified)
- [ ] native Linux server checks (host unavailable; unverified)
- [x] prepare opt-in real acceptance for automatic delivery/reconnect; new paid work requires explicit owner authorization of the workspace/action
- [ ] owner-authorized Phase 6.6 real-provider validation (not run; no milestone-style commit)

Result interpretation by a model is optional and deferred unless separately
authorized and safely bounded; factual delivery must work without it. Reuse the
existing Kernel/Supervisor, durable history and confirmation boundaries. Never
replay uncertain mutations. Phase 7 Memory was deferred during Phase 6.6 and is separately authorized below.
Phase 8 Scheduler and Phase 10 Delegation Compiler remain deferred; no prompt
rewriting or Web redesign belongs to Phase 6.6. See the [acceptance matrix](docs/PHASE_6_6_ACCEPTANCE.md),
[ADR 007](docs/adr/007-durable-delegation-result-delivery.md) and DEVLOG.

Limits: no mid-generation provider resumption, automatic mutation retries,
profile mutations or worker auto-approval. Sanitized previews are ephemeral;
validated completed records remain authoritative. See ADR 005.
Conversation persistence is not semantic memory. The owner separately authorized
deterministic Phase 7.1 on 2026-10-19; prior acceptance gaps remain open.

## Phase 7 — Lain memory and private world

**Goal:** give Lain durable continuity independent of worker sessions.

Memory provenance and scoped, owner-approved project knowledge/preferences may later provide **evidence** for optional intent-aware delegation. Phase 7 does **not** implement prompt rewriting or silently promote inferred preferences to task requirements. See [Intent-Aware Delegation](docs/INTENT_AWARE_DELEGATION.md).

### Phase 7.1 — The Shape of Memory

Owner-authorized implementation from fetched `origin/main` `17a676d`, dated
2026-10-19. Independent continuity and worker supervision are equally important.
Implementation is distinct from pending real-provider/native platform acceptance.

- [x] typed memory records, scope, provenance, authority and lifecycle
- [x] forward-only migration 007 in existing SQLite, transactional FTS5 trigram search
- [x] direct owner remember/inspect/disable/enable/supersede/delete without a model
- [x] scoped project knowledge separated from global preferences and internal reflection
- [x] Actor search/list/inspect and proposals using existing persisted confirmation
- [x] minimal Candidate/Accepted/Rejected review, durable retry identity and stale-write conflicts
- [x] explicit factual SessionSummary from normalized session/Turn history, without automatic memory promotion
- [x] API-only CLI commands and stable typed machine JSON; Web stays frozen
- [x] deterministic scope/Unicode/privacy/injection/restart/approval/evidence/migration regressions
- [x] native macOS locked build/fmt/Clippy/tests (96 passed, 3 opt-in ignored) and CLI/PTY fixtures
- [ ] separately authorized real-provider Actor retrieval of previously persisted memory (not run)
- [ ] native Windows/ConPTY acceptance (host unavailable; unverified)
- [ ] native Linux server acceptance (host unavailable; unverified)

Real-provider Phase 6.6 result delivery and earlier native Windows/Linux obligations
remain pending; Phase 7.1 fixture results do not upgrade them. No new paid inference,
actual Codex work or installed owner daemon/database changes were authorized here.
See [MEMORY.md](docs/MEMORY.md), [acceptance](docs/PHASE_7_1_ACCEPTANCE.md) and
[ADR 008](docs/adr/008-independent-owner-reviewed-memory.md).

### Phase 7.2 — private World and continuity follow-up (unimplemented)

Suggested next scope, requiring separate owner authorization:

- [ ] small owner-inspectable World directory/service with bounded notes/artifacts
- [ ] explicit memory/summary/evidence references in project notes
- [ ] richer candidate review/retention and uncertainty presentation where justified

Journals, automatic summary extraction, personality evolution, daily reflections,
consolidation schedules, embeddings and agent-to-agent memory sharing are not
Phase 7.1 work. Do not automatically begin Phase 7.2 or Phase 8.

Exit criteria:

- a completed worker session can produce a summary;
- selected knowledge becomes inspectable Lain memory;
- raw transcript is not automatically copied into memory;
- memory survives backend replacement.

## Phase 8 — Scheduler and resident routines

**Goal:** make 24h residency useful without continuous inference.

Deliverables:

- [ ] persistent scheduler
- [ ] owner timezone
- [ ] wake triggers
- [ ] wake policy/cognitive budget
- [ ] daily reflection
- [ ] memory maintenance
- [ ] unresolved-work review
- [ ] observable wake history: why did Lain wake?

Exit criteria:

- Lain can perform bounded scheduled reflection/maintenance;
- the system can run for days without unnecessary model polling;
- background actions are attributable and inspectable.

## Phase 9 — Second and third worker backends

**Goal:** prove backend abstraction.

Suggested order:

1. Claude Code
2. DeepSeek Harness

Deliverables per backend:

- [ ] probe/version
- [ ] create/resume
- [ ] prompts
- [ ] structured events
- [ ] cancellation
- [ ] approvals where supported
- [ ] capabilities
- [ ] profiles
- [ ] contract tests
- [ ] backend-specific docs

Exit criteria:

- Web/TUI APIs require no backend-specific redesign;
- Lain can supervise mixed concurrent sessions;
- unsupported features degrade by capability rather than special-case failure.

## Phase 10 — Advanced orchestration

Only after core supervision, result delivery and provenance-aware memory are stable. The optional **Delegation Compiler** belongs here, not in the ordinary Lain chat loop. See [Intent-Aware Delegation](docs/INTENT_AWARE_DELEGATION.md).

Potential features:

- [ ] Lain chooses backend/profile according to explicit routing policy
- [ ] optional, owner-controlled intent-aware delegation: Off by default; ContextOnly, Clarify and Propose modes
- [ ] immutable original request + versioned resolved intent; **substantive rewriting** of the worker instruction permitted only when faithful
- [ ] mandatory per-dispatch **Changed/Unchanged notice**, inspectable exact submitted text and original-versus-final diff for modified instructions
- [ ] attributed memory/project evidence, clarification and frozen dispatch receipt
- [ ] worker-capability-aware brief formatting that cannot invent owner intent or widen privileges
- [ ] fidelity/clarification/cost evaluations against unchanged direct delegation, including negative and adversarial cases
- [ ] task decomposition into several worker sessions
- [ ] dependency graph between WorkSessions
- [ ] compare/review work between agents
- [ ] workspace worktree isolation
- [ ] resource/cost budget
- [ ] cross-agent handoff
- [ ] session templates
- [ ] reusable project playbooks

Guardrail:

This phase must not become "Lain reimplements a universal agent harness".

The orchestration layer should continue to delegate execution to workers. The optional compiler may rewrite **the outgoing worker instruction**, but it must preserve the original request and resolved intent, visibly report Changed/Unchanged, expose the exact text dispatched and provide a meaningful diff when changed. It must not replace worker planning, expose private memory without authorization or bypass Kernel owner confirmation. These new items are future proposals and remain unchecked.

## Phase 11 — External integrations

Potential:

- Telegram bot
- mobile notifications
- webhook/API automation
- Git hosting events
- calendar/time-based actions

All integrations must call the same Kernel API/domain layer.

No integration should contain its own Lain memory or worker supervisor.

# Near-term implementation order

When coding begins, prefer:

~~~text
1. domain types
2. SQLite/storage
3. session supervisor skeleton
4. Codex process/App Server adapter
5. event normalization
6. public API
7. CLI/TUI
8. Web
9. profiles
10. Lain Actor
11. memory
12. scheduler/background cognition
~~~

This ordering is deliberate.

Do not start by building memory or persona before worker supervision and durable state exist.

# Deferred questions

Make these decisions when implementation evidence exists:

- further crate extraction when real backend boundaries justify it (Phase 1 uses one crate);
- remote browser session authentication, TLS/origin/CSRF and credential rotation;
- direct Codex App Server vs Codex ACP as initial driver;
- world filesystem schema;
- exact Lain model/provider;
- embedding/vector search necessity;
- additional profile capabilities and explicit provider health probes;
- native Linux terminal/process lifecycle validation.

Do not prematurely freeze choices that do not affect current architecture.

Phase 1 resolves the initial HTTP/storage choices to Axum and rusqlite with
bundled SQLite. These are implementation choices, not new domain contracts.
Phase 3 resolves temporary runtime ownership, history ordering/retention and
terminal technology in [ADR 001](docs/adr/001-durable-work-and-runtime-attachment.md).
Phase 4 resolves same-origin static delivery and interim browser authentication
in [ADR 002](docs/adr/002-web-origin-and-authentication.md).
