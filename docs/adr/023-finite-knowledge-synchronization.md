# ADR 023: Finite selected-source knowledge synchronization

Date: 2026-10-27 (Asia/Shanghai). Status: implemented; validation recorded in
[acceptance](../KNOWLEDGE_SYNC_ACCEPTANCE.md). Extends the explicit bridge in
ADR 022 without changing the independent service, observer or model authority.

The independent source currently offers bounded HTTP change pages, not a push
subscription. One persisted per-source timer therefore performs selected GETs.
This is a narrow archive synchronizer, not a general scheduler or model loop.
Archive storage, private disclosure, semantic memory, contact and Worker grants
remain independent. Registration, old grants and configuration enable no timer.

An authenticated administrator explicitly selects a current archive revision,
expiry (within that archive grant and 31 days), 1–10,000 requests, 5-second to
24-hour interval and 1–32 changes per read. The server freezes its selected reader
configuration identity. Each policy replacement uses compare-and-swap, creates a
new retained revision and explicitly starts a new allowance; it cannot erase old
spent attempts. This is not a renewal on boot or an Actor capability.

Migration 021 appends the revisioned source-policy/usage table, enabled nowhere by
default. A scheduled read's attempt reservation, Prepared receipt, policy revision
and prior freshness invalidation commit together. The receipt reuses existing
read/claim/cancel/atomic cursor/Unknown recovery and preserves its sync revision.
No reservation is refunded for failure, cancellation, restart or an unused
Prepared request. The existing global 10,000-read ledger cap still applies.

There is one owned runtime slot and at most one unfinished archive read. The
synchronizer waits for ownership before reserving and uses the earliest persisted
deadline or an archive/shutdown notification, without idle polling. Each successful
read schedules the next interval from its completion. A backlog also waits that
interval and keeps bodies unavailable until caught up. Current cursor and source
epoch protections apply to manual and scheduled reads alike.

Failed/Cancelled/Unknown receipts stop that policy, without automatic retry or
replay. Completed policies can resume after restart with their committed cursor,
remaining allowance and next deadline. Expiry, archive revision changes, reader
configuration/credential unavailability or a changed remote source stop collection.
Scope replacement/revocation cancels old work; expiry and delivery completion
checks reject late results. Config file changes are detected at the next scheduler
wake, not claimed as an instantaneous filesystem watch.

Inspect the current policy (including used count, pending/last receipt and stop
reason) and its linked read job. Historical policy/usage revisions remain stored.
Their pending field is the last snapshot at replacement; inspect the linked read
receipt for a superseded operation's final outcome. The latest 100 revisions are
available through the authenticated sync `/grants` route/CLI.
Revocation also invalidates the old freshness lease. Exhaustion stops future GETs;
the last successful snapshot retains only ADR 022's existing 30-second lease and
original retention. Long intervals can leave caches unavailable between reads;
continuous availability or complete message coverage is not promised.

No model inference, source mutation, new QQ sender, personal migration, deployment
of Kernel, derived memory or external Worker control is authorized by this code.
Deterministic native process tests exercise scheduling/restart/cancel and synthetic
caption changes. Hosted read-only validation, when selected, uses an isolated
temporary instance and an exact finite source budget, then deletes its copies.
