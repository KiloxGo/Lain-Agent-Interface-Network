# D3c autonomous isolated replay

Date: 2026-10-26. Runner implemented and exercised on native Windows. **No new
real model, personal-instance access, real QQ or naturalness acceptance.** D3 and
the combined two-stage goal remain open.

## Contract and limits

`scripts/daily_replay.py` launches an isolated native daemon and a local synthetic
OneBot pair (1001 ↔ 2002). It injects new events automatically; no owner-sent QQ
test messages are needed. It neither connects to NapCat nor opens personal state,
fetches URLs, installs a service, approves work, or imports old private transcripts.
Worker execution is disabled; review-required actions are recorded and stop the
corpus rather than being approved. Historical source collection remains off.
Temporary memory authority covers only this synthetic binding and expires at the
campaign deadline; it is explicitly revoked on exit.

The current 16 scenes contain synthetic variants of greeting, passing by, joking,
venting, declining advice, an explicit technical question, disagreement, honest
experience claims, three separately sent burst inputs, goodnight followed by a
question, and remember/correct/recall/forget/after-forget requests. They are not a
complete D3 evaluation: generation-time changes, source injection/reading/sharing,
time-zone/multi-day continuity and actual devices still need their respective
domain/process/live evidence. A corpus request does not prove a memory effect.

Each scene persists actual incoming/batch/Actor IDs, whole normalized turns,
outbound text, delivery receipts, memory effects and a budget snapshot. Reused
Actor IDs across scenes fail. Mechanical line/character/punctuation observations
are review aids, **not** a naturalness score or a punctuation-removal policy.
No extra model judge calls are added. Confirmed means the simulated transport's
acknowledgement; it does not prove actual QQ delivery or human reading.

The runner first boots **without model/QQ configuration**, installs the finite
Kernel budget, stops, then starts the same database with approved configuration.
Binary hashes are checked at each daemon start. Tools and reconsideration spend
the same Kernel quota. Exhaustion/expiry or a failed/pending-review Actor stops
further scenes; no retry, quota transfer or fallback provider exists.

`control.json` lives outside `instance/`, so replacing only the instance cannot
replace campaign identity/deadline. Intent is flushed before input injection.
Existing output is always refused, including after interruption; the runner has
**no automatic resume**. Inspect uncertain work instead of replaying it. Do not
delete controller state or create a new directory/ID to replenish an authorized
campaign. Exit stops the budget, revokes memory, stops the owned daemon, then
boots the same database headlessly to reconcile pending requests as Unknown and
stops again. Closure/recovery failures remain explicit and fail the run.

The final budget counts every reserved request. Request inspection returns at
most the latest 100 entries; this does not cap accounting. Normalized token totals
may omit unreported usage. Real-mode `paid_calls` is null: upstream billing is
unknown. The runner's own Responses client has retries disabled; a separate bridge
must be inspected before authorizing a claim of one upstream attempt per adapter
call. The operator's verification reference records that evidence, not a new
automatic audit of the bridge.

## Modes and five-step use

1. Build native binaries before running processes. For free rehearsal:
   `python scripts/daily_replay.py --mode fixture --out target/d3c/new-free-run
   --bin-dir target/q2a/debug --max-requests 32 --duration-ms 120000`.
   Choose a new directory only for a **new free rehearsal**. Fixture mode refuses
   real Actor configuration and creates its own fake Responses service.
2. Before paid replay, obtain a new human decision covering the exact finite cap,
   duration, binary/config hashes, synthetic memory scope and verified upstream
   attempt behavior. The old 57/60 campaign is closed. A review-only release
   proposal is rejected as authorization.
3. Record that decision in the JSON shape below, bind it to one absolute campaign
   directory and immutable UUIDs, and make the already authorized SecretRef
   available in the launch environment. The runner reads only that one namespaced
   variable, not personal env files or global Worker credentials. Never paste a
   raw credential into arguments, configuration, the record or Git. Native Kernel
   validation still validates the Actor configuration.
4. Run `python scripts/daily_replay.py --mode real-model --out <bound-directory>
   --bin-dir <authorized-binaries> --max-requests <cap> --duration-ms <duration>
   --authorization <decision.json> --actor-config <approved-actor.json>`.
   Real mode creates **no fake Responses server**. All transport events remain
   synthetic; it cannot be used for personal QQ acceptance.
5. Inspect owner-only `control.json` and retained synthetic `instance/` evidence.
   Check actual replies/effects/receipts and final reservations separately. A
   stopped/interrupted run is inspect-only. Real QQ upgrade/contact/source scope
   remains a separate part of the consolidated live decision and rollback plan.

The decision JSON requires these fields (placeholders are deliberately unusable):

```json
{
  "kind": "authorized_isolated_real_model_replay",
  "human_authorization_reference": "reference to the new actual human decision",
  "campaign_directory": "ABSOLUTE_NEW_TARGET_DIRECTORY",
  "campaign_id": "IMMUTABLE_UUID",
  "budget_id": "IMMUTABLE_UUID_FROM_APPROVED_PARTITION",
  "max_requests": 48,
  "duration_ms": 21600000,
  "start_before_ms": 0,
  "expires_at_ms": 0,
  "actor_config_sha256": "APPROVED_HASH",
  "binary_sha256": {"lain": "APPROVED_HASH", "laind": "APPROVED_HASH"},
  "synthetic_memory_tests": true,
  "source_collection": false,
  "worker_execution": false,
  "real_qq": false,
  "upstream_max_attempts_per_adapter_call": 1,
  "upstream_verification_reference": "actual offline inspection evidence"
}
```

The example's 48/6h values are proposals, not product defaults or an allowance.
Authorization binds the directory, cap, duration, fixed UTC deadline, IDs and exact binaries/config;
it cannot be reused at a different path. There is no runtime permission mechanism
that manufactures human authorization from this JSON.

## Evidence

Native Windows, Python 3.13.5 and websockets 15.0.1, using unchanged D3b native
schema-16 binaries listed in INFERENCE_BUDGET_ACCEPTANCE.md:

- Ten Python boundary/process tests passed: exact scope/hash/SecretRef checks;
  tampered/proposal/expired/retrying/missing-secret refusal; immutable controller
  and no overwrite; credential rejection and measurements; real-process cap,
  fresh Actor/receipt/ledger reconciliation; full 16-scene fake corpus; interruption
  before sending; interruption after an actual local HTTP adapter attempt.
  Real-mode setup separately proves no fake provider is constructed and only the
  selected namespaced model credential is inherited; no provider is contacted.
  A crashed daemon is also recovered headlessly with its budget/memory disabled.
- The in-flight interruption retains one spent request, recovered as Unknown,
  no pending requests, disabled memory and no leaked owned process. No replay.
- Four release-staging tests passed after the shared ACL change. The manifest now
  fingerprints the three repository script dependencies; scripts are referenced,
  not copied into the four-file binary proposal package.
- Five schema-15→16 synthetic upgrade/full-backup/rollback checks were rerun after
  that ACL change: all passed, 3 fake requests, 0 paid calls, cleanup successful.
- Free cap-4 rehearsal stopped at 4 fake requests with no fifth input; full corpus
  and all tests use fake providers only. **0 paid calls**. Final logs are ignored
  under `target/d3c/`; no personal state or credentials are committed.

The first local rehearsal failed before startup because the Windows ACL helper
unnecessarily requested WRITE_OWNER while setting an already-owned directory's
owner. It now first verifies current ownership and changes only the DACL, retaining
owner-only verification and rejecting foreign ownership. Subsequent Windows
workspace-directory and temporary-directory staging/process checks passed. Other
early failures (launcher setup prohibited inside Git, then a channel-list response
shape mismatch) are retained in `replay-second`/`replay-third` control records;
neither made a model request. The runner now uses process-only API credentials
and direct typed API configuration, without creating local launcher credentials.

Real-model quality, actual read→experience→share→recall on QQ, personal upgrade,
multi-day use, native Linux/macOS and W1–W4 are **unverified/open**. The runner
supports exact environment SecretRefs; protected personal env-file loading and
bridge inspection must be selected within new access authorization, not inferred
from past tests or this document.
