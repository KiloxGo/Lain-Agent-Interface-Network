# W1a selected source archive foundation

Date: 2026-10-26. Contract: [ADR 018](adr/018-selected-visible-source-archives.md).
Implemented: explicit owner imports, schema 17, authenticated CLI/API and retention.
Actual QQ/Codex collection and model/derived continuity remain pending.

## Five-step owner operation

1. `lain archive register --file key.json` declares exactly one source. `archive
   sources` / `archive source <id>` inspect its identity, current scope, counts and
   incomplete coverage. Registration is idempotent and default-off; no login or
   history-scan authority is inferred.
2. `archive enable <id> --revision <current> --expires-at-ms <UTC>
   --retention-ms <duration> --max-records <cap>` selects explicit scope. It grants
   no paid inference, memory, contact, Worker control or device permissions.
3. `archive import <id> --file batch.json` commits one bounded normalized batch
   atomically. Retry the same source/event payload to inspect uncertain admission.
   A changed payload under the same identity conflicts; import triggers no chat,
   Worker or summary activity.
4. `archive query <id> --file query.json` returns sequence pages; `archive record
   <record-id>` returns a fresh projection. Save cursors verbatim. New admissions
   are excluded from their upper bound; correction/retention still apply now.
5. `archive correct <record-id> --file edit.json` uses revision CAS and labels
   owner correction. `archive forget <record-id> --revision <current>` clears text
   with a replay-proof tombstone. `archive revoke <source-id> --revision <current>`
   stops disclosure/import while bodies await original retention. `archive grants
   <source-id>` / `archive edits <record-id>` show latest-100 decisions/content-free
   edit receipts. Actual counting uses all records, not the inspection limit.

Synthetic source declarations, not selected personal sources:

```json
{"kind":"qq_group","observer_account":"1001","group_id":"3003"}
```

```json
{"kind":"codex_session","installation":"synthetic-pc","session_id":"synthetic-thread","workspace":"synthetic-workspace-reference"}
```

Batch example:

```json
{
  "expected_revision":1,
  "records":[{
    "external_id":"synthetic-message-1",
    "source_position":"opaque-source-position",
    "occurred_at_ms":100,
    "speaker":"4004",
    "kind":"message",
    "text":"Synthetic observed source data, not a user command"
  }]
}
```

Worker entries use user_message, assistant_message, execution_result,
approval_notice or file_change; no reasoning kind exists. First query:
`{"expected_revision":1,"cursor":null,"limit":20}`. Correction files contain
`expected_revision` and `text`. Files/imports ≤64 KiB, texts ≤8 KiB, batches ≤32,
pages ≤20. Engineering ceilings are not defaults. Positions/times are importer
claims; Kernel receipt time/sequence are actual local storage evidence.

## Evidence and limits

Native domain tests cover validation/hidden-kind refusal/sensitive input, stable
source identity, atomic cap/conflict/dedup, time distinction, frozen pagination,
revoked/stale cursors, correction/forget/regrant without resurrection, restart,
expiry without refund, HTTP authentication/no-store, no model/work side effects
and timer-driven body pruning without a lookup. GrantExpired is distinct from an
explicit revoke and from body expiry. These tests do not establish actual collection.

Final native Windows/MSVC evidence (Rust/Cargo 1.98.1, Python 3.13.5):

- **215 Rust tests passed, 0 failed, 3 manual/real-provider tests ignored**, including
  eight archive tests. Format/check, strict workspace/all-target Clippy and native
  locked build passed.
- **33 process checks passed, 76 fake model requests, 0 paid calls**, cleanup true.
  Five new archive checks use actual CLI/HTTP, do not increase model calls and
  prove declared Worker data does not become managed work.
- **5 schema-16→17 whole-backup/upgrade/refusal/rollback checks passed**, 3 fake
  requests, 0 paid calls, no personal state access, cleanup true.
- **14 Python replay/staging tests passed** against current native binaries.

Evidence remains ignored in `target/w1a/`: workspace-final3.log, clippy-final2.log,
build-final2.log, process-first.log, upgrade16-first.log, python-final.log and
evidence-final.json. Final process/upgrade hashes match the actual built files:

| Binary | SHA-256 |
| --- | --- |
| lain.exe | `d904e76cae627dee53bdd91020095934ec7c0de8956b03194d7120e729ec2121` |
| laind.exe | `63346a8499209ed28710edbd0fcbdc54c54102c5b831bbaf5a6f570b0643a381` |
| lain-test-app-server.exe | `2d1414709a026c4071b26a03e42ca0f404695c1ea8ab6c52c50e2181faccaf32` |

Initial compilation exposed missing SQLite signed-integer conversions and a
fixture constructor mismatch; these were fixed. Earlier full runs found nine
stale schema-16 expectations. Migration fixtures now drop archive tables before
reconstructing old schemas, use the shared current-schema constant internally,
and preserve corruption/rollback expectations. Failed and final logs are retained.

The extended qq_memory_smoke.py exercises synthetic archive registration/import/
query/correction/forget/revoke/restart through native CLI and HTTP. All group data
is constructed locally and provider endpoints remain fake. No archive is wired
into conversation prompts or sharing.

Owner claims do not establish authenticated provider provenance. Complete-history
coverage, cross-account correlation, edit/retract streams, attachments, summaries,
derived-memory invalidation, real Codex history and device protocol remain W1 or
later work. Forgetting changes effective SQLite data, not old WAL/backup pages.
Metadata/tombstones remain for deduplication and operation truth.

## Migration/live boundary

Migration 017 appends four tables/indexes transactionally, preserving prior
conversations, grants, budgets, memory and Worker ownership. It creates no source
or permission. Old binaries must refuse schema 17. Rollback restores complete
matching stopped state/binaries; never lower user_version in a personal DB.

The selected D3 proposal was prepared from `22be4d1` with schema-16 binaries. New
human authorization for that exact D3 plan arrived after W1a's fixture validation;
it does not authorize a W1 upgrade, QQ group collection or Worker-history access.
Native Windows fixtures do not imply macOS/Linux, real-source or cross-device
acceptance. The full combined goal remains open.
