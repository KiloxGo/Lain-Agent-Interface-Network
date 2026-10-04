# Roadmap

**Status:** working plan  
**Last reviewed:** 2026-10-04

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
- [ ] add initial ADRs for major architecture decisions
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
Native Linux lifecycle validation remains outstanding. Phase 3 has not started.

Exit criteria:

- two Codex sessions can work concurrently in different workspaces;
- frontend disconnect does not stop either;
- Kernel can restart and represent/recover session state correctly;
- no terminal UI scraping is required.

## Phase 3 — Headless CLI and TUI

**Goal:** make terminal users first-class before building a complex graphical frontend.

Target CLI surface may include:

~~~text
lain status
lain work list
lain work start
lain work attach <id>
lain work send <id>
lain work cancel <id>
lain work logs <id>
lain profile list
~~~

TUI target:

- active session list;
- session timeline;
- approval view;
- prompt/steer input;
- profile/backend information.

The TUI talks only to the public Kernel API.

Exit criteria:

- normal worker supervision is practical without a browser;
- no direct backend-specific logic exists in the TUI.

## Phase 4 — Web control plane

**Goal:** remote cross-device access.

Deliverables:

- [ ] authenticated Web client
- [ ] live WorkSession dashboard
- [ ] session detail/timeline
- [ ] approvals
- [ ] reconnect/state resync
- [ ] create/steer/cancel sessions
- [ ] profile selection
- [ ] responsive mobile layout

Visual direction can become more Lain-like after the operational UI is reliable.

Exit criteria:

- open browser on another device;
- inspect server-side work;
- close browser;
- return later;
- observe continued work and current state.

## Phase 5 — Runtime profiles and provider switching

**Goal:** provide CC-Switch-like configurability at a higher abstraction level.

Deliverables:

- [ ] RuntimeProfile registry
- [ ] SecretRef integration
- [ ] at least two Codex profiles
- [ ] per-session profile selection
- [ ] endpoint/provider override
- [ ] redacted resolved-profile inspection
- [ ] workspace default/allowed profile policy
- [ ] profile validation/health

Exit criteria:

- two Codex sessions can run simultaneously with different profiles without mutating global Codex configuration.

## Phase 6 — Lain Actor

**Goal:** bring back Lain herself on top of the working control plane.

Deliverables:

- [ ] persona/identity definition
- [ ] Lain conversation API
- [ ] model adapter for Lain
- [ ] typed tools/capabilities for work control
- [ ] inspect/list/create/steer/cancel work through Lain
- [ ] context builder
- [ ] no raw secrets available to Lain
- [ ] explicit distinction between direct answer and delegated work

Example:

~~~text
owner: Ask Codex to inspect Klog's auth tests.

Lain:
  work.create(profile=codex/official, workspace=Klog, ...)
~~~

Exit criteria:

- owner can operate worker sessions conversationally through Lain;
- Lain never implements the worker coding loop herself.

## Phase 7 — Lain memory and private world

**Goal:** give Lain durable continuity independent of worker sessions.

Deliverables:

- [ ] memory schema and provenance
- [ ] explicit remember/forget operations
- [ ] project knowledge
- [ ] SessionSummary model
- [ ] session-completion summarizer
- [ ] MemoryCandidate pipeline
- [ ] basic retrieval, likely SQLite FTS
- [ ] private World directory/service
- [ ] journal/project notes
- [ ] memory inspection UI/CLI

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

Only after core supervision is stable.

Potential features:

- [ ] Lain chooses backend/profile according to explicit routing policy
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

The orchestration layer should continue to delegate execution to workers.

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
- exact WebSocket/SSE split;
- direct Codex App Server vs Codex ACP as initial driver;
- world filesystem schema;
- exact Lain model/provider;
- embedding/vector search necessity;
- frontend framework;
- TUI framework;
- worker process detach semantics on Kernel shutdown.

Do not prematurely freeze choices that do not affect current architecture.

Phase 1 resolves the initial HTTP/storage choices to Axum and rusqlite with
bundled SQLite. These are implementation choices, not new domain contracts.
