# ADR 014: Bounded Actor processing and source-linked reading experiences

Date: 2026-10-26 (Asia/Shanghai). Status: accepted for D1b implementation.
This extends [ADR 013](013-bounded-text-acquisition.md) within the selected
[combined delivery goal](../DAILY_COMPANION_IMPLEMENTATION.md). It does not grant
real-source/model use, proactive contact or a personal-instance upgrade.

## Permission and invocation

An acquisition ReadingScope may now include an explicit optional `processing`
scope: one existing private ChannelId, a cumulative model-request ceiling 1–100,
and a fixed UTC day/display offset in minutes (-720 to +840). Missing processing
is disabled, including existing schema-12 grant snapshots. Fetch permission,
private replies, private MemoryGrant and historical SourceGrant do not supply it.
The scope permits bounded model interpretation and later disclosure to that one
private binding; it supplies no memory-writing, Work, Worker or contact authority.
Grant replacement still invalidates old bodies; enabling processing cannot
silently process a previously acquired body under an old grant revision.

A successful acquisition event may create one processing job, or the administrator
may explicitly request processing of a currently eligible body. Job identity is
unique per ReadingId. It uses the existing Actor/model adapter and configuration,
not a second planning framework. A dedicated conversation and immutable Kernel
activity request are linked transactionally to the source/job. `reading_activity`
explicitly identifies the turn as Kernel-initiated, rather than a human statement.
It cannot acquire private chat, memory/source-grant contexts or any tool action.
The request contains a reason/ID, not a duplicated source body.

Only one unfinished processing job is admitted; normal private conversations and
Workers continue independently. No tools are advertised in this Actor mode and
returned calls are rejected before creating actions. The existing four-request,
reported-token and 120-second Actor ceilings also apply. No automatic retry,
catch-up processing, reflection loop or extra planner/replyer is introduced.

Before each model request, Actor counter increment and the grant's durable request
reservation commit in one transaction. Reservations count across jobs, failures,
cancel, uncertain calls and restart. They are conservative prepared attempts, not
proof of provider receipt or precise billing. A new explicit grant decision is the
only new budget epoch. Existing/Unknown jobs return their recorded identity without
launching another request. Budget and content-free audits are inspectable; reported
tokens are partial provider evidence and currency cost remains unavailable.

## Evidence and publication

Read the current full stored body through the credential filter before selecting
at most 8 KiB on a UTF-8 boundary. Each request supplies attributed untrusted data
with URL, acquisition time, original/input lengths and truncation. Check source
scope, retention, withdrawal, cancellation and fingerprint after the response and
again on assistant-body/terminal commit. Account returned usage before discarding
an invalid response. Private/background previews are held. External requests already
transmitted cannot be recalled and unknown usage can still be billed.

Only a completed nonempty model summary can create a ReadingExperience. Its creation
and Actor Completed state share the SQLite transaction. Store at most 4 KiB of
summary with original-length/truncation metadata. Keep source ID/URL, actual input
extent, acquisition versus processing-completion times, Actor receipt and grant
revision. This proves summary generation from the supplied excerpt; it never proves
comprehension, full-document reading, fact verification or participation in somebody
else's activity. Acquisition-only records never become summary experiences.

`ReadingResult.processing` preserves actual past acquisition/summary generation
after content becomes unavailable. Current empty retrieval does not mean it never
happened. Raw acquisition, generated summary, activity/Actor audit and semantic
memory remain distinct. No experience is automatically promoted to semantic memory.

## Recall and invalidation

Authenticated administrative search selects a half-open UTC completion-time window
at most 31 days, scans at most 100 latest candidates and returns at most three
records, optionally filtered by literal topic. Both acquisition and completion
timestamps remain visible, including delayed processing across midnight. Kernel
supplies current/yesterday/tomorrow UTC boundaries under the explicitly configured
fixed offset. This is not IANA timezone/DST scheduling support or a product default.

Two typed read tools, `experience.search` and `experience.inspect`, are available
only in the selected active private binding. Lookup receipts/audits contain IDs,
scope and time, not summary copies. Actual fresh evidence is projected for the
next request: up to three selected records, 1 KiB per summary excerpt and 8 KiB
total records. Selection revisions/fingerprints persist in PrivateChatContext.
Reconsideration clears the selection. Revalidate at response, action/final commit,
Ready admission and prewrite. Old uncertain/confirmed delivery truth is preserved.
Source-influenced assistant/actions are omitted from later model-history copies;
canonical operational history remains inspectable by the administrator.

Search also supplies at most three content-free committed summary-operation
receipts in the selected grant/time window, even when no Active summary remains.
They contain IDs, completion/observation times and the projection state at lookup,
never URL, source text, old summary or old correction. This preserves “a summary
was actually generated” after withdrawal without restoring the content. A persisted
grant stamp protects operation-only reads too: revoke/expiry/replacement is checked
before another model request, response, commit and send. No-match metadata never
proves the forgotten topic or independent verification.

Experience-influenced turns cannot silently propose/remember/correct semantic
memory without a separate derived-memory design. Existing forget and Work approval
boundaries remain enforced. Root reading jobs have no tool authority at all.

Administrator correction is CAS and preserves actual source/processing facts.
Its replacement summary is explicitly `owner_correction`, not model interpretation
or independent verification. Revisions change; old drafts/fingerprints become stale.
Content-free edit history preserves operation/time, without replaying old corrections.

Withdrawal suppresses the exact URL, removes all acquired bodies for that source
and invalidates their derived experiences, including admitted in-flight work.
Activity tombstones survive later grant changes and prevent old processing replay.
An explicit administrator restore permits only new acquisitions under an otherwise
valid grant; it restores neither old bodies nor withdrawn experiences, nor budget.
Suppression and restore decisions are audited. Grant changes/expiry also remove
derived summary content; the existing finite retention timer drives cleanup.
SQLite pages/WAL, backups, prior provider disclosures and canonical Actor audits
are not claimed securely erased. This scope does not implement arbitrary archive
or memory deletion through a summary endpoint.

Migration 013 appends job/reservation/experience/read-audit/edit/suppression tables
and indexes. It creates no old jobs, summaries, reads, corrections or grants.
Native and live evidence remain separate in [D1b acceptance](../EXPERIENCE_ACCEPTANCE.md).
