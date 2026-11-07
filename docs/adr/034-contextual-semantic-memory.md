# ADR 034: Explicit context and semantic-memory lineage

Date: 2026-11-04. Status: accepted; deterministic core implemented/verified,
complete stage acceptance remains in progress.

## Problem

An authenticated owner transport is not sufficient evidence that a conversational
entry may disclose any uploaded device or project record. Host filesystem paths
also cannot identify projects across Windows devices and a Linux Kernel. A fresh
retrieval check alone leaves old tool results, generated replies and accepted
derived memories able to repeat a withdrawn source.

## Decision

Keep Global, Internal, Channel and legacy daemon-local Workspace scopes. Add
Project(ProjectId), Device(DeviceId) and DeviceWorkspace(DeviceId, WorkspaceId).
Only explicit registry mappings establish shared project identity. There is no
cross-host canonicalization, path inference or automatic legacy promotion.

An owner may select a versioned conversation memory context. It identifies an
active device, one admitted workspace and optionally an explicitly mapped active
project, with exact reviewed device/project/mapping revisions. A null selection
is an explicit clearing tombstone. Exact request retries recover the original
decision and cannot restore old authority. Context cannot change while a turn is
Running or PendingConfirmation; revocation/expiry still takes immediate effect
at the next guarded boundary. Channel-owned conversations cannot select PC
context. This is a memory-disclosure context, never a Worker execution grant.

Each new Actor turn captures the trusted selection in Kernel storage, separately
from provider protocol objects and model arguments. Generic model reads check
the captured context and persist the returned MemoryId/revision/scope before
disclosure. Project, Device and DeviceWorkspace are inaccessible without the
matching context. Channel recall keeps its separate existing grant/source path.
Owner administrative inspection remains distinct from model disclosure.

Record origin and visibility separately. New records/candidates have Kernel-set
origin; old JSON may omit it and is not rewritten or assigned an invented
physical device. An OwnerApi origin does not become DeviceA merely because the
owner chose DeviceA visibility. An OwnerSelectedActorContext names the reviewed
context and deliberately does not attest which physical PC produced a statement.
Existing source, authority, actual confirmation time and correction metadata
retain their independent meanings.
Reviewed channel Actor candidates derive Channel origin from the actual persisted
Actor turn; ordinary owner entry is not used as a substitute for channel origin.

Before model input, retained historical turns must have the same captured context
and current memory versions. Their dependencies propagate to the new turn. Stale
or differently scoped messages/actions are excluded from model input; stored
owner audit history remains separate. Inference admission and publication check
the same durable dependencies. Source-influenced previews are held until the
guarded result commits. Stale results cannot be approved or used to make another
memory; cleanup, denial and interruption remain possible without replay. Decline
and pending-cancellation receipts are operational Tool messages, not generated
Assistant speech; rejecting a stale proposal does not need to disclose its source.

Derived memories retain the source Actor's dependency lineage. Corrections,
disable/delete, device/project revocation and mapping changes make affected
derived reads unavailable. Candidate review never changes an inferred claim into
an explicit statement or a cross-scope disclosure grant. Contextual proposals
cannot promote device/project content into Global/Internal/legacy Workspace;
non-global retrieved memory must retain its exact scope when used in a proposal.
This deliberately conservative rule can be extended only with a separate explicit
disclosure contract, not a prompt or ordinary boolean approval.

## Consequences and limits

### Private evidence and publication (schema031)

Channel origin and channel visibility remain independent. A channel-bound Actor
cannot reuse an ordinary entry's history, nor can an ordinary entry acquire the
channel's history by guessing its ConversationId. Selected private memory,
archives, incoming sources, experiences, media and source-bearing share receipts
make the turn privately influenced. That influence is monotonic: refreshing input
does not enable a generic memory proposal that promotes the private material.

Before disclosure, store metadata-only leases for each private snapshot used in
the current model input, cumulatively rather than replacing the last recall.
Retained history propagates leases. Each original batch keeps exact per-row
state/snapshot hashes; a remaining batch row cannot hide partial deletion,
correction or suppression. Source grants, revisions, retention/freshness and
source-specific disclosure receipts retain their existing checks. Bound inherited
and direct private snapshots to 32 each, private metadata to 96 KiB and the full
Actor context to 128 KiB; original batches have at most 512 fingerprinted rows.
When history would exceed a bound, omit the entire historical turn.

Only replacement of the complete model input may discard its old private leases.
It permanently retires excluded message/action IDs from normal publication and
later model history; immutable raw audit snapshots remain stored. Retirement is
bounded by the existing 32-message/four-action Actor envelope. A simple recall or
source-vector refresh cannot perform this operation. Correct/forget effects
rebuild the model input and preserve only the exact committed metadata receipts;
they never replay the effect or relabel stale provider text as a receipt.

Final Running-to-Completed publication rechecks dependencies even when the body
was saved by an earlier transaction. HTTP turn/history/duplicate-submit/decision/
cancel/reading responses use a checked ActorSnapshot projection, while internal
ActorTurn records preserve audit and recovery evidence. An unavailable projection
keeps the actual identity, state, revision, usage and timing, withholds messages,
actions and private contexts, and carries a fixed SourceUnavailable indicator.
Only exact already-persisted MemoryEffect metadata may remain visible. A withheld
PendingConfirmation turn is still pending and may be cancelled; it cannot expose
or approve the unavailable proposal. Private read-tool arguments/results are
never normal frontend history, even when an input rebuild has retired the leases.

SSE queues bounded wakeups rather than serialized turn bodies and re-reads the
projection when the HTTP stream consumes them. Private previews remain held and
queued previews recheck current disclosure conditions. QQ conversation delivery
also validates the full Actor lineage at creation, claim and immediately before
transport; fixed operational failure/approval notices retain their separate
content-free path. A closed source-change notice constructor accepts no caller
message text and does not erase invalidated lineage.

Schema031 gates this internal serialization format without rewriting legacy rows.
Missing old lineage remains unknown, never an inferred grant. These Kernel and
wire protections do not yet implement live client-cache invalidation: in-memory
terminal transcript refresh, startup resources and change subscriptions remain
the next delivery increment, with native two-machine and deployment acceptance
still separate.

The continuous Lain identity remains on the Kernel. Different context selections
do not create independent personalities or merge databases. The conversation can
continue after explicit selection changes, with restricted history omitted.

Schema030 adds selection/receipt and per-turn context/dependency tables. It does
not rewrite legacy memory, infer mappings, collect files, call providers, change
channel grants or migrate personal data. Current retrieval stays deterministic
and bounded: at most 64 direct source versions per turn, 48 inherited historical
versions with whole replies omitted when necessary, and a bounded deduplicated
derived-source traversal (depth64/256 verified versions). Legacy generated
memories without tracked lineage remain owner-inspectable but are excluded from
model retrieval. Legacy generated replies/actions are not reused as current
evidence; original owner user messages keep their separate meaning. Startup
snapshots, client context selection UI, cache revocation,
real HTTPS device acceptance and database rollback require their own stage
evidence. An implementation test does not establish deployment or daily use.
