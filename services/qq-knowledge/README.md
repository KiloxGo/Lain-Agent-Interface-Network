# QQ Knowledge Service

An independent single-user message and image archive for Lain and Utakera.
It imports no Rust/Lain runtime, needs no Lain database, and can be copied and
deployed from this directory alone. Source messages, image bytes and model-derived
descriptions are separate from Lain's semantic memory and persona.

The observer uses a forward OneBot WebSocket and can call only `get_login_info`
and `get_image`. It has no message-send, history, account-management, Worker or
shell capability. New messages from explicitly listed streams are normalized;
other streams are discarded. No automatic friend/group enumeration or backfill.

## API contract, version 1

Additive scoped export: authenticated status and changes include persistent
`store_id`. `GET /v1/sources/qq:1001:private:2002/changes?after=0&limit=16`
filters before returning message bodies and adds `source`, `from_after` and
source-specific `through` metadata. Keep `(store_id, source_id, next_after)`
together; reject a changed store/source identity. A scoped cursor ahead of that
source is a 409. Disabled sources still export current tombstones; an empty page
does not erase history or prove complete collection. The original unfiltered
endpoint remains compatible, and also accepts an optional `source_id` filter.
Use the dedicated source path for strict clients: an older unfiltered server
returns 404 instead of silently ignoring an unknown query parameter. No Kernel
or Utakera dependency is introduced.
Kernel's independently authorized bounded bridge is described in
[ADR 022](../../docs/adr/022-scoped-knowledge-archive-pulls.md).
Its optional finite standing synchronization scope is defined in
[ADR 023](../../docs/adr/023-finite-knowledge-synchronization.md); it requires no
service-side collection, model or Kernel dependency.

All endpoints require `Authorization: Bearer <key>` over HTTPS. Three independent
keys have admin, ingest and read roles; give Utakera **only the read key**. The
ingest key can submit selected-source events and inspect operational status but
cannot retrieve messages or images. Responses use `Cache-Control: no-store`.
Keys are instance-level, not multi-user/tenant accounts. Rotate the corresponding
protected environment entry and restart the service to revoke a key.

| Method/path | Role | Result |
| --- | --- | --- |
| `GET /v1/status` | any valid key | Sources, connection gaps, counts, finite vision usage |
| `GET /v1/messages?source_id=...&q=...&after=0&limit=50` | read/admin | Active messages, ordered by immutable receipt ID; literal text/description search |
| `GET /v1/changes?after=0&limit=100` | read/admin | Monotonic change receipts and **current** message/tombstone projections |
| `GET /v1/images/{image_id}` | read/admin | Original validated image bytes, while the source/message remains active |
| `POST /v1/ingest/onebot` | ingest/admin | Durable event receipt; exact duplicate is idempotent, changed same ID conflicts |
| `POST /v1/ingest/recall` | ingest/admin | `{source_id,external_id}` removes observed message and orphan media |
| `POST /v1/ingest/coverage` | ingest/admin | Selected stream's connection/media gap metadata, no message body |
| `POST /v1/sources` | admin | CAS enable/revoke/configure one explicit source |
| `POST /v1/messages/{id}/forget` | admin | `{}` clears message/body/derived description; preserves a tombstone |

Source registration example (values are illustrative, not deployed account grants):

```json
{"account":"1001","kind":"private","peer":"2002","expected_revision":0,
 "enabled":true,"vision":true,"retention_ms":2592000000}
```

`kind` may be `private` or `group`. Identity is
`qq:<observer_account>:<kind>:<peer>`. Explicitly adding a source in the server
does not modify the observer's local allowlist. Both must select it.

Ingestion takes `{"event":<OneBot message event>,"images":[{"file":"image.png",
"base64":"<actual image bytes>"}]}`. The event requires `post_type=message`,
`self_id`, `message_type`, `user_id`, `message_id`, integer `time` in seconds,
and an array of message segments; groups also require `group_id`. Text, images,
mentions and reply references preserve segment order. Other kinds are marked
unsupported, not interpreted as text commands. Image URLs/local paths are not
stored or fetched by the server. A missing image is explicitly unavailable;
metadata is not represented as a downloaded/understood image.

Responses distinguish provider/source time from Kernel-independent receipt time.
Image records include MIME, dimensions, frame count, content hash, processing
state, model and prompt revision. `description` is an inference, never original
message text, verified OCR or proof of a person's identity.

Utakera can page `/v1/changes`, commit each page and its `next_after` in one local
transaction, and remove cached bodies/images on non-active tombstones. Apply
changes by their sequence even if the message revision is unchanged: image
processing also publishes changes. An old receipt projects **current** state,
so it cannot disclose a since-revoked old body. This is not a historical/MVCC
snapshot or proof of complete QQ history. The service has its own v1 contract;
Utakera's `0.1-draft` source envelope and RSS snapshot API remain distinct.

## Storage and vision limits

- Initial source retention is an explicit 30-day deployment setting, configurable
  per source from one minute to 31 days. It is an implementation choice, not a
  claimed user preference. Source revocation clears its active messages and media;
  re-enabling does not resurrect old IDs. Record tombstones/change/usage metadata
  survive. No automatic permanent semantic-knowledge selection or embeddings.
- Up to 100,000 event identities, 2 GiB live media, 8 MiB per image, 24 million
  pixels, 8 attachments/event and 12 MiB request body. Capacity rejection is visible;
  the observer's bounded outbox keeps transient deliveries for at most one day
  (256 MiB / 10,000 items). Confirmed uploads are removed locally.
- Accept JPEG/PNG/GIF/WebP based on actual decoded bytes. Preserve originals;
  supply only the first frame, at most 1280×1280 JPEG, to DeepSeek. Animated-image
  metadata does not claim the whole animation was interpreted.
- Description cache identity is scoped source + actual SHA-256. Repeated messages
  can refer to one stored asset without another model call. Different sources do
  not gain each other's disclosure scope.
- Native DeepSeek Responses, model `deepseek-flash`, no tools, one HTTP attempt,
  45-second timeout, 512 output tokens. The initial deployment has a **10-request
  lifetime ceiling**, independent of the old D3 budget. Every prepared request
  counts conservatively; restart/failure/revoke cannot refund it. The same DB rejects
  a changed cap. No idle model polling, automatic provider retry or budget refill.
  Missing key/disabled vision/exhaustion leaves content stored with explicit state.
- Source disable during inference discards the result while preserving usage.
  Crash while attempting becomes Unknown without retry. Reported tokens can be
  incomplete; this service cannot provide an exact provider invoice.

Known runtime keys and common credential-shaped text are filtered at ingest and
read, including descriptions and rotated-key views. This is not a guarantee that
all arbitrary secrets in natural-language messages or image pixels are detectable.
Only selected images are sent to the configured external provider; URLs/tokens
from QQ CDN resolution are never passed to it. Source/image instructions cannot
authorize actions: the service exposes no execution tools.

SQLite/WAL free pages, protected backups, prior provider disclosures and independent
client copies are not securely erased by logical deletion. Clients must consume
tombstones. The deployment does not migrate or upload the personal Lain DB.

## Run and operate

1. Copy this directory, create three random API keys and configure protected
   `KNOWLEDGE_ADMIN_TOKEN`, `KNOWLEDGE_INGEST_TOKEN`, `KNOWLEDGE_READ_TOKEN`;
   optional `KNOWLEDGE_DEEPSEEK_KEY` and `KNOWLEDGE_VISION_CAP=10`. Never commit them.
2. Build the Dockerfile, mount an owner-only data directory as `/data` for UID
   10001, bind container 8788 to loopback, and place an HTTPS reverse proxy in front.
   Run exactly **one Gunicorn worker**, four threads, as the Dockerfile specifies.
   The supplied Compose file uses `/etc/lain-knowledge/runtime.env` and
   `/var/lib/lain-knowledge`; create the latter with owner UID/GID 10001 and mode
   0700 before `docker compose up --build -d`. The supplied server is already
   deployed; its current read-only client connection file is protected separately.
3. Register each selected source using its current revision. Configure the local
   observer JSON with `self_id`, `private_peers`, `groups`, `napcat_endpoint`,
   `api_url`, protected `credentials_file` and `outbox_path`. Its credential JSON
   contains only `ingest_token` and `napcat_token`; it never needs the model key.
4. In a Python 3.11+ environment with requirements installed, run
   `python -m lain_knowledge.connector --config <protected-config.json>`. The
   observer authenticates the actual QQ account before accepting events. Incoming
   image URLs are restricted to QQ image CDNs, fetched over HTTPS with pinned public
   DNS results, certificate verification, no redirects/proxy or local-file fallback.
5. Give Utakera the HTTPS base URL and read key through its server/Keychain settings.
   Inspect `/v1/status` for source coverage and finite cost. Stop the observer to
   stop collection; revoke a source to clear its hosted content; stop the container
   to close the API. Revoke/rotate API keys independently from source retention.

The local observer records gaps on disconnect/overflow/media failure and retries
only idempotent outbox uploads with bounded backoff. OneBot has no durable delivery
ack/backfill in this adapter: a crash before outbox commit can lose an event, and
the authentication boundary has a receive gap. It never claims complete coverage.
Invalid/conflicting events are recorded as a local gap rather than silently
changed into successful ingestion. A server source revoke stops/purges that
source's pending uploads on rejection; offline local copies remain bounded by
their own one-day expiry and cannot be remotely erased while offline.
An OS lock next to the outbox enforces one observer instance; the process owns and
releases it, rather than treating the presence of a file as proof of activity.
For Windows startup, directly schedule the native background interpreter with
resolved physical paths; a PowerShell parent can leave child processes behind
when stopped. The installed current-user task has been checked for zero remaining
observers on stop and exactly one authenticated runtime on restart.

Use SQLite's online backup API (or stop the service before copying) for consistent
backups. Keep API/model secrets separate from source exports. Upgrade a versioned
image only after backing up persistent state, retain the previous image/config,
and restore the matching complete stopped state for incompatible schema rollback.

Tests: `PYTHONPATH=. python -m unittest discover -s tests -v`. Native platform,
real-provider and real-NapCat evidence are separately recorded in
[`../../docs/QQ_KNOWLEDGE_ACCEPTANCE.md`](../../docs/QQ_KNOWLEDGE_ACCEPTANCE.md).
