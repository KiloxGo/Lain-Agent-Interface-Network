# ADR 037: Conversation-bound daily briefing evidence

Date: 2026-11-05. Status: accepted design; implementation in progress, not deployed.

## Problem

Private archive recall selects at most three records per turn. It is insufficient
for a day briefing. Its private-channel grant cannot authorize an ordinary
terminal turn. A generated paragraph also cannot safely identify "the second
item" after correction, revocation or restart.

## Decision

Add an independent, default-off, finite **conversation** briefing grant. The
owner explicitly selects the real conversation, source IDs and import revisions,
earliest permitted message time, fixed UTC offset and record bound. This grants
reading/model disclosure to that one conversation through its QQ or terminal
entries; it grants neither collection, contact nor memory writes. Existing
channel recall/memory grants retain their meaning and are not copied or widened.

First build a durable, text-free day manifest before model processing. A request
ID fixes the reviewed grant revision, computed yesterday interval, source
high-water marks and record revisions. Retries return the original manifest,
including across midnight/restart, rather than silently select a different day.
Changed requests with the same identity conflict. Entries are ordered by Kernel
archive sequence, not a possibly duplicated or out-of-order platform timestamp.
The half-open day filter uses reported occurrence time, falling back explicitly
to receipt time. Scope clipping and record bounds remain visible.

Read pages return complete retained record texts under a bounded total text-byte
budget; they never silently truncate the end of a notification. Every read
rechecks conversation grant, all selected import grants, source freshness and
manifest record versions. Changed/forgotten records invalidate the old manifest;
clients cannot resume from a stale list and treat its old summary as current.
New records after the manifest's high-water are outside that snapshot, not proof
of an empty rest of day. Unknown timestamps, unavailable bodies, finite history,
source gaps and coverage limitations remain explicit.

Subsequent implementation must attach generated items/citations to this
manifest, perform bounded model processing through the existing inference
ledger, and recheck dependencies before inference/publication/history reuse.
Follow-up selection must re-read source evidence. Memory selection must reuse
the memory engine with proper provenance and independent write authority;
this read grant is not a new standing memory grant.

## Limits and migration

Schema 033 is additive; upgraded databases receive no enabled briefing grants.
New tables join the existing client invalidation/deadline mechanism. No source
body or summary is copied into the manifest. Administrative inspection is not
proof that a model turn disclosed data. Model audit and publication integration
remain required before the full feature is available. Fixed offsets do not
claim IANA/DST support. A manifest covers retained imported data as of creation,
not complete upstream history or continuous collection.

No cloud deployment, source enablement or memory permission is implied by this
ADR. Existing personal databases and seven-day trials remain intact.

## Implementation checkpoint: finite model batches

Local schema034 adds one immutable page-to-Actor binding and source-gated model
result per batch. Each batch reads at most four complete records/16KiB, performs
one accounted inference and returns at most four cited items plus an explicit
omission flag. At most 64 batches are permitted per manifest. The existing Actor
owns model execution, timeout, cancellation and Unknown recovery; this adds no
generic planning/tool runtime. Processing uses an internal conversation with no
Worker, private-chat or semantic-memory tools. Same page identity is recovery,
never replay. Source currency is checked before inference and publication; body,
validated items and completion commit atomically. Ordinary restores withhold stale
activity bodies and later ordinary model history drops generated activity text.

Actual native HTTP fixtures and one synthetic-input real-provider batch pass.
One earlier real call remains Unknown with no proven cause. Natural conversation
tools, whole-day orchestration, item follow-up and memory integration remain
unimplemented at that checkpoint; conversation grant semantics and goal acceptance are unchanged.

## Implementation checkpoint: finite ordinary conversation

Local schema035 now adds a durable finite run and per-foreground-turn disclosure
metadata. Event-driven coordination reuses batch execution, stops at 64 batches/
20 minutes/manifest expiry, and retains Unknown after restart without replay.
Ordinary yesterday/read/inspect tools require the same real conversation's grant.
They capture manifest lineage before disclosure, gate future inference and speech,
exclude generated source history, and keep generic memory writes unavailable.
A stopped run cannot admit a previously prepared request or accept late results;
closed source-change notices finish without restoring disclosure authority.

Reports have readable Kernel source labels, stable item numbers and a 12 KiB
whole-item bound with honest continuation. Native Windows terminal and simulated
QQ flows pass on synthetic data. A synthetic-source actual-model terminal run
passes natural retrieval and numbered original follow-up after adding readable
source labels and explicit fixed-offset clocks; earlier output/tool-budget
failures remain recorded. The clock supplies source-relative date anchors, not
general calendar parsing. Selected-memory integration, real sources and cloud deployment
remain open. [Current scope and evidence](../DAILY_BRIEFING.md).
