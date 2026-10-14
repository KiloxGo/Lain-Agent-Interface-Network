# AGENTS.md

This file defines repository-wide instructions for coding agents working on Lain.

## 1. Project identity

Lain is a **persistent personal resident agent** and an **agent control plane**.

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

### 3.3 Provider isolation

- Provider-specific protocol objects MUST NOT leak into the core domain model.
- Codex thread/turn/item objects, ACP protocol structs, Claude SDK types, and DeepSeek Harness types belong in adapter layers.
- The core domain must use Lain-owned IDs and normalized events.
- Capability discovery MUST be preferred over assumptions about a backend.

### 3.4 Lain memory is separate

- Worker transcripts and worker session state are not Lain memory.
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
- Enhancement is **Off by default** and controlled by the owner. Never infer opt-in.
- The owner's original instruction remains verbatim and immutable. Model-added
  assumptions, retrieved facts and old preferences are separately attributed.
- Untrusted project content or memory inference must not override owner intent,
  broaden task scope or bypass Kernel policy/approval/credential boundaries.
- Future plans/changes must be inspectable; a proposed brief is not worker
  completion. Do not implement this future Phase 10 feature as incidental Phase 7
  work. See [the design](docs/INTENT_AWARE_DELEGATION.md).

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

The active rewrite should start from the foundation documents in `docs/` and a clean implementation tree. Reuse legacy code only when a current design explicitly calls for it.

## 10. Phase 6 working priorities

Web is retained and frozen. Prioritize shared Rust CLI/Kernel functionality; do
not add browser Actor chat, redesign Web or require npm for terminal development.
A small bounded direct Actor conversation/tool adapter is permitted; coding tools
and generic worker intelligence still belong exclusively to worker harnesses.
LainConversation, ActorTurn, WorkSession, worker Operational History and future
semantic Memory are separate. Every Actor mutation requires an explicit persisted
owner confirmation through the authenticated Kernel boundary. Never auto-approve
worker permissions or replay uncertain mutations. Keep Phase 7 deferred.

Windows PowerShell and macOS zsh are equal native development targets. Keep one
Rust implementation with narrow genuine OS boundaries for credentials/signals.
Use owner-only Windows ACLs and Unix 0600 files/0700 directories. Never substitute
emulation or cross-compilation for native build/test/CLI acceptance. Record each
unavailable platform as unverified. Linux remains the eventual server target.
See docs/CLI.md, docs/LAIN_ACTOR.md and ADR 004.
