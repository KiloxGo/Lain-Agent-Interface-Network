# Q2-B1 — Bounded private-history source lookup

Date: 2026-10-25, Asia/Shanghai. Implemented in the working tree and validated on
native Windows/MSVC with deterministic providers. No commit or deployment.

## Scope and evidence rules

This slice provides an independent default-off, CAS-revisioned, persistent,
revocable and optionally expiring source-read grant for one existing private
binding. Its explicit half-open received-UTC window is at most 31 days and ends
before the decision; incoming high-water prevents later records entering it.
No new collection, semantic memory writes/restoration, experiences, contact,
Worker permissions or general grant system is included.

Incoming/Batch/ActorTurn/Delivery remain the records of truth. Migration 010 adds
two ledgers and one receive-window index. The index avoids unbounded historical
candidate enumeration before the 512-row scan budget. No duplicate chat bodies,
invented reads or confirmations are backfilled. Existing schema-9 snapshots remain
byte-identical through migration. See [ADR 011](adr/011-bounded-private-source-lookup.md)
and [the contract](MEMORY.md#implemented-q2-b1-private-source-lookup).

## Requirement-to-evidence map

| Requirement | Deterministic evidence |
| --- | --- |
| Default-off, CAS, persistence, expiry, independence from memory | `source_grants_default_off_revision_persistence_expiry_and_no_memory_expansion` |
| Binding/ID/window/query/high-water/cursor isolation, suppression and secrets | `source_window_cursor_binding_suppression_and_sensitive_boundaries` |
| Scan 512, count/8-KiB budget, JSON escaping, UTF-8, next cursor | `source_scan_count_bytes_utf8_and_stable_cursor_are_bounded` |
| Clock rollback, receive-time ties and later backdated inputs cannot bypass high-water | `source_cursor_handles_receipt_ties_clock_rollback_and_late_backdated_inputs` |
| Newly configured opaque credentials checked before excerpting | `newly_configured_credentials_are_checked_in_full_sources_before_excerpting` |
| No source text in live preview before response revalidation | `source_influenced_preview_is_held_and_can_be_discarded_without_text_disclosure` |
| Every selected record re-read, IDs-only receipts, atomic audit failure | `source_prepared_disclosure_rechecks_every_selection_and_audit_failure_is_atomic` |
| Generation, actual send/receipt, Unknown, merged input scope, derived reply withholding | `source_projection_keeps_generated_delivered_unknown_and_merged_scope_distinct` |
| Schema 9 migration preserves raw snapshots, no permissions or reads invented | `schema_nine_source_migration_preserves_snapshots_and_creates_no_fake_reads` |
| Original outside recent 20 turns, restart, actual IncomingId search then inspect, later sanitized history | `source_actor_uses_actual_ids_after_eviction_restart_and_sanitizes_later_history` |
| In-flight revoke/narrow/expiry/disable/forget discards calls and counts consumed tokens | `source_revocation_narrowing_and_expiry_during_generation_discard_text_and_calls` |
| Historical injection cannot approve Work | `historical_instruction_never_approves_work_even_with_valid_source_grant` |
| Authenticated management CLI method/path/body, required range/revision options | `parsed_channel_commands_use_authenticated_management_contracts` |
| Actual native processes, real HTTP/WS frames, no seeded target database rows | Extended `scripts/qq_memory_smoke.py`, below |

All source coverage is `partial_no_backfill`; no match does not prove absence.
PreparedForModelRequest means authorization/preparation, not an HTTP receipt,
successful inference or comprehension. Old model-bound requests cannot be recalled.
Every prior selection in an active turn must remain unchanged and authorized;
opaque provider continuation is preserved only inside that condition. Response
previews are held until revalidation. New model history omits source-derived
assistant/action observations, retaining original user requests and stored audit.
Recognized/known secrets are filtered; encoded/unknown secrets are not reliably
detectable. Historical statements do not become current Active memory.

## Native repeatable validation

Use the native Windows PowerShell workspace, offline locked dependencies, existing
Python/websockets, and an isolated target. Build/test and smoke must be sequential
because Windows locks running binaries. No npm, new package/framework or CI.

```powershell
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
rtk proxy cargo fmt --all --check
rtk proxy cargo clippy --offline --locked --workspace --all-targets --target-dir target/q2a -j 2 -- -D warnings
rtk proxy cargo test --offline --locked --workspace --target-dir target/q2a -j 2
rtk proxy cargo build --offline --locked --workspace --target-dir target/q2a -j 2
```

After that build, one repeatable process acceptance command is:

```powershell
rtk proxy python -X utf8 scripts/qq_memory_smoke.py --bin-dir target/q2a/debug
```

The extended script retains all original Q2-A scenarios and additionally checks
default-off/authenticated source management, grant persistence, actual source ID
search/inspect after eviction/restart, partial no-match/pagination, actual delivery
metadata, IDs-only receipts/audits, Q2-A correction/forget suppression, injected
historical Work instructions and in-flight source revocation. It checks a later
actual model HTTP request contains no old source quote or source capability.
Its provider derives IDs from actual query evidence; target source messages arrive
through the fake OneBot wire. SQLite is inspected only for evidence, not to seed
successful source lookup.

The normal source timeout scenario shortens only its disposable Actor timeout,
withholds an actual HTTP response and requires Unknown, two request attempts, no
additional tool dispatch, no cached source body and successful restart recovery.
The existing `--inject-failure startup|model` paths and the new `source` injection
must exit 1 with bounded cleanup. The JSON success report includes checks, model
request count, zero paid calls, cleanup and binary hashes. Failure diagnostics
retain only safe stage/error locations. Temporary state and fixture credentials
are deleted; no personal directory/database/token is accessed.

Native Windows results (Rust/Cargo 1.98.1, Python 3.13.5, websockets 15.0.1):

| Gate | Actual result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| Strict workspace/all-target Clippy | PASS |
| Locked offline workspace tests | 138 passed, zero failed, three manual tests ignored; 15 suites |
| Locked offline workspace build | PASS |
| Extended QQ memory/source process acceptance | 18 checks passed, 53 loopback fake model requests, zero paid calls, cleanup true |
| Broad `native_smoke.py --platform windows` against the same target | PASS, 496.34 seconds, zero quota; setup/ACLs, launcher/doctor/restart, streamed Actor, Worker review and legacy memory/summary paths |
| Startup / model / source failure injections | Each expected exit 1 and complete temporary-service/state cleanup |

Local ignored evidence: `target/q2a/q2b1-gates.json`, per-gate logs,
`q2b1-qq-process.json` (including tested binary SHA-256), `q2b1-qq-process.log`,
`q2b1-failures.json`, three failure logs and `q2b1-native-smoke.json` / `.log`. Unit and API/CLI tests run real native
binaries where specified; fixtures never establish real-provider quality.
The first extended run incorrectly expected Failed on provider timeout; the
existing Actor correctly reported Unknown. Its fixture assertion and expected
Windows connection-abort handling were fixed; that failed-run log is retained as
`q2b1-qq-timeout-assertion-failure.log`. No product lifecycle behavior was changed
to make that test pass. Historical real-provider failures remain untouched.
The first broad native run was externally cut off at 360 seconds despite live
progress. Its owned launcher/daemon were gracefully stopped and the verified
fixture removed; `q2b1-native-wrapper-timeout.log` is retained. The 496.34-second
rerun completed all checks with a larger outer bound and removed its temporary
state. This wrapper interruption is not a product-failure or real-provider claim.


## Manual upgrade and real acceptance — five steps

These are instructions for a later explicitly authorized real upgrade/test, not
actions performed by development. Each QQ message can use up to four model calls
within existing budgets. Keep personal chat, tokens and backups outside Git.

1. **Select and back up the instance while stopped.** Prefer a disposable separate
   instance first. If later upgrading `F:\Lain-Q1-Acceptance`, retain its consistent
   stopped database/config/old-binary backup and use matching newly built `lain.exe`
   and `laind.exe`. Check the startup script's actual daemon path; changing only
   the CLI does not upgrade the service. Schema 010 is forward-only; rollback
   requires restoring the matching backup, not opening it with an old executable.
   Do not rerun initial setup over existing state or run two QQ consumers.
2. **Verify identity and default-off scope.** With the selected external
   `LAIN_LOCAL_DIR` and its existing protected credential-loading flow, run
   `lain --json channel list`, `channel show <id>` and `channel source-grant <id>`.
   Require authenticated Online and the intended account/peer. Fresh migration
   shows disabled/revision 0. Identify a harmless old IncomingId and receive time
   using the existing raw channel snapshot; a copied QQ account number is not a
   ChannelId. Do not expose private source text in development logs.
3. **Enable only the chosen past window with a short expiry.** Read the current
   revision, then run `channel source-enable <id> --revision <current> --from-ms
   <inclusive-UTC-ms> --to-ms <exclusive-UTC-ms> --expires-at-ms <future-UTC-ms>`.
   End no later than now and span at most 31 days. `source-search --text <keyword>`
   and `source-inspect <id> <incoming-id>` must return real IDs, receive time,
   truncation/coverage and delivery truth. They must reject outside-window IDs;
   memory permission stays unchanged. Copy any next_cursor JSON unchanged.
4. **Test actual conversational lookup.** After harmless original text naturally
   leaves recent context, restart the same stopped-safe instance and ask about it.
   Inspect the resulting ActorTurn: source.search/source.inspect receipts must
   cite actual IncomingIds and `source-reads` must show prepared source IDs under
   the correct grant revision. A fluent answer alone is not acceptance. Distinguish
   “said then” from current memory; Unknown does not mean delivered. If also testing
   already-authorized Q2-A correction/forget, confirm the old source is excluded
   and no semantic record is recreated by lookup. Avoid artificial paid filler.
5. **Revoke and verify later context, then stop.** Run `source-revoke <id>
   --revision <current>` and inspect `source-grant`, `source-grants`, `source-reads`.
   New reads must fail and later replies must not reintroduce old tool-derived
   quotes. A separately chosen short expiry should have the same enforcement
   even if stored enabled remains true. Do not provoke real uncertain sends for
   testing. Record safe IDs/status/results and keep the grant off after testing;
   no observer, group, Worker archive, summaries or Q3 is implied.

## Remaining gaps and uncommitted scope

Native macOS/Linux for this slice, ConPTY, real-provider tool behavior, natural
keyword choice, real QQ reconnect/revoke races and multi-day companionship remain
unverified. Prior failed real-provider acceptance records are preserved. Fake
protocol success is not real-model acceptance and does not finish Q2.

All code/docs are uncommitted. Existing Q1 diagnostics and Q2-A work remain intact;
the Q2-B1 additions are the source wire contract, migration 010, storage/Kernel
projection, Actor revalidation/history isolation, API/CLI reads/grants, tests and
the extended existing smoke. No commit/push/deploy or personal instance migration.
