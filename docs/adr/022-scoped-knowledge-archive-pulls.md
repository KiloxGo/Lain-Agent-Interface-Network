# ADR 022: Explicit scoped QQ knowledge archive pulls

Date: 2026-10-27 (Asia/Shanghai). Status: implemented explicit-pull slice. Extends
ADRs 019–021; the independently deployable source service stays independent.

The next W1 slice is one explicit, authenticated, bounded source pull, not an
implicit ongoing collector. A registered QQ group/private source needs an active
finite archive grant. Administrator `knowledge-pull` selects one source, immutable
RequestId, current grant revision and optionally the preceding Lain read receipt.
Registration/import/recall permission alone never starts network collection.

The daemon-selected protected configuration supplies the exact HTTPS service base,
expected persistent store UUID and a read-only SecretRef. It is not model input.
HTTP has no proxy/redirect/retry, an 8-second connection window, 12-second request
and 15-second outer HTTP deadline, bounded body/page and exact source
validation. Plain HTTP is allowed only for loopback synthetic fixtures. The service
uses a dedicated source export path (old unfiltered versions fail 404), filters
changes before export and returns instance identity, source
epoch and committed cursor/high-water metadata. A reset instance or cursor/source/
configuration mismatch fails closed; native source data is not authority.

Reuse durable archive read states, immutable receipt IDs, cancellation and Unknown
boot recovery. Import/update/tombstone, source freshness and Completed cursor
commit together. A failed/cancelled/recovered read does not advance or auto-replay.
Continuation is tied to source/local grant/reader identity and cannot rewind the
last committed pull. Remote source revoke/epoch changes invalidate existing copied
content; a new local grant is needed to select a new remote epoch. Local owner
forget never resurrects; remote changes never silently overwrite owner correction.

Kernel keeps a bounded attributed projection, not another original-media store.
Original message text and external-model image descriptions remain separately
labelled with actual model/extent/truncation. Image bytes remain in the independent
service. Observed projections are not sender quotations, semantic memory, Lain's
participation or independent verification. Existing private recall needs its own
grant and all version/memory/Worker safeguards continue.

For a private source, exact observer-account/peer/native-message matches respect
the Kernel's existing private memory source suppressions. No matching by text or
guessing cross-account event equivalence. Capability context can include a
content-free last-read/freshness pointer; withheld content is not past absence.

Only a pull caught up to the selected source grants a **30-second freshness lease**;
backlog/offline/expired leases withhold copied bodies from archive/Actor/delivery
projections. This is a bounded snapshot, not an instantaneous remote revocation
guarantee. A later explicit pull applies current tombstones/changes or refreshes an
unchanged source; retained data does not gain a new retention lifetime. Continuous
event ingestion and longer-lived derived memory remain active follow-up work.

Migration 020 adds source/record associations without enabling any old source,
copying the personal database, starting models, changing Worker permissions or
installing a personal-instance upgrade. Validate with isolated HTTP/Kernel/native
fixtures and actual hosted protocol reads separately; real chat quality and full
W1–W4 acceptance remain open.
