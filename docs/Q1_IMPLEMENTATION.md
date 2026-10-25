# Q1 implementation and acceptance

Status: implemented; final-revision deterministic gates passed on native Windows,
2026-10-24. See the dated verification below.
Basic real QQ/model private conversation was manually confirmed by the user on
2026-10-24. Full Q1 reliability, experience and deployment acceptance remains open.

C1 (2026-10-25) supersedes Q1's fixed collection window and always-reply flow with
bounded trailing batching, explicit wait/no-reply and draft reconsideration. The
Q1 records below remain historical evidence; current contracts and offline/live
acceptance are in [C1 acceptance](C1_ACCEPTANCE.md) and [QQ_CHANNEL.md](QQ_CHANNEL.md).

## User-confirmed private conversation — 2026-10-24

The user supplied a screenshot of multiple Chinese friend-private exchanges,
including a greeting, a short follow-up, an identity question and a reciprocal
greeting, and explicitly confirmed ordinary conversation works. This is
**user-manually-confirmed real private send/receive evidence**, not a fixture run.
The configured Actor was DeepSeek Flash; no fresh paid call was made by the coding
agent to reproduce the screenshot, and no usage count or model revision is inferred.

Earlier local troubleshooting verified the online Lain account through the
installed NapCat's authenticated interfaces and hot-loaded the saved forward-WS
configuration when port 3001 was not listening. The running Kernel then established
its connection. This does not establish a complete live reconnect/restart test.
The screenshot does not verify grant revocation, long-term memory, delivery crash
boundaries, timing/naturalness over days, cloud deployment or all Q1 exit criteria.
Phase 6.6/7.1 historical real-provider failures remain unchanged.

## 2026-10-24 diagnostics repair

Authenticated `GET /v1/channels/status`, the existing channel snapshot's connection
field, and `lain channel list/show` expose ephemeral Kernel-owned observations.
Handshake stays Authenticating until the account is verified. Restart does not
restore Online; task exit/shutdown reports Stopped. Closed failure categories
cover connection, authentication, account mismatch, identity timeout, protocol and
disconnect; repeated identical failures do not flood logs. No raw error/frame,
credential or chat content is retained in this observation. Existing reply-grant,
deduplication, receipt and Unknown/no-replay semantics are unchanged.

The new deterministic evidence, Q2-A memory boundaries and future manual upgrade/
real acceptance steps are in [Q2_A_ACCEPTANCE.md](Q2_A_ACCEPTANCE.md). This development
does not update the running personal instance or migrate its database.

## Selected slice

One explicitly configured Lain account and bound user exchange text through a
NapCat OneBot 11 forward WebSocket adapter. The Kernel keeps the existing local
authenticated API and Actor. It owns durable channel bindings, incoming messages,
bounded batches, outgoing intents and receipts. Transport authentication, native
identifiers, protocol parsing and connection recovery belong to the adapter.

`channels` contains platform-neutral application/domain contracts; `channels::napcat`
is the composition adapter. Actor, semantic Memory, Worker and `lain-wire` do not
import NapCat types. A fixture channel exercises the same application interface.
This is a conversation boundary, not Utakera's proposed observation Bridge.
Utakera's `BRIDGE_INTEROP.md` and `SOURCE_PROTOCOL.md` at `8e30fd2` were read as
proposals: no shared database, dependency on its runtime or frozen Bridge schema.

## Reliability design

- Binding identity and conversation are durable. The configured native account
  and peer cannot silently change under an existing binding.
- Accept only authenticated, bound private text input. Normalize then validate
  before storage; reject secrets and oversized input. Deduplicate by binding and
  native message identity, not text. Reused identity with different content conflicts.
- Persist input before scheduling. A fixed short collection window and finite
  batch/queue bounds prevent endless debounce or unbounded input accumulation.
  Inputs arriving during generation remain queued; other conversations and Workers
  continue. This slice does not repeatedly cancel inference on every keystroke.
- Persist batch request identity before Actor submission. Actor's existing durable
  idempotency reconnects the batch to the same turn, including commit-before-link
  crashes. Unknown Actor work is inspected, never regenerated automatically.
- Persist generated response separately from outgoing delivery. Claim an intent
  before transport submission. Interrupted claimed sends become Unknown on restart;
  lost acknowledgement never retries automatically. A positive platform receipt
  is not evidence the human read the message.
- Reply permission is a narrow, persisted, revocable binding grant. Check before
  processing and send; revocation invalidates queued work. It never confirms Actor
  actions, accepts memories or approves Workers. Existing confirmation remains.
- No replay/history guarantee from NapCat is assumed. Reconnect restores the live
  channel; downtime may leave a source gap. No blanket exactly-once claim.

## Required evidence before finishing this goal

- Native Windows format, targeted tests, relevant regression tests and lint.
- Fake protocol server: authentication/account mismatch, normal text, self/group/
  wrong-peer/duplicate events, reconnect and receive during inference.
- Real storage with model/channel doubles: batches, recovery, send errors and
  uncertain receipts, revocation, crash boundaries, no permission bypass.
- Configuration and safe deployment instructions, inspectable failures, and a
  checklist of separate real NapCat/model/native-platform acceptance.
- Existing Phase 6.6/7.1 real-provider gaps remain open. Documentation and tests
  must not upgrade this implementation to full Q1 or production acceptance.

## Deterministic evidence mapping

Tests live in `crates/lain-kernel/src/channels/tests.rs` unless noted otherwise.
All fixtures use task-owned temporary databases, fake model responses and loopback
protocol servers. They never use a real QQ account or paid model.

| Requirement / failure boundary | Authoritative test |
| --- | --- |
| Stable mapping, exact-input dedup, conflicting IDs, sticky revocation and immutable route | `durable_binding_dedup_grants_and_restart_never_resurrect_sends` |
| Actor commit before batch linkage; interrupted dispatch claim becomes Unknown | Same test, actual database close/reopen at both boundaries |
| Split input, acceptance during blocked generation, recent conversation and persona | `batching_and_new_input_during_generation_use_same_conversation_without_blocking` |
| Offline generated reply revoked, explicit re-grant does not resurrect it | `offline_draft_revocation_and_explicit_regrant_do_not_replay_old_work` |
| Unknown outcome is never retried; API credential/pattern input rejected before storage | `uncertain_transport_is_attempted_once_and_sensitive_input_is_not_saved` |
| Group/self/wrong account/peer/subtype/media filtering, literal CQ output boundary | `napcat_filter_is_fail_closed_and_literal_text_cannot_become_actions` plus wire test's explicit text segment |
| Queue capacity, fixed merge window, batch bound, expiry and oldest work outside inspection page | `queue_and_batch_limits_expire_without_catchup_or_snapshot_starvation` |
| Restart before submission, during inference and after saved response | `restart_resumes_prepared_work_but_never_regenerates_interrupted_or_ready_work` |
| Revoke during generation; reject old-grant and expired submissions | `revocation_during_generation_prevents_delivery_and_stale_submission` |
| Failed input-to-batch and batch-to-outbox transactions cannot expose partial state | `persistence::channels::tests::failed_batch_and_outbox_commits_roll_back_all_linked_state` |
| “同意” cannot activate memory; candidate remains subject to existing review | `chat_agreement_cannot_confirm_memory_mutation` |
| Administrator auth, NapCat token separation and stale-revision rejection | `grant_api_requires_kernel_auth_and_compare_and_swap` |
| Real WebSocket fixture: header, account verification before ingest, reconnect, concurrent input, lost receipt | `napcat_wire_identity_reconnect_receive_during_model_and_lost_receipt` |
| Matched rejection, malformed success, unrelated echo and missing receipt timeout | `napcat_failed_malformed_and_missing_receipts_never_retry` |
| Running subprocess Worker survives channel chat/revocation; terminal Actor still works | `tests/actor.rs::channel_chat_and_revocation_leave_running_worker_and_terminal_chat_independent` |

The last full native Windows workspace run completed with **111 passed, 0 failed,
3 intentionally ignored** (two manual real Codex probes and one interactive ConPTY
test). Format checking, workspace Clippy with `--all-targets -- -D warnings` and
workspace build also passed. This includes 12 channel tests, transactional rollback
coverage, the Worker/terminal independence integration test and native CLI ACL tests.

After that run, protocol review corrected outgoing `send_private_msg.user_id` from
a JSON string to a number and updated the wire fixture assertion. Configuration
and internal identities remain strings. The user requested pushing then doing
testing separately, so the correction was not rebuilt or retested at that
checkpoint. The 2026-10-24 verification below covers the corrected tree; the
earlier run remains pre-correction evidence.

Environment: native x86_64 Windows/PowerShell, Rust 1.99.1. The initial LLVM-MinGW
20261020 build passed Kernel tests but the CLI test executable failed before test
entry with Windows access violation `0xc0000005`. This is not a passing CLI result.
The successful full run used GCC 16.2.0 / MinGW-w64 14.0.0 MSVCRT with
`RUSTFLAGS=-C link-self-contained=no`, matching the documented
[Rust Windows GNU toolchain contract](https://doc.rust-lang.org/rustc/platform-support/windows-gnu.html).
No WSL, emulation or cross-compilation counts as native acceptance. MSVC was not
verified at that checkpoint; the follow-up below supplies separate MSVC evidence.
macOS and Linux remain unverified for Q1.

## Native Windows final-revision verification — 2026-10-24

Rechecked `560ddf9`, including the numeric outgoing `user_id` correction, on native
x86_64 Windows/PowerShell with Rust/Cargo 1.98.1, `x86_64-pc-windows-msvc`.
No GNU linker override, emulation, real QQ account or paid model was used.

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed.
- `cargo test --offline --locked --workspace --no-fail-fast`: **111 passed,
  0 failed, 3 intentionally ignored**, including the numeric wire assertion.
- `cargo build --offline --locked --workspace`: passed. MSVC emitted two
  informational linker-output warnings about creating `.lib`/`.exp` files.

The initial Clippy attempt failed before code checking because direct crates.io
access timed out and the local index lacked locked `webpki-roots` 1.0.10. The
successful retry used the existing Windows proxy in the Cargo process only;
no global proxy/Cargo configuration or lockfile was changed. Tests and build then
used the populated cache offline.

The native CLI smoke exposed test-harness portability issues: its ACL fixture
inherited incompatible PowerShell 7 modules, its saved-path assertion compared
Windows path spellings rather than file identity, and its streaming fixture's
one-second pause could end before the Windows CLI inspected Running. The harness
now selects Windows PowerShell's own modules, uses `os.path.samefile`, and gates
fixture completion on the actual CLI history snapshot. Its stale schema-7
assertion is updated to schema 8. These changes affect fixtures only; no production
permission, provider or channel behavior changed.

After all functional assertions passed, the Windows cleanup also exposed an open
SQLite inspection handle: Python's connection context manager ends a transaction
but does not close the connection. The inspection now uses `contextlib.closing`.
A focused Windows check executes the actual inspection block against a temporary
schema-8 database and verifies that its file can immediately be deleted.
The Windows CLI fixture timeout is 90 seconds rather than 30: repeated native
PowerShell ACL checks exceeded the old bound during a cold local setup retry.
Other hosts retain the existing 30-second bound; no product timeout changed.
The fixture's launcher-readiness deadline is also 90 seconds on Windows (20 on
other hosts), so an env-file/ACL startup is not interrupted before the daemon can
install its signal handlers. A retry had exceeded the old 20-second bound before
the daemon began serving; it is recorded as a failed harness run, not a pass.

The final `python -X utf8 scripts/native_smoke.py --platform windows` rerun exited
**0 / PASS** with the repaired harness. It covered private setup/reuse/permissions,
native launcher/shutdown/restart, doctor, authenticated CLI, actual streaming
before completion, action confirmation, fake Worker independence, scoped memory
and summary commands, Actor memory retrieval and restart persistence. Temporary
state cleanup completed. All providers were loopback fixtures and consumed zero
quota. The embedded acceptance runner still reports native interactive/ConPTY and
full real-provider acceptance as incomplete; those were not silently promoted by
the successful deterministic smoke.

These results close the deferred final Rust revision gates. Live NapCat/model
conversation, installed-version compatibility, persona/timing, deployment,
interactive ConPTY and other native hosts remain separate acceptance work.

Reproduce the deterministic gates with a working native Rust toolchain:

```text
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --no-fail-fast
cargo build --locked --workspace
```

The fixture/model tests do not need `LAIN_NAPCAT_CONFIG`, QQ login, model credentials
or an existing personal database. They are not permission to enable the manual
ignored Codex probes. For focused diagnosis use `cargo test --locked -p lain-kernel
channels::tests` and the separate persistence/Actor tests named in the table.

## Upstream basis

Reviewed 2026-10-23: [NapCat configuration](https://napneko.github.io/config/basic),
[event definitions](https://napneko.github.io/onebot/event),
[API compatibility](https://napneko.github.io/develop/api).
The documented V4/OneBot 11 surface includes forward WebSocket, array message
segments, `get_login_info` and `send_private_msg`. Actual installed NapCat version
and message identity/receipt behavior must be recorded during live acceptance;
documentation and protocol fixtures do not establish a tested runtime version.
