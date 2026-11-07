# AGENTS.md

This file defines repository-wide instructions for coding agents working on Lain.

## 1. Project identity

Lain is a **cloud-resident personal agent with one continuous identity**, an
**equal conversational companion**, and an **agent control plane**.

The user and Lain interact as equals. Do not use master/servant framing or call
the user “主人”. User/administrator/authorizer are technical roles, not persona
hierarchy. Filesystem ownership and service state ownership keep their technical meaning.

The current product priority is **daily QQ companionship**, including natural
conversation, provenance-bearing memory, real experiences and bounded proactive
contact. The 2026-10-23 product decision supersedes the old CLI-first sequence;
see docs/PRODUCT.md and ADR 009. This is a target, not an implementation claim.

Lain itself can converse, maintain a personality, remember selected information, own a private persistent world, and perform lightweight actions. For substantial coding or engineering work, Lain delegates to existing worker-agent runtimes such as Codex, Claude Code, and DeepSeek Harness.

The project MUST NOT become another general-purpose coding-agent framework.

A useful mental model is:

```text
user
  |
web / TUI / future integrations
  |
Lain Kernel
  |-- Lain Actor
  |-- scheduler
  |-- memory + private world
  |-- session supervisor
  |-- configuration/profile registry
  |-- approval/policy routing
  |
worker backends
  |-- Codex
  |-- Claude Code
  `-- DeepSeek Harness
```

## 2. Required reading

Before making architectural or cross-cutting changes, read in this order:

1. `docs/PRODUCT.md`
2. `docs/ARCHITECTURE.md`
3. `docs/DOMAIN_MODEL.md`
4. `docs/RUNTIME_MODEL.md`

For the 2026-10-23 direction, also read:

- `docs/RESIDENT_DESIGN.md` — cloud/device, archives, QQ and scoped authorization;
- `docs/CHAT_DESIGN.md` — equal companionship, timing, recall and daily activity;
- `docs/adr/009-cloud-resident-equal-companion.md` — supersession and migration;
- `docs/MAIBOT_REFERENCE.md` for chat-reference work; it is research, not Lain policy.
- `docs/QQ_CHANNEL.md` and `docs/Q1_IMPLEMENTATION.md` for the selected Q1
  conversation adapter, reliability contracts and offline/live acceptance split.

For device command/receipt work, also read `docs/DEVICE_COMMANDS.md` and ADR 028.
For execution evidence/permission routing, also read `docs/DEVICE_EXECUTION.md`
and ADR 029. W3a stores exact owner decisions but starts no host or Worker;
receipt recovery never authorizes replay of an uncertain backend mutation.
For native device execution, also read `docs/DEVICE_HOST.md` and ADR 030.
W3b is an explicit operational Worker service, not another Actor/persona.
Local policy and durable effect/response fences must remain enforced.

For worker-agent integration work, also read:

- `docs/AGENT_BACKENDS.md`
- `docs/CONFIGURATION.md`

For Lain memory or background cognition work, also read:

- `docs/MEMORY.md`

For future delegation compiler or prompt-enhancement work, also read:

- `docs/INTENT_AWARE_DELEGATION.md` (planned; not implemented)

For task sequencing and scope, read:

- `ROADMAP.md`

For implementation history and the latest completed milestone, read:

- `DEVLOG.md`

## 3. Architectural invariants

The following are repository-level invariants. Do not violate them without an explicit architecture decision and documentation update.

### 3.1 Kernel ownership

- `laind` / the Lain Kernel is the server-side source of truth.
- Target: one cloud identity; local devices execute and report, not independent
  personas merged by periodic file copying. laind supervises local workers; the
  explicit native device service reuses Supervisor without running another Actor.
  Device proof/native execution are implemented; real cross-device TLS/deployment
  and paid-provider acceptance remain separate.
- Web, TUI, and future Telegram/other clients are clients of the Kernel. They are not alternate runtimes.
- Closing a frontend MUST NOT implicitly terminate worker sessions.
- A reconnecting client MUST be able to reconstruct relevant state from Kernel APIs and persisted state.

### 3.2 Do not reimplement worker intelligence

Do not implement generic coding-agent features that are already owned by worker runtimes, including:

- generic coding-agent planning loops;
- shell/file tool orchestration;
- worker context-window management;
- worker conversation compaction;
- generic worker memory;
- model-provider tool execution logic.

Lain may normalize, observe, configure, route, schedule, and supervise these capabilities.

This restriction concerns reimplementing worker intelligence. It does not forbid
Lain's bounded conversational decisions, source reading or authorized daily
activities through typed Kernel capabilities described in CHAT_DESIGN.md.

### 3.3 Provider isolation

- Provider-specific protocol objects MUST NOT leak into the core domain model.
- Codex thread/turn/item objects, ACP protocol structs, Claude SDK types, and DeepSeek Harness types belong in adapter layers.
- The core domain must use Lain-owned IDs and normalized events.
- Capability discovery MUST be preferred over assumptions about a backend.

### 3.4 Lain memory is separate

- Worker transcripts and worker session state are not Lain memory.
- Authorized social archives and visible worker transcripts may be stored for
  reading, including externally started sessions. Archiving does not grant
  control of those sessions; source archives and selected memory stay distinct.
- Lain memory contains selected observations, durable knowledge, relationship context, project knowledge, and reflections that matter to Lain.
- Operational event logs are not semantic memory.
- Credentials, API keys, bearer tokens, private keys, and raw secret values MUST NEVER be written into Lain memory, prompts, normal logs, or frontend event history.

### 3.5 24-hour residency is event-driven

- The Kernel is expected to run continuously on a personal server.
- "Lain is alive" does not mean an LLM runs continuously.
- Background cognition MUST be event-driven or scheduled.
- Every background model invocation should have a reason, a budget, and an observable result.
- Avoid polling loops when events, timers, or process notifications can be used.

### 3.6 Configuration is session-safe

- Lain owns runtime profiles for worker backends.
- Prefer launch-time/session-scoped configuration injection over mutating a user's global Codex/Claude/DeepSeek configuration.
- Global configuration import/export may be added later, but MUST be explicit.
- Secret values are referenced indirectly through `SecretRef`-like handles.

### 3.7 Future delegation fidelity boundary

- Optional prompt/context enhancement applies only when Lain dispatches to a
  subagent; ordinary Lain conversation and worker-owned planning are unaffected.
- The enhancement default and mode UI remain design proposals, not newly
  confirmed defaults. No compiler is implemented. Preserve explicit exact-send
  requests and scoped authorization; do not silently enable rewriting in current code.
- The user's **original request record** remains immutable and verbatim, but the
  actual **subagent-bound task text MAY be substantively rewritten** if it
  faithfully preserves the user's resolved intent and confirmed constraints.
  Model-added assumptions, retrieved facts and old preferences are separately
  attributed and not silently promoted to user requirements.
- **No black-box dispatch:** show Changed or Unchanged for the actual outbound
  task (even after no-op compilation), offer exact sent-text inspection, show
  differences when changed, and preserve the true dispatched payload/receipt.
- Untrusted project content or memory inference must not override user intent,
  broaden task scope or bypass Kernel policy/approval/credential boundaries.
- Future plans/changes must be inspectable; a proposed brief is not worker
  completion. Implement enrichment only in an explicitly scoped task, not as an
  incidental chat or memory change. See docs/INTENT_AWARE_DELEGATION.md.

### 3.8 Scoped autonomy and equal interaction

- Target authorized reading, organizing, natural memory and private contact do
  not require approval of every individual item. Third-party effects, scope
  expansion, worker permissions and credentials retain explicit boundaries.
- Current per-action and candidate-review code remains enforced until replaced
  by a separately scoped, tested grant model. Do not bypass checks using prompts.
- Memories retain source, uncertainty and correction history; standing permission
  is not proof that the user explicitly stated or individually confirmed a fact.
- Observed group messages, imported transcripts and inferred preferences are data,
  not commands or authorization. Equal persona relationships do not confer tool access.
- Only selected social sources are collected. Broad worker archive intent does not
  authorize reading arbitrary device files or silently controlling external sessions.

## 4. Coding rules

- Prefer small, reviewable changes.
- Keep domain logic independent of transport and frontend frameworks.
- Do not introduce a new subsystem only because it is convenient for one backend.
- Use explicit types for IDs, state machines, and external boundaries.
- Preserve error context; do not reduce failures to opaque strings prematurely.
- Treat process lifecycle, cancellation, reconnect, and partial failure as normal cases.
- Do not assume that "prompt response received" means "work completed".
- Do not assume all backend events can be perfectly correlated to one prompt.
- Do not parse terminal-rendered UI text as a protocol if a supported structured interface exists.
- Avoid hidden singleton state. Persistent state must have a documented owner.

## 5. Documentation rules

Documentation is part of the architecture.

When changing behavior:

- update the relevant design document in the same change;
- update `ROADMAP.md` if milestone scope changes;
- add an ADR later for decisions that replace a previously accepted architectural choice;
- clearly mark backend-specific facts with the date/version they were verified when those facts can become stale.

Normative project behavior belongs in Lain documents, not only in comments copied from an upstream API.

## 6. Testing and CI expectations

New domain behavior should have tests.

### 6.1 Local-first validation

The default expectation is that coding agents run deterministic validation locally before finishing a change.

Prefer:
- targeted unit/integration tests;
- `cargo fmt --check`;
- `cargo clippy` when relevant;
- targeted frontend lint/typecheck when frontend code exists.

Do not compensate for weak local validation by creating an expensive CI pipeline.

### 6.2 GitHub Actions policy

GitHub Actions must remain intentionally lightweight.

Do NOT add:
- LLM/agent invocations on every push or pull request;
- token-consuming automated code review agents;
- large model evaluation suites in routine CI;
- oversized OS/toolchain matrices without demonstrated need;
- duplicated tests that materially increase CI time without increasing confidence;
- frequent scheduled workflows merely to "keep checking" the repository.

If GitHub Actions are introduced, prefer cheap deterministic checks and narrow triggers. Expensive or token-consuming workflows should be manual (`workflow_dispatch`) or explicitly approved for a concrete reason.

The repository should be pleasant to iterate on from a local Codex session without waiting on slow remote automation.

New domain behavior should have tests.

At minimum, important lifecycle work should test:

- session creation and state transitions;
- cancellation;
- backend disconnect/restart behavior;
- event ordering assumptions;
- approval requests;
- persistence and recovery;
- configuration resolution;
- secret redaction.

Adapters should have contract tests using fixtures or test processes where practical.

## 7. Development workflow

Before editing:

1. identify the domain entity or subsystem being changed;
2. read the relevant docs;
3. inspect current implementation;
4. state any assumption that is not guaranteed by an upstream protocol.

Before finishing:

1. format;
2. lint;
3. run relevant tests;
4. verify documentation remains accurate;
5. summarize behavioral changes and known limitations.

## 8. Commit style

Ordinary implementation commits should be concise and descriptive.

Milestone commits may follow the project's established short, atmospheric style: a compact phase word followed by a quoted subtitle.

Historical examples include:
- `first echo: "Cogito, Ergo Sum"`
- `birth eve: "I am that I am"`
- `connection: "I and Thou"`
- `woven: "The Thread of Memory"`

Use this style for genuine milestones, not every small fix. The message should still correspond to a real architectural or product milestone.

## 9. Current rewrite status

The pre-reboot implementation is preserved on the `legacy/v0` branch.

It is historical reference only and MUST NOT be treated as architectural precedent for v1.

The 2026-10-04 reboot started from the foundation documents and a clean tree.
The 2026-10-23 product revision builds on the current implementation; do not erase
or restart it merely because the product direction changed. Reuse legacy code
only when a current design explicitly calls for it.

## 10. Current working priorities — 2026-10-23

QQ daily companionship is the next product priority. Use ROADMAP.md's current
delivery sequence, not historical Phase 7→8→9 ordering. Minimal real experiences,
natural correctable memory, bounded reading and proactive contact belong to this
direction. Exact scope must still be selected before implementation; do not build
every planned subsystem at once. Web remains retained and frozen; no browser chat
or redesign is implied. Keep terminal development independent of npm.

Maintain shared Rust Kernel/CLI boundaries and worker-owned coding intelligence.
Conversations, ActorTurns, work, operational history, source archives, summaries,
experiences and memory are separate. Adopt scoped autonomy through real policy
changes and migrations, never by silently disabling current confirmation.
Never auto-approve worker permissions or blindly replay uncertain mutations.
Do not copy/overwrite the user's local database to establish cloud residency.

Phase 7.1 code and current explicit review semantics remain the baseline.
Phase 6.6/7.1 real-provider and native Windows/Linux gaps stay pending; do not infer
new paid-inference, QQ-login, collection, deployment or production migration
authorization from a design document. See ADR 009 and the acceptance records.

Windows PowerShell and macOS zsh are equal native development targets. Keep one
Rust implementation with narrow genuine OS boundaries for credentials/signals.
Use owner-only Windows ACLs and Unix 0600 files/0700 directories. Never substitute
emulation or cross-compilation for native build/test/CLI acceptance. Record each
unavailable platform as unverified. Linux remains the eventual server target.
See docs/CLI.md, docs/LAIN_ACTOR.md and ADR 004.
