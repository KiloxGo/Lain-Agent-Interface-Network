# Lain Memory

## D1b experiences remain separate — 2026-10-26

ReadingExperience is a source-linked projection of actual bounded Actor summary
generation, not semantic memory, a diary of every input or independent verification.
An optional reading processing/disclosure scope selects one private binding; it is
independent of MemoryGrant/SourceGrant/reply authority. Owner correction retains
its own attribution; withdrawal suppresses the exact URL and invalidates derived
content while keeping operation facts. The summary cannot resurrect via old
model-history copies or unsent speech. Experience-influenced turns cannot silently
propose/remember/correct semantic memory until derived-memory scope/provenance is
separately implemented. Ordinary existing candidate review and forget remain intact.
See [ADR 014](adr/014-source-linked-reading-experiences.md).

**Status:** Phase 7.1, Q2-A private-binding memory and Q2-B1 bounded private source lookup

**Last reviewed:** 2026-10-25

## C1 private conversation consumption

Natural recall may paraphrase relevant evidence without displaying internal UUIDs
in every social reply. Existing grant, inspection/effect receipts and source
APIs/CLI retain exact IDs/revisions/provenance for explanation on request. No
permission is inferred from ordinary conversation or a persona instruction.

Private turns retain only the latest request's selected memory IDs/revisions,
rechecking them after generation, on final text/new-action commits and before
ordinary draft dispatch. Revoke, expiry, correction and forget invalidate stale
recollection. Reconsideration refreshes current evidence and discards opaque
continuation; already committed memory effects remain truthful IDs-only records
so they are not presented as unexecuted or blindly replayed. Later history excludes
unconfirmed drafts and withholds changed-memory-derived replies. These checks add
no background extraction or wider sources. See [ADR 012](adr/012-bounded-private-conversation.md)
and [C1 acceptance](C1_ACCEPTANCE.md).

C1.1 preserves historical private memory actions in model context as action
state plus correlated typed `MemoryEffect` metadata, without selection arguments,
source quotations or old result bodies. Failed/Unknown actions do not become
committed effects. Full canonical action records remain inspectable separately.
Under a currently valid memory grant, each fresh private memory context also
includes the binding's latest three committed effects, in commit order, marked
`kernel_operation_metadata`. These receipts contain operation/IDs/grant revision/
time, not remembered text, and survive semantic correction/deletion. They grant no
capabilities and do not broaden sources. Active recall can be empty after a real
deletion; it does not prove the record was never saved. Inferred selection remains
inferred, but a successful scoped effect is committed without a second ordinary
candidate approval. See [C1.1 evidence](C1_1_ACCEPTANCE.md).

## Current target — natural, scoped and correctable memory

[PRODUCT.md](PRODUCT.md) and [ADR 009](adr/009-cloud-resident-equal-companion.md)
change the target from mandatory review of every model selection to autonomous
remembering under scoped standing authorization. Q2-A implements the narrow
private-binding policy below; existing Phase 7.1 candidate review remains enforced
for ordinary `memory.propose` and all sources outside this grant.

The user and Lain are equals. Relationship knowledge records shared experience
and evolving understanding, not a master/servant hierarchy or unquestionable
psychological labels. A temporary impression may guide gentle interaction while
retaining uncertainty; user correction must change subsequent behavior.

### Storage layers and provenance

Authorized social records and visible Worker transcripts, including external
sessions, may be archived. Archives are not semantic memory; access does not mean
Lain read them or participated in the work. Summaries, factual evidence, experiences,
selected memory, temporary attention and reusable templates have independent roles.

Automatic writes must record their source and authorization basis without
claiming an individual user confirmation. Existing Explicit/Inferred/
UnverifiedSelection records and their confirmation fields retain their real meaning.
The source/authority distinction survives acceptance, correction and summarization.

### Retrieval, correction and lifecycle

Target both small contextual recall and deliberate source lookup. Every path
respects scope/disclosure policy, including cross-channel recall. One shared
identity does not permit private material to be disclosed to arbitrary contacts.
Whole-store prompt injection, mandatory vector databases and global fuzzy merging
are not product requirements.

Corrections should identify affected records and invalidate or update derived
summaries, profiles and caches. Ambiguous bulk changes require clarification;
clear corrections can use standing authorization. Removal of semantic memory,
source archive, attachments and backups are distinct operations with explicit
retention choices. Do not claim secure erasure or allow re-import to silently
resurrect intentionally deleted content.

Minimal real experiences and bounded autonomous activities now belong to the
QQ-first plan; historical blanket deferrals below do not block their scoped
design. Q2-A adds selection inside existing conversation turns, with no separate
extraction loop, scheduler, richer World or embedding engine. Detailed proposal:
[RESIDENT_DESIGN.md](RESIDENT_DESIGN.md), [CHAT_DESIGN.md](CHAT_DESIGN.md).

## Implemented Q2-B1 private source lookup

An independent SourceGrant defaults off. Administrative decisions select one
existing private binding and a half-open UTC Unix-millisecond **Kernel receive**
window, `[from_received_at_ms, to_received_at_ms)`, at most 31 days, ending no later
than the grant decision. Each revision captures the committed Incoming sequence
high-water, optional expiry and CAS revision. Replacing/revoking invalidates old
cursors and admitted turns. A grant cannot collect future/other sources, write or
restore memory, approve Work/Worker permission or enable proactive contact.

Search uses literal Unicode-lowercased substring matching over authorized incoming
text, not semantic relevance or outgoing-text search. Results order by receive
time, then committed sequence for ties; no-match and all coverage are `partial_no_backfill`.

| Bound | Value |
| --- | --- |
| Admin search / Actor search | 1–10 / 1–3 records |
| Candidate scan per page | 512; explicit scan limit and continuation |
| Encoded records per page / model evidence | 8 KiB |
| Incoming excerpt / generated or outgoing excerpt | 1,024 / 512 UTF-8 bytes, also bounded after JSON escaping |
| Query | 256 UTF-8 bytes, no empty text or control characters |
| Grant history / read audit page | latest 100 / 50 decisions or reads |

A cursor binds channel, grant revision, query/window digest, incoming high-water
and last examined (receive time, sequence) pair. It advances only over examined
rows, including filtered rows. Inspect checks the same scope and suppressions for one actual
IncomingId; guessing a UUID does not broaden access. Full source values are checked
for recognizable credentials before excerpting. The admission API also applies
configured credential filtering. Unknown/encoded secrets are not reliably detectable.

Records retain input/batch/conversation/ActorTurn/delivery IDs, receive time,
processing and generation state, model-request attempts and actual delivery state.
Saved, processed, generated, platform-confirmed and Unknown are different. A model
request attempt or prepared-disclosure audit never proves comprehension or HTTP
receipt. Associated reply text is withheld when merged sibling inputs exceed
scope, generation lies outside the window, inspected memory changed/deleted, or
the turn itself used source tools. Metadata records the withholding reason.

Correct/forget/disable source suppressions from Q2-A also exclude these raw inputs
from this lookup, including administrative access through these new endpoints.
Existing authenticated raw channel inspection retains its own previous boundary;
forgetting semantic memory does not erase operational chat, audits or backups.

The Actor uses two typed readonly tools. Persisted receipts contain IDs only.
Every selection influencing a turn is freshly projected before each request and
after responses. Changed authorization, content or suppression ends that turn with
a static scope-change notice and no further call. Source-influenced streaming text
is held; actual paid/used tokens are still accounted when a response is discarded.
Past source-reading assistant/action observations are omitted from later model
context, while original user messages and stored audit snapshots stay intact.
Already transmitted model requests cannot be recalled. No new extractor, polling
loop, background model job, general grant service or memory store is introduced.

Migration 010 adds only grant/read ledgers and a receipt-window index. Existing
snapshots and confirmation fields remain untouched; no reads are backfilled.
[ADR 011](adr/011-bounded-private-source-lookup.md) records the decision;
[Q2-B1 acceptance](Q2_B1_ACCEPTANCE.md) separates fixtures from live experience.

## Implemented Q2-A private memory

An authenticated administrator explicitly grants one existing private ChannelId
permission to select Episodic/Preference memory. Default revision 0 is disabled.
Enabling, changing or revoking appends a revision with optional expiry and a source
sequence watermark; stale revisions conflict. Each new revision excludes earlier
inputs from new autonomous selection, including already queued messages. It does
not delete existing selected memory. Expiry/revocation stops both private recall
and new effects; an in-flight provider request cannot be recalled. Re-granting
permits recall of retained memories but does not resume stale turns or backfill.

The Kernel checks actual binding/conversation/batch, captured/current grant
revision, deadline, reply permission and Running turn again under the storage
lock. Memory grants never authorize Worker actions, other tool mutations, persona
changes, contact or other social sources. There is no grant tool in the Actor.
Generic Actor memory tools cannot select Channel scope or inspect it by guessing
an ID. Authenticated management memory APIs/CLI can inspect it explicitly.

`memory.remember`, `memory.correct` and `memory.forget` require an IncomingId and
an exact bounded quotation from this turn's real current input. The Kernel assigns
scope and provenance; models cannot supply authorization or explicit authority.
ChannelSelection includes actual conversation/input/batch/ActorTurn/action IDs,
quote (at most 1,024 UTF-8 bytes), record time, grant revision and optional expiry.
Its authority is Inferred and `owner_confirmed_at_ms` is null. This is selected
interpretation supported by a source, not a verified fact or individual approval.
Schema 009 leaves old memory snapshots and actual numeric confirmation timestamps
unchanged, and creates no semantic memory from old conversations.

Each existing model request receives fresh attributed data: at most three relevant
Active records and 4,096 encoded bytes of records, plus at most 4,096 encoded bytes
of current-source excerpts (each at most 1,024 bytes). Selection must fit one
bounded recall record. The deterministic lookup uses at most eight literal words
or non-ASCII bigrams and at most three candidates per term from existing SQLite
search, then ranks those candidates by term overlap and recency. It is
not semantic/vector search or guaranteed natural-language understanding. The
`memory.recall` tool can choose better short keywords within the same Actor budget;
its receipt contains IDs and the next request gets freshly read records.

Correct/forget requires a single Active same-binding target previously returned
to this turn, with its inspected revision. Storage allows at most one committed
correction/forget per turn, preventing bulk edits split over multiple tool calls.
Correction creates a new MemoryId and
marks the old record Superseded with the replacement link. Forget removes the
semantic row/FTS projection. Both suppress the original incoming source; later
actions cannot re-select that old source. Suppression is intentionally at whole
message granularity. Explicit later new messages can support a new selection.
Ambiguous targets and bulk requests must be clarified; there is no bulk tool.

Effects, relations, suppression and content-free receipts commit together. One
ActionId yields one effect. Restart retains receipts/tombstones and does not replay
uncertain actions. Administrative disable/supersede/delete also suppresses original
ChannelSelection sources. A fresh recall excludes inactive/deleted/superseded
records, with no persistent semantic-context cache. Inspection stores only IDs and
revisions. Independently retained dialogue, action arguments, prior superseded
versions, audit and backups can still contain old text: forgetting is not a purge
or secure erasure of those stores, or erasure of an earlier provider request.

Administration and manual acceptance: [CLI.md](CLI.md),
[Q2_A_ACCEPTANCE.md](Q2_A_ACCEPTANCE.md). Decision and narrow supersession:
[ADR 010](adr/010-scoped-private-conversation-memory.md). Full Q2 observation,
archives, consolidation and derived-profile invalidation remain future work.

## Earlier foundation and Phase 7.1 behavior

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
