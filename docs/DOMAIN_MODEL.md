# Domain Model

## Selected-memory reply references — local candidate, 2026-11-07

The private memory_answer view selects ephemeral finite short read references
bound to exact action/memory/revision tuples from this ordinary ActorTurn's
successful current-conversation active selected-memory reads, or an actually
empty query page. Raw save/provenance IDs are not read choices. It is not a new
persisted entity, Tool, grant, memory mutation or source inspection. Quoted saved
text and typed origin remain distinct from independent truth; query coverage and
optional deadline/capture facts come from the read, never model-added facts.
Mixed work, other origins/scopes and management-only reads remain outside this
bounded view. No schema or wire DTO changes. See MEMORY.md.

## Same-record citation context — local candidate, 2026-11-07

source_context is optional transient report-citation projection, not another
archive, selected quote, original-inspection receipt, grant or MemoryDraft.
Available text is an exact bounded paragraph span around a unique full checked
quotation in the same current cited record, with before/after omissions; unavailable
status has no text. All source/origin/version/dependency and confidentiality gates
remain. Source context is untrusted data and not proof other paragraphs lack
conditions. Original quoted fields/query/selected-memory construction remain.
No migration; older JSON result readers can ignore the added projection field.

## Direct original-view request — local candidate, 2026-11-07

A conservative recognized Chinese direct phrase in the latest current User
message constrains only the existing source-only final evidence selection. It is
not a new persisted intent, grant, source, Worker control or arbitrary language
classifier. Quoted/source/tool/assistant/older messages do not grant this request.
After actual source reads, report-only completion is rejected; checked original
or a scoped actual empty lookup is required. Empty lookup remains non-absence
evidence. Catalog reference validation stays separate from whole-reply intent;
both final representations still enforce the same publication path. No schema.

## Selected source wording — local candidate, 2026-11-07

An optional complete quote section belongs to the existing reviewed MemoryDraft
string, not a new memory/source entity. It contains only actual selected item/group
citations, including typed origin and each citation's clock basis. Uncited batch
inputs remain provenance dependencies, not automatically remembered text. The
unverified selection authority/conversation visibility/finite expiry stay.

Candidate validation reconstructs the exact original full draft under current
sources and unchanged provenance; it may also recognize the exact optional quote
addition when the complete body fits4096bytes. No generic suffix stripping, partial
quote/group acceptance or mutation of existing candidates/records is permitted.
This retains known old-format review/restore compatibility without new wire fields
or a schema change. User correction remains separately attributed, not source text.

## Briefing presentation detail — local, 2026-11-07

The optional detailed flag in the private answer envelope selects presentation
of already-read report notes, not a domain state, new source, permission or memory.
The private briefing_answer function is an alternate final-output representation
of that envelope, not a persisted/executable Tool or an operation receipt.
Canonical item references remain unchanged when an identical checked quotation
is rendered once with another selected item's pointer. Generated title/summary/
uncertainty stay in full receipts and are labelled in the expanded view; concise
source excerpts do not become independently verified facts. Original lookup,
retention/publication dependencies and all effect/review semantics remain.

## Briefing final evidence selection — local candidate, 2026-11-07

The source-only answer envelope is a private application format, not a new domain
entity or provider protocol. It contains manifest/item/citation handles whose
content must already occur in this ActorTurn's Executed briefing read receipts.
It cannot supply facts, prose, author identity, permissions or additional sources.
Rendered Assistant content retains existing manifest-dependent disclosure and
recovery semantics; the original user request and action receipts are unchanged.
Titles remain labelled generated summaries, quotations retain their origin and
excerpt limits, and receipt-fallback clocks cannot define sender-relative dates.
No selected-memory or Worker state transition is introduced. See DAILY_BRIEFING.

## Phrase lookup and reader gate — deployed, 2026-11-06

BriefingRead.query optionally selects one bounded literal phrase within the same
manifest's processed, attributed items and checked quotations. It is a read
filter, not a new source/archive/search entity or semantic-memory fact. Original
item/citation identity, group relationships, ordinary counts and disclosure
dependencies remain. Empty results still depend on the manifest. Schema040 gates
new persisted query arguments; it adds no table or authority. Missing/null accepts
legacy actions. Normal39→40 maintenance is public-accepted on the original file
and identity; no source or sharing authority was added. See ADR041.

## Current default-read freshness and extraction projection — 2026-11-06

Default latest first-page Actor reads compare the existing bounded yesterday
selection with the still-readable historical manifest. A content-free refresh
notice is an ordinary read result, not a new entity, authority or admitted run.
Normal disclosure capture still binds the historical source gates. Explicit IDs
and later pages keep historical semantics; unavailable old bodies remain denied.
Only the separate preparation/coordinator path starts finite processing.

Extraction clock labels are a private projection after typed page/fingerprint
admission, derived by the same formatter as ordinary citations. They do not
modify ArchiveRecord, wire structs, manifest fingerprints or old results. Receipt
fallback remains distinct from sender occurrence and cannot anchor relative days.
Both repairs are deployed on the original39 database. Real-source acceptance is
pending, and actual-model prose remains fallible; see DAILY_BRIEFING_ACCEPTANCE.

## Preparation receipts and fresh day selection — 2026-11-06, deployed repair

Existing Actor Action.result can hold a closed, content-free `briefing_prepared`
receipt while Executing: actual Actor/action/conversation/grant revision and
manifest ID. It binds creator and reuse actions equally. Normal tool completion
replaces it with actual run status; it is not a completion result or source/body
grant. New manifest insertion and action receipt share the existing transaction.
No wire field, table, schema migration or alternate memory/entity is introduced.

Fresh ordinary requests compare a newly computed bounded selection with the
cached immutable manifest. Selected versions, bounded omission/fallback state
and meaningful coverage matter; unrelated arrivals and observation/cursor clocks
do not. Existing request identities stay bound to their first accepted selection.
Unknown runs and historical manifests retain their original meanings. This repair
has native Windows/isolation and Linux deployment evidence; the original39 database
and reader remain. Real-source product acceptance is still pending; see DAILY_BRIEFING.

## Reviewed source configuration — 2026-11-06, local increment

Existing ArchiveSource/ArchiveGrant/KnowledgeSyncGrant remain the owners of source
identity, retained imports and finite synchronization. KnowledgeReaderStatus exposes
only local config/credential availability and its fingerprint, not connection
liveness or secrets. Optional SetKnowledgeSync.expected_reader_revision fences a
reviewed launch configuration; stopping remains independent. It is not data-source
authority. No new persistence table or semantic-memory type is introduced.


## Finite briefing relationships — 2026-11-06, local schema039

BriefingStage distinguishes Extract, Index and Compare(item_number). A run can be
Reconciling before becoming terminal. One briefing_relations row binds each stage
key to its actual Actor, immutable input fingerprint and checked result. Parent
candidates only reference earlier original numbers; a non-Different judgment
requires real quoted evidence from both supplied items. ReportedCorrection also
names the actual correcting item, independent of arrival/number ordering.

Groups preserve original item identities and relationship direction; numbers are
not reassigned after duplicate folding. Semantic matching remains a model
interpretation with finite discovery, never proof of exhaustive understanding.
BriefingMemoryReconciliation is an immutable provenance reference, not another
semantic store or source authorization. Internal activity conversations are
operational contexts of the same Actor, not separate Lain personas.


## Reviewed conversation correction — 2026-11-06, local schema038

ConversationCorrectionEvidence binds one current User MessageId, real conversation,
Actor/action, previous MemoryId/revision and inherited finite expiry. It represents
an explicitly reviewed user correction, independently attributed from the old
source report. MemorySelectionReview carries the actual replacement candidate;
MemorySelectionEffect optionally names its committed replacement ID, never body.
Existing semantic, candidate and effect stores remain their owners. Schema038 is
a JSON-reader compatibility gate with no extra table. Superseding old content,
accepting new content and recording the operation are one transaction.


## Reviewed briefing memory — 2026-11-05, local schema036

Conversation memory scope binds a real cloud ConversationId, independent of
private-channel identities and device/project contexts. BriefingSelection
provenance records the actual item, extraction turn, all batch input versions,
selecting action/turn, finite source authority and original conversation.
Individual review authorizes one fixed unverified episodic selection.
ReviewedMemoryReceipt is content-free historical write evidence projected from
accepted candidates; it is separate from channel MemoryEffect and source-body
availability. It survives interrupted acknowledgements and source withdrawal,
never authorizing replay. [Contract and incomplete scope](adr/038-reviewed-briefing-memory.md).

## Briefing run and foreground disclosure — 2026-11-05

BriefingRun belongs to a conversation/manifest and persists finite progress,
start/end/deadline, omission and terminal reasons. Completed means processed
selected records, never complete upstream history or verified semantic truth.
Schema035 also stores per-turn action/manifest disclosure dependencies without
copying source bodies. Readable report pages retain stable item numbers and exact
original lookup; foreground disclosure is not memory-write authority. Source
changes withdraw derived speech, while a closed operational notice may end the
turn without restoring authority. [Details](DAILY_BRIEFING.md).

## Briefing extraction activity — 2026-11-05

BriefingActivity binds a target conversation/manifest/page to a single internal
Actor turn. Schema034 stores the immutable input fingerprint and optional
validated batch output. BriefingItem IDs survive restart and carry model origin,
category, sender-relative quoted time, uncertainty and exact source-version
citations with original text origin and occurrence/receipt timestamps. A batch
has one inference request, four-record/item bounds and an explicit omission flag;
it is not the user's memory or proof of complete upstream/day coverage. Current
source validity gates both structured items and ordinary Actor snapshots.

## Daily briefing manifest — 2026-11-05

BriefingGrant is an explicit, finite read/model-disclosure decision for one real
ConversationId and selected source/import revisions. It is independent of old
private-channel recall and memory grants. BriefingManifest freezes one request's
yesterday interval, fixed offset, scope clipping, high-water metadata and bounded
record-version dependencies without source bodies. BriefingPage returns complete
texts with a bounded byte/count cursor after current authorization and all-record
version checks. These are evidence, not generated items, semantic memory or proof
of complete upstream collection. Schema033 and [ADR037](adr/037-conversation-bound-daily-briefing.md)
are local implementation; model and memory integration remain in progress.

## Operational sync identity — 2026-11-05

Schema032 introduces a persistent Kernel instance ID, a per-open boot ID and a
transactional invalidation revision. These identify storage/connection generations,
not separate personas or semantic-memory facts. A bounded bootstrap carries
existing project/device/context/work pointers and checked Actor projections.
No source archive or Worker transcript becomes Lain memory through synchronization.
See [CLIENT_SYNC](CLIENT_SYNC.md) and [ADR035](adr/035-client-startup-and-live-invalidation.md).

## Conversation memory context — 2026-11-04

ConversationMemoryContext is an explicit owner selection with a CAS revision,
exact request receipt and optional MemoryContextSelection. The selection binds
device/workspace and optional reviewed project/mapping versions. Each new Actor
turn separately captures that context and bounded source-memory revisions.
Origin names a collection path or owner-selected context independently of the
record's visibility; it does not invent physical device attestation. Missing
legacy origin remains unknown. See [ADR034](adr/034-contextual-semantic-memory.md).

## Stable project identity and assignments — 2026-11-04

ProjectId identifies an explicitly created project independently of display name,
host path and repository origin. Project holds Active/Archived state and a CAS
revision. ProjectWorkspace binds a DeviceId/DeviceWorkspaceId pair to an optional
ProjectId with its own revision, original RequestId and reviewed project/device
versions. Null is a retained unbinding tombstone. Neither an assignment nor its
historical receipt grants Worker execution or semantic-memory disclosure.
See [registry and remaining scope](CLOUD_CONTINUITY.md).

## W4 permission application observations — 2026-11-03

`DeviceApprovalReport` carries an approval ID, exact immutable owner decision and
Received/Applied stage within `ExecutionEvidence`. It is a device observation,
not an owner receipt, grant, verified artifact or user-goal acceptance. Reports
remain bounded, cumulative and monotonic; cloud persistence rejects unrecorded
or changed decisions. Missing stage evidence stays unconfirmed. Terminal cancel
reviews/pending outcomes are disposable client state over the existing command
revision and `cancel_requested`, not new work or persistent Lain memory.

## W4c continuation family — 2026-11-03

DeviceContinuation binds a child command to its root/parent/reviewed revision,
original local session and previous turn. Child execution/turn IDs remain unique;
only that explicit family can reuse the session. Parent continued_by is an
operational head pointer with a revision, not a new task input or memory. Child,
lineage, head change and any confirmed Actor receipt commit together. A family
does not become a cloud-local Supervisor session. [ADR033](adr/033-same-device-session-continuation.md).

## W4b remote conversation origin — 2026-11-03

DeviceWorkOrigin identifies the actual conversation and optional confirmed
Actor/action. DeviceWorkRecord joins that origin to current DeviceCommand truth;
it is not a WorkSession, memory or guaranteed notification history. Origin/link,
command admission and Actor partial receipt commit together. Snapshot pages are
bounded current projections, not authority to replay effects. See [DEVICE_WORK.md](DEVICE_WORK.md).

## W4a display projections — 2026-11-02

The terminal draft, surface selection, review-reading progress and cached rows
are frontend-only state, not Lain memory or new runtime identities.
`DeviceCommandSummary` is an owner-only metadata projection of existing commands;
it carries no original task/output/approval and grants no execution authority.
Details retain the existing command/evidence distinction. See
[terminal contract](TERMINAL_DESIGN.md).

## W3b native operational ownership — 2026-11-02

The device-owned immutable Intent binds the exact command input, manifest/policy
revisions and deterministic Lain execution/session/turn IDs. Launch, submission,
permission response, cancel and stop fences are operational receipts, not Lain
memory, conversation or independent persona. The local Kernel stores actual
Worker metadata; cloud reports remain evidence DTOs rather than cloud-local
Supervisor sessions. A response fence and an owner decision receipt are distinct
from actual backend acknowledgement. See [DEVICE_HOST.md](DEVICE_HOST.md).

## W3a execution evidence and approval receipts — 2026-11-02

ExecutionEvidence is an optional device report with stable execution/session/turn
UUIDs, phase, bounded pending approval and factual result. It is not a cloud
WorkSession. DeviceApprovalDecision stores one immutable exact-scope owner choice
per command/approval, with its own RequestId. It is distinct from both a device
event receipt and actual backend response/application. Device-reported paths are
references, not verified artifact bytes. See [DEVICE_EXECUTION.md](DEVICE_EXECUTION.md).

## W2b command and event records — 2026-11-02

DeviceCommand binds a Lain ID to the original administrator RequestId, device
revision, admitted WorkspaceId, device-local profile, sandbox, task and deadline.
Its revision/state and independent cancel_requested flag are Kernel projections
of admission and device-reported facts. CommandEventReceipt preserves the exact
per-command sequence/RequestId, observation, server receive time and historical
published revision. CommandChange has a separate per-device server sequence and
metadata only. It is a resync hint, never executable authority. Expired/revoked
accepted work becomes Unknown; a late fact does not authorize another start.
See [ADR 028](adr/028-durable-device-commands.md).

## Private conversation media — 2026-10-28

InputSegment preserves text, visual reference, QQ expression and quoted-message
marker order; original Input.text remains the user's text only. MediaId identifies
a current-event segment independently of signed transport URL rotation.
ConversationMedia belongs to IncomingId, ChannelId and reply epoch. Its states
are Pending, Ready, Unavailable, Interrupted, Expired and Withdrawn. MediaFrame
describes normalized pixels and actual sampled animation positions; bytes are
private operational BLOBs. PrivateChatContext separates all supplied media/status
versions from viewed_media (actual pixel influence, including retained history).
Neither is a semantic memory or collection grant. See
[ADR 027](adr/027-private-visual-conversation.md).

## W2a device identity and reported inventory — 2026-10-28

DeviceId and DeviceWorkspaceId belong to Lain. An administrator-admitted immutable
RegisterDevice document enters Pending, then exact credential/manifest proof
makes it Active; explicit revoke is terminal. Expiry is checked independently.
DeviceReport has an independent monotonic sequence/request receipt, device policy
revision, reported time and Kernel receive time. Workspace availability and
capability versions are device claims, never Worker authority or an Online
guarantee. Devices own paths; Kernel stores declarations without opening them.
No remote WorkSession is invented. See [ADR 026](adr/026-explicit-device-enrollment.md).

## W1f source-derived semantic selection — 2026-10-28

ArchiveMemoryGrant is a binding-local finite write policy, independent of archive
storage, private recall and MemoryGrant. ArchiveSelection is inferred provenance
over actual request/action/source epochs, a bounded flattened dependency set and
an expiry. MemoryEffect optionally identifies the archive-write epoch. Suppression
is binding-local; original archives remain separate. Exact-ID unavailable targets
can expose management metadata without their semantic body. See
[ADR 025](adr/025-selected-source-memory.md).

## W1e selected-source network authority — 2026-10-27

KnowledgeSyncGrant retains revisioned finite scope, frozen reader identity, used
requests, next deadline, last read, pending state and stop reason. Scope binds one
archive epoch; ArchiveReadJob.sync_revision attributes scheduled receipts. Import,
private recall and model budgets remain separate. Migration 021 enables nothing.
See [ADR 023](adr/023-finite-knowledge-synchronization.md).


## W1d independent-source projections — 2026-10-27

ArchiveSourceKey adds QqPrivate. ReadArchive (the ReadWorkerArchive compatibility
alias remains) carries a reader kind/revision in its durable ArchiveReadJob;
update/withdrawal counts distinguish changes from new records. KnowledgeArchiveState
records independent store/source/local-grant identity, committed receipt/cursor and
local observed/fresh-until times. Native event identities/tombstones stay separate
from archive text, and KnowledgeProjection separates observed original excerpts
from external-model image interpretations. See [ADR 022](adr/022-scoped-knowledge-archive-pulls.md).

## W1c archive recall — 2026-10-27

ArchiveRecallGrant belongs to one ChannelId independently of archive storage and
reply/semantic-memory authority. Scope selects source/import revisions, finite
expiry and explicit UTC offset. ArchiveRecallQuery bounds source/time/literal
topic/count; PrivateChatContext keeps recall revision and ArchiveRecallVersion
record/source epochs. Audits contain IDs/versions/operation time only. Observed
reports retain original occurrence claims, receipt time, origin and excerpt bounds.
See [ADR 021](adr/021-scoped-archive-conversation-recall.md).

## W1b explicit source-read receipts — 2026-10-27

ReadWorkerArchive binds an immutable administrator request to ArchiveSourceId,
grant revision, bounded limit and optional preceding Lain receipt. ArchiveReadJob
records Prepared/Reading/Completed/Failed/Cancelled/Unknown, times, record IDs/
counts, omissions, waiting position and safe failure/code. Native cursors stay
private. AdapterObservation origin distinguishes reads from owner imports and
corrections; no WorkSession or ActorTurn is implied. Migration 018/contracts:
[ADR 020](adr/020-explicit-visible-worker-archive-reads.md).

## W1a visible archives — 2026-10-26

ArchiveSourceId/ArchiveRecordId identify declared QQ-group/Codex-session sources
and imported visible events. ArchiveGrant bounds import/disclosure; ArchiveRecord
separates occurrence/position claims from Kernel receipt UTC/commit sequence.
OwnerImport/OwnerCorrection and incomplete coverage preserve provenance without
fabricating participation/control. These are not private SourceRecord or managed
WorkSession IDs. See [ADR 018](adr/018-selected-visible-source-archives.md).

## D3b inference records — 2026-10-26

InferenceLimit is an immutable ID/ceiling/UTC-expiry specification. Revisioned
InferenceDecision can enable or stop it while retaining its totals. InferenceRequest
records one Actor/request number, typed conversation/private/reading/sharing reason
and Prepared/Attempting/Returned/NotAttempted/Unknown phase, without content or
credentials. It is not semantic memory, action permission or proof of billing.
Schema 016 and boundaries: [ADR 017](adr/017-kernel-wide-inference-budget.md).

## D3a receipt projections — 2026-10-26

ShareQuery bounds UTC claim time/topic/record count. Private ShareVersion binds a
DeliveryId and receipt fingerprint, with an ExperienceVersion only when source
content was disclosed. Automatic recent pointers contain no source words. A
withheld projection does not erase the historical delivery outcome. Original
outbound text is not replaced by a corrected summary. Contract and migration 015:
[ADR 016](adr/016-sharing-receipt-continuity.md).

## D2 sharing records — 2026-10-26

ShareGrant is independent contact authority; Attention is an explicit revisioned
availability restriction. ShareCandidate binds an actual experience revision and
fingerprint, source age, input watermark and expiry to a Share/Skip draft.
ActorTurn.sharing_activity denotes a Kernel activity with no tools/private scope,
mutually exclusive with reading_activity. ShareDelivery records a single attempted
contact and its true transport outcome, not a reply Batch or memory. Migration 014
and lifecycle: [ADR 015](adr/015-bounded-proactive-sharing.md).

## D1b concrete reading experiences — 2026-10-26

`ReadingModelScope` is an explicit optional reading-grant component selecting one
private ChannelId, an aggregate processing-request ceiling and a fixed UTC offset.
`ActorTurn.reading_activity` identifies a Kernel-originated reading task, mutually
exclusive with private contexts/tool actions. Its original request contains no
source-body copy. `ReadingExperience` links source/processing receipts, actual
input/truncation, acquisition/completion times and a bounded summary. Summary origin
is Model or OwnerCorrection; neither means independently verified. Active/Withdrawn/
Expired/Revoked are projection availability, separate from historical operation
truth. `ExperienceVersion` binds private disclosure to actual revision/fingerprint.
See [ADR 014](adr/014-source-linked-reading-experiences.md).

## D1a concrete acquisition records — 2026-10-26

`ReadingGrant` is an independent revisioned administrator decision with optional
`ReadingScope`; it is default-off and selects exact URLs, expiry, retention and
attempt/body caps. `StartReading` binds an immutable RequestId, URL and grant
revision. `ReadingActivity` records admission/start/finish UTC milliseconds,
terminal uncertainty/cancellation/failure and actual acquired bytes. `ReadingResult`
is a separate expiring untrusted text body, not memory, summary or experience.
`ReadingProcessing::AcquiredTextOnly` states actual past acquisition even after
the body becomes unavailable; it never denotes comprehension/verification.
States and invalidation: [ADR 013](adr/013-bounded-text-acquisition.md).

**Status:** proposed resident-domain extensions plus existing concrete domain contracts

**Last reviewed:** 2026-10-24

## Q2-B1 private source lookup — 2026-10-25

Q2-B1 implements `SourceGrant`, `SourceReadContext`, `SourceQuery`/`SourceCursor`, `SourceRecord` and `SourceAudit` in `lain-wire::source`. SourceRecord projects existing private Incoming/Batch/ActorTurn/Delivery records without copying their bodies. A grant selects one binding, revision, fixed half-open received-UTC window, incoming high-water mark and optional expiry. SourceAudit stores IDs/scope/operation and preparation attempts, never comprehension or original text. These are separate from MemoryGrant, MemoryEffect, semantic memory and Work authorization. See [ADR 011](adr/011-bounded-private-source-lookup.md).

## Current direction and vocabulary

The user and Lain interact as equals. User, account identity and authorizer are
technical concepts, not a master/servant relationship. Historical owner wording
must not become persona language. See [PRODUCT.md](PRODUCT.md) and
[ADR 009](adr/009-cloud-resident-equal-companion.md).

The following are **target concepts**, not a list of exact Rust types or migration
names. The narrow Q1 implementation below realizes only part of binding/delivery
and reply authorization:

| Concept | Meaning | Distinct from |
| --- | --- | --- |
| UserIdentity / ChannelBinding | Verified link from one person to channel accounts | Nickname matching or a role in persona |
| SourceRecord | Authorized social/visible Worker record with source/time/gaps | Selected Memory or an instruction |
| SourceAsset | Attachment with origin and retention rules | Unbounded copy of the local filesystem |
| Experience | Evidence of something Lain actually read/did/observed | Imported work falsely claimed as participation |
| AttentionItem | A bounded unresolved concern or potential share | A standing authorization or guaranteed task |
| Activity | A reasoned reading/organizing/sharing job with budget/outcome | Worker Turn or generic agent planning loop |
| StandingGrant | Revocable authorization over sources/actions/targets/resources | Model assertion that permission exists |
| DeliveryIntent | A selected outgoing message with expiry and receipt | ConversationNotice display acknowledgement |
| Device | Paired execution/observation endpoint and native capabilities | An independent Lain identity |
| ExternalSessionArchive | Readable native history with source and completeness | Automatically controllable WorkSession |
| WorkRequest | User goal and agreed follow-up boundary | A single session or one provider Turn |
| Skill / TemplateRevision | Reusable method/resource and applicable version | Every remembered preference |

Current Conversation, ActorTurn, WorkSession, Turn, Approval, ConversationNotice,
MemoryCandidate and SessionSummary remain separate and retain existing meanings.
Do not rename or change their wire/schema contracts just because these target
concepts are described. Implementation may use fewer types if distinctions remain explicit.

### Cross-source identity and evidence

Source events need stable identities scoped by connector/account/session; platform
message IDs cannot be assumed globally unique. Occurrence time, receipt time and
source order are distinct. Observation of an external session does not create
control rights or prove current runtime status.

Legacy MemoryProvenance and user confirmation fields retain actual Phase 7.1
semantics. Q2-A introduces ChannelSelection under an explicit private-binding grant
and schema 009; it uses Inferred authority and a null individual confirmation
timestamp. Temporary impressions retain evidence,
validity and uncertainty; correction must affect retrieval and derived records.

See [RESIDENT_DESIGN.md](RESIDENT_DESIGN.md) for lifecycles and ownership.

### Concrete Q2-A records and Q1 observations

`MemoryGrant` is an administrator decision for one ChannelId: enabled, revision,
optional expiry, change time and source sequence watermark. Each change appends
a revision; default revision 0 is disabled. ActorTurn optionally captures a
`ChannelMemoryContext` (binding/batch/grant/deadline), never a new authorization.
`MemorySource::ChannelSelection` records the real conversation/input/batch/turn/action,
bounded quote and grant revision/expiry. `MemoryEffect` is a content-free,
action-idempotent receipt for Remember, Correct or Forget. A new source is
model interpretation even when its quoted source was explicit user text.

`ConnectionStatus` is ephemeral: NotStarted, Connecting, Authenticating, Online,
Reconnecting or Stopped, with a closed optional failure category and change time.
It is independent of binding persistence, reply authorization and Kernel Ready.
See [MEMORY.md](MEMORY.md) and [QQ_CHANNEL.md](QQ_CHANNEL.md).

### Concrete Q1 channel records

Schema 008 adds `Binding`, `Incoming`, `Batch` and `Delivery` in `channels::domain`.
Their UUIDs belong to Lain. Opaque account/peer/external IDs are adapter-scoped
references, not QQ structs. A binding maps an immutable route to one durable
`ConversationId` and a revisioned reply grant. Revocation/re-grant invalidates work
from an older revision without editing conversation history.

An Incoming record is queued, batched, revoked or expired. A Batch owns the immutable
text actually submitted and its stable Actor request UUID; prepared/running records
recover the same request. Finished batches may produce one Delivery. Deliveries move
from ready to sending, then confirmed/not_sent/rejected/unknown, or are revoked/expired
before dispatch. A confirmed receipt proves platform acknowledgement, not human reading.
These records are operational conversation history, never automatically semantic memory.
See [QQ_CHANNEL.md](QQ_CHANNEL.md) for bounds and [Q1_IMPLEMENTATION.md](Q1_IMPLEMENTATION.md)
for the implementation/real-acceptance distinction.

## Historical model and concrete implemented semantics

## 1. Purpose

This document defines the vocabulary of Lain's core domain.

The goal is to prevent backend terminology from leaking into the system and to avoid multiple overlapping concepts such as task, job, thread, conversation, run, and session meaning different things in different modules.

The names in this document should be preferred in core code, API design, storage, and documentation.

## 2. Top-level model

~~~text
Lain
 |
 +-- Conversation
 |
 +-- Memory
 |
 +-- World
 |
 +-- Schedule
 |
 +-- WorkSession
       |
       +-- WorkerBackend
       +-- RuntimeProfile
       +-- Workspace
       +-- Turn
       |    `-- Events
       |
       +-- Approval
       +-- Artifact
       `-- Summary
~~~

## 3. LainActor

LainActor represents Lain as a resident personal agent.

It owns no worker runtime itself.

Responsibilities:

- converse with the owner;
- interpret intent;
- delegate work;
- inspect active work;
- propose memory updates;
- produce reflections/summaries;
- use bounded capabilities exposed by the Kernel.

The LainActor MUST NOT directly hold raw credentials.

## 4. Conversation

A Conversation is a user-facing conversational context with Lain.

It is not the same as a worker session.

A conversation may:

- remain pure chat;
- create one or more WorkSessions;
- discuss existing WorkSessions;
- survive across multiple frontend connections.

Do not model a worker session as merely a nested chat message.

### C1 private conversation decisions

`PrivateChatContext` belongs to an ActorTurn admitted by the existing reply grant.
It references the actual ChannelId/BatchId, original expiry/revision, admitted input
sequence, bounded wait/reconsideration counters, decision and optional deadline.
Only the latest request's selected memory IDs/revisions and at most four bounded
historical source selections/fingerprints are retained, not source/memory bodies.
Legacy snapshots default to no private-chat context. These fields grant
no additional source, memory or Worker permission.

Supplements preserve Incoming IDs, text and times. Their batch association and new
Actor user messages commit atomically; the root request and Batch.text do not
change. Reply/Waiting/NoReply/Superseded/Notice/Interrupted describe conversational
decisions, never human receipt or completed Work. Delivery adds `superseded` for
obsolete unsent ordinary drafts and an explicit purpose (conversation/approval/
failure) so a new social input cannot discard required notices. Already Sending,
Confirmed and Unknown retain their transport evidence. See [ADR 012](adr/012-bounded-private-conversation.md).

## 5. WorkerBackend

A WorkerBackend identifies a family of external agent runtimes.

Examples:

~~~text
codex
claude-code
deepseek-harness
~~~

A backend describes how to communicate with a type of worker, not a running worker process and not a provider configuration.

Do not encode a specific API provider into BackendKind.

## 6. RuntimeProfile

A RuntimeProfile is a named configuration used when starting or resuming a worker runtime.

Examples:

~~~text
codex/official
codex/local
claude/official
claude/gateway-a
deepseek/official
~~~

A profile may contain:

- backend kind;
- model/provider;
- endpoint;
- reasoning/effort settings where supported;
- environment variables;
- sandbox defaults;
- approval defaults;
- secret references;
- backend-specific extension fields.

A RuntimeProfile is configuration. It is not a running process and is not a WorkSession.

## 7. WorkerRuntime

A WorkerRuntime is a live runtime/process/connection capable of hosting or controlling one or more WorkSessions.

Depending on the backend, one runtime may manage exactly one session, several sessions, or a durable remote session independent of the local process.

Examples include a Codex App Server process, an ACP adapter process, a Claude Agent SDK sidecar, or a DeepSeek Harness process.

Important invariant:

~~~text
WorkSession != WorkerRuntime
~~~

A process can die while a provider-native session remains resumable.

## 8. WorkSession

A WorkSession is Lain's normalized representation of one durable unit of delegated worker work.

It has a Lain-owned ID and may also store a provider-native session ID.

Provider IDs are adapter metadata and must not become the public identity of the WorkSession.

A WorkSession should minimally carry:

- Lain session ID;
- backend kind;
- runtime profile ID;
- workspace reference;
- provider session ID if available;
- normalized state;
- created/updated timestamps.

## 9. WorkSessionState

Core states should be intentionally small and provider-neutral.

Initial candidate:

~~~text
Created
Starting
Ready
Running
WaitingForApproval
Idle
Completed
Cancelled
Failed
Disconnected
Unknown
Archived
~~~

Important rules:

- Disconnected means the control connection is unavailable, not necessarily that provider work stopped.
- Unknown means the Kernel cannot currently prove the actual backend state.
- Completed should represent a terminal work state according to the backend/session contract.
- frontend disconnect MUST NOT change WorkSessionState.

Backend-specific sub-state belongs in metadata/debug fields.

## 10. Turn

A Turn is one unit of instruction/execution inside a WorkSession.

~~~text
user/Lain prompt
      |
      v
Turn
      |
      +-- worker output
      +-- tool activity
      +-- approvals
      `-- completion/failure
~~~

A follow-up prompt usually creates another Turn in the same WorkSession.

Do not equate Turn with process execution or the whole session.

Different backends may not expose a perfect turn model. The adapter may synthesize one when meaningful.

## 11. Event

An Event is a normalized observation that something happened.

Events should carry:

- a Lain event ID;
- timestamp;
- WorkSession ID;
- optional Turn ID;
- event kind;
- normalized payload;
- optional provider metadata/provenance.

Candidate event kinds:

~~~text
SessionStateChanged
TurnStarted
TurnCompleted
AssistantMessageDelta
AssistantMessageCompleted
ToolStarted
ToolUpdated
ToolCompleted
CommandStarted
CommandOutput
CommandCompleted
FileChanged
ApprovalRequested
ApprovalResolved
UsageUpdated
BackendDisconnected
BackendReconnected
ErrorObserved
ArtifactProduced
~~~

Events are operational truth, not memory.

## 12. Approval

An Approval represents an explicit decision requested by a worker runtime.

Examples:

- execute a command;
- write outside an allowed area;
- access network;
- use a sensitive tool.

An approval should preserve:

- approval ID;
- session/turn relation;
- approval kind;
- human-readable description;
- supported decisions;
- optional expiry.

Lain may explain an approval request, but the Kernel must preserve the original structured decision.

Never reduce approval to typing yes/no into a terminal.

## 13. Workspace

A Workspace is a filesystem/project scope in which a worker operates.

Examples:

~~~text
/home/you/codebase/example
/home/you/projects/lain
~~~

A WorkspaceRef should be normalized and validated server-side.

Workspace is different from Lain's private World.

~~~text
Workspace != World
~~~

Workers may receive access to a Workspace without receiving access to Lain's World.

Future workspace policy may include allowed backends, concurrency limits, read-only mode, branch/worktree isolation, and remote workspace mapping.

## 14. Artifact

An Artifact is a durable output worth referencing independently of event history.

Examples:

- generated report;
- patch/diff;
- test result bundle;
- image;
- exported file;
- worker-produced summary.

Artifacts have provenance: which session, which turn, which backend, where stored, and what content type.

An Artifact is not automatically Memory.

## 15. SessionSummary

A SessionSummary is a normalized, durable synopsis of a WorkSession.

It may include:

- objective;
- important actions;
- files changed;
- tests/results;
- unresolved issues;
- decisions;
- candidate knowledge worth remembering.

A summary is an intermediate knowledge product. It does not automatically become Lain memory.

## 16. Memory

A Memory is information intentionally retained as part of Lain's continuity.

Important distinction:

~~~text
EventLog        = what happened operationally
SessionSummary  = compressed understanding of worker work
Memory          = what Lain chooses or was instructed to retain
WorldFile       = an artifact in Lain's private space
~~~

These may reference each other but are not interchangeable.

## 17. Schedule

A Schedule describes a persistent future trigger.

Examples:

- daily reflection at a configured local time;
- run consolidation every week;
- user-created task;
- delayed follow-up.

A Schedule produces a scheduled job/trigger. It does not itself contain LLM logic.

## 18. WakeTrigger

A WakeTrigger is an event that may cause LainActor inference.

Candidate sources:

~~~text
UserMessage
WorkerMilestone
WorkerCompleted
ApprovalNeedsExplanation
ScheduledReflection
MemoryMaintenance
ExplicitAutomation
~~~

Most WorkSession events should not be WakeTriggers.

A wake policy decides whether cognition is justified.

## 19. SecretRef

A SecretRef is a handle to a credential.

Examples:

~~~text
secret://openai/main
secret://anthropic/gateway-a
secret://deepseek/main
~~~

SecretRef may be stored in ordinary configuration. The resolved secret value may not.

## 20. Identity rules

Lain-owned identifiers should be stable even if provider identities change.

Recommended strong IDs:

- ConversationId
- WorkSessionId
- TurnId
- EventId
- ApprovalId
- ArtifactId
- RuntimeProfileId
- ScheduleId
- MemoryId

Use strong newtypes in Rust rather than passing generic strings where practical.

## 21. Relationship diagram

~~~text
Conversation
    |
    | may create/reference
    v
WorkSession -------------------- RuntimeProfile
    |                                |
    | uses                           | selects
    v                                v
Workspace                       WorkerBackend
    |
    +-- Turn
    |    |
    |    +-- Event*
    |    +-- Approval*
    |    `-- Artifact*
    |
    `-- SessionSummary
             |
             | may produce candidates
             v
           Memory

LainActor
  |-- participates in Conversation
  |-- controls WorkSession through capabilities
  |-- reads/writes Memory through Memory Service
  |-- reads/writes private World through World Service
  `-- receives selected WakeTriggers from Scheduler/Event layer
~~~

## 22. Anti-patterns

Avoid:

~~~text
AgentSession = everything
Task = both schedule and worker session
Message = every possible event
Memory = all persisted text
Backend = API provider
ProcessId = session identity
frontend-local UUID = authoritative session identity
~~~

If a new concept cannot be explained relative to this model, update this document before spreading the new term across code.

## 23. Concrete session model

`WorkSessionId`, `TurnId`, `EventId`, and `ApprovalId` are distinct UUID newtypes
generated by Lain (UUID v4, serialized as UUID strings). `RuntimeProfileId` is a
validated name so names such as `worker/official` remain usable. `BackendKind`
is a validated opaque name, not an API-provider enum. Names are 1–128 ASCII
characters in nonempty slash-separated segments containing letters, digits,
underscore, hyphen, or period. Phase 5 adds a durable profile registry.

A WorkSession stores ID, backend, runtime profile ID, canonical absolute workspace
path, logical state, runtime attachment, and created/updated UTC Unix milliseconds. Creation requires an
existing directory with a UTF-8 path. Recovery validates the record without
requiring that the workspace still exists. Timestamps never move backward on
transitions, even if the host clock does. Provider-native session/turn identities
and selected session options are private operational metadata, never public IDs.
Profiles determine the backend. `codex/local` is the compatibility/default profile,
with explicit selection or a canonical workspace default available in Phase 5.

The candidate state names in section 9 are adopted. Exact allowed transitions:

| From | Allowed next states |
|---|---|
| Created | Starting, Cancelled, Failed |
| Starting | Ready, Cancelled, Failed, Disconnected, Unknown |
| Ready | Running, Idle, Cancelled, Failed, Disconnected, Unknown |
| Running | WaitingForApproval, Idle, Completed, Cancelled, Failed, Disconnected, Unknown |
| WaitingForApproval | Running, Idle, Cancelled, Failed, Disconnected, Unknown |
| Idle | Running, Completed, Cancelled, Failed, Disconnected, Unknown |
| Disconnected | Unknown, Ready, Running, WaitingForApproval, Idle, Completed, Cancelled, Failed |
| Unknown | Disconnected, Ready, Running, WaitingForApproval, Idle, Completed, Cancelled, Failed |
| Completed / Cancelled / Failed | Archived |
| Archived | none |

Same-state and unlisted transitions are rejected without writes/events. Terminal
sessions cannot be reopened. Disconnected/Unknown can return to an observed
state only with supervisor evidence; the transition function does not establish
that evidence. `Cancelled` records confirmed cancellation, not a request to
cancel. `Idle` represents a completed turn in a reusable session; `Completed`
requires a terminal session contract. The current adapter leaves reusable sessions
Idle after successful, failed or cancelled turns; it never treats one completed
turn as WorkSession completion. An approval can be cleared by turn termination,
hence WaitingForApproval can transition directly to Idle.

## 24. Implemented Turn and Approval semantics

`WorkSession != Turn`: a session owns many instruction/execution cycles. Turn
has a Lain TurnId, WorkSessionId, state, cancellation state, timestamps and optional
safe failure category/code. States are Starting, Running, Completed, Cancelled,
Failed and Unknown. Starting is persisted before submission. A transport failure
can leave submission uncertain; Unknown is not a failed or cancelled outcome.
Only backend evidence reconciles it. A pending turn without a provider identity
cannot safely be matched to provider history by guesswork and remains Unknown.

Cancellation records NotRequested, Requested, Rejected, Acknowledged or Confirmed.
Only observed interruption confirms cancellation. Acknowledgement can race normal
completion; Completed plus Acknowledged is valid. Rejected cancellation can be
retried. Disconnect retains request evidence but makes unfinished turns Unknown.

Approval has its own Lain ID, session, optional turn, kind, sanitized description,
available decisions, status, chosen decision and timestamps. Kinds are Command,
FileChange and Permissions. Decisions are ApproveOnce, Deny and (where supported)
Cancel. There is no blanket/session approval or automatic approval policy.
Pending becomes Responding before sending the decision; server resolution makes
it Resolved. Resolution means the request was handled, not that a tool succeeded.
Lost connections expire unanswered requests and mark possibly delivered decisions
Unknown. Connection-local request IDs are not replayed after restart.

Events carry EventId, UTC timestamp, WorkSessionId and optional TurnId. Durable
events additionally have history_order and bounded payload truncation metadata. Lifecycle
and approval events carry normalized domain values. Live observations include
OutputProgress (character counts), AssistantMessageCompleted, CommandStarted,
CommandCompleted, successful FileChanged, ToolActivity and ErrorObserved.
BackendDisconnected and BackendReconnected describe transport ownership. Raw
provider request objects and secret-bearing protocol diagnostics are excluded.

## 25. Runtime attachment and operational history

WorkSession is durable logical work; WorkerRuntime is temporary infrastructure.
Public `runtime` has Detached, Starting, Attached, Stopping and Lost observations,
independent of WorkSessionState. Process IDs are never public identity. Idle plus
Detached is healthy; a prompt/resume uses the same Lain ID and private provider
binding. Planned detach emits runtime changes rather than BackendDisconnected.
Unexpected idle loss can retain Idle plus Lost; active unconfirmed loss is Unknown.

Operational history is an indexed normalized timeline, not a worker transcript,
SessionSummary or Lain semantic memory. Stable EventIds are reused in SSE and
history. `history_order` is a database-wide increasing sequence, so a session's
orders can have gaps. Timestamp is display information, not the ordering clock.
Meaningful state/turn/cancellation/approval changes, completed assistant messages,
commands, successful file changes, tools and safe errors are durable. Unknown turn
and expired/uncertain approval recovery use TurnStateChanged/ApprovalStateChanged;
they do not fabricate completion. SessionCreated includes the initial Lain snapshot.
Only OutputProgress is ephemeral, with no history_order or replay guarantee.

State and associated events commit atomically before publication. Observation-only
events commit their own row before publication. The history remains useful after
worker detach and daemon restart. Retention is indefinite in this milestone with
bounded individual payloads; pruning is future explicit maintenance. See the
[query/bounds contract](DEVELOPMENT.md#11-operational-history).

## 26. Runtime profiles, frozen configuration and failures (Phase 5)

RuntimeProfile combines a ProfileDefinition and monotonic revision. Definition
fields are stable Lain ID, backend, driver, enabled, optional model/provider/base
URL/reasoning effort, sandbox default, approval policy and optional SecretRef.
Unsupported fields are rejected; no worker-native JSON, arbitrary argv/env or raw
credential belongs in the public model. Disabling affects creation only; sessions
retain snapshots, including after an edit. There is no cascade deletion.

ResolvedRuntimeProfile contains profile ID/revision, backend/driver, selected
model/provider/endpoint/effort, sandbox/approval policy and SecretRef. It lives in
private worker metadata, with a credential-free inspection API. Registry revision
and session revision are distinct. Revision 0 identifies migrated Phase 4 local
metadata. SecretRef resolves only `secret://env/<name>` through the restricted
environment store. Profile configuration freeze does not freeze the credential.

WorkerFailure retains safe kind/message/code and optional `retryable`; old records
decode with None. AuthenticationFailed, RateLimited, ProviderUnavailable,
PermissionDenied, SessionNotFound, SessionNotResumable and Unknown supplement the
existing categories. Adapters use structured evidence, never provider error prose.
Unavailable covers runtime transport loss; Protocol and Unsupported remain
distinct. Session-not-found/not-resumable are reserved for reliable evidence, not
guessed from a generic HTTP 404 or missing connection. ErrorObserved is an
observation and never changes a turn to Failed; terminal turn evidence is required.

## Phase 6 concrete conversations and actions

`ConversationId`, `ActorTurnId`, `MessageId`, `ActionId` are distinct Lain-owned
UUID newtypes. Conversation has creation/update timestamps. Indexed ActorTurn
stores a unique conversation/request-id pair, sequence/revision, state, messages,
actions/results, error and model-request/token counters. Message carries ID,
conversation, role/content/timestamp/status. These records are independent of
WorkSession and provider-native threads and are not semantic Memory.

Actor states are Running, PendingConfirmation, Completed, Failed, Cancelled,
Unknown. Action states are Proposed, Executing, Executed, Rejected, Failed,
Unknown. Same request ID/text returns the existing turn; changed text conflicts.
Owner confirmation is an explicit state transition, never inferred from model
prose. Executed means the typed service accepted the operation, not that worker
execution completed. Created WorkSession IDs survive partial delegation failure.
Restart marks unfinished execution Unknown and never replays a mutation. Pending
proposals survive for explicit owner review. See [LAIN_ACTOR.md](LAIN_ACTOR.md).

## Phase 6.6 concrete delegation and notices

Delegation is a Kernel-owned correlation record keyed by confirmed ActionId,
containing ConversationId, ActorTurnId, WorkSessionId, optional worker TurnId,
exact confirmed instruction, canonical workspace, profile ID, submission state
and actual receipt. The session retains frozen resolved profile configuration.
Work.create can be Prepared with no worker Turn while starting its runtime;
allocation makes it Submitting before RPC. Accepted/Rejected/Unknown describe
submission evidence, independently of Actor approval and authoritative worker
execution. Work.send links a later Turn in the same session. No one-session or
one-delegation assumption is made for a conversation.

ConversationNotice is an immutable observation, with typed NoticeId, source/target
IDs, ordered cursor, timestamp, optional source history order, outcome, bounded
ResultEvidence and mutable owner display acknowledgement. Worker outcomes are
Completed, Failed, Cancelled or Unknown; Unknown can later be followed by a
distinct reliable terminal observation. SubmissionUnknown covers a created
session without a reliably submitted Turn. Repeated action/outcome observations
produce one notice. Notices are neither Actor messages/turns nor semantic Memory.
Acknowledgement confirms display metadata only, never work correctness or policy
approval. See [ADR 007](adr/007-durable-delegation-result-delivery.md).

## Phase 7.1 concrete memory and summaries

Memory, Candidate and Summary use distinct Lain UUIDs (`MemoryId`, `CandidateId`,
`SummaryId`). The shared public value types live in `lain-wire::memory` and are
used by Kernel domain services; they contain no backend/transport protocol objects.
MemoryRecord has a typed kind/scope/source/authority/status, revision, selected
bounded content, timestamps, explicit owner confirmation and optional supersession.
ProjectKnowledge requires Workspace; Reflection requires Internal. Explicit,
Inferred and UnverifiedSelection authority are source-bound and cannot be chosen
by the model. Accepting a suggestion never changes its origin into an owner quote
or its worker evidence into independently verified truth.

Actor MemoryCandidates use their confirmed ActionId as a durable review identity
wrapped in CandidateId. Candidate/Accepted/Rejected is independent of
Active/Disabled/Superseded memory status. Direct owner-authored memory bypasses
candidate review because the authenticated command is itself the exact statement.
Source references are Kernel-owned request IDs, Actor conversation/turn/action IDs,
or SummaryIds. No provider thread object is a memory identity. See
[MEMORY.md](MEMORY.md) for creation/revision/retention invariants.

SessionSummary records one WorkSession and worker Turn, outcome, workspace,
creation time, source Turn update time and immutable normalized history cursor.
Its selected report, failed commands/errors and changed paths are observed evidence,
with limits and uncertainty. It is not a ConversationNotice, semantic Memory,
worker correctness certificate or persona update. Creation and inspection need no
model; owner-selected project facts can enter a separately reviewed candidate.
