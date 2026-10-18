# Phase 6.6 acceptance: durable delegation and result delivery

Updated 2026-10-18. Status: implementation and native macOS deterministic
validation passed: locked workspace build, fmt, warnings-denied Clippy,
83 Rust tests passed / 3 opt-in tests ignored, and full native CLI/PTY fixtures.
Native Windows/ConPTY and Linux are unavailable/unverified. No Phase 6.6 paid
provider run has been authorized or executed. Historical Phase 6.5 real evidence
is preserved in [its separate record](PHASE_6_5_ACCEPTANCE.md).

## Requirements and evidence

| Requirement | Authoritative implementation / deterministic evidence |
|---|---|
| Source conversation/ActorTurn/action, exact instruction, session and specific worker Turn | `delegation.rs`, migration 006, Kernel origin validation; `terminal_delivery_preserves_exact_dispatch_and_completed_actor_without_inference` |
| Atomic session/receipt before launch; one dispatch; denial and duplicate confirmation | `save_with_actor_receipt`, existing Actor reliability tests and delegation tests |
| Actual terminal/Unknown evidence becomes a separate notice without inference or reopening ActorTurn | Store event projection; completed Actor snapshot equality and model-call counts in delegation/native tests |
| Durable cursor/order, isolation and concurrency | Notice-page API/SSE; `conversation_sse_concurrent_results_reconnect_and_authenticated_acknowledgements`, `independent_delegations_return_out_of_order_in_one_conversation` |
| Later Turns in one WorkSession remain distinct | First delegation test submits work.send and checks distinct Turn/report with no stale command failures |
| Crash after commit before publish; duplicate terminal observations; atomic rollback | `notice_commit_is_atomic_idempotent_and_recovers_without_publication` commits without a broadcast and opens actual recovery |
| Partial startup, lost prompt acknowledgement, failure, cancellation and disconnect | `failed_worker_start_retains_atomic_created_session_receipt`, `denial_failure_cancellation_and_restart_uncertainty_are_distinct` |
| Receive-before-ack loss, repeated acknowledgement and cross-conversation rejection | Conversation SSE/ack integration test; notices stay pending until explicit display ack |
| Slow/closed subscribers and bounded queues | `slow_or_closed_notice_consumers_release_bounded_delivery`; production SSE uses the same timeout helper and durable cursor catch-up after broadcast lag |
| Useful, sanitized, bounded actual report/commands/files/errors | Result projection and fixture `result-evidence`/`history-output`; command count remains distinct from Completed |
| No second owner input; preserve partially typed wrapped Unicode | Native `run_delivery_pty` gates the worker until input is partially typed, verifies the automatically displayed report, unchanged Actor snapshot and model-call count, then exact subsequent input; also exit-before-completion, pending-result replay and actual daemon restart |
| Ordinary notice replay deduplicates; no exactly-once claim | `replay_deduplicates_visible_notices_before_acknowledgement`, API cursor reconnect and native acknowledged reopen; ADR 007 at-least-once contract |
| Compact human receipts, exact approval surface, unchanged machine snapshot | Renderer proposal/receipt regression plus frozen JSON comparison; native exact-argument confirmation |
| Notice during another Actor preview; per-turn latency/error state | Renderer interleave regression retains preview identity and reconciles final text once; renderer dedup/timing state resets between ActorTurns |
| Unix/drive/UNC paths and command feedback | Input classification test and native literal-path requests; one-component slash-word ambiguity is documented in CLI.md |
| Phase 6.5 and older persistence remains readable | `phase_6_5_migration_preserves_completed_conversation_without_guessing_links`, existing Actor record, Phase 1 and profile migration tests |
| Boundaries preserved | Shared Rust CLI/Kernel; existing redaction/approval/profile/recovery suites; no Memory, Scheduler, compiler, backend or Web implementation |

Routine entrypoints (zero provider quota):

```text
zsh scripts/check-native.zsh
powershell -NoProfile -File scripts/check-native.ps1
```

Each must run on its actual native host. Native macOS PTY evidence is not ConPTY
evidence. `python3 scripts/native_smoke.py --platform linux` is available for a
native Linux host. Per-platform outcome and exact test totals belong in DEVLOG.

## Opt-in real-provider procedure

Do not run automatically. Obtain explicit owner authorization for the Actor
provider budget and the exact workspace/action before spending quota or creating
Codex work. An earlier Phase 6.5 allowance is not new authorization. A concrete
work.create proposal must still receive its own normal persisted owner approval;
worker permission requests are never auto-approved.

1. Build/install the current Rust CLI and daemon using the documented locked
   Cargo commands. Inspect open work before explicitly stopping/restarting an
   owned daemon with the same database and saved private configuration. Do not
   adopt, kill or replace an unknown service. Credentials stay outside Git.
2. Run `lain doctor`; verify the Actor configuration and supported Codex backend.
   Open `lain chat --timing`. Record ConversationId and the `/status` model-call
   baseline. Diagnostics/local slash commands do not invoke the Actor.
3. Send one request for a read-only inspection of the specifically authorized
   daemon-host project. Review the actual persisted work.create proposal,
   workspace, profile, sandbox and exact task. If it is not the authorized scope,
   deny/stop; do not manufacture a successful receipt or replay the mutation.
4. Explicitly approve that proposal once. Record ActionId, WorkSessionId and worker
   TurnId from the real receipt. Confirm the CLI returns to `You>` while work
   remains independent. If it finishes immediately, record that limitation;
   do not claim that a slow-running-input case was observed.
5. Send no further owner text to trigger completion. Wait for the automatic
   `Worker>` result in the same conversation. It must match the actual worker
   Turn and retain failed/cancelled/unknown distinctions. A pending permission,
   provider omission, timeout or failure is evidence of that limitation, not a
   successful completion. Do not grant additional permission through this script.
6. After the notice, inspect `/status` and `lain notices <conversation-id>` /
   `lain delegations <conversation-id>` / `lain work turns <session-id>` /
   `lain work history <session-id>`. Compare the model-call count with the count
   after submission: merely finishing/delivering work must add zero calls.
   Check exact dispatched instruction and authoritative terminal evidence.
7. Exit and reopen `lain chat --conversation <conversation-id>`. Acknowledged
   notices should not repeat; explicit `lain notices` inspection still returns
   them without inference. A deliberately interrupted render-before-ack can
   replay under the documented at-least-once contract.
8. Optionally, after verifying no open work/approvals, gracefully restart only
   the owned daemon and inspect the same notice/link IDs and cursors. Do not
   create another worker or ask the Actor merely to test recovery.

Store only safe IDs, states, timestamps/timings and content digests in acceptance
evidence outside Git. Never include credentials, raw provider frames or private
project reports in normal logs. Record actual provider/model/backend versions,
omissions and failures. No atmospheric milestone commit is warranted before real
acceptance; ordinary implementation commits remain appropriate.

## Limits and next boundary

Notices are deterministic observations, not model interpretation or semantic
memory. Completion does not prove task correctness. Unknown can be followed by
a later reconciled terminal observation. Results are bounded through the
establishing normalized event; later or unattributed output remains in history.
Old Phase 6.5 receipts are retained, not guessed into new correlations. Delivery
is at least once; lost/failed/unqueued display acknowledgements may cause replay
on a fresh open. Retention remains indefinite with bounded individual records.
Phase 7 stays deferred; native acceptance and any owner-authorized real result
delivery run are the next validation steps.
