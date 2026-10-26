# ADR 016: Fresh sharing receipts in private conversation

Date: 2026-10-26 (Asia/Shanghai). Status: accepted for D3a implementation.
Extends ADR 014/015 without new collection, contact, memory or Worker authority.
D3a is part of D3, not completion of the combined daily-companion/shared-work goal.

## The missing continuity

D2 can deliver a source-linked proactive message, but its dedicated Actor is not
the private conversation's transcript. Copying that draft into chat would confuse
generation with delivery and preserve old source text after correction/withdrawal.
A current empty search also must not become a claim that no message was ever sent.

Keep the durable ShareDelivery authoritative for attempt/transport facts. Project
that record freshly for one private binding. Confirmed means transport acknowledgement,
Unknown means uncertain delivery, NotSent/Rejected mean no successful delivery.
None proves user reading, agreement, current presence or relationship state.

## Recent context and explicit reads

An ordinary private model request may receive one content-free terminal receipt
pointer from the previous 24 hours: actual IDs, claim UTC time, transport state,
receipt presence and explicitly unknown user-read status. It excludes Sending to
avoid destabilizing routine chat while an in-flight acknowledgement settles.
This is a bounded context choice, not a product contact/retention default.
It adds no model invocation and cannot initiate contact.

No source URL, summary, outbound words or hidden draft is automatically injected.
This keeps unrelated chat and reviewed/current-source memory operations available.
When its words or subject matter, Actor can use `sharing.search` or
`sharing.inspect`; these are typed private reads, not send/retry functions.

Search selects a half-open UTC **claim-time** window of at most 31 days, literal
topic up to 256 bytes, at most three records and 100 recent candidates. Matching
uses only the currently permitted projection, never withheld old text. Coverage
is partial; empty results do not prove no contact. Inspect uses an actual
DeliveryId and enforces the current private binding, including when guessing IDs.
Original and current source processing times retain their separate semantics.
Claim time is not an exact server-send or acknowledgement timestamp; lookup
observation time is labelled separately, and user-read status is unknown.
Where no configured day offset is available, UTC timestamps do not establish a
user-local yesterday; named-zone/DST support is still outside this slice.

Tool receipts contain IDs, coverage and operation semantics only. Fresh evidence
is projected again into the next request, bounded to 8 KiB of records and at most
three unique deliveries. A full original outbound message (at most D2's 2048 bytes)
can appear only while the original experience revision/fingerprint and current
reading disclosure/retention scope still match. Its attributed experience projection
preserves actual excerpt/summary extent and model-versus-owner-correction origin.

ShareGrant revocation/expiry closes new contact; it does not erase historical
transport facts or separately revoke a still-current reading disclosure scope.
Correction, withdrawal, reading revocation/expiry, body retention or new secret
filtering withholds old outbound words and experience content. It does not relabel
a real Confirmed receipt as never sent, nor replace the original quote with a new
summary. Current corrected experiences remain separately inspectable under ADR 014.

## Freshness, memory and persistence

PrivateChatContext now carries at most three content-free ShareVersions: actual
delivery fingerprint and, only if source content was disclosed, its ExperienceVersion.
Existing request/response/body/completion/Ready/prewrite guards recheck them.
Reconsidering new input clears selections and stamps within the existing finite
request/reconsideration limits; there is no new retry or repair-model loop.
Returned usage is recorded before a stale response is rejected.

Assistant/tool copies from past share-source-influenced turns are excluded from
later model history. Original operational records remain inspectable. Source-only
read turns cannot silently propose/remember/correct semantic memory, matching ADR
014; content-free automatic pointers do not impose that restriction. They do not
relax exact-current-source evidence, ordinary review or Worker approval checks.

Migration 015 adds content-free `share_reads` with Actor/action/request IDs, selected
delivery IDs, actual disclosure versions and preparation UTC time. Disclosure audit
and Actor version update share a transaction; failed audit prevents model disclosure.
Preparation is not proof of provider receipt, comprehension, successful response
or a message read. Older snapshots default to an empty version vector and are not
rewritten by migration. No personal installation is implied.

The authenticated CLI/API expose fresh receipt search/inspection and the latest
100 content-free read audits. Historical administrator outbox inspection remains
an operational archive, distinct from model disclosure or semantic memory deletion.

## Evidence and remaining scope

See [D3a acceptance](../SHARING_RECALL_ACCEPTANCE.md). Native fixture evidence is
not real-model naturalness, public-source or real-QQ acceptance. D3 still needs
the complete authorized campaign/budget/upgrade package and multi-day review;
W1–W4 remain open. No prompt compiler, source discovery or generic agent framework
is introduced by this change.
