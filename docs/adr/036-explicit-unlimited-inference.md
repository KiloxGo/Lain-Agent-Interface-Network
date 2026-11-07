# ADR 036: Explicit global unlimited inference admission

Date: 2026-11-05. Status: implemented; operational activation is a separate step.
Extends ADR017 and the existing single-private-channel daily mode.

## Decision

An authenticated administrator may explicitly select `unlimited:true` through
the existing revision-checked inference management API, or
`lain inference unlimited --revision N`. This removes the aggregate request-count
and expiry ceiling for all Lain Actor reasons. It does not remove per-turn
request/tool/token/time limits, activity limits, source/grant checks or Worker
permission review. It neither enables activities nor supplies a model credential.
The Actor has no tool for changing its quota.

The user explicitly authorized this operational mode on 2026-11-05 after the
finite trial approached exhaustion. There is no new default or automatic refill.
Finite limits, one private binding without an aggregate ceiling, and global
unlimited are mutually exclusive enable modes. Old JSON defaults `unlimited` to
false, and false is omitted when serialized. No database migration is required.

## Lifecycle and audit

Enable requires the current decision revision and no prepared/attempting requests.
Every invocation still reserves and attempts once through the same durable ledger.
Global unlimited has no BudgetId or expiry; admission is Available and remaining
requests is null. Prior finite BudgetIds and their exact spent totals remain
inspectable and cannot be renewed by reopening the same ID.

Stop remains available during requests, retains the previous mode metadata and
denies future reservations/entry. A changed revision also blocks an old prepared
reservation. Return/recovery behavior and uncertain-call accounting are unchanged;
there is no automatic retry or replay. Restart retains the explicit mode.

## Compatibility and rollback

This is an intentional extension of ADR017's finite-only re-enable rule. It is an
explicit audited management decision, never a return to revision-zero defaults.
Old readers ignore the extra field. An old Kernel sees enabled/no finite limit as
stopped for ordinary chat, so rolling back the binary after activation fails
closed. Before such a rollback, stop or select an old-compatible mode through the
revision-checked API; never delete ledger rows or edit database snapshots.

## Validation

Deterministic tests cover removing an exhausted finite ceiling, continued ordinary
chat, old totals, restart, mutually exclusive modes, pending-reservation fencing,
stop-before-entry and restoring the same exhausted finite ID. Existing finite,
private-only, cancellation, expiry and audit tests remain applicable. Actual
deployment/activation evidence belongs in DEVLOG, not inferred from these tests.
