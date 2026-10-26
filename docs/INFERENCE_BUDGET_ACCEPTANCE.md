# D3b global inference budget and review-only release staging

Date: 2026-10-26. Contract: [ADR 017](adr/017-kernel-wide-inference-budget.md).
Status: implemented; native Windows deterministic/process/rollback checks passed;
no live acceptance.
This completes neither D3 nor the combined daily-companion/shared-work goal.

## Selected behavior

One explicit persistent ceiling covers every Lain Actor adapter call across
chat, reading, sharing, tools and reconsideration. Failed/cancelled/uncertain and
prepared-but-unattempted reservations remain spent; no turn/grant/restart refill.
An untouched schema-15 migration retains previous limits and creates no paid grant.
After enable, stop/expiry/exhaustion fail closed until an explicit finite decision.
Worker model usage, upstream proxy retries and actual currency bills are outside
this ledger; the current own HTTP adapters disable client retries explicitly.

## Evidence

Native Windows/MSVC, Rust/Cargo 1.98.1, Python 3.13.5:

- Workspace **207 passed, 0 failed, 3 manual/real-provider tests ignored**;
  format/check, strict workspace/all-target Clippy and locked native build passed.
- Eight new domain tests passed initially and in the full suite. They cover three activity types
sharing a ceiling, concurrent last-slot races, failure/cancellation, stop/reopen,
restart uncertainty, pre-adapter expiry, atomic audit/counter failure and immutable
IDs/revisions/legacy in-flight restrictions.
- Final native HTTP + fake Responses/OneBot suite: **28 checks, 76 fake model
  requests, 0 paid calls**. It proves shared activity accounting, persistent stop,
  same-ID reopening without refund, and rejection of a fresh unbound conversation
  after the last private-chat allowance was consumed.
- Synthetic full backup/upgrade/refusal/rollback checks: **5 from schema 15→16**
  and **5 from schema 11→16**, each 3 fake requests/0 paid calls. The schema-11
  source is a hash-verified copy of installed executable files only, with new
  synthetic state. No personal database/configuration/credentials were accessed.
  Both reports set `personal_state_accessed=false`; all process cleanup passed.

Final reports match actual binary hashes. Ignored local evidence: `target/d3b/`
`workspace-first.log`, `clippy-first.log`, `build-first.log`, `process-final.log`,
`upgrade-final.log`, `upgrade-installed11.log`, `package-tests-final.log`,
`evidence-final.json`. The process acquisition body is explicitly synthetic seeded
state; it does not establish real-source reading/model quality or actual QQ contact.

| Binary | Final process-tested SHA-256 |
| --- | --- |
| lain.exe | `40625db88f3019c3b43be814a9bdf7c63dff25cb9c194301f387faa15f2390e4` |
| laind.exe | `eeda062f049232701a50bc91281bec3f2200d514ca9de6e41b66cb809bf949f0` |
| lain-test-app-server.exe | `b087e3cd3cccc01b469e847c025f99bbec1df1f5701af28e5c66a716d6a4d66d` |

The release packager has four native Python tests: copied hashes/selected files,
tamper/failed-evidence refusal, no overwrite/target overlap, explicit scope and
credential-URL rejection. Owner-only ACL protection is shared with the existing
synthetic upgrade/full-rollback fixture and is rerun there after extraction.
The local draft/final package directories are `target/d3b/review-package-draft`
and `target/d3b/review-package-final`; these are review-only artifacts, not installed
services. The final package is generated after commit with clean Git provenance.

## Daily budget controls (five steps)

1. `lain inference show` distinguishes ExistingLimitsOnly from active, stopped,
   expired and exhausted. No default paid allowance is inferred from migration.
2. After provider/cost authorization, use `inference enable --revision <current>
   --id <immutable UUID> --max-requests <ceiling> --expires-at-ms <UTC>` with no
   pending adapter requests. All original source/contact/memory rights still apply.
3. `inference requests [--budget <UUID>]` and `inference budgets` show content-free
   phases/typed reasons/usage. Counts are reservations, not exact server receipts.
4. `inference stop --revision <current>` blocks future calls. It retains counts
   and does not claim to cancel/refund an already-attempting provider request.
5. Reopen the identical ID to use only its remaining allowance; a changed ID is
   a new explicitly authorized budget. At a campaign boundary also close temporary
   memory/reading/contact grants independently. Never create a fresh database to
   replenish an existing real campaign or silently increase/transfer its caps.

## Review-only release package

`scripts/prepare_daily_release.py` requires final Windows schema-16 test/build/
process/rollback evidence and matching binary SHA-256s. It stages only `lain.exe`,
`laind.exe`, `proposal.json`, and a five-step review/backup/upgrade/rollback note
under workspace `target/`, with owner-only ACLs. Existing output or overlap with
the proposed instance is refused. It never opens that instance or loads secrets;
the proposal is explicitly **not authorized** and records dirty/clean Git state.

All caps/duration/recipient/source arguments are explicit proposals, not product
defaults or real-operation permission. A proposed split such as 48 isolated real-
model replay calls plus 12 personal-QQ calls totals at most 60 only if both fixed
Kernel caps are enforced. It does not create a shared ledger across instances.
No automatic quota transfer, new budget ID or fresh database is permitted during
that campaign. The later runner must persist its original IDs/start deadline and
verify any existing provider bridge's upstream behavior before paid calls.
Campaign controller state must survive a personal-instance rollback; restoration
must close that campaign, not resume it against a legacy daemon lacking this gate.
For a real upgrade, bootstrap without model/QQ configuration, install the approved
finite budget first, then restart with approved runtime settings; compatibility
mode must not admit an unmetered reply before the campaign starts.

Candidate first source: [RFC 20 plain text](https://www.rfc-editor.org/rfc/rfc20.txt),
checked as a public text/plain candidate through browser research on 2026-10-26.
This is not a Lain acquisition or a granted source. The actual Kernel must still
enforce public-address/DNS/TLS/body/retention limits, and the owner must approve
the exact URL/retention/processing/contact settings. No article body is packaged.

## Remaining live work

The autonomous isolated replay runner now has native fixture validation; see
[D3c evidence](DAILY_REPLAY_ACCEPTANCE.md). Inspect current installed state only within new authorization; obtain
one concrete consolidated provider/budget/source/memory/contact/stop/upgrade scope.
Old C1.1's campaign is closed. No paid model, real QQ, credential access, personal
migration, source collection or device installation occurred here. Naturalness,
multi-day continuity, upstream billing and native Linux/macOS remain unverified.
