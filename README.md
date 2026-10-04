# Lain Agent Interface Network

> A persistent personal resident agent and control plane for long-running AI workers.

**Status:** Phase 1 Kernel skeleton implemented; worker integration starts in Phase 2.

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
                       HTTP / WS
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
      `-- HTTP + WebSocket API

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

The workspace currently contains one `lain-kernel` crate with a reusable library
and the `laind` binary. It provides durable session metadata, conservative restart
recovery, an in-process event bus, and an authenticated local read-only HTTP API.
It does not start workers or invoke models yet.

Quick start (PowerShell):

```powershell
$env:LAIN_API_TOKEN = [Guid]::NewGuid().ToString('N') + [Guid]::NewGuid().ToString('N')
cargo run --bin laind
```

The default listener is `127.0.0.1:8787`; the default database is
`data/lain.sqlite3` relative to the working directory. All endpoints require
`Authorization: Bearer <LAIN_API_TOKEN>`. Stop with Ctrl-C.

See [the development guide](docs/DEVELOPMENT.md#7-phase-1-workspace-and-local-operation)
for Linux commands, API examples, checks, storage ownership, and limitations.
Before implementing new architecture, read `AGENTS.md` and `ROADMAP.md`.
