# Daily companionship and shared work delivery

Selected by the user: 2026-10-26 (Asia/Shanghai). Both large stages are one active
goal; completing a slice below does not complete the goal. Baseline: `3bfb855`.
Retain existing history and C1.1's failed real responses alongside successful
repairs. Product direction is in PRODUCT.md and ADR 009.

## Delivery sequence

| Milestone | Deliverable | Status |
| --- | --- | --- |
| D1a | Independent exact-URL text acquisition grant, durable bounded activities, acquisition receipts, cancellation/recovery and retention | Implemented; validation in READING_ACCEPTANCE.md |
| D1b | Actor reading/processing receipts, source-linked recent experiences, date/topic retrieval, correct/revoke invalidation | Implemented; validation in EXPERIENCE_ACCEPTANCE.md; live quality pending |
| D2 | Sharing candidates, separate contact grants, explicit timezone/quiet periods/DND, cooldown and no-response/expiry controls; safe QQ delivery | Implemented; validation in SHARING_ACCEPTANCE.md; live contact pending |
| D3 | Complete read → experience → share → later recall chain, cumulative durable model budgets, autonomous replay and authorized live acceptance | In progress: D3a receipt continuity, D3b global ceiling/staging, D3c isolated autonomous runner implemented; real-model/QQ/long-term acceptance pending |
| W1 | One selected read-only social source and one supported visible Worker archive; identity, cursors, gaps, retention and derived invalidation | In progress: W1a–W1e source/recall/sync and W1f selected inferred memory with correction/invalidation implemented; real/long-term acceptance pending |
| W2 | Device pairing/revocation, capability and workspace ownership, durable command/receipt/event/resync protocol | In progress: W2a explicit identity/proof/reported inventory implemented and locally validated; command/event/resync pending |
| W3 | One native PC/Codex execution path, frontend independence, unknown-command reconciliation, true approval and artifact evidence | Pending |
| W4 | CLI/TUI Chat/Work/device integration and bounded authorized follow-up; full cross-chain acceptance | Pending |

Naturalness and continuity remain criteria across these milestones, with regression
cases from C1.1. No global one-sentence rule, punctuation stripping, fabricated
activities, random phrasing or automatic Worker approval. No intent-enhancement
compiler or Web redesign is selected. One additional social source/Worker adapter
comes before broader adapter coverage.

D1a labels an acquisition-only result **AcquiredTextOnly**. D1b adds a separate
optional processing scope with a persisted cumulative budget and a Kernel-initiated
Actor mode with no tools. Only an actual completed summary creates a source-linked
experience; attempted inference never proves comprehension or verification.
Neither scope grants semantic-memory writes or contact. Active/corrected/withdrawn
experiences are recallable only in the selected private binding with fresh evidence.

## Evidence and authorization

Keep deterministic state/protocol tests, real-model simulated-transport replay,
actual QQ/device checks, user feedback and multi-day use separate. Existing C1.1
used 57/60 real requests; that campaign is closed. Development of these milestones
does not reuse its remaining allowance or its temporary personal-instance grants.

Implement and validate locally first. Once there is a concrete build, backup/
rollback plan and scope, consolidate the required live decision: provider/budget,
exact real URL/source allowlists and retention, continuing memory authority,
contact recipient/quiet periods/frequency, target device/workspace and installation.
Absent that decision, live inference, personal migration, new collection, device
installation and proactive QQ remain unaccepted. A local two-process test is not
cross-device/cloud acceptance. Native Windows evidence does not imply Linux/macOS.

Commit/push verified implementation milestones without credentials, personal
archives/DBs or local evidence. Stop the overall goal only after both stages'
completion and integration requirements have evidence; missing live and long-term
checks remain open instead of becoming an implementation-only substitute.
