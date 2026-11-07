# W1d explicit QQ knowledge archive pulls

W1e now adds a separate finite standing synchronization scope; see
[current synchronization acceptance](KNOWLEDGE_SYNC_ACCEPTANCE.md). The explicit
pull evidence and then-remaining synchronization work below describe W1d.

2026-10-27 (Asia/Shanghai). Contract: [ADR 022](adr/022-scoped-knowledge-archive-pulls.md).
This slice connects one selected independent source to Kernel archives. Continuous
collection, persistent derived memory and full W1–W4 remain active work.

## Delivered contract

- Independent service has a persistent store UUID and source-filtered current
  change/tombstone export. Old unfiltered clients remain compatible. Consumers
  persist `(store_id, source_id, next_after)` together; a bare global number is
  not a source identity. Kernel continuations use its own committed receipt IDs.
  Strict readers use the dedicated `/v1/sources/{source}/changes` path, so older
  servers cannot silently ignore a new filter and return unrelated source bodies.
- `qq_private` source keys join existing selected QQ-group keys. One explicit
  authenticated `knowledge-pull` uses a current finite archive grant and the
  daemon-selected HTTPS base/expected store UUID/read-only SecretRef. No model
  tool, automatic collector, proxy, redirect or request retry is added.
- Bounded 1–32-event, at-most-1-MiB, 15-second operations reuse durable archive
  receipts and boot Unknown recovery. Prepared admission withholds the old cache;
  import/update/withdrawal, source state and Completed cursor commit together.
  Idempotent receipt retries make no second GET. Reader/source/grant identities
  cannot be swapped or a completed cursor rewound.
- Original message excerpts and separately attributed external-model image
  descriptions form a `knowledge_projection`; native message/asset IDs and
  model/prompt/processing policy survive. At most two image descriptions (512
  UTF-8 bytes each) and 2048 original-text bytes, explicit truncation/omissions,
  at most 8192 projection bytes. No image bytes, whole database or original-media
  ownership moves to Kernel. This is observation, not participation/verification.
- Caught-up snapshots get a 30-second lease from local commit, bounded by the
  local source grant and original data expiry. Request transit adds age; this
  is not instantaneous remote revocation. Backlog/failure/cancel/expired leases
  withhold copied bodies. Freshness refresh never renews original retention.
- Remote source revoke/epoch changes clear copies. Local forget never resurrects;
  remote changes invalidate rather than overwrite an owner correction. Even
  unseen upstream tombstones retain identity without creating invented content.
- Existing private recall remains separately granted. Freshness and source
  changes invalidate in-flight/ready speech. Exact same-account/private-peer/
  native-message matches also respect existing private memory source suppression;
  this does not invent cross-account or fuzzy-text equivalence. Content-free
  snapshot metadata explains withheld caches without asserting no past event.

## Evidence

Native Windows/MSVC: **236 passed, 0 failed, 3 manual ignored**; format, strict
workspace/all-target Clippy and locked build pass. All **14 Python script tests**
pass. The full native process regression passes **36 checks / 83 fake model calls**;
schema19→20 full stopped backup/upgrade/old refusal/rollback passes **5 checks /
3 fake calls**. Both report cleanup true and zero paid calls.
Six new domain/HTTP tests cover actual HTTP filtering/auth failures, instance/
source/identity/cursor mismatch, updates/tombstones/no resurrection, atomic receipt
failure, retention/lease expiry, source epochs, owner-correction conflicts, cancel/
shutdown and private disclosure with unchanged record revision. The private test
also checks that a matching previously suppressed input cannot be redisclosed.

The standalone service passes **15 tests on native Windows and actual server
Linux/Python 3.12**. The additive protocol is deployed as
`lain-knowledge:e789046d957c` behind the existing HTTPS route. A stopped complete
protected data/credential backup was made before replacement. Unchanged pinned
dependencies and Dockerfile were compared with the prior release, then its exact
verified runtime image was reused after a redundant dependency download stalled.
The owned build was stopped; the old serving container stayed running until the
tested replacement was ready to install. Failed build evidence was retained.
Normalized SHA-256 hashes of all five deployed service Python modules match the
local release sources.

Service vision reservations stayed **1/10, 322 reported tokens** through upgrade.
No counter or credential was reset. Rollback uses the previous container with
current compatible data, preserving accepted events and spent reservations; a
backup is not authority to refund model attempts. Production failure/rollback was
not deliberately injected. Personal Lain remains on schema16, separately stopped.

The new native smoke command defaults to a synthetic HTTP source and tests actual
CLI/daemon/cursor/restart/caption-update/recall/cleanup with zero model calls.
`--cloud-client` explicitly reads only one selected existing hosted source using
the selected read key into an owner-only temporary instance, then removes that
copy. It sends no QQ messages, starts no Worker and changes no cloud source.
Hosted source reading and synthetic caption tests are different evidence; neither
proves a new real QQ image or conversational quality.

The final native synthetic bridge passes **3 checks / 5 source GETs / 5 durable
read jobs**. The actual hosted-source bridge passes **2 checks / 2 read jobs**,
including one selected stored record, process restart, immutable receipt retry
and incremental refresh. Both have zero Actor/inference/memory creation, zero
model requests, zero QQ sends and confirmed temporary-copy cleanup. All four
process/bridge/upgrade reports match the final binary hashes. Their JSON/logs and
`evidence-final.json` remain ignored under local `target/w1d`.

The first hosted restart checks exposed a three-second connection timeout; it is
now eight seconds within the twelve-second request/fifteen-second outer HTTP
bound, still one attempt. A separate initial synthetic cleanup failure held a
Python SQLite connection open on Windows; explicit closing fixes the cleanup.
These failed runs remain alongside the passing evidence, not converted to passes.

## Use in five steps

1. Use an isolated/dev instance, or a separately authorized stopped complete
   schema20 upgrade. The old D3 schema16 staging helper must not stage this build.
2. Set `LAIN_KNOWLEDGE_CONFIG` to protected JSON containing `endpoint` (exact HTTPS
   service base), `store_id` (from authenticated status), and `secret_ref`; resolve
   that handle from the protected daemon environment using the read key.
3. Register the exact `qq_private` / `qq_group` source and enable finite archive
   retention/capacity. Enabling storage or private recall alone fetches nothing.
4. Run `lain archive knowledge-pull <source> --request-id <uuid> --revision <n>
   --limit 16`. Inspect `archive read-job <uuid>` and `archive knowledge-state
   <source>`; continue with a new RequestId and `--after <last committed receipt>`.
5. Independently enable that source's private recall scope if desired, and recall
   only while caught-up/fresh. Revoke the local archive/recall scope to stop it.
   Current explicit pulls must refresh the short lease; continuous refresh is
   not implemented or silently authorized by this slice.

## Remaining work

Continuous selected-source ingestion and its finite standing network budget,
cross-account lineage, source-linked durable summaries/memory, actual new QQ
image coverage, real-model/multi-day naturalness and W2–W4 devices/execution remain
open. Native macOS/Linux Rust Kernel execution remains unverified here; the actual
server Python tests and Windows-to-server GET do not substitute for it.
## Observed reply references — deployed, 2026-11-06

The independent service already exports normalized `segments` with reply IDs.
The Kernel knowledge adapter now retains those IDs in its existing attributed
archive projection instead of dropping them. At most128 normalized segments are
accepted; reply IDs must be nonempty, control-free and at most80 UTF-8 bytes.
At most4 ordered observed markers retain `{native_reference_id, segment_index}`
and explicit `resolved_target:null`, with an omitted-reference count. These are
raw reply-segment observations, not actual target message identities. The old
unreleased `targets/native_message_id` shape is no longer emitted for references;
the current message's own native identity is unchanged. `lookup_scope=current_source_only`
restricts potential lookup; `target_resolution=not_attempted` reports what the
adapter actually did. Neither claims that the target exists or belongs to that
source. Referenced message bodies are not loaded, verified, inferred or granted
by the reference. `target_chronology=unknown_without_verified_target_clock` also
keeps a reply marker/ID/import order from asserting that an unresolved target
was earlier or later; no target clock has been loaded. The HTTP import regression
checks this field together with the existing ID, scope, update, withdrawal and
no-target-read assertions. Model wording requires separate actual readback.
This refines the unreleased58dd/615622metadata shape; no deployed
reply-reference format needs migration.
Other segment bodies are discarded; original text/image projections retain
their existing limits and the complete projection must still fit8192bytes.

Missing/null segments remain compatible. Messages without replies retain their
old fingerprint/projection. A changed reply ID or position changes the existing
normalized message fingerprint, so ordinary versioning invalidates old derived
results without extending retention. Withdrawal still clears the whole body,
including reply metadata; a tombstone carrying segment data is rejected.
No schema, wire/domain entity, new source authority, extra HTTP lookup or separate
knowledge-service deployment is introduced. The Kernel increment is deployed in
source94922bbab226. Native checks, actual-service/model/nativeTUI readback and
guarded public/backup acceptance retain separate scopes in DAILY_BRIEFING_ACCEPTANCE.
Real-source consent remains pending; one synthetic case is not general model
compliance. No source is enabled by the program update.
