# W1e finite selected QQ-source synchronization

2026-10-27 (Asia/Shanghai). Contract: [ADR 023](adr/023-finite-knowledge-synchronization.md).
Builds on explicit W1d pulls; the combined large goal remains incomplete.

## Implemented

An independent default-off source grant selects current archive revision, expiry,
request cap, interval and page limit. Each scheduled attempt and immutable read
receipt commit together. A single owned slot and persisted timer reuse existing
source cursors, caption updates, tombstones, freshness, cancel and Unknown boot
recovery. Failed/cancelled/uncertain reads stop rather than retry. Restart never
refills the allowance. Original retention and independent recall grants remain.
Migration 021 enables no old source. No service API change is needed.

## Operation

Use an isolated/dev instance or a separately authorized complete stopped upgrade
to schema21. D3's schema16-only staging helper must not stage this binary.
Configure the exact HTTPS/store UUID/read SecretRef as in
[W1d](KNOWLEDGE_BRIDGE_ACCEPTANCE.md), register the exact source, and enable finite
archive retention. Then write a bounded decision JSON:

```json
{
  "expected_revision": 0,
  "scope": {
    "archive_revision": 1,
    "expires_at_ms": 0,
    "max_requests": 100,
    "interval_ms": 20000,
    "limit": 16
  }
}
```

Replace `expires_at_ms` with the desired future Unix millisecond timestamp, within
the archive scope and 31 days. The zero example is intentionally invalid.

- `lain archive knowledge-sync-enable <source> --file <decision.json>`
- `lain archive knowledge-sync <source>`
- `lain archive read-job <last_read>`
- `lain archive knowledge-sync-revoke <source> --revision <current-sync-revision>`

Authenticated API: GET/POST `/v1/archives/sources/{id}/knowledge/sync`.
The model cannot grant collection. A separate private recall decision is still
required to discuss these records. Finite sync stops at its limit/expiry/failure;
inspect `used_requests`, `pending`, `last_read` and `stopped`. Configuration changes
stop the selected policy at its next wake. No automatic grant renewal.

## Validation

Native Windows/MSVC workspace: **242 passed, 0 failed, 3 manual ignored**;
format, strict workspace/all-target Clippy and locked native build pass. Six new
domain tests cover exact allowance/cursors across restart, atomic reservation/job
rollback, Unknown without replay, source/config/revoke/expiry late-result gates,
invalid/stale decisions, schema20 byte-preserving default-off migration, and
revocation when the decision ledger is full.

The native synthetic bridge passes **6 checks / 10 GETs / 10 durable read jobs**.
It checks actual scheduled caption updates and withdrawal, default-off/auth,
restart without refill, exactly three scheduled successes, in-flight cancellation,
and one failed HTTP 503 attempt that stops without retry. Retained policy usage
still includes all five scheduled reservations after replacement/revocation.

The actual hosted source (only the existing selected owner private source) passes
**3 checks / 4 read jobs**: two explicit pulls plus exactly two separately enabled
scheduled reads, with process restart and allowance exhaustion. Temporary local
copies are removed. It neither changes the cloud source nor claims new real QQ
image or chat-quality evidence. Service store identity is unchanged; the final
authenticated status still reports **1/10 vision reservations / 322 usage tokens**.
No service rollout or model invocation was needed for this compatible client.

The full native process regression passes **36 checks / 83 fake model requests**.
Actual native schema20→21 full stopped backup/upgrade/old-binary refusal/complete
rollback passes **5 checks / 3 fake requests**, with ACL-preserving backup and no
personal state access. All **14 script tests** pass without skips. Bridge runs
create zero Actor turns, inference or semantic memory; all process/bridge/upgrade
reports confirm cleanup and match the final three binary hashes. This slice
initiated **zero real model requests and zero real QQ sends**.

Logs/JSON remain ignored under local `target/w1e`. Initial compile/Clippy failures
and the prior-checkout shared-cache mismatch are retained; rebuilding the matching
wire source resolved the latter before the old-binary upgrade proof. No failed
run is counted as passing. Kernel/macOS and Kernel/Linux native runs were not
available and remain unverified. The personal schema16 instance was not upgraded.

## Remaining

Source-linked durable derived memory and multi-day conversational continuity,
actual new QQ images, W2–W4 device/Worker integration and native macOS/Linux Rust
acceptance remain open. Hosted Python service evidence is separate from Kernel
platform verification. The personal instance remains stopped on schema16.
