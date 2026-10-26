# Independent QQ knowledge service acceptance

2026-10-27 (Asia/Shanghai). This is a new requested service/deployment slice,
not completion of either previous large stage. Scope and boundaries:
[ADR 019](adr/019-independent-qq-knowledge-service.md).

## Implemented

Standalone Python/SQLite API, source allowlists/revisions, received text/ordered
image/mention/reply segments, source-scoped raw-image cache, bounded direct
DeepSeek image descriptions, conservative durable vision limit, current-state
change feed, search, original-image retrieval, source revoke/message recall/forget
and retention cleanup. Independent read-only NapCat observer has a durable upload
outbox; it imports no Lain runtime and has no QQ send/history/control method.
Utakera can use a read key without ingest/admin authority. Source-to-Kernel
semantic memory/long-term continuity and Apple client UI remain separate work.

## Deterministic evidence

- **13 tests pass on native Windows Python 3.13** with Pillow 13.0.0/websockets 17.3.
- **13 tests pass on actual server Linux x86_64**, Python 3.12 in the production
  image, including the real local WebSocket handshake/get_image/upload fixture.
- Cases cover unauthorized streams/keys, read-key write rejection, concurrent
  duplicate delivery, changed-ID conflict, source isolation, CAS revoke/re-enable,
  no resurrection, raw image dimensions/hash/URL exclusion, literal Chinese search,
  secret-shaped text, model description search/cache, failed-call cap, restart
  Unknown/no replay, revoke during inference and expiry/body/description clearing.
- These use synthetic events/provider fixtures; they are not actual QQ-image or
  natural-chat-quality evidence. Routine CI was not enlarged.

The original Rust W1+D3 merge independently passes **217 tests, 0 failed,
3 manual ignored**, and strict workspace/all-target Clippy on native Windows.
The initial shared-target cross-worktree cache produced stale wire exports;
cleaning only the rebuildable lain-wire package and rerunning resolved it.
Personal Lain remains on the narrow schema-16 release, with its grants/budget
closed; no schema-17 or personal-database cloud migration was performed.

## Actual server and provider evidence

- Authorized SSH reached Ubuntu 22.04 x86_64. A dedicated deployment key and pinned
  host key are protected outside Git; the login password was not saved. Existing
  Caddy/Docker sites were inspected before adding a separate route. No other
  project's process/database was stopped or migrated.
- The API is actually hosted at `https://hub.example.com/lain-knowledge` behind the
  existing verified public TLS certificate. Container binds only 127.0.0.1:8788,
  uses UID 10001, a read-only filesystem and persistent owner-only data directory.
  Public unauthenticated status returned 401; read key returned 200 and cannot
  POST ingestion (403). TLS verification was enabled, with no redirect bypass.
- A real synthetic PNG (640×320) traveled through the public API and original-byte
  download with equal SHA-256. Exactly **one real DeepSeek Flash request** described
  its printed text, red square and blue circle; reported total **322 tokens**.
  The same message re-delivery deduplicated without a second call. Initial service
  budget remains **1/10**; it is independent of the old D3 48+12 split.
- The synthetic test source was revoked and content removed after validation.
  Personal history was not imported for testing. This test proves actual hosted
  storage/TLS/provider processing, not reception of an actual QQ image.

Protected deployment/build/smoke reports and API keys remain outside source Git.
The read key for Utakera is in the protected local `utakera-client.json`, separate
from admin/ingest/model credentials. No key values are printed here.

## Remaining evidence and limitations

The actual Windows scheduled task is **Running** and its own protected outbox
health is `online_observed_only` after the real NapCat account handshake. The
cloud received its connection-coverage receipt and the upload queue drained.
Only account <bot-qq> / private peer <user-qq> is enabled; group allowlist is
empty. The package's AppData virtualization initially made scheduled-task paths
unresolvable; installation now resolves the physical path before registering the
hidden current-user task. Registration alone was not treated as a running service.
An actual newly received QQ image has not yet been observed in
this slice; the full real QQ→CDN→server→provider chain stays pending until then.
No group is enabled. Receiving pure images does not yet make the existing Lain
Actor multimodal or grant it automatic knowledge-service reads. No native Apple
Utakera UI, multi-day quality, embeddings, full historical backfill or cross-device
Worker milestone is claimed. Logical purge cannot erase prior provider disclosure,
offline client copies, free SQLite pages/WAL or protected backups.
