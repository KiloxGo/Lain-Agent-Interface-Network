# Lain Memory

**Status:** normative foundation  
**Last reviewed:** 2026-10-04

## 1. Purpose

Lain has memory that belongs to Lain.

This is distinct from:

- worker-agent context;
- worker transcripts;
- operational event logs;
- browser chat state;
- configuration;
- secrets.

The previous prototype treated memory largely as extracted key/value tags and chat archives. The rewrite should use a clearer lifecycle and stronger provenance.

## 2. Core principle

> Persistence is not memory.

A piece of text being stored somewhere does not make it Lain memory.

Lain memory is information intentionally retained because it contributes to continuity, understanding, relationships, project knowledge, or future behavior.

## 3. Memory layers

### Episodic memory

Records meaningful events.

Examples:

- the decision to reboot Lain around a control-plane architecture;
- completion of an important worker session;
- a deployment failure and its cause.

Episodic memory is time- and provenance-heavy.

### Semantic memory

Durable knowledge abstracted from episodes.

Examples:

- a project's architecture;
- stable workflow preferences;
- architectural principles of Lain itself.

Semantic memory can be refined or contradicted.

### Relationship memory

Stable context about the owner-Lain relationship and interaction preferences.

This category should be conservative.

### Project knowledge

Durable knowledge scoped to a project/workspace:

- architecture decisions;
- known technical debt;
- important commands;
- unresolved risks;
- links to prior WorkSessions.

Project knowledge should include provenance.

### Reflective memory

Higher-level observations produced during bounded reflection.

Reflective memory is not automatically truth just because a model generated it.

## 4. What is not memory

Do not directly promote the following into semantic memory:

- every user utterance;
- every Lain response;
- every shell command;
- every worker event;
- every file diff;
- raw API responses;
- credentials;
- access tokens;
- private keys;
- huge copied source files.

Those belong in transcript, event, artifact, config, or secret systems.

## 5. Memory pipeline

~~~text
source
  |
  +-- owner conversation
  +-- completed WorkSession
  +-- world note
  +-- scheduled reflection
  |
  v
MemoryCandidate
  |
  v
validation / policy
  |
  +-- reject
  +-- defer
  `-- accept
        |
        v
MemoryRecord
        |
        +--> retrieval index
        `--> optional human-readable projection
~~~

Do not let an LLM silently write arbitrary final memories by emitting hidden XML tags.

## 6. MemoryCandidate

A candidate should carry:

- source;
- kind;
- content;
- optional confidence;
- optional importance;
- scope;
- provenance/evidence.

Acceptance policy may vary by source.

Examples:

- explicit "remember this" from owner -> strong candidate;
- inferred preference -> cautious;
- technical fact from worker -> require session provenance;
- reflective inference -> mark as reflective.

## 7. Provenance

Every important memory should answer:

- where did this come from?
- when?
- explicitly stated or inferred?
- which WorkSession/Turn/Conversation supports it?
- has it been superseded?

Possible references:

~~~text
conversation:<id>#message:<id>
work:<id>#turn:<id>
artifact:<id>
world:/journal/...
manual
~~~

Memory can be wrong; provenance is therefore first-class.

## 8. Updates and contradiction

A newer fact may reinforce, refine, supersede, contradict, or invalidate an older memory.

Prefer explicit relations rather than destructive replacement.

~~~text
memory A --superseded_by--> memory B
memory C --contradicts--> memory D
~~~

The active semantic view can prefer the latest trusted record while preserving history.

## 9. Retrieval

Retrieval should be layered:

1. deterministic scope/filter;
2. keyword/full-text search;
3. semantic retrieval if later justified;
4. recency/importance/provenance ranking;
5. bounded context assembly.

A vector database is not a required starting point.

SQLite FTS plus structured metadata may be sufficient for v1.

## 10. Context injection

Memory context passed to LainActor should be:

- relevant;
- bounded;
- provenance-aware;
- separated from instructions;
- free of raw secrets.

Do not dump the entire memory store into the system prompt.

## 11. Session summaries

Completed WorkSessions should not automatically become memory.

~~~text
raw provider events
   |
normalized operational record
   |
SessionSummary
   |
MemoryCandidates
   |
acceptance
   |
Memory
~~~

SessionSummary may be retained even when no memory candidate is accepted.

## 12. Private world

Lain's private World is related to memory but not identical.

Conceptual layout:

~~~text
world/
  identity/
  journal/
  projects/
  relationships/
  notes/
  artifacts/
  inbox/
  scratch/
~~~

A journal entry may reference Memory IDs. A memory may point to a world file as supporting material.

Human-readable files are valuable because the owner can inspect them outside Lain.

## 13. Identity memory

Foundational identity includes persona, stable behavior constraints, self-description, and values/interaction style.

Do not implement identity solely as mutable semantic memory.

Identity should have its own controlled document/configuration layer. Memory can inform it but should not casually overwrite it.

## 14. Forgetting and retention

Possible actions:

- remove from active retrieval;
- archive;
- redact;
- delete;
- mark superseded;
- preserve operational audit while deleting semantic projection.

Retention policy should be explicit by data category.

## 15. Privacy

Memory may contain sensitive personal/project information.

Requirements:

- authenticated owner access;
- workers do not receive full memory by default;
- only relevant excerpts are shared;
- secrets never enter memory;
- logs should avoid unnecessary duplication;
- future external integrations get minimal scope.

## 16. Reflection

Reflection is a controlled memory-producing process.

Example daily reflection:

~~~text
inputs:
  selected conversations
  SessionSummaries
  unresolved work
  recent memory candidates

outputs:
  journal entry
  zero or more reflective MemoryCandidates
  zero or more project updates
~~~

Reflection must be bounded and must not recurse continuously on itself.

## 17. Inspectability

The owner should eventually be able to inspect:

- memory content;
- type/scope;
- creation time;
- provenance;
- explicit vs inferred origin;
- supersession/contradiction links;
- last retrieval/use;
- why it was selected for context.

Memory should not be an opaque embedding store.

## 18. Initial storage direction

For first implementation:

- SQLite for structured MemoryRecord metadata;
- SQLite FTS for retrieval;
- optional human-readable world files;
- no required external vector database.

Schema should leave room for embeddings later.

## 19. Minimal first version

V1 can be intentionally small:

1. explicit owner remember operation;
2. SessionSummary-derived project facts;
3. basic episodic records;
4. provenance;
5. full-text retrieval;
6. inspect/delete/disable retrieval.

Do not implement automatic personality drift in v1.

## 20. Anti-patterns

Avoid:

- hidden memory tags as the primary architecture;
- saving all transcripts and calling them memory;
- treating LLM extraction as unquestionable truth;
- mixing secrets with memory;
- letting workers freely browse all personal memory;
- auto-updating identity from ordinary conversation;
- embedding-first architecture without demonstrated need.

## Future consumer: intent-aware delegation (not implemented)

Provenance-aware project knowledge, explicitly stored aesthetic standards and
owner-approved habits/constraints may later inform an **optional**, delegation-only
compiler. A memory hit is evidence, never permission to rewrite the owner's
current request or declare an inferred preference mandatory. Retain origin,
confirmation status, last relevant time, scope and supersession; surface conflicts
and stale facts instead of silently expanding prompts. Workers receive only the
minimum authorized context, not all of Lain's memories or private World. The
feature defaults **Off**, belongs to later orchestration work and must not be
built as part of initial Phase 7 Memory. See
[Intent-Aware Delegation](INTENT_AWARE_DELEGATION.md).
