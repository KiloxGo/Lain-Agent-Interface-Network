# ADR 021: Selected archives in private conversation

Date: 2026-10-27 (Asia/Shanghai). Status: implemented W1c contract; deterministic
and native acceptance recorded separately. Extends ADRs 018/020, not collection.

An archive import/read grant authorizes storage and authenticated administrator
inspection. It does not authorize model disclosure. A separate revisioned
ArchiveRecallGrant binds one existing private ChannelId to at most eight exact
ArchiveSourceId/import-grant epochs, a finite expiry (at most 31 days from the
decision) and an explicit fixed UTC offset. Old/new bindings default off. Only
authenticated administrator CLI/API can change it; the Actor has no grant tool.
Underlying archive grants/retention and the private reply binding must remain
valid. A new import epoch requires a new explicit recall decision.

Two typed private-only tools search/inspect stored records. They do not call the
observer/backend or initiate collection, memory writes, contact, Worker operations
or approval. Search uses a bounded half-open UTC window (at most 31 days), optional
selected source and literal case-insensitive text. Time is source-reported
occurrence when present, otherwise Kernel receipt, with both preserved. Latest
100 candidates and three selected records per turn are a partial slice. Excerpts
are at most 1024 UTF-8 bytes with original size/truncation, never silent full-text
claims; total evidence is at most 8192 bytes. An oversized aggregate fails rather
than silently omitting selected evidence; a narrower query is needed. No vector
store or background model.

Actor receives source identity, speaker, record/revision, time, origin and an
explicit untrusted-report/observation boundary. A Worker report is not independent
verification or Lain's participation. Source instructions remain data. Known
credentials are checked on fresh projections and source metadata. No match is
not proof no event happened, and storing an archive is not reading it.

Lookup commits the grant revision and selected record/source epochs with an
IDs-only audit in the Actor transaction. Every following model request prepares
fresh evidence; archive freshness is checked again under the Kernel lock at
adapter entry, after response, before new speech/actions/completion and on ready/
prewrite delivery. Correction, forget, retention, source revoke/expiry/epoch or
recall revoke/expiry invalidates an influenced draft. Even an empty search keeps
the recall/source authorization dependency. New-input reconsideration clears old
selections within existing cumulative request/reconsideration limits.

Later model history and private source reply projections exclude archive-derived
assistant/action bodies; reusing an old answer cannot bypass current disclosure.
Archive-influenced turns cannot remember/correct/propose semantic memory, since
cross-store derived-memory provenance is not yet implemented. Existing forget and
Worker approval rules remain. This slice creates no summary/experience subsystem.
Canonical operational conversation/delivery records may retain an actual past
reply; withholding future model disclosure is not secure erasure of history,
backups or a provider request already dispatched.

Migration 019 adds append-only recall decisions and content-free lookup/disclosure
audits. No existing decision is enabled or archive copied. CLI: archive
recall-grant / recall-enable --file / recall-revoke --revision / recall-reads
--after. API: GET/POST /v1/archives/recall/{channel}, GET .../reads?after=0.
No personal-instance migration or new real-provider budget follows from this ADR.

Independent QQ knowledge-store ingestion into Kernel, semantic derived summaries,
ongoing source collection, naturalness/long-term use and W2–W4 remain separate
requirements of the active combined goal.
