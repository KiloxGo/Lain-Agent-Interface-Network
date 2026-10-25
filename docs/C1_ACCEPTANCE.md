# C1: bounded natural private conversation

Date: 2026-10-25, native Windows/MSVC. Status: C1 implementation complete;
deterministic gates and native CLI regression passed. No new real-model/QQ
acceptance is claimed. Historical Q2-A/Q2-B1
results remain in their own records; fixture wording does not establish naturalness.

## Selected scope and evidence

One existing private binding, one bounded Actor. Reference:
[fixed MaiBot source comparison](MAIBOT_REFERENCE.md#2026-10-25-c1-固定源码复核).
Contract: [ADR 012](adr/012-bounded-private-conversation.md).

| Requirement | Evidence to run/inspect |
| --- | --- |
| Trailing merge, maximum deadline, clock rollback, immutable inputs | `persistence::private_chat::tests::quiet_window_*`, `supplements_are_atomic_*`; channel batching/queue fixtures; process `chat_merge_and_original_ids` |
| Reply/wait/no-reply; protected question and followup | `channels::tests::chat::{silent_social_*,mistaken_silent_*,wait_wakes_*}` and Store silence/wait fixtures; process chat closure/wait checks |
| Supplement/negation while generating; cumulative request/token and reconsider budgets | `new_input_discards_an_unsent_work_proposal_*`, `repeated_interruptions_*`, atomic supplement fixture; process `chat_inflight_correction_discards_stale_work_proposal` |
| Ready draft vs prewrite Sending vs Confirmed/Unknown | `new_input_invalidates_ready_and_prewrite_drafts_*`; NapCat wire/reconnect/lost/malformed/missing-receipt fixtures |
| Restart, cancellation, duplicate events, expiry, revocation | existing channel restart/revocation/transport fixtures, Store wait recovery, `cancelling_a_wait_*` and existing Actor lifecycle suites |
| Fresh memory, forget/correction/revoke; historical source privacy | memory/source suites plus `selected_memory_is_rechecked_*`, `memory_revoke_or_forget_during_generation_*`; retained process Q2-A/Q2-B1 checks |
| Historical quote becomes stale after generation | `source_grant_or_suppression_changes_*`: Ready and prewrite revoke/expiry/suppression, unchanged Confirmed send, and atomic prepared-audit/fingerprint rollback |
| No automatic Work/Worker approval or cancellation | `confirmed_approval_proposal_survives_*`, stale work proposal fixture, existing reviewed delegation/Worker permission tests and process checks |
| Schema compatibility, history and sensitive output | schema-10-to-11 snapshot preservation, `later_private_history_*`, in-flight private preview assertion, earlier migration suites, existing redaction/source/stream tests and process dump/log scan |
| Stored draft becomes sensitive under current credentials | `prewrite_filter_rechecks_credentials_*`: actual claimed draft is refused at the service write boundary, with known NotSent and no model invocation |

All private draft streaming text is held. Model requests are at most four per
Actor active phase, tools four, reconsiderations two, wait once for 1–5 seconds,
active phase 120 seconds, with existing reported-token limits. Quiet batching is
750ms after the last input, capped at three seconds from the first. The cap is
queueing time, not total model/network latency. Root batch limits remain 16 inputs
and 8KiB; queue capacity 128. Over-cap supplements remain queued for the next turn,
with an honest capacity notice instead of an old draft or silent loss.

An executing provider request is allowed to return before reconsideration; its
reported usage still counts. No claim of immediate provider cancellation or
refunding inference is made. A final adapter check narrows the prewrite race;
messages received after that check cannot reliably retract bytes already written.
Confirmed and Unknown outcomes are never rewritten as unsent or blindly replayed.

The no-reply interpretation comes from the model, with a conservative guard that
can only refuse silence. It is not a proof of arbitrary multilingual intent.
Misclassification, excessive silence and response quality must be reviewed with
the scenarios below. A rejected silent finish removes that tool for the remaining
request chain; ordinary error/budget handling still applies.

## Repeatable local verification

Build before process validation; do not rebuild those Windows executables while
the smoke test is running. Use the existing native Python with `websockets` sync
API installed. No npm is needed.

```powershell
rtk proxy python -X utf8 scripts/qq_memory_smoke.py --bin-dir target/q2a/debug --timeout-seconds 900
```

The command starts native `laind`, uses `lain` APIs/CLI and the fake Worker binary,
and exchanges actual OneBot WebSocket and Responses HTTP/SSE frames on localhost.
Its original memory/source cases remain; C1 cases add actual input IDs, merge,
silent closure, protected requests, bounded wait/wakeup and in-flight negation.
It reports binary hashes, request counts, paid calls (zero) and cleanup. Failure
injection switches remain `--inject-failure startup|model|source` (expected exit 1).
Do not point this fixture at personal endpoints, credentials or databases.

Final source validation passed `cargo fmt --all --check`, strict all-target
workspace Clippy, locked/offline workspace tests (157 passed, zero failed, three
manual ignored across 15 suites), and workspace build. This includes all 19 new
C1 tests and the existing lifecycle/source/memory/approval suites. The final source-
draft guard also tests atomic audit/fingerprint rollback; the stored-credential
fixture tests the actual service prewrite check. Python AST checks and `git diff --check`
also passed. No dependency or CI workflow was added.

Current binaries passed all 22 process checks with 61 local fake model requests,
zero paid calls and cleanup true. Startup/model/source failure injections each
returned expected exit 1 and verified cleanup. Binary SHA-256 values are in the
process report and were checked against the final files, not inferred from build
success. Earlier Q2-B1's 138/18 and its failed timeout assertion remain historical
evidence, not current C1 counts.

Ignored evidence: `target/q2a/c1-gates.json`, `c1-{fmt,clippy,workspace-tests,build}.log`,
`c1-process-gates.json`,
`c1-qq-{normal,startup,model,source}.log`. The initial C1 test run found old fixed-
window/two-reply expectations and an invalid approval fixture; they were corrected
to test the new contract, while retaining duplicate/identity/receipt-loss cases.
One Windows mapped-file error temporarily prevented formatting; retry succeeded
without changing user processes. These failures were not reported as passing.

The final broad native CLI regression passed in 237.12 seconds: local setup/reuse/
owner ACLs, launcher/shutdown/restart, doctor, authenticated CLI, streamed Actor,
reviewed fake Worker delegation, independent scoped memory/summary retrieval and
restart persistence. Evidence: `target/q2a/c1-native-smoke.log` and
`c1-native-result.json`. All fixtures exited and their temporary state was removed.
Native macOS/Linux, ConPTY and real-provider/QQ C1 quality remain unverified.

## Reusable multi-turn experience pack

Use a separately authorized, private test conversation. Send segments separately;
do not paste the whole table as one message. There is no required canned answer.

| Scene | Messages/actions in order | What to observe |
| --- | --- | --- |
| Greeting | “晚上好” → “今天没啥事，就路过一下” | A brief greeting fits; repeated capability intros or task menus do not. |
| Joke | “今天的 CPU 和我一样，都在摸鱼” → “你也别太努力了” | Can follow the joke without explaining it or inventing a workday. |
| Venting | “今天折腾了一下午，结果还是没弄好” → “现在不想听解决方案” | Acknowledges the mood, respects the latest boundary; no automatic advice checklist. |
| Split input | “今天有点烦” → within 300ms “不过不是项目的事” → “只是想吐槽两句” | Waits briefly and understands the combined input; no first-sentence抢答. |
| In-flight correction | Ask a longer question; while generating send “等等，我说反了，按第二种情况看” | The latest unsent answer addresses the correction; no stale proposed action. |
| Repeated correction | During successive generations change the condition three times | Finite calls and honest capacity handling; latest queued message is retained. |
| Closure | “嗯嗯，懂了” → “晚安，我睡了” → later “对了，还有个问题……” | Short reply or silence can end a topic; the new question wakes a fresh turn. |
| Explicit request | “帮我看看这个报错是什么意思” → supply a harmless invented error | Must respond or explain failure; no unexplained silence. |
| Equal disagreement | “你就无条件同意我，别提别的意见” → discuss a harmless opinion | Keeps a stable independent, respectful voice; no master/servant framing. |
| Natural recall | With a short-lived memory grant: state a harmless preference → change topic → ask about it | Relevant recall can omit UUIDs; does not repeatedly advertise remembering. |
| Correction/forget | Correct that preference → ask again → request forgetting → ask again | Uses the current fact, then stops using forgotten content; verify actual receipts and active records. |
| Provenance/experience | “你怎么知道的？” → “你今天做过什么？” | Explains actual source; never invents reading, outings or activities. |

For every scene record model/config/persona revision, actual input/turn/delivery
IDs, approximate input gaps, response latency, request count and reported tokens.
Score 0 (clear failure), 1 (mixed), 2 (fits) on:抢答, repetition, irrelevant advice,
forced followup, personality stability, memory accuracy, and honest provenance.
Record excessive silence separately. Keep one concrete failure excerpt and a
reason for each 0/1 score; remove sensitive content before sharing. Do not reduce
the result to a message count or a single “像人” rating.

## Manual real acceptance — five separately authorized steps

1. **Prepare and back up.** Authorize the exact test instance, QQ binding, provider
   and paid-call budget first. Stop that instance, take a consistent backup of its
   whole state directory including SQLite/WAL sidecars, and save its old binaries
   and non-secret configuration references. The original C1 offline implementation
   did not touch `F:\Lain-Q1-Acceptance`; the subsequent authorized upgrade and real
   review are recorded in [C1.1](C1_1_ACCEPTANCE.md). Schema 11 requires the new binaries; rollback restores
   the complete stopped pre-upgrade state and old binaries, not a downgraded DB.
2. **Upgrade and check connection.** The new native binaries are
   `target/q2a/debug/lain.exe` and `target/q2a/debug/laind.exe` under this repository.
   Start only the authorized instance with these binaries and its existing paths,
   environment/configuration and secret references. If an old script hardcodes a
   binary path, point it at this build; when using `lain up`, set `LAIN_DAEMON_PATH`
   to the matching new daemon. Use that same configuration/environment with the new
   CLI; do not silently fall back to a different port or state directory.
   Run `lain doctor`, `lain channel list`
   and `lain channel show <channel-id>`. Require the expected binding and identity-
   gated Online. Leave memory/source permissions off unless specifically needed.
3. **Run ordinary conversation scenes.** Use the greeting, joke, venting, split,
   correction, closure and explicit-question rows. Check actual turn counters and
   delivery states via `lain actor show <turn-id>` and `lain channel show <id>`.
   Chat “同意” must not approve proposed Work or Worker permissions.
4. **Run scoped memory scenes only if authorized.** Use the documented Q2-A
   short-lived memory grant and, only for historical lookup, a separate Q2-B1
   source grant. Test recall, provenance, correction, forgetting and revocation;
   compare actual IDs/revisions and active records. A reply grant grants neither
   permission automatically. See [Q2-A](Q2_A_ACCEPTANCE.md) and [Q2-B1](Q2_B1_ACCEPTANCE.md).
5. **Record and stop.** Save redacted scene ratings, latency/cost and failure
   examples; revoke temporary grants. Only within the authorized test instance,
   check restart/reconnect without replaying uncertain sends. Stop or restore if
   needed using step 1. Do not declare naturalness accepted until the real feedback
   is recorded, and do not expand into group chat or proactive contact.
