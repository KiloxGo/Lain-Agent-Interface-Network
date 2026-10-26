# D1a bounded text acquisition

This is the historical D1a/schema-12 evidence at `f4ec0d4`. Current development
adds separately enabled D1b processing and experiences under schema 13; see
[D1b acceptance](EXPERIENCE_ACCEPTANCE.md). Acquisition-only grants remain default
processing-off, including existing snapshots. The personal release remains schema 11.

Date: 2026-10-26, native Windows. Implementation contract: [ADR 013](adr/013-bounded-text-acquisition.md).
This is an acquisition foundation within the [combined goal](DAILY_COMPANION_IMPLEMENTATION.md).
It does not complete experiences, autonomous Actor reading, contact or device work.

## Current verification

Native format, strict all-target workspace Clippy, workspace build and the full
workspace test run passed: **167 passed, zero failed, three manual ignored**.
One additional atomic-rollback test was then added; the focused reading suite
passed all **10 tests**, including that new case. Production code/binary hashes
did not change after the workspace run. Final strict Clippy includes the new test.

The first compile attempt exposed unsigned SQLite parameter conversions and a
test's incorrect ApiToken constructor; both were corrected. A subsequent workspace
build/test attempt ran out of F-drive space before tests; it is a failed attempt,
not a pass. Only verified regenerable repository target artifacts were removed
with Cargo stopped. Successful retries used process-scoped `CARGO_INCREMENTAL=0`,
`CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0`; no global build setting
or user data was changed.

The actual native process regression passed **23 checks** with 61 fake requests,
zero paid requests and complete cleanup. Upgrade rehearsal separately passed all
five checks: schema 8 → 12, full stopped backup/ACL preservation, old-binary refusal,
complete rollback and no replay (three fake requests, zero paid, cleanup true).
The schema 11 → 12 unit test and existing older-schema snapshot tests are distinct
from this schema-8 process rehearsal. No personal DB was used.

Evidence is ignored under `target/d1a/`: gates.json, reading-tests.log,
process-regression.log and upgrade-rehearsal.log. Validated native binary SHA-256:
CLI `979d893908393517bc482c318aa55a9043a925e2cad8bdb3ad9ad6890a45a0a1`,
daemon `1f7444b67b035660e4ae1a441002adea6b45278c6d4d940189a7a32bcc3213fa`.

Tests cover scope/URL validation and special IPs; actual local socket acquisition
with DNS pinning, no auth/cookie, redirects, MIME, declared/chunked oversize, secret
filtering and incomplete bodies; persisted budget/idempotency/Unknown recovery;
late-result cancellation/revocation and retention; schema 11 → 12 preservation;
current-credential disclosure filtering and authenticated management. The separate
maintenance test checks SQLite directly so a result inspection cannot falsely take
credit for timer-driven deletion. Injected transaction failures prove content/state
and grant/body-invalidation changes roll back together without false receipts.

The existing QQ process fixture adds a synthetic interrupted Pending acquisition,
boots the actual daemon and checks CLI/API auth, default-off grant, Unknown/no replay,
idempotency, retained attempt consumption and CAS revoke. It deliberately performs
no public-source GET and no new model call. Successful socket acquisition is separate
unit-level transport evidence; seeded interruption is recovery evidence only.

No real model, public article, new QQ contact, personal upgrade or device connection
is authorized or performed by these tests. Personal release C1.1 remains separate.
Real TLS/public routing, source compatibility, naturalness, multiple-day continuity
and native Linux/macOS are unverified. An acquired body is never a verified fact.

## Operations (administrator; five steps)

1. Inspect `lain reading grant` and select exact allowed UTF-8 plain-text HTTPS
   URLs, expiry, retention, total attempts and size; no defaults imply permission.
2. Apply `lain reading enable --revision <current> --url <exact-url>
   --expires-at-ms <utc-ms> --retention-ms <duration> --max-requests <count>
   --max-bytes <bytes>`. Repeat `--url` only for additional selected documents.
3. Submit `lain reading start <exact-url> --revision <grant-revision>
   --request-id <uuid>`; preserve that ID. A lost response is inspected/repeated
   with the same ID, never treated as permission to start another request.
4. Use `lain reading list` / `show <id>` / `cancel <id>`; Completed with
   AcquiredTextOnly means a complete body was acquired, not read/understood/summarized.
5. Disable with `lain reading revoke --revision <current>`. Old bodies are removed;
   activity and grant metadata remain inspectable with `reading grants --after <n>`.

API: GET/POST `/v1/reading/grant`; GET `/v1/reading/grant/history?after=<revision>`
(100 decisions/page); GET/POST `/v1/reading/activities` (50 latest for list);
GET `/v1/reading/activities/{id}`; POST `/v1/reading/activities/{id}/cancel`.
All use existing Kernel administrator authentication and no-store responses.
No new model tool, public listener or Web dependency.

## Upgrade and rollback boundary

Schema 12 migrates existing schema 11 without enabling grants or starting activities.
Do not replace the running personal binary for a code test. A future authorized
upgrade must first stop the selected instance, take a consistent whole-directory
backup including credentials/ACLs, verify a versioned installed binary and restart.
Older binaries reject the newer schema. Rollback restores the complete stopped
pre-upgrade backup with its matching binary, retaining later state separately;
it does not merge databases or silently downgrade. Use the prior C1.1 rehearsal
mechanism with schema-12 expectations. No personal migration is claimed here.
