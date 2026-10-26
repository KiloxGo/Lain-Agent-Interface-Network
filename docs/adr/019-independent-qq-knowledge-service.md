# ADR 019: Independent QQ message/image knowledge service

Date: 2026-10-27 (Asia/Shanghai). Status: selected by the user's appended goal.
The user requested received QQ messages/images, DeepSeek visual interpretation,
storage decoupled from Lain, a keyed API for Utakera, and actual deployment to
the supplied server. This extends the active combined goal; it does not declare
the preceding companionship/device milestones complete.

## Ownership and scope

`services/qq-knowledge` is a separately runnable/deployable Python service, with
its own SQLite database and API contract. It imports no Kernel, Rust or worker
runtime and does not require copying an existing Lain database. The source store
owns normalized source events/media and finite model-derived image descriptions.
Lain Kernel continues to own persona, semantic memory, conversations, activities,
work and permissions. Utakera receives a read-only source API, not Worker control.
The existing W1a owner-import archive remains unchanged; automatic linkage to
Kernel archives/derived continuity is not claimed implemented by this service.

The first installed observer selects only the established bot/owner private pair.
The server and local observer must both list a stream; group code exists but no
actual group is enabled. There is no friend/group enumeration, hidden reasoning,
arbitrary device scanning, history backfill, control takeover or QQ send method.
Source records/third-party text/image instructions are data, never authority.
This separate storage authorization does not enable personal semantic memory or
the old Kernel historical SourceGrant.

The observer verifies NapCat login identity before accepting messages. It calls
only get_login_info/get_image, resolves selected media through a bounded HTTPS
public-DNS-pinned QQ-CDN downloader and posts an idempotent durable outbox. No raw
CDN URL or local path reaches the cloud store/provider. Unavailable media retains
its original segment with an explicit gap. Disconnect, pre-auth receive and
pre-outbox crash gaps remain visible; this is observation, not complete history.

## Data and derived content

Event identity includes observer account, stream kind, peer and native message
ID. Re-delivery of identical normalized content is idempotent; changed content
under the same ID conflicts. Image identity is source-scoped actual SHA-256;
original decoded JPEG/PNG/GIF/WebP bytes are separate from bounded descriptions.
Only the first image frame, resized to at most 1280×1280, goes to the provider.
MIME/dimensions/frames, provider/model/prompt version and reported usage preserve
actual processing extent. A description is not original text, verified OCR,
semantic memory, participation or an understanding of an entire animation.

A source's configured finite retention applies to messages and orphan media/
descriptions. Revoke, observed recall and forget clear content and publish current
tombstones. Re-enable/re-delivery cannot resurrect old IDs. Monotonic change
receipts project current state, including later captions, rather than exporting
old retained bodies. Utakera must apply tombstones and follow change sequence.
SQLite free pages/WAL, offline clients, prior provider disclosures and backups
have distinct retention/erasure limits; no secure-erasure promise is made.

The deployment explicitly chooses 30-day source retention, 2 GiB live media,
100000 event identities, 8 MiB/image, 24 MP/image, eight images/event and 12 MiB
ingest bodies. These are adjustable implementation limits, not learned user facts.
The observer's outbox is bounded to one day/256 MiB/10000 events and removes
confirmed uploads. An offline observer cannot instantly receive revocation.

## Model, authentication and deployment

Use one direct native DeepSeek Responses attempt per pending asset, no tools or
SDK retry. The initial independent database has an immutable **10-request lifetime
ceiling**, separate from the closed D3 campaign. Prepared/failed/unknown requests
remain spent across restart; an exhausted queue does not silently refill. Missing
key/permission prevents inference; source changes invalidate late results.
The model is event-driven and has no idle polling. Provider invoices/internal
implementation remain unknown; reported normalized tokens are partial evidence.

Admin, ingest and read Bearer keys are separate, rotated through a protected
runtime credential file. Only the read key is supplied for Utakera. The cloud
service binds loopback behind verified public HTTPS, runs as an unprivileged
container user with a read-only filesystem/capability drop, and has a persistent
owner-only data volume. Existing unrelated server services remain separate.
The selected local observer has only NapCat and ingest credentials; no model key.
No credentials or private DB/chat/media belong in Git or normal logs.

## Upstream references verified 2026-10-27

- [NapCat segments](https://napneko.github.io/onebot/segment) and
  [read APIs](https://napneko.github.io/onebot/api) describe image metadata and
  get_image. Actual media coverage remains adapter/version dependent.
- [DeepSeek vision](https://api-docs.deepseek.com/guides/vision/) currently supports
  deepseek-flash images via Responses input_image. Request limits here are lower
  than provider limits.
- MaiBot inspected at `062d45cf8bd4272ac57d7ec03fcf405e5a458211`:
  `src/chat/image_system/image_manager.py` saves before describing, scopes cached
  processing by content hash and uses a configured VLM task. We adopt those
  mechanisms, independently implemented; no GPL code/prompt is copied, no emoji
  learning/automatic memory synchronization, multi-model planner or tool framework.
- Utakera inspected at `d7fb53b059187c3e224818fb2bdae74e92fad178`: independent
  connectors, source provenance, bounded storage and keyed service access align
  with its design. Its source protocol is explicitly exploratory; the implemented
  RSS API is not falsely represented as this QQ service contract. Native Apple UI
  integration requires its own implementation/acceptance.

Implementation/API/operation details:
[standalone README](../../services/qq-knowledge/README.md).
Actual layered evidence: [acceptance](../QQ_KNOWLEDGE_ACCEPTANCE.md).
