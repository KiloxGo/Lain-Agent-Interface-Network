# D3a sharing-receipt continuity acceptance

Date: 2026-10-26. Contract: [ADR 016](adr/016-sharing-receipt-continuity.md).
Status: implemented; native Windows deterministic/process/rollback checks passed;
live quality unaccepted.
This is a D3 slice, not a replacement for its full scope or W1–W4.

## Selected result

Actual read/summary → D2 draft/attempt/receipt → later private conversation can
inspect the precise source-linked outbound words while authorized. After source
correction/withdrawal/revocation, content is withheld but the actual transport fact
remains available. A content-free recent pointer supports follow-up without copying
all old messages or disabling unrelated reviewed memory operations.

## Evidence

Native Windows/MSVC, Rust/Cargo 1.98.1:

- Workspace **199 passed, 0 failed, 3 manual/real-provider tests ignored**.
  Seven new tests cover source/receipt projection, binding/time/topic limits,
  uncertain outcomes, source changes during generation/prewrite, memory isolation,
  actual restart/history and failed-audit atomicity/no provider invocation.
- Formatting/check, locked build and strict workspace/all-target Clippy passed.
- Final native HTTP/OneBot suite: **26 checks, 75 fake model requests, 0 paid calls**.
  It verifies actual typed recall after restart, original source extent and transport
  receipt, correction/withdrawal without old-body replay, and content-free audits.
- Synthetic schema **14→15** upgrade/full rollback: **5 checks, 3 fake requests,
  0 paid calls**, old-binary refusal, full backup/ACL preservation and no replay.
  Only verified previously built schema-14 executables were copied into scratch;
  `personal_state_accessed=false`. Both process suites report successful cleanup.

The acquisition body is explicitly seeded synthetic data. Actual socket-fetch
evidence is separate in D1a's unit tests; this is not public-source acquisition,
real-model output/naturalness or real QQ acceptance. Model request counters are
prepared reservations, not exact billed requests or proof a provider received them.

Final reports match the actual binary hashes. Ignored logs under `target/d3/`:
`workspace-final4.log`, `clippy-final4.log`, `build-final4.log`, `process-final.log`,
`upgrade-final2.log`, `evidence-final.json`; earlier failures/intermediate results
are retained rather than presented as final evidence.

| Binary | Final process-tested SHA-256 |
| --- | --- |
| lain.exe | `fa72720140079a2cd3a3ae92ceff5bf6fbce821f9cff3793829d285c20912471` |
| laind.exe | `937ebe871f0cdc4982cf33d5c68ebb6f24c70ab1ec0cf505b736d452879ac3da` |
| lain-test-app-server.exe | `2111366cad853206ca4d1d07a99a6d9780541a46db7697df41404b2861090f7c` |

Initial tests exposed a missed closed Wire ToolCall registration for the new typed
reads; it was fixed before acceptance. A later fixture incorrectly put a Proposed
memory action on a Running turn; the test now uses the actual model/tool path to
the required PendingConfirmation state rather than bypassing that invariant.
Final review also restricted topic matching to permitted text/source/identifier/
state values, excluding JSON schema keys. A parallel-suite diagnostics warning
capture returned zero; the fixture now installs its global subscriber in an
isolated child before any adapter events and retains its original three-failure,
one-safe-warning assertion. The final complete suite includes that check.

## Operation (five steps)

1. Keep reply, reading acquisition/processing and proactive contact grants separate.
   Closing contact does not by itself erase previous outgoing operations.
2. `lain sharing recall <channel> --from-ms <UTC> --to-ms <UTC> [--text <topic>]
   [--limit 1..3]` searches permitted receipt projections, with partial coverage.
3. `lain sharing receipt <channel> <delivery>` inspects one actual receipt. Check
   `state`, `content_status`, original source revision and actual source extent;
   Confirmed is never a user-read receipt.
4. Source correction/withdrawal or reading-grant revocation prevents future model
   use of old words. Inspect current corrected experiences separately. Withheld
   content is different from a deleted transport fact or recalled QQ message.
5. `lain sharing audit <channel>` shows the latest 100 prepared reads and versions,
   never outbound/source bodies. Ordinary private chat can use the same typed reads
   within existing request/tool/reconsideration caps, without a new send authority.

## Migration and open work

Schema 015 is forward-only. Back up a stopped instance and use matching binaries;
rollback restores the complete old state/binary set, never only user_version.
The synthetic upgrade rehearsal is separate from any real personal installation.
No original credentials, private data or deployed instance are part of this run.

Existing per-grant reading/share request reservations remain enforced across restart.
An explicit new grant is a new administrator budget decision. A concrete new live
campaign cap, real source allowlist/retention, continued memory/contact settings,
installation/rollback authorization and multi-day use remain to be completed in
D3. The previous real-model campaign is closed; none is reused here. Real model,
public source, QQ timing/quality and native Linux/macOS remain unverified.
