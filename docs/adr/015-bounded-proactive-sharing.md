# ADR 015: Independent, bounded private sharing

Date: 2026-10-26 (Asia/Shanghai). Status: accepted for D2 implementation.
Extends ADR 014 within the combined daily-companion/shared-work goal. It does
not authorize live model calls, real sources, QQ contact or a personal upgrade.

## Authority and attention

Migration 014 adds independent per-binding ShareGrant decisions, explicit
Attention state, candidates, Actor jobs, request reservations, transport receipts
and a boot watermark. Existing reply/memory/source/reading grants remain intact;
none grants contact. A missing ShareGrant is off. A missing Attention decision is
blocking, rather than assumed availability. Reply admission must also remain on
so the bound user's response is observable. Enabling replies cannot enable sharing.

A ShareScope explicitly supplies exact canonical HTTPS URLs (1–8), expiry, a
fixed UTC offset (-720..840 minutes), quiet windows (0–8, possibly overnight),
cooldown, per-local-day attempt ceiling, per-grant send/model ceilings, candidate
TTL, source maximum age and post-input quiet time. Empty quiet windows explicitly
select no quiet hours. No policy field has a product default. Engineering caps:
send/day/model ceilings 1–100; cooldown/TTL/source age 1 ms–31 days; user quiet
1 ms–1 day. These bounds are not recommended user settings. Offset is fixed,
not a named-zone/DST implementation. Overlapping windows jump to their ends;
full-day coverage blocks until configuration changes without timer polling.

Attention uses revision-checked administrator decisions: available, indefinite
DND, or DND until an explicit UTC deadline. This slice exposes CLI/API controls;
natural-language DND interpretation and calendar/device attention signals are
not implemented. DND restricts ordinary proactive contact, not replies, approvals
or accepted-work result delivery.

## Source and draft

Only actual current ReadingExperiences whose processing scope permits the target
binding are eligible. Both grants, experience revision/fingerprint, source body
retention and age are checked again at model disclosure, draft publication and
immediately before transport write. Source commands cannot supply authority.

Automatic consideration only includes reading Actors newer than both grant and
boot watermarks. No old-experience catch-up occurs after enabling or restarting.
An administrator can explicitly consider an eligible older experience. Candidate
identity is unique per binding/experience/revision across grant epochs; repeating
consider cannot retry a terminal candidate or its uncertain delivery. A correction
may create a different revision, subject to all other limits.

The existing Actor runs a dedicated `sharing_activity`, mutually exclusive with
reading/private contexts and all tool actions. It receives bounded attributed
experience evidence, no social history or semantic memories, and may return
Share or Skip. JSON decision/reason/text are validated (512-byte reason,
2048-byte message). This is a draft decision, never proof of sending. There is
one unfinished share draft globally, no model retry loop, and no automatic
memory promotion. Model quality and worthwhile-topic selection require real
evaluation; a fake draft is only protocol evidence.

Request reservations commit with the Actor counter, before any provider call;
all failed/cancelled/uncertain reservations remain counted across restart. Returned
usage is recorded before a stale result is rejected. Aggregate request/send caps
are scoped to one explicit grant revision; changing a grant is a new administrator
decision, not an automatic refill. No-response, daily and cooldown enforcement
span grant revisions and all persisted delivery records, independently of the
latest-100 inspection page. Token reports are provider-reported, not invoices.

## Admission, concurrency and delivery

Candidates are Prepared/Drafting/Ready before a send claim. New user input
supersedes unsent candidates; queued/running input, private inference and pending
confirmation block ordinary sharing. Current replies and batches are considered
first. Any in-flight share blocks another share; any Confirmed/Unknown share
after the latest genuine input blocks further ordinary contact until new input.
NotSent/Rejected still consume attempt/day/cooldown allowance. Silence is not a
read receipt, relationship inference or permission to retry.

A share has its own delivery record, never a fabricated user Batch. NapCat reuses
the existing authenticated identity gate, literal text segment, bounded queue,
echo correlation and timeouts via a typed reply/share envelope. The service and
transport both recheck the saved envelope/current policy; a changed attention
revision or user input invalidates a queued claim. Once a write may have happened,
revocation cannot promise recall. Confirmed/Unknown/NotSent/Rejected retain the
existing precise meanings. No uncertain-send retry exists.

The service owns bounded asynchronous share-send tasks so receipt waiting does
not stop current chat inference. The physical NapCat writer remains serial;
an already written request may occupy its receipt window. Shutdown abandons
owned sends; subsequent recovery marks Sending as Unknown and unsent work as
Expired, advances the boot watermark and never replays it. Model reservations
and historical transport receipts remain intact. Candidate expiry, attention,
quiet-period and cooldown deadlines use finite timers; no idle model poll exists.

Unsent invalidated candidates discard their text. Historical model and attempted
outbound records remain operational evidence and are not automatically placed in
future model context or semantic memory. Withdrawing a source stops future
disclosure/send; it cannot delete a QQ message already sent or erase backups.

## Remaining integration and evidence

See [D2 acceptance](../SHARING_ACCEPTANCE.md). D3 still needs complete-chain
experience/receipt recall, live-quality and multi-day acceptance plus its concrete
upgrade/rollback authorization package. W1–W4 remain separate unfinished parts of
the same goal. D2 does not imply autonomous source discovery, scheduled reading,
real QQ approval, remote device execution or a new generic agent framework.

D3a follow-up (2026-10-26): [ADR 016](016-sharing-receipt-continuity.md) now
implements fresh receipt/source projections for subsequent private conversation.
Historical operational records remain separate from current model disclosure.
