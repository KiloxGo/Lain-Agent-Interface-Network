# ADR 025: Selected source-derived memory

Date: 2026-10-28. Status: accepted for the W1f implementation slice.

## Context

W1c permits conversation over selected archives, W1d imports independent QQ
projections, and W1e synchronizes those projections under finite grants. None of
these makes archive text semantic memory. Daily continuity needs small selected
interpretations whose sources can still be corrected, revoked or forgotten.

## Decision

Add an independent, default-off `ArchiveMemoryGrant` for one private binding.
Its revisioned scope selects at most eight exact archive source/import epochs,
an expiry of at most 31 days, retention of at most 31 days and 1–1000 writes.
Creation enables no source reading, recall, background inference, contact or
Worker action. A current private MemoryGrant and ArchiveRecallGrant are still
required. Replacing a policy is an explicit new epoch/allowance, never a restart
refill. Migration 022 adds policy history and binding-local source suppression;
existing memory and authorizations are unchanged.

The existing conversational Actor may call `archive_remember` after actual
bounded archive selection. The tool is advertised only when its separate scope
is usable. It accepts only a short episodic interpretation or preference, not
source IDs, authority, scope or arbitrary evidence supplied by the model. At most
one successful archive selection per ActorTurn uses the existing turn/tool/model
limits. There is no extractor, idle model loop or wholesale transcript promotion.

The Kernel captures all selected archive versions, inspected memory versions and
current input IDs, flattening any prior source-derived lineage. Limits are 16
archive records, 16 memory dependencies and 16 input IDs; a complete recalled
record must fit 4096 bytes. The derived expiry cannot exceed any parent, source,
recall, memory or write-scope expiry. An action-idempotent transaction inserts the
memory, charges the policy and commits the content-free MemoryEffect together.

`MemorySource::ArchiveSelection` has Inferred authority and no individual owner
confirmation timestamp. Evidence retains real binding/conversation/batch/turn/
action/request IDs and policy epochs. A matching per-request archive disclosure
preparation audit is required. This proves preparation of supplied excerpts,
not provider receipt, comprehension, participation or independent verification.
Image descriptions remain external-model interpretations, distinct from sender
words. A standing grant does not individually confirm any inferred fact.

Reads and speech gates check policy epochs, retention, source record versions,
knowledge freshness leases, suppressions and flattened memory dependencies.
Changed or unavailable evidence withholds the derived body and invalidates old
drafts without upgrading a cache miss to proof of absence. Renewing freshness can
restore only the same unexpired record, never extend its retention. Existing
retention timers remove expired semantic rows and their FTS entries.

An exact-ID private management lookup may return an unavailable target's ID and
revision without its body. With a current quoted user request, inspected-revision
correction/forget remains possible. Forget suppresses the selected source records
and original input IDs for this binding, leaving original administrator archives
under their separate retention. It does not refund a write. Dependents of changed
or removed memory are withheld. This is not cross-account fuzzy deduplication or
secure erasure of logs, backups or already dispatched provider data.

User correction creates a new ChannelSelection from the current user quotation,
retaining inferred interpretation rather than external-source authority. The
transaction clears transient stale dependencies and the Actor rebuilds input
from current evidence plus committed effect metadata. Old source-influenced
assistant/tool bodies are excluded from later model history and private source
projections, including indirect source-memory use across tool continuations.

This narrowly supersedes ADR 021's blanket deferral of source-memory selection.
Generic memory promotion from archive/experience/history influence remains
blocked. The separate typed operation does not authorize memory from reading
experiences or historical private-source lookup. Existing candidate review and
Worker permissions remain enforced.

## Consequences

Selection is conservative: all disclosed dependencies count, so one changed
dependency can withhold a broader interpretation. There is no automatic repair,
re-extraction or new inference after revocation. Scope/lineage caps reject complex
selections rather than silently dropping evidence. Multi-day quality, personal
installation and actual QQ/image source continuity require separate evidence;
native fixtures do not establish those outcomes.
