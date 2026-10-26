# ADR 018: Selected visible source archive foundation

Date: 2026-10-26. Status: accepted for W1a implementation. Product basis:
PRODUCT.md, RESIDENT_DESIGN.md and DAILY_COMPANION_IMPLEMENTATION.md.
This adds owner operations/storage, not permission to access personal sources.

## Decision

Kernel owns selected source identities, default-off revisioned import/disclosure
grants, bounded visible records, content-free edit receipts and body retention.
Schema 017 appends archive_sources/grants/records/edits. Private Incoming,
SourceGrant, semantic memory, Actor and WorkSession retain their meanings.

The selected source keys declare one QQ group under an observer account or one
Codex session under an installation/workspace. Their opaque identities are owner
claims: registration/import verifies neither login, connectivity, runtime version,
device identity nor control capability. W1a's only producer is an explicit owner
import of normalized visible data. Actual QQ/Codex read adapters remain W1 work.

Import creates no conversation, Actor, memory, experience, managed work or outgoing
message. Third-party instructions and approval notices remain data. No archive
model tool, extractor, provider poller or external-session resume is introduced.

## Boundaries

- Explicit source scope selects expiry, body retention and admission cap. Engineering
  ceilings: 64 sources; 31-day scope/retention; 10000 records/grant; 32 records and
  64 KiB/import; 8 KiB/text; 20 records/query. These are not product defaults.
- Scope grants import/disclosure only. Existing reply, history, memory, contact,
  Worker and device permissions confer no archive authority or inverse rights.
- External event identity is scoped by source, including account and target.
  Exact replay returns the same Lain ID without extending retention/spending more
  admission. Same identity with different payload conflicts and rolls back the
  entire batch. Different IDs/accounts remain separate even with identical text;
  no fuzzy/global-ID deduplication. Correlation needs future adapter evidence.
- Source occurrence time/position are unverified. Kernel receipt UTC and commit
  sequence are independent. Query cursors bind source, grant revision, after and
  a frozen sequence upper bound; current bodies are projected freshly. This is
  neither provider backfill, an old-body MVCC snapshot nor complete coverage.
- All imported slices report OwnerImport and incomplete/unverified coverage.
  Import is not proof that Lain observed, read, verified or participated in events.
- Owner correction requires current record revision, active original grant and an
  unexpired body. It replaces effective text, increments revision and labels
  OwnerCorrection. It does not forge source wording or renew retention. Reimport
  of the original payload returns the corrected record.
- Forget clears effective text and retains a tombstone/identity and content-free
  receipt; same forget is idempotent. Original-payload replay cannot restore text.
  Revoke stops disclosure/import, leaving raw bodies until their original deadline.
  Regrant starts a new epoch and does not disclose old epoch bodies.
- Startup pruning, a nearest-deadline event-driven timer and fresh lookup checks
  enforce body expiry; clearing a body advances its revision. Expiry/forget do not
  refund admission. Metadata/digests/receipts, SQLite/WAL and backups have distinct
  lifecycles. This is not secure physical erasure; metadata/backup retirement needs
  a later explicit policy. No old/new text is copied into the edit audit.
- Inputs and owner responses apply known-credential/pattern checks, including
  credential rotation. CLI reads one bounded explicit JSON file, never scans
  directories. Hidden reasoning kinds and attachments are unsupported.

## Consequences

Both selected adapters can reuse durable identity/grant/cursor semantics without
turning operational conversations into archives. A receipt proves Kernel storage,
not authentic provider observation. Platform edits/retractions, source gaps and
actual adapter version/capability verification remain unimplemented here.

Future model disclosure must add typed bounded lookup, current source/record
revision checks through response/send and derived invalidation. Until then no raw
archive content enters model context. Actual read-only QQ observation, supported
Codex visible-history reading, source-to-continuity integration, paired-device
execution and complete W1–W4 acceptance remain open.
