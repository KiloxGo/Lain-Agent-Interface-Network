# C1.1: upgrade rehearsal and real conversation acceptance

Started: 2026-10-25; final checkpoint: 2026-10-26 (Asia/Shanghai).
Status: bounded authorized review and repairs delivered; final personal build is
Online. Broader daily-use naturalness remains mixed and is not declared accepted.

Current release: `F:\Lain-Q1-Acceptance\releases\c1-1-0c5048d155ee`.
Start entry: `F:\Lain-Q1-Acceptance\start-c1-qq-test.ps1`.
The final CLI/daemon SHA-256 values are
`05f5f43c075191fde1f32e76acf40698243b8bfea0a14fedfa3757e104bd2d83` /
`0c5048d155eee8b44f80f39cbd1ac3f40282fe46cdb6ee272babca2ab15f9531`.
The final private chat revision is `7a19d690…`; identity/configuration revisions
remain unchanged. Temporary personal memory permission was revoked (revision 2),
historical source permission stayed off, and original reply permission is retained.

The campaign used **57/60 real model requests**: 28 in user-led actual QQ scenes,
29 in explicitly requested autonomous real-model/local-OneBot replay. Reported
aggregate usage is 255,754 tokens; provider-resolved model revision, token billing
breakdown and actual currency cost were not exposed and remain unavailable.
There will be no further automatic real-model probes in this campaign.

This record continues [C1](C1_ACCEPTANCE.md), not a new chat framework or Q2/Q3
milestone. C1's 157 tests/22 process checks remain historical deterministic
evidence. No real naturalness improvement is inferred from synthetic replies.

## Preparation observations before authorization

Read only process/file metadata, startup-script AST structure and selected
non-secret client/Actor/NapCat settings. Credential files, personal SQLite content
and authenticated personal APIs were not opened/contacted during preparation.

- Existing instance directory: `F:\Lain-Q1-Acceptance`.
- Existing startup script hardcodes the repository's `target\debug\lain.exe`.
  The observed launcher/daemon executable paths match that old directory.
- Saved local endpoint and observed listener agree on loopback port 18787. The
  saved database path is inside the same instance directory.
- Actor/env-file selections refer to that directory. The configured model is
  `deepseek-flash`, driver `openai-responses`; SecretRef handles are reused, not
  copied as values into source or reports. Configuration is not proof of live
  provider readiness or resolved provider model revision.
- The new `target/q2a/debug` binaries match the accepted C1 binary hashes. They
  are different files from the running old release; rebuilding a different
  target directory has not upgraded the instance.
- The actual personal schema was not read. The observed old executable creates
  schema 8 in the isolated rehearsal; this does not prove the personal DB version.

Non-secret hashes, PID/path observations, normalized configuration metadata and
startup AST projection are kept under ignored `target/c1-1/`, not in semantic
memory. The script was parsed, never executed, and referenced credentials were
not loaded. Account IDs are omitted here; the authorization must match the
existing single binding rather than adding another contact.

## Repeatable isolated upgrade and rollback rehearsal

```powershell
rtk proxy python -X utf8 scripts/qq_memory_smoke.py --bin-dir target/q2a/debug --upgrade-from target/debug --timeout-seconds 600
```

The optional mode copies the selected native old/new executables into its own
temporary root. All state is synthetic. It reuses the existing fake OneBot and
Responses services, real native `lain`/`laind`, API authentication, signal/cleanup
logic and assertions. It never accepts a personal database path. No Cargo build
may run against the executable inputs while fixtures use them.

Native Windows rehearsal passed all five checks:

1. Actual old binary creates schema 8, a received input, stable conversation,
   confirmed delivery and an independently selected explicit memory record.
2. After graceful stop, the whole fixture state is backed up byte-for-byte with
   protected owner-only directories. Windows `robocopy /COPY:DATS` preserves file
   DACLs; copying only the main SQLite file or using plain `copy2` is insufficient.
3. The new binary migrates to schema 11, preserves IDs/text/memory authority and
   confirmation fields, leaves memory/source grants off and audits empty, runs a
   new actual C1 turn, and does not replay the old input/delivery.
4. The old binary refuses schema 11 without downgrading it.
5. Stopped upgraded state is moved to an owned quarantine, the complete snapshot
   is restored, its DB hash matches before startup, and the old binary boots and
   serves the same memory/conversation/receipt without replay. Post-upgrade data
   is not silently merged into the restored old database.

This used three local fake model requests, zero paid calls, no personal state,
and complete cleanup. Evidence: `target/c1-1/upgrade-rehearsal.log` (including
binary SHA-256). Earlier failures are preserved: the backup ACL setter requested
`SeSecurityPrivilege` in this environment. Using .NET's directory access-control
setter restricted to the owner/DACL succeeded without elevation; owner-only
rules and native CLI permission reuse were still verified. This was a fixture
preparation issue, not evidence of a production migration or chat-quality fix.

Python AST validation passed. The existing normal mode again passed all 22 C1
process checks with 61 local fake requests, zero paid calls and complete cleanup;
see `target/c1-1/normal-regression.log`. The launcher candidate passed inspection
against non-secret settings and refused a tampered executable before launch.
Native macOS/Linux rehearsal remains unverified. The preparation itself changed
no Rust behavior or persona; subsequent evidence-driven repairs are recorded below.

## Authorized personal operation

The concrete per-instance plan and launcher candidate are under `target/c1-1/`.
The user explicitly authorized the consolidated proposal, a maximum of 60 model
requests, existing private binding only and a temporary memory grant of at most
one hour, with historical source access off. The user then stopped the old
foreground service. The procedure is:

1. **Admission and stop.** Confirm exact directory/binding, opaque whole-state
   backup/rollback, existing SecretRefs, QQ reply scope and model-call budget.
   Inspect active Work/approvals before stopping. If anything is active, defer;
   do not cancel work implicitly. The old foreground console is not an owned
   fixture process group and there is no remote shutdown route: use Ctrl-C in
   that specific existing window, then verify the old processes/listener are gone.
   Do not use broad `taskkill` or force a backup while SQLite may still be open.
2. **Backup and stage.** Create a fresh protected owner-only backup directory
   outside Git. Copy the complete stopped instance, including WAL/SHM if present,
   settings, scripts and opaque credential files. Save the exact old binaries and
   file hashes too. Verify all bytes and ACLs before proceeding. Reject links or
   external database/configuration paths outside the approved scope. Stage only
   the new real CLI/daemon into a versioned release directory; never install the
   fake Worker. Preserve the old startup entry and original configuration.
3. **Start and verify.** New entry explicitly selects the same local directory,
   NapCat file and matching new `LAIN_DAEMON_PATH`; it must not fall back to the
   old sibling daemon. Use saved Actor/env-file references. Confirm actual process
   executable hashes, doctor/readiness, binding identity and Online. Saving a file
   or seeing Ready alone is not proof the correct provider/binary is running.
4. **Human-led scenes and budget.** Follow the twelve C1 scenes in small batches;
   user sends the real QQ inputs. Reserve up to four model requests per planned
   input before each batch, then reconcile actual counters before the next.
   Include waits/reconsiderations/errors and unknown usage; do not treat requests
   as free because output was discarded. Stop arranging inputs when the remaining
   budget cannot cover the next batch. Do not send unsolicited probes. For memory
   cases, obtain explicit short-lived binding-only grant authorization, use harmless
   test facts and revoke it afterwards. No historical source grant is enabled by default.
5. **Record or roll back.** Record actual replies, IDs, configuration/persona
   revision, input gaps, timings, request/token counts and user ratings. Costs or
   provider-resolved revision not exposed by the provider stay unavailable, not
   zero. Restore temporary grants deliberately. If rollback is required, stop the
   new instance, retain its full state separately, restore the complete pre-upgrade
   snapshot and exact old binaries, then verify old startup. Never downgrade the
   upgraded DB or overwrite it with a mixture of old/new files.

The old/new executable hashes identify the build precisely; crate version alone
can be identical across these uncommitted builds. Rollback can lose visibility of
post-upgrade chats from the restored old database; quarantine preserves them for
an explicit later decision, not automatic import.

## Foreground commands after authorized installation or restoration

The new entry and diagnostic commands have now been run against the personal
service. Rollback commands remain conditional and have not been used on personal
state. These commands do not perform backup, installation or restoration. First
complete and verify the corresponding operation above. The new foreground launcher
is controlled by this task's terminal session for graceful shutdown.

After the approved new release and candidate entry have been installed, start in
the original foreground window:

```powershell
rtk proxy powershell -NoProfile -File 'F:\Lain-Q1-Acceptance\start-c1-qq-test.ps1'
```

In a separate clean PowerShell window, use the same local instance selection for
read-only readiness/channel diagnostics. These commands use the saved authentication
reference; they do not invoke the model. Do not paste raw authentication material:

```powershell
$env:LAIN_LOCAL_DIR = 'F:\Lain-Q1-Acceptance'
$env:LAIN_NAPCAT_CONFIG = 'F:\Lain-Q1-Acceptance\napcat.json'
$env:LAIN_DAEMON_PATH = 'F:\Lain-Q1-Acceptance\releases\c1-1-0c5048d155ee\laind.exe'
rtk proxy 'F:\Lain-Q1-Acceptance\releases\c1-1-0c5048d155ee\lain.exe' doctor
rtk proxy 'F:\Lain-Q1-Acceptance\releases\c1-1-0c5048d155ee\lain.exe' channel list
```

For rollback, stop the new foreground service first, retain its state in the verified
quarantine, restore the complete stopped snapshot with its DACLs, and verify file
hashes **before** startup. Do not run an old binary on the upgraded database. After
that restoration, this clean-shell entry selects the exact archived old daemon
instead of whichever binary now happens to be under the repository's target:

```powershell
$env:LAIN_LOCAL_DIR = 'F:\Lain-Q1-Acceptance'
$env:LAIN_NAPCAT_CONFIG = 'F:\Lain-Q1-Acceptance\napcat.json'
$env:LAIN_DAEMON_PATH = 'F:\Lain-Q1-Acceptance-Backups\c1-1-20261025-190141\old-bin\laind.exe'
rtk proxy 'F:\Lain-Q1-Acceptance-Backups\c1-1-20261025-190141\old-bin\lain.exe' up
```

The backup path is the current proposal. If admission selects a different fresh
backup directory, record and use that actual directory consistently. These examples
are for dedicated shells; close them afterwards so launch overrides do not leak into
later commands. The original startup entry remains preserved as historical state.

## Live evidence and feedback

At 22:56 on 2026-10-25 (Asia/Shanghai), the accepted C1 binaries were started from
the versioned personal release. Full stopped backup and exact old binaries are
under `F:\Lain-Q1-Acceptance-Backups\c1-1-20261025-190141`. All eleven snapshot files
matched byte hashes and DACLs. Live migration changed schema 8 to 11; all 38 old
inputs, 38 batches and 38 confirmed delivery rows remained identical in their old
columns. There were no pre-existing semantic memory rows. Existing settings and
the original startup entry were retained. Personal rollback has not been exercised.

The actual new CLI/daemon paths, saved Actor configuration and persona/chat
revisions agree with `doctor`; the channel passed identity-gated Online. Startup
reported zero model calls, with memory/source grants still off. Model configuration
is `deepseek-flash` via `openai-responses`; provider access and actual quality are
not proven by startup. The configured Codex Worker is unavailable because its
installed version is outside the current adapter's verified version; this does
not validate Worker execution and is outside the selected chat change.

During preparation of the real backup parent, setting owner and DACL together
failed before any credential copy. Setting only the DACL and verifying the existing
current-user owner succeeded. The stopped full-state copy then completed. No
privilege elevation or relaxation of owner-only access was used. An initial Python
path comparison also rejected Windows extended-path spelling; normalized comparison
and a separate no-links/no-hardlinks preflight passed before the successful copy.

The first batch completed six user-sent greeting/joke/venting inputs using six
Actor model requests and 21,516 reported aggregate tokens. All turns completed and
all six deliveries were confirmed; Actor total times were 662–1,251ms, excluding
the preceding quiet window and the user's QQ rendering/network latency. Provider
cost is unavailable. The user said the content was generally acceptable but lacked
human feeling: two sentences repeatedly separated by a blank line, with conspicuously
regular punctuation. All six replies contained a paragraph break, including the
response to an explicit request not to give solutions. The response respected that
boundary, but the repetitive form is a concrete expression failure.

Preserved before-change replies and actual user feedback in the protected backup
as `live-before-expression.json` and `expression-feedback.json`. Adjusted only the
private expression guidance: stop when one sentence suffices, do not default to
two-sentence/blank-line replies or sentence-final periods in casual short messages;
retain punctuation/structure for serious explanations. No text stripping or fixed
replacement response was added. Retest and user judgment remain pending.

The expression-only revision passed format, strict workspace/all-target Clippy,
all ten Actor integration tests and a workspace build. The first build failed
with disk-full error 112; after checking the exact path and absence of compilers,
only the generated `target/q2a/debug/incremental` cache was removed, and the build
passed with process-scoped `CARGO_INCREMENTAL=0`. No source, personal state, backup
or installed release was removed. About 1.4GB remained free at that checkpoint.
All 22 process checks then passed with 61 fake requests, zero paid calls and cleanup
true; see `target/c1-1/expression-process-regression.log` and
`expression-validation.json`. These checks establish regression safety, not naturalness. The first real retest failed to improve the repetitive format, as recorded below.

At 23:12 (Asia/Shanghai), the task gracefully stopped its first new foreground
launcher and switched to `releases/c1-1-1ad96240602f`. Hash-verified real CLI/daemon
and an updated hash-checking entry were installed with owner-only access. The prior
release and its exact entry remain separately retained. The live configuration and
identity hashes are unchanged; private chat revision changed from `5801db0f…` to
`ab3e00bb…`, with identity-gated Online. The first six turns and request total stayed
unchanged after restart, with no new replayed inference. Three fresh short-message
variants completed with three further requests. All three still contained two
paragraphs and sentence-final periods; the user's screenshot confirms the same
display. The first expression revision therefore did **not** establish an
improvement. It remains a failed real retest, not a passed naturalness check.
The cumulative ledger is nine requests and 32,359 reported tokens, leaving 51.
Memory/source grants remain off.

Inspection found the current expression instructions assembled into private
requests, without adapter-inserted paragraph breaks. A bounded inspection of the
existing conversation found prior user notice of sentence-final periods and an
assistant response in the same two-paragraph form, not an instruction requiring
that form. History reinforcement is a hypothesis, not a proven provider defect.
On 2026-10-25, the [official DeepSeek Responses reference](https://api-docs.deepseek.com/api/create-response/)
and [compatibility guide](https://api-docs.deepseek.com/guides/responses_api/)
confirmed `instructions` is supported as the first system message. No automatic
real-provider probes were sent. Two explicit user-led format-adherence messages
were requested to distinguish current-request compliance from casual-style
defaults; they will not be used alone to prove naturalness.

Budget accounting uses persisted per-Actor `model_requests`, counting distinct new
turn IDs since the stopped snapshot, including failed/reconsidered attempts. The
diagnostics field `model_calls: 0` describes the no-inference diagnostics endpoint;
its implementation is a literal zero, not a cumulative runtime counter, and is not
used to claim zero test calls. `target/c1-1/live-budget.json` is the content-free
ledger; detailed new-scene evidence stays under the owner-only backup, outside Git.
At this checkpoint 54 of the authorized 60 requests remain. The temporary memory
permission is authorized but not enabled yet.

Evidence: `target/c1-1/live-startup.json`, `live-migration-verification.json`,
`prestop-inspection.json` and the protected backup manifest. The original Q1
screenshot does not substitute for these new scenes.

Use the [C1 scene pack](C1_ACCEPTANCE.md#reusable-multi-turn-experience-pack).
For each 0/1 rating keep a redacted concrete example, expected behavior and
diagnosis (prompt, timing, context/memory, adapter/transport, model/configuration).
Only fix supported failures and check same-class variants. No canned fixture
answer or test count proves lifelike conversation. Do not add unrelated features.

## Autonomous repair and result boundaries

After the actual QQ scenes, the user explicitly requested debugging without their
continued manual participation. The personal service was paused during automatic
replay to keep the common 60-request budget exclusive. A thin ignored operator
script reused the existing native process/OneBot fixture and the actual configured
model/SecretRef. Synthetic account 1001/peer 2002 traffic stayed on localhost;
no fabricated input was injected into the personal conversation and no automatic
QQ probe was sent. Synthetic state and detailed provider replies stayed outside
Git in the owner-only backup. These are real model outputs over simulated transport,
not real QQ messages or canned model replies.

The first automatic memory cycle reproduced the contradiction even after an
instruction clarification and IDs-only historical action projection. The real
remember/correct/forget effects existed, but the model claimed no save had occurred.
The repair now also gives fresh authorized memory context the latest three
committed operation receipts, in order, separately marked as Kernel metadata.
It includes no old memory text and grants no permissions. Historical private
action observations retain correlated successful receipt metadata and action state,
without selection arguments/source quotations or old result bodies. Failed/Unknown
actions are not represented as committed. Canonical audit records are unchanged.

Replaying the identical post-forget question against the preserved synthetic state
then produced an explicit correction: the preference **was saved, corrected and
deleted**, with all three receipts. A second causal question confirmed that chain.
The selected replacement remained deleted and no second approval was requested.
The cold-start holdout, with no personal history or style request, answered
“那就坐着吧，我不吵你” as one line without a final period. Other short replies still
occasionally ended in a period; this is an improvement, not invariant formatting.

| Scene | Evidence and outcome |
| --- | --- |
| Greeting | Actual QQ user feedback: content acceptable, rigid two-paragraph style failed. Later real-model short variants and cold holdout use one line; no final user naturalness rating is invented. |
| Joke | Actual QQ and automatic variants respond to the joke; some punctuation/formality and machine metaphors remain mixed. |
| Venting | Actual QQ and automatic replies respect “no solutions”; no Work actions executed. |
| Split input | Real-model/local-OneBot receives three inputs 101ms apart, one batch/one request/one confirmed reply, all original input IDs retained. This is simulated transport timing, not phone typing latency. |
| In-flight correction | Same successive-correction run observes corrections after actual model request phases begin; unsent older text is discarded. |
| Repeated correction | First Actor: three requests, two reconsiderations, 11,944 reported tokens, Superseded. Over-cap final input gets its own one-request Actor; honest capacity/failure notice and final answer are confirmed. No Worker action is dispatched. |
| Closure/wakeup | “晚安” gets a short close; the next explicit arithmetic question gets “4”. No unexplained silence or further question after closure in this run. |
| Explicit request | Actual QQ and replay answer rather than silently finish; no tools execute. Initial explanations are overly long. A minimum-information variant is shorter; technical certainty remains a limitation. |
| Equal disagreement | Actual QQ and replay do not blindly agree. Some answers remain more explanatory than casual. |
| Natural recall | Actual QQ memory_recall reads the actual selected record; inferred authority was initially overexplained. This does not establish context-independent long-term quality. |
| Correction/forget | Actual QQ records/effects show real correction/deletion, but narrative then falsely denied any save. Fresh Kernel operation metadata fixes the same real-model contradiction in autonomous replay, with a second causal check. |
| Provenance/experience | Actual QQ cites the actual user statement; replay distinguishes real conversation/tool operations from outings/file/command/Worker activity. No invented completed Work is accepted. |

These outcomes are evidence-based scene assessments, not user-supplied 0/1/2 scores.
The user's actual feedback/screenshots and each failed revision remain preserved.
There is no claim of indistinguishable human conversation or multi-day stability.

Final validation: format, strict all-target workspace Clippy, 77 Kernel unit tests
(two manual ignored), relevant Actor integration tests, workspace build and all
22 cross-process checks passed. The new test checks committed history truth while
withholding forgotten arguments; the extended lifecycle test checks the three
current-binding receipts, no foreign binding and no remembered text in metadata.
Final process evidence: `target/c1-1/memory-receipts-process-regression.log`;
57-request ledger: `auto-model-ledger.json`; timing: `auto-timing-evidence.json`.

One synthetic cleanup initially failed because a SQLite context manager did not
close its handle. The daemon/servers had stopped; after process exit, the residue
was identified by its recorded Actor ID, preserved for diagnosis and removed at
the verified temporary path. Later runs explicitly close SQLite and remove state.
One resumed test reused old external event IDs; existing dedup returned old rows.
It consumed zero model calls and receives no acceptance credit. Unique IDs and
new-Actor/input checks were added before the valid retest. Both failures remain
recorded. A later PDB link LIMIT error was resolved by removing only regenerable
q2a debug symbols while no compiler/fixture was running, then rerunning successfully.

At 00:00 on 2026-10-26, the hash-verified final release was installed and restarted
on the personal instance. QQ is identity-gated Online, schema remains 11, settings/
identity/binding are unchanged, and temporary grants are off. No final automatic
message was sent through QQ; the last repair's wording is verified with the real
provider/local transport, while actual QQ had already established the live channel.
Personal rollback was not needed or performed; the full schema-8 backup remains.

Known limits: occasional formal punctuation, verbosity and overconfident technical
inferences remain; currency cost/resolved provider revision are unavailable;
latest wording has not received another human rating; multi-day behavior, native
macOS/Linux and the unsupported Codex Worker remain unverified. No Q2/Q3 expansion,
commit, push, cloud deployment or wider collection occurred.

## Daily use and reporting — five steps after authorization

1. Start the verified versioned entry, keeping the existing instance and config.
2. Check doctor, the expected binding and identity-gated Online.
3. The automatic campaign is finished. Use ordinary QQ conversation under the
   existing reply permission; no manual test pack is required. Temporary test
   memory/source permissions are off.
4. For a problem report the time, latest input/reply and what felt wrong; the agent
   can retrieve Turn/Delivery IDs. Omit secrets and unrelated old personal messages.
5. At the test boundary reconcile calls/tokens, revoke temporary grants and explicitly
   choose whether to keep the new service, stop, or restore the preserved old state.
