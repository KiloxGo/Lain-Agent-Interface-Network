# Lain Agent Interface Network

> A persistent personal resident agent and control plane for long-running AI workers.

**Status:** Phase 6 CLI-first Actor implementation; native Windows and real-provider acceptance pending. Web is retained and frozen.

Lain is designed to live continuously on a personal server. You can talk to her directly, but she is also able to create, supervise, configure, observe, steer, and summarize long-running worker-agent sessions such as Codex, Claude Code, and DeepSeek Harness.

The project deliberately does **not** attempt to rebuild those worker runtimes.

## Why Lain exists

Coding agents are becoming capable, but they are usually experienced through separate CLIs, applications, session stores, provider settings, and permission systems.

Lain provides a persistent layer above them:

- one always-on identity;
- one place to see all active work;
- one API for Web, terminal, and future integrations;
- one scheduler for background routines;
- one profile system for choosing backend/model/API configuration;
- one approval surface;
- one personal memory and private world that belongs to Lain rather than to any worker.

A worker may disappear. Lain remains.

## Core experience

### Chat

You can talk to Lain as a persistent character/agent.

She may answer directly for conversational or lightweight tasks.

### Work delegation

For substantial work, Lain can delegate:

```text
"Ask Codex to investigate the auth bug in Klog."

        |
        v

Lain -> create worker session -> Codex works in the repository
```

You should then be able to:

- watch the session progress;
- inspect shell/tool/file activity in normalized form;
- send another instruction to the same session;
- approve or reject sensitive actions;
- disconnect the UI while work continues;
- return from another device and resume observation;
- ask Lain what happened.

### Resident behavior

The server-side Kernel runs continuously.

Lain may wake for bounded background activities such as:

- summarizing finished worker sessions;
- extracting durable project knowledge;
- updating her own memory;
- producing a daily or weekly reflection;
- checking scheduled tasks.

Continuous residency must not imply continuous model inference.

## Target architecture

```text
                  +----------------------+
                  |       Clients        |
                  | Web / TUI / future   |
                  +----------+-----------+
                             |
                       HTTP / SSE
                             |
+--------------------------------------------------------+
|                      Lain Kernel                       |
|                                                        |
| Lain Actor      Scheduler       Event Bus              |
| Memory          Private World   Profile Registry       |
| Session Supervisor              Approval/Policy        |
| Persistence                     Secret References      |
+----------------------------+---------------------------+
                             |
                     backend adapters
                             |
            +----------------+----------------+
            |                |                |
          Codex          Claude Code     DeepSeek Harness
```

See `docs/ARCHITECTURE.md` for the normative architecture.

## Design principles

1. **Use existing harnesses.** Lain supervises workers instead of reimplementing them.
2. **Kernel is the source of truth.** Frontends may disappear without stopping work.
3. **Lain has her own continuity.** Her memory is not a copy of worker transcripts.
4. **Backends are replaceable.** Provider-specific protocol details stay behind adapters.
5. **Configuration is explicit.** Backend/model/API profiles are first-class objects.
6. **Secrets are not memories.** Credentials never belong in Lain's semantic world.
7. **Always-on does not mean always-thinking.** Background cognition is scheduled or event-triggered.

## Repository status

The project has entered a clean architecture reboot.

The pre-reboot implementation is preserved in the `legacy/v0` branch. The active rewrite intentionally does **not** carry the old Rust kernel, React frontend, chat loop, or memory implementation forward.

The rewrite is documentation-first:

- `AGENTS.md` — instructions for coding agents working in this repository;
- `docs/PRODUCT.md` — product definition and non-goals;
- `docs/ARCHITECTURE.md` — system boundaries and component responsibilities;
- `docs/DOMAIN_MODEL.md` — shared domain vocabulary;
- `docs/RUNTIME_MODEL.md` — 24h daemon, scheduling, wake and recovery model;
- `docs/AGENT_BACKENDS.md` — worker backend integration rules;
- `docs/MEMORY.md` — Lain's independent memory;
- `docs/CONFIGURATION.md` — runtime profiles and provider routing;
- `ROADMAP.md` — staged implementation plan;
- `DEVLOG.md` — milestone history, decisions, problems, and next-step handoff.

## Expected deployment

The primary deployment target is a personal Linux server running Lain continuously.

A typical future deployment may look like:

```text
systemd / container
  `-- laind
      |-- SQLite / durable state
      |-- Lain world directory
      |-- worker subprocesses / adapters
      `-- HTTP + SSE API / optional built Web assets

browser -----------+
terminal TUI ------+--> laind
future bot --------+
```

The Web UI should be usable remotely. The TUI should feel native to terminal users and provide session attach/inspect/steer workflows rather than being a second implementation of the Kernel.

## Non-goals

Lain is not intended to become:

- a new generic coding-agent harness;
- a replacement for Codex, Claude Code, or DeepSeek Harness;
- a terminal scraping wrapper around interactive CLIs;
- a generic vector database product;
- a frontend whose state disappears when the browser closes;
- a system that stores raw secrets inside prompts or memories.

## Development

The workspace contains `lain-kernel` (library and `laind`) and `lain-client`
(`lain`, HTTP/SSE only). It supervises real Codex workers with durable sessions,
detachable runtimes, turns, approvals and indexed operational history. Idle workers
release their process; follow-ups resume the same session lazily. Worker execution
remains owned by Codex.

Windows PowerShell and macOS zsh use the same Rust codebase and commands:

```text
cargo build --locked --workspace
cargo install --locked --path crates/lain-kernel --bin laind --force
cargo install --locked --path crates/lain-client --bin lain --force
lain setup --local
lain up
```

In another terminal: `lain doctor`, `lain status`, `lain chat`. Setup keeps the
local token in an owner-only per-user file; no token copying between terminals.
`up` launches the existing Kernel in the foreground; Ctrl-C gracefully stops it.
No npm, browser or Vite is required. See [CLI.md](docs/CLI.md) for PowerShell/zsh
binary paths, native credential permissions, PATH conflict diagnostics, remote
mode, existing work/profile/approval commands and platform acceptance entrypoints.

Chat requires its own explicit model configuration and namespaced SecretRef;
Codex login is separate. [LAIN_ACTOR.md](docs/LAIN_ACTOR.md) documents setup,
persona, persistent conversations, typed tools, owner confirmation and recovery.
`lain --json ask` supports scripting; mutations remain pending until explicitly
approved. Existing workers run independently of chat/observers. Conversations are
operational continuity, not semantic memory. Phase 7 has not begun.

The React/TypeScript Web client remains available through the existing optional
`LAIN_WEB_DIR` workflow, documented in [WEB.md](docs/WEB.md). Web development is
paused; Phase 6 has no browser chat or redesign. Public Internet/TLS/session-cookie
authentication remains unimplemented; remote clients need trusted transport.

Native macOS deterministic build/test/CLI smoke is validated. Windows and Linux
checks must run on their native hosts before acceptance is claimed. Real paid
Actor/Codex inference acceptance is opt-in and unverified. Exact evidence and
limitations are in [DEVLOG.md](DEVLOG.md).

Before architectural work, read `AGENTS.md` and `ROADMAP.md`.
