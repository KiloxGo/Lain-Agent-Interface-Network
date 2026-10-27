# W1b visible Worker archive read acceptance

2026-10-27 (Asia/Shanghai). Scope: [ADR 020](adr/020-explicit-visible-worker-archive-reads.md).
This is the explicit selected Codex read/import slice, not W1 or the combined goal
completion. The independently deployed QQ source store remains separate.

## Delivered

- An authenticated, explicit CLI/API read operation for a registered
  `codex/local` source with exact thread/workspace, current finite archive grant,
  immutable RequestId and bounded page size. No collector starts merely because
  a source/import grant exists.
- Native 0.160.1 metadata/turn/item pagination, terminal-turn selection, active-turn
  waiting position, source/grant/version-bound private continuation and visible
  text/command/path normalization. Reasoning, hooks/raw responses/diffs/unknown
  bodies are excluded; omitted items are counted. Observing is not participation
  or independent verification of a Worker report.
- Migration 018, durable read states/receipts, idempotency, atomic import/cursor
  completion, one owned transport slot, cancellation/revoke/shutdown and Unknown
  recovery without replay. No WorkSession, ActorTurn, semantic memory or model call.
- Independent `archive_read_version` diagnostics. Managed Worker control remains
  restricted to verified 0.160.0; 0.160.1 read support does not relax that boundary.

## Deterministic/native protocol evidence

Native Windows/MSVC workspace: **223 passed, 0 failed, 3 manual ignored**. Format,
strict workspace/all-target Clippy and locked native build passed. New coverage:

1. Three actual subprocess/HTTP contract tests exercise pagination, dedup,
   owner idempotency, cross-source continuation rejection, reasoning/diff exclusion,
   source identity mismatch, credential rejection and active-turn revisit.
2. Cancel, source revoke and Supervisor shutdown stop a blocked reader without
   publishing late history. Native process queries confirm the fixture child no
   longer exists after shutdown. No control method is accepted by the fixture.
3. Three persistence tests cover receipt/import transaction failure, unchanged
   archive after rollback, owned continuation, Prepared/Reading boot recovery,
   no replay, single unfinished admission and revoke blocking late publication.

The full suite exposed an existing 30 ms SourceGrant expiry test that could expire
during SQLite fsync under load. It now tests exact timestamp boundaries and forces
persisted expiry deterministically, preserving the behavior assertion. The failed
run and intervening SQL integer-conversion compile fix remain in ignored logs.

The native fake OneBot/model process suite passes **33 checks / 76 fake calls**;
the synthetic **schema17→18 full backup/upgrade/old refusal/full rollback** suite
passes **5 checks / 3 fake calls**. Both report 0 paid calls and cleanup true.
These checks use synthetic instances, not the personal database.

## Actual native Codex read evidence

Installed CLI **0.160.1** generated the matching schemas. An isolated CODEX_HOME
with a local fake Responses endpoint produced one completed native synthetic turn
(one fake request, zero real-provider calls, no tools). The process exited.
Metadata/turn/item APIs then read persisted user/assistant messages after restart,
without thread/resume or another model request. An initially unused thread had
returned method errors; those are retained instead of called empty-history success.

Using the actual final Lain CLI/daemon against this same explicitly selected
synthetic source produced two Completed one-item read jobs and two archive records
with `adapter_observation` origin. There were **zero read-side Worker turns and
zero Lain inference reservations**. Native `thread_items`/`thread_turns` rows were
unchanged. Source revoke withheld both archived bodies; owned daemon stopped.
The final binary hashes match the process/upgrade evidence. No personal Codex
history was read, copied or uploaded, and no real provider budget was reused.

Protected/ignored evidence lives under local `target/w1b`, including final
workspace/Clippy/format/process/upgrade logs and `native-kernel-final-report.json`;
the synthetic native fixture home/schema output is outside source history.

## Use in five steps

1. Run the new binary on an isolated/dev instance or follow a separately
   authorized complete stopped backup/upgrade plan. Personal Lain remains on
   schema16; it has **not** been upgraded for W1b.
2. Register an exact CodexSession key using installation `codex/local`, the native
   session ID and matching canonical local workspace; no directory scan.
3. Explicitly enable a finite archive grant (expiry, retention, record ceiling).
4. Run `lain archive read <source> --request-id <uuid> --revision <current>
   --limit 4`; inspect `archive read-job <uuid>`. Use returned `next` with a new
   explicit read's `--after`; a waiting position is not an automatic polling job.
5. Query stored records; cancel with `archive cancel-read`, or revoke the source.
   Cancellation/revoke cannot erase prior source disclosure/provider history;
   existing retention, correction and forget rules still apply.

## Still open

No actual authorized personal Worker archive, automatic ongoing collector,
source-to-Actor continuity or cross-store derived invalidation is accepted here.
The read contract requires paginated 0.160.1 and an available matching local
workspace; unknown versions/legacy-only or missing workspaces fail closed. macOS
and Linux **Rust/native Codex** execution remain unverified; the independent QQ
Python service's Linux tests do not close that gap. W1 continuity, W2–W4 devices/
execution, real daily-contact recall and multi-day naturalness remain active work.
