# W1f selected source memory

2026-10-28 (Asia/Shanghai). Contract: [ADR 025](adr/025-selected-source-memory.md).
This is one W1 continuity slice; the combined daily-companion/device goal remains
open. Implementation is isolated from the running schema16 personal instance.

## Operation

Use an isolated instance or a separately authorized stopped whole-instance
backup/upgrade to schema22. Schema16 staging scripts must not install these
binaries. A rollback restores the complete old instance and binaries.

First register/import/read the exact selected archive source and enable its
finite retention scope. Enable private memory and independent archive recall for
the desired binding. Then inspect `lain archive memory-grant <channel>` and write
an explicit decision JSON file:

```json
{
  "expected_revision": 0,
  "scope": {
    "sources": [{"source_id": "actual-source-uuid", "grant_revision": 1}],
    "expires_at_ms": 0,
    "retention_ms": 86400000,
    "max_writes": 20
  }
}
```

Replace the source ID/epoch and expiry with the actual selected values. The
placeholder/zero example is intentionally invalid. Expiry must fit the original
archive scope and be within 31 days; retention is also at most 31 days.

- `lain archive memory-enable <channel> --file <decision.json>`
- `lain archive memory-grant <channel>`
- `lain archive memory-grants <channel>`
- `lain archive memory-revoke <channel> --revision <current-revision>`

Authenticated API: GET/POST `/v1/archives/memory/{channel}` and GET
`/v1/archives/memory/{channel}/grants`. The Actor selects cautiously after reading
actual archive excerpts; enabling a grant alone triggers no model call. Inspect
MemoryId/provenance and channel memory-effect receipts to distinguish a committed
selection from generated text. Inferred selections have no individual owner
confirmation, including when a source was administrator-imported/corrected.

Correction/forget uses existing private memory tools and a current user quote.
When a source is unavailable, exact-ID lookup returns only management metadata.
Original archives, past QQ deliveries and backups retain their own lifecycles.
Deleting a derived memory suppresses its contributing source records for this
binding and does not replenish write allowances.

## Validation

Native Windows/MSVC: **254 Rust tests passed, 0 failed, 3 manual ignored**.
Format, strict workspace/all-target Clippy, locked native build and all **14
Python script tests** pass. Ten new domain tests cover default-off/atomic failure and
idempotent accounting; source/policy/retention invalidation; knowledge freshness
without renewal; quoted owner correction; forget/suppression; missing disclosure
and replacement-policy race; flattened dependency/restart behavior; schema21
byte-preserving default-off migration; unavailable-body private management; and
invalid scope/interpretation rejection without spending or persistence.

`scripts/archive_memory_smoke.py` is a native isolated CLI/API/Responses/OneBot
fixture. It exercises actual IDs, source selection, inferred memory, restart,
recall, correction/forget input reconstruction and source revocation. Its scripted
responses establish protocol behavior, not humanlike conversation quality. It
passes **5 checks / 16 fake model requests**, including actual post-correction and
post-forget model input with no old source/memory/answer sentinels. It confirms
that a new quoted user correction remains usable after the old source is revoked.

The full native chat regression passes **36 checks / 83 fake model requests**.
The source bridge regression passes **6 checks / 10 GETs / 10 durable jobs**,
including scheduled caption edits/withdrawal, restart accounting, cancellation
and HTTP failure without retry. Native schema21→22 rehearsal passes **5 checks /
3 fake requests**, with full ACL-preserving stopped backup, original state and
private-chat wire preservation, default-off new authority, old-binary refusal and
complete rollback. All reports confirm cleanup and the same final binary hashes.

Intermediate test failures are retained rather than counted as success: missing
fixture receipts, a fixture exceeding the four-tool bound, an existing daily-mode
test reopening before Actor release, and an upgrade assertion issued after the
fixture daemon had stopped. The Actor dispatch review also found and fixed the
new write tool being classified as a historical source read before its mutation
handler; the native selection workflow covers this route. Logs remain ignored
under local `target/w1f`, including `workspace-final2.log`, `clippy-final2.log`,
`archive-memory-native-final.log`, `process-regression.log`,
`bridge-regression.log` and `upgrade21-final.log`.

This slice initiated zero paid requests and zero real QQ sends. It did not enable
personal sources, ingest new images, migrate personal state or grant Worker
permissions. The separately maintained daily personal instance remains schema16.
Real multi-day
continuity, W2–W4 and native macOS/Linux Kernel acceptance remain open.
