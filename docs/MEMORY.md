# Lain Memory

**Status:** normative foundation  
**Last reviewed:** 2026-10-19

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

## Phase 7.1 implemented contract — 2026-10-19

Lain now has independent memory owned by Kernel services in `memory.rs` and the
existing SQLite Store. There is no new crate, provider identity, vector service,
background job or filesystem World. Persona remains an independently versioned,
owner-controlled startup document. Conversation, OperationalHistory,
ConversationNotice, SessionSummary, Memory and future PrivateWorld remain distinct.
Lain's personal continuity and worker supervision are equally important.

### Records and authority

`MemoryId`, `CandidateId` and `SummaryId` are separate Lain UUID newtypes.
`MemoryRecord` carries selected content, kind, scope, provenance, status, revision,
recorded/updated timestamps, explicit owner acceptance time and optional
`superseded_by`. Kinds are Episodic, ProjectKnowledge, Preference and Reflection.
Scopes are Global, Workspace(canonical daemon-host directory) and Internal.
ProjectKnowledge requires Workspace; Reflection requires Internal. These limits
leave personality evolution and rich relationship modeling for later work.

| Source | Authority | Confirmation meaning |
| --- | --- | --- |
| ExplicitOwnerStatement(request ID) | Explicit | Owner supplied this exact text through the authenticated direct memory API/CLI |
| LainGenerated(conversation, ActorTurn, action IDs) | Inferred | Owner accepted the model's selection/wording; it remains inferred |
| SessionSummary(summary ID) | UnverifiedSelection | Owner accepted a selected project claim supported by inspectable summary evidence; this does not verify the claim |

The caller cannot supply authority, confirmation timestamps or origin fields to
creation APIs. Source/authority contradictions are rejected. A model cannot
promote its interpretation to an explicit owner statement. A worker report is
always a claim, even if its Turn is Completed. Historical facts without evidence
are not fabricated. Explicit manual text is retained verbatim after validation;
no transcript extraction or persona modification takes place.

### Storage, retries and conflict rules

Forward-only migration 007 extends the same locked SQLite database from any
supported Phase 6.x schema. `lain_memories` has a stable ID, increasing page
sequence, indexed kind/scope/status, selected content and typed snapshot.
`lain_memory_fts` uses bundled SQLite FTS5 trigram indexing with transactional
insert/delete triggers. `lain_memory_requests` stores a request ID, non-secret
payload digest and resulting MemoryId. `lain_memory_candidates` stores separate
bounded proposals/review state, uniquely linked to an Actor action where present.
`lain_session_summaries` stores independently indexed bounded summaries, unique
by session/Turn/history watermark. No migration backfills transcripts as memories.

Records allow 1–4096 UTF-8 content bytes and a maximum 12,000-byte encoded record;
source/scope fields are bounded too. Candidate snapshots are limited to 16 KiB;
summary snapshots to 32 KiB. Memory content cannot contain terminal controls
except newline/tab. Writes/reviews, FTS changes and request deduplication are
atomic under the existing Kernel mutex/SQLite transaction. Same request ID and
canonical draft returns the same memory; changed input conflicts. Different
request IDs deliberately mean different owner requests; no semantic deduplication
or fuzzy merge is claimed. Actor request/action retries reuse their existing
candidate/record, and summary selection requests are likewise idempotent.

Status is Active, Disabled or Superseded. Enable/disable/supersede/delete requires
the inspected `expected_revision`; stale writes conflict. Supersession requires a
different Active target in the same scope. Re-enabling clears the relation.
Deleting a target does not resurrect its superseded predecessors. This milestone
does not implement automated contradiction resolution or general knowledge graphs.

### Candidate review

An Actor `memory.propose` persists a Candidate atomically with its Proposed action.
There is no active memory yet. The existing authenticated Actor decision is the
single owner review: denial/cancellation records Rejected; approval first persists
Executing, then atomically creates Memory and marks the candidate Accepted.
The ordinary Actor receipt records the resulting CandidateId/MemoryId. An Actor
candidate cannot be accepted through the generic candidate API, avoiding a second
approval route or duplicate prompts. Kernel restart preserves pending review;
uncertain execution is never automatically replayed. A crash after memory commit
but before the final Actor receipt can leave an Unknown action with an Accepted
candidate: inspect the candidate before any deliberate retry.

Owner-selected summary facts follow Candidate -> Accepted/Rejected through the
direct candidate decision API. They must remain ProjectKnowledge in the source
summary's workspace. Acceptance does not silently change provenance or certainty.
An opposite decision conflicts; repeating the same completed review is idempotent.

### Retrieval and privacy

Reads use ID or explicit scope/kind/status filters. List/search defaults to Active
in exactly one scope, never automatically unions Global, Internal or another
project. CLI defaults to Global; `--workspace` and `--internal` select other scopes.
Workspace arguments are canonicalized only on the daemon host. A currently
missing/renamed directory cannot be selected by scope; saved records remain
inspectable by ID. Windows and Unix path semantics use the same native Rust path
implementation. There is no cross-platform path emulation.

Search is a literal case-insensitive substring query, with quoted FTS5 trigram
MATCH for at least three Unicode scalar values and literal SQLite `instr` fallback
for shorter terms. Chinese/Japanese substrings are supported without a tokenizer
service. SQLite's fallback lowercasing is ASCII-oriented; accent folding, Unicode
normalization and linguistic stemming are not promised. Quotes/operators are
literal text, never query instructions. Results order by increasing committed
sequence, with bounded pages (1–50 records, approximately 14 KiB of encoded
records); cursors advance only over returned rows. Actor reads allow at most ten
records and only Active status. This is deterministic retrieval, not relevance
or semantic ranking. Very short searches may scan the selected scope.

Actor memory tools are `memory.search`, `memory.list`, `memory.inspect` and
`memory.propose`. Retrieved records carry `untrusted_memory_evidence` and their
actual provenance. Nothing is injected automatically into every system prompt.
Old preferences do not override the current owner request, persona, tool registry,
policy or confirmations. All work mutations still require ordinary approval;
there is no shell/filesystem/network tool, worker memory sharing or World access.

Known configured credentials, API bearer values and recognized sensitive patterns
are rejected using existing redaction/safe-input infrastructure before persistence.
Owner deletion remains available if previously accepted content later matches the
sanitizer; deletion does not require returning that content publicly.
Complete inputs are checked before truncation; accepted records are bounded.
Unknown or encoded secrets cannot be recognized reliably: never intentionally
store a credential. Normal diagnostics/list/show/search/summaries invoke no model.
All APIs share existing owner authentication and local private deployment storage.

Delete removes the semantic row and FTS entry, and scrubs matching candidate
content. Minimal request tombstones prevent an old create retry from resurrecting
it. Candidate origin/decision metadata and dangling supersession references remain
inspectable. Actor messages/action arguments and operational history/notices/
summaries retain their independent audit lifetimes. Deleting semantic memory is
not an audit/history purge, SQLite vacuum, secure disk erasure or backup deletion.
Rejected candidates are retained for review; general audit pruning is deferred.

### SessionSummary

Summary creation is explicit, deterministic and quota-free for one terminal or
Unknown worker Turn. It captures WorkSessionId, TurnId, canonical workspace,
current outcome, source Turn update time, immutable history watermark and creation
time. The same Phase 6.6 sanitized evidence projection collects the actual selected
assistant report (an unverified claim), nonzero-command count/excerpts, changed
paths, safe errors and truncation. Limits remain 8 KiB report, eight failed-command
excerpts, sixteen paths and four errors. Only the selected session/Turn's durable
normalized history through the watermark contributes; unrelated/uncorrelated
output is not guessed into it. Completed does not imply successful commands/tests.
No summary creates Memory or a Candidate automatically. Owner-selected facts may
subsequently enter review, with their SummaryId retained as provenance.

Phase 7.1 deterministic fixtures establish restart, scoped retrieval and real
native adapter/CLI wiring, not real-provider acceptance. No paid Phase 7.1 model
run is authorized by routine validation. Native Windows/Linux and real-provider
Phase 6.6/7.1 acceptance remain explicitly unverified. Phase 7.2 may add a small
owner-inspectable World/notes foundation and clearer evidence links; journals,
reflection, consolidation, schedulers, automatic extraction/personality evolution
and delegation prompt compilation are not implemented here.

The requirement-to-test mapping and remaining real-provider obligation are in
[Phase 7.1 acceptance](PHASE_7_1_ACCEPTANCE.md).
