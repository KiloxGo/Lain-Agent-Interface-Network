# D2 private sharing acceptance

Date: 2026-10-26. Contract: [ADR 015](adr/015-bounded-proactive-sharing.md).
Status: implemented and native Windows deterministic/process checks passed;
not live accepted. Complete daily-companion/shared-work goal remains open.

## Scope

Independent contact grant, explicit attention/quiet windows, bounded source-linked
Share/Skip Actor drafts, revision/input/source rechecks, cumulative request/send
budgets, no-response control, distinct NapCat delivery and restart without catch-up.
Default-off migration 014; no personal database, credentials or instance upgrade.

## Automated evidence

Native Windows/MSVC, Rust/Cargo 1.98.1:

- Workspace: **192 passed, 0 failed, 3 manual/real-provider tests ignored**.
  Twelve new D2 tests cover policy/time, actual Actor decisions, stale responses,
  restart, schema-13 upgrade, all-history quotas and current-chat concurrency.
- `cargo fmt --all`, strict workspace/all-target Clippy and locked native build
  passed. After the last code change, full tests and Clippy/build were rerun.
- Native process suite: **25 checks, 69 fake model requests, 0 paid calls**;
  temporary daemon/HTTP/OneBot services and state cleaned successfully.
- Schema 13→14 unit evidence preserves existing binding/grant/experience/Actor
  snapshot bytes, creates no automatic contact and leaves attention unconfigured.
- Binary-only schema 11→14 synthetic upgrade: **5 checks, 3 fake model requests,
  0 paid calls**, complete backup/ACL preservation, old-binary refusal, full rollback
  and no replay. It copied only installed executable files into synthetic state;
  `personal_state_accessed=false`, cleanup passed. Report: `upgrade-final.log`.

Local logs are under ignored `target/d2/`: `workspace-final.log`,
`clippy-final2.log`, `build-final2.log`, `process-first.log` and `evidence.json`.
The first service test exposed a real missing Kernel wake-up when a different
Actor instance published an experience; this is fixed. Earlier failed logs
(SQLite count type, service timeout, test-module Clippy position) are retained.

Final process-tested binary SHA-256:

| Binary | SHA-256 |
| --- | --- |
| lain.exe | `bb0cf0e727d2b20a84bf8137b0df883310a21fb1871056e836514f369101f9bc` |
| laind.exe | `bc7e6edecda03f8ad347d5abc553b6f08adb1ed202ad25f01d9e5781d5e9979d` |
| lain-test-app-server.exe | `c1e64b79b11cab0c2d8a930f0f29fee7131d5952180180caaa4c7ca123bce987` |

The process fixture explicitly seeds a synthetic acquired body, then runs actual
native HTTP Actor processing, a separate sharing scope/attention decision, JSON
Share draft and authenticated local fake OneBot receipts. It does not download a
real source or use a real model. D1a has separate local socket-fetch evidence.

## Daily controls (five steps)

1. `lain sharing show <channel>` inspects independent scope, attention, aggregate
   reservations for the current grant revision and the latest 100 candidates/
   deliveries. Enforcement uses all records. Default scope is off.
2. Prepare a ShareScope JSON containing every field in ADR 015; use
   `lain sharing enable <channel> --revision <current> --scope <path>` only after
   the real recipient/source/budget settings have been authorized.
3. Explicitly choose `lain sharing available <channel> --revision <attention>`
   or `lain sharing dnd <channel> --revision <attention> [--until-ms <UTC-ms>]`.
   CLI revision conflicts require fresh inspection, not blind retry.
4. New eligible reading experiences may produce one draft and possibly a send;
   `lain sharing consider <channel> <experience>` explicitly considers an older
   eligible experience without replaying an existing candidate. Inspect actual
   Actor, source extent and transport outcome before claiming success.
5. `lain sharing revoke <channel> --revision <grant>` closes contact. DND is a
   reversible timing restriction. Reading/processing/reply/memory grants remain
   separate. Revocation cannot retract an already written QQ message.

## Upgrade and rollback boundary

The scope JSON requires these exact fields (there are no omitted-field defaults):

| Fields | Meaning |
| --- | --- |
| `urls`, `expires_at_ms` | Exact canonical HTTPS allowlist and UTC expiry |
| `utc_offset_minutes`, `quiet_windows` | Fixed local-day offset; windows contain `from_minute`/`to_minute` in 0..1439 |
| `cooldown_ms`, `user_quiet_ms` | Attempt cooldown and silence required after genuine user input |
| `max_per_day`, `max_send_attempts`, `max_model_requests` | Local-day send cap, grant-wide attempt cap and grant-wide draft request cap |
| `candidate_ttl_ms`, `source_max_age_ms` | Summary-completion TTL and maximum acquisition age |

Use actual authorized values, not inferred defaults. An empty window array is an
explicit selection; attention is still a separate required decision.

Schema 014 is forward-only. Back up the complete stopped instance (including its
protected credential files, without printing them), preserve old binaries and
entry points, migrate a backup copy first, and validate identity/grants/receipts.
An old binary must refuse schema 014. A rollback restores the entire matching
backup and binary set while both instances are stopped, never just `user_version`
or one database table. Reconfirm actual platform paths/service state at operation
time. This document is a procedure, not proof of an installed upgrade.

## Open acceptance

No paid model requests, real public source, real QQ sharing, natural-language DND,
calendar, DST, multi-day naturalness, personal migration or native Linux/macOS
acceptance is claimed. D3 must add source-linked sharing-receipt conversation
continuity and the authorized complete-chain review. W1–W4 remain unimplemented.
The old C1.1 budget is closed and supplies no authority for these operations.
