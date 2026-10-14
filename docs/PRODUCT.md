# Product Definition

**Status:** normative foundation  
**Last reviewed:** 2026-10-04

## 1. One-sentence definition

**Lain is an always-on personal resident agent that provides a persistent identity, memory, private world, and unified control plane for multiple external worker-agent runtimes.**

## 2. Product thesis

Modern coding agents are individually powerful, but they are fragmented:

- each has its own CLI or application;
- sessions live in different stores;
- status and approvals are surfaced differently;
- model/API configuration differs per tool;
- long-running work is difficult to supervise uniformly;
- there is no persistent personal identity spanning them.

Lain should not compete with those harnesses.

Lain should sit above them.

She is the stable presence that knows the user, remembers selected things, receives requests from multiple interfaces, decides whether to answer directly or delegate, and supervises delegated work until the user no longer cares about it.

## 3. What Lain is

Lain is simultaneously four things.

### 3.1 A resident personal agent

Lain has:

- a persistent identity and persona;
- direct conversational ability;
- selected long-term memory;
- project and relationship knowledge;
- a private persistent storage space;
- scheduled or event-triggered background routines.

Her continuity exists independently of any one worker backend.

### 3.2 An agent control plane

Lain can manage worker sessions:

- create;
- resume;
- inspect;
- steer with follow-up prompts;
- cancel;
- close/archive;
- surface progress;
- route approval requests;
- summarize outcomes.

The long-term target is to manage multiple sessions concurrently.

### 3.3 A configuration/router layer

Lain manages named runtime profiles such as:

```text
codex/official
codex/local
claude/official
claude/gateway-a
deepseek/official
```

Profiles may define:

- backend;
- model;
- reasoning level where supported;
- endpoint/provider;
- approval policy;
- sandbox/policy settings;
- environment;
- secret references.

A session chooses a profile without requiring the user to manually rewrite global CLI configuration.

### 3.4 A multi-interface service

The same server-side Lain should be reachable through:

- Web;
- a terminal-first TUI/CLI;
- future third-party integrations such as Telegram.

These are different views of one resident system, not independent Lain instances.

## 4. What Lain is not

The project MUST resist scope drift.

Lain is **not**:

- another implementation of Codex or Claude Code;
- a generic ReAct/tool loop framework;
- a replacement for ACP or vendor agent protocols;
- a generic shell/file editing agent;
- a clone of a worker's context-management system;
- a transcript warehouse presented as "memory";
- a generic RAG/vector-database project;
- a browser application that owns task truth;
- a collection of terminal automation scripts.

If a worker runtime already provides reliable reasoning/tool execution for a class of tasks, Lain should integrate it rather than reproduce it.

## 5. Primary user

The initial product is single-user and personal-server-first.

Assumptions for v1:

- one trusted owner;
- one always-on server;
- multiple projects/workspaces;
- multiple worker backends;
- remote Web access;
- local or remote TUI access.

Multi-user tenancy is not a v1 goal. Authentication is still required for remote access.

## 6. Product behaviors

### 6.1 Conversational behavior

The user can talk to Lain without creating a worker session.

Lain should be able to discuss:

- current work;
- past work that entered her memory;
- project state;
- her own notes/world;
- ordinary conversation.

Conversation is not merely a control syntax.

Lain has a recognizable persona.

### 6.2 Delegation behavior

The user can request work naturally:

> Use Codex to investigate the failing auth tests.

Lain may translate that into a structured work request:

```text
backend/profile: codex/official
workspace: ~/codebase/Klog
instruction: investigate the failing auth tests
```

The worker owns the coding-agent loop.

Lain owns supervision and presentation.

### 6.3 Steering behavior

After delegation, the user can add instructions:

> Tell that session not to touch database migrations.

Lain sends a follow-up to the existing worker session where supported.

The session should not need to be restarted merely because the frontend changed.

### 6.4 Observation behavior

The user should be able to answer:

- What is running?
- Which backend is doing it?
- Which workspace is involved?
- What was the last meaningful activity?
- Is it waiting for approval?
- Did the last instruction succeed, fail, or get cancelled?
- What changed?
- How much did it cost/use, if the backend exposes usage?

Lain presents normalized information while retaining access to backend-specific details for debugging.

### 6.5 Background behavior

Useful autonomous routines include:

- summarize a completed worker session;
- update project knowledge;
- create a daily reflection;
- consolidate duplicate/stale memory;
- remind Lain of unresolved work;
- execute explicitly configured schedules.

Autonomy should be bounded and inspectable.

The system MUST NOT invent a constant stream of unnecessary self-prompts to simulate life.

### 6.6 Optional intent-aware subagent delegation (future)

On an owner-selected basis, Lain may improve a **subagent handoff** by attaching
small, provenance-bearing project constraints, saved preferences and relevant
context, or by asking a necessary clarifying question. This must preserve the
original request **verbatim** and distinguish explicit requirements from Lain's
inferences. Prompt enhancement is **Off by default**, optional at dispatch, and
never rewrites ordinary conversation with Lain. Users can inspect, reject or edit
proposed additions. Existing policy, permissions and owner confirmation remain
binding. This is not a second general-purpose agent harness. See
[the future design](INTENT_AWARE_DELEGATION.md).

## 7. Chat and Work

"Chat" and "Work" are product concepts, not necessarily two isolated runtimes.

A single conversation with Lain may naturally transition:

```text
user -> Lain conversation
          |
          +-- direct response
          |
          `-- create/inspect/steer worker
```

The UI may expose separate Chat and Work views because they serve different information needs.

The Kernel should still model delegation as Lain using work-control capabilities, not as switching to an unrelated application.

## 8. Lain's private world

Lain owns a durable space that worker agents do not own.

Conceptually:

```text
Lain World
  |-- identity
  |-- memory
  |-- journal
  |-- project knowledge
  |-- relationships
  |-- notes
  |-- artifacts
  |-- inbox
  `-- scratch
```

This world is not synonymous with the database.

Some parts may be represented as human-readable files, others as indexed records.

The important product property is ownership: this information belongs to Lain's continuity.

## 9. Worker independence

Worker backends are replaceable.

A Codex session may disappear, a Claude profile may be disabled, or DeepSeek Harness may change protocol. Lain should retain:

- her own identity;
- normalized session metadata;
- selected summaries;
- project knowledge that she intentionally kept;
- user-visible history needed to understand prior work.

Do not make Lain's continuity depend on one vendor.

## 10. Configuration experience

The user should be able to define reusable profiles and say things like:

- "Use official Codex."
- "Run this with the cheap Claude gateway."
- "Use the strongest profile for this review."

The first implementation may require explicit profile names.

Later, Lain may select profiles according to user policy, cost, availability, or task type. Automatic routing must remain explainable and overridable.

## 11. Experience goals

The project should feel more like a personal operations console than a chatbot wrapper.

Desired qualities:

- **persistent** — work and identity survive UI disconnects;
- **calm** — status is observable without noisy polling;
- **terminal-native** — CLI/TUI users are first-class;
- **cross-device** — a phone/browser can observe server-side work;
- **inspectable** — background actions and memory changes have provenance;
- **provider-agnostic** — no single harness defines the product;
- **personal** — Lain has continuity beyond task execution.

## 12. Success criteria for the first useful release

A first useful release should allow the owner to:

1. run `laind` continuously on a server;
2. connect from at least one frontend;
3. start multiple Codex work sessions in different workspaces;
4. observe normalized progress;
5. send follow-up instructions;
6. handle approval requests;
7. reconnect without losing session truth;
8. define at least two runtime profiles;
9. talk to Lain independently of a worker session;
10. let Lain persist a small amount of explicit, inspectable memory;
11. summarize completed worker work into her own project knowledge.

Supporting every backend is not required for the first useful release.

## 13. Product guardrails

Before adding a major feature, ask:

1. Does this improve Lain's identity, continuity, supervision, routing, or interface?
2. Is an external worker already responsible for this capability?
3. Can the feature remain backend-neutral?
4. Does it create persistent state? If so, who owns that state?
5. Could it expose credentials or sensitive worker output to memory?
6. Would it still make sense if Codex were replaced tomorrow?

If the answer to the last question is "no", the feature probably belongs in a backend adapter rather than the core.

## Phase 6 implemented priority

The current primary experience is CLI-first: setup/up/doctor/chat/ask and explicit
work control. Web is retained as a frozen compatible client. Lain's direct
conversation is independent of worker sessions and models, with typed reads and
owner-confirmed delegation. Durable conversation is operational continuity;
semantic memory, World and background cognition remain Phase 7+ targets.
Windows PowerShell and macOS zsh are equal development platforms; native evidence
is tracked separately. See [CLI.md](CLI.md) and [LAIN_ACTOR.md](LAIN_ACTOR.md).
