# ADR 011: Bounded private-history source lookup

Date: 2026-10-25 (Asia/Shanghai).
Status: accepted and implemented for Q2-B1; native deterministic evidence in
[Q2-B1 acceptance](../Q2_B1_ACCEPTANCE.md). Real-provider experience remains unverified.

## Decision

Source lookup is a separately administered, default-off permission for one existing
private binding. It neither inherits nor extends the reply/memory grant. Each
revision explicitly selects a half-open UTC Unix-millisecond **receipt-time**
window, at most 31 days, ending no later than the decision, and captures the
committed incoming sequence high-water mark. Optional expiry and compare-and-swap
revision checks apply. Revocation/replacement invalidates admitted older turns and
cursors. No model may establish permission. No new source is collected.

Reuse Incoming, Batch, ActorTurn and Delivery as the original records. Migration
010 adds grant decisions, content-free read audits and a receipt-window index;
there is no duplicate archive or second semantic store. Original data and old
confirmation fields are unchanged. A source is a historical statement, not Active
memory, proof of comprehension or a verified fact. A model-request attempt is not
proof the provider received or read it. Delivery Unknown stays Unknown.

Search/inspect checks binding, current grant revision/deadline/window/high-water,
current reply authorization for Actor use, and Q2-A source suppressions. Search
scans at most 512 candidate rows per page, returns at most ten administrative or
three Actor records and at most 8 KiB of encoded records. Literal keywords are at
most 256 UTF-8 bytes; incoming excerpts are at most 1,024 bytes and each generated/
outbound reply excerpt at most 512 bytes. Original lengths and truncation remain
visible. Pagination fixes the channel, revision, window, query digest and incoming
high-water and last examined (receive time, sequence) pair. The receipt-time index
is sought directly, with sequence breaking equal-time ties; sequence high-water
excludes later backdated inputs. Changed filters require a new cursor. No-match pages and all coverage
remain explicitly partial, including gaps and sanitization/exclusion.

Associated reply text is returned only when its generation time and every input
in the merged batch satisfy the same readable window/high-water/suppression
policy. Metadata can report a withheld reply. Source-derived turns' replies are
withheld rather than recursively expanding source authority. Replies whose
private-memory inspections now refer to changed/inactive/deleted memory are also
withheld. Administrator raw channel inspection retains its previous independent
boundary. Semantic forgetting is not archive/audit/backup erasure.

Two closed read tools, source.search and source.inspect, operate on the current
binding implicitly. Their receipts/audits contain IDs and policy metadata, never
raw historical text. At each model request selected sources are freshly projected
under the storage lock and audited as prepared for a model request, not read or
understood. They enter as bounded attributed untrusted data. There is no extractor,
polling loop, additional model job, new Work permission or memory-write authority.

Assistant text and action observations from past source-reading turns are excluded
from subsequent model-history context; original user requests remain intact.
Within a source-influenced turn, opaque provider continuation is preserved only
while every previously disclosed selection in that turn still revalidates. All
selected projections are re-read and fingerprinted before each additional request
and after each response. No raw source cache is persisted in tool receipts. If the
selected source projection/grant changes while inference is in flight, discard
the returned source-influenced text/call and finish with a static scope-change
notice. No stale tool dispatch or extra model request follows. Already transmitted
provider requests cannot be recalled. Source-influenced streaming text is held
until the post-response check; final committed output uses the usual snapshot.

All source text is checked for known credentials before truncation/disclosure.
Reads, prepared-disclosure audits and administrative decisions are inspectable;
audits preserve source IDs/revision/operation/time without source bodies or raw
search strings. Existing memory candidates, Worker approvals and uncertain-effect
recovery remain enforced. Real/personal instances are not migrated or enabled by
development. Q2-B1 stops before experiences, summaries, observation or contact.
