# ADR 012: Bounded private-conversation decisions and draft reconsideration

Date: 2026-10-25 (Asia/Shanghai).
Status: accepted and implemented; native Windows deterministic acceptance complete.
Real-model/QQ conversation quality remains a separate, unverified acceptance.

## Reference and choice

Review MaiBot at commit `062d45cf8bd4272ac57d7ec03fcf405e5a458211` (upstream
commit timestamp 2026-10-23T17:46:39Z). Its message debouncer, explicit wait/stop,
experimental message steering and separate behavior/expression prompts are
references. Lain keeps one bounded Actor and its Kernel tools, no extra Planner/
Replyer job. No upstream implementation or prompt text is copied into product.
Exact source locations and differences are in MAIBOT_REFERENCE.md.

## Lifecycle

The existing reply grant admits a private-chat context on new channel turns.
It authorizes no extra source, memory, Work or Worker permission. Terminal turns
without that context keep the current protocol and prompts. Add two closed chat
tools: bounded wait, and an explicit no-reply finish. Plain assistant text remains
the reply decision. Decisions are model interpretations, not proof of user intent.

Input collection uses a trailing 750-ms quiet window capped at three seconds from
the first input. It still respects 16 inputs / 8 KiB / 128 queued inputs and the
original expiry. A deadline already passed before a later input is not extended.
No typing status or human presence is fabricated.

| Event/state | Transition |
| --- | --- |
| New accepted inputs | Queued -> bounded merge -> Prepared -> Running |
| Ordinary final response, input current | Actor Completed/Reply -> delivery Ready |
| Chat wait, once per turn | Running/Waiting, 1–5 seconds, woken by new input or deadline |
| Wait completes | Executed receipt; same bounded turn continues with current data |
| No-reply finish accepted | Completed/NoReply, no delivery; future input starts a turn |
| New input during inference | Account returned usage; discard stale text/calls; adopt eligible supplements and reconsider |
| Reconsider budget exhausted with new input queued | Completed/Superseded with an honest capacity notice, no old reply; new queued input triggers the next ordinary turn |
| New input before an ordinary Ready draft sends | Ready -> Superseded; retain draft and actual IDs |
| New input after Sending claim, before adapter write check | Refuse write as known NotSent; do not claim it was sent |
| Sending/Confirmed/Unknown | Preserve actual effect evidence; no blind replay |
| Restart during inference/wait | Existing Unknown recovery, no automatic inference retry |

Original Incoming IDs, routing/external IDs, text and receipt times, and the first
request/Batch.text are immutable. Lifecycle and batch-association fields can change.
Supplements retain their original IDs/text/times and are linked transactionally to
the same batch and appended as user messages. Total batch bounds still apply.
Two reconsiderations, one wait, four model requests/four tools, reported-token and
120-second active-phase bounds apply; new inputs never reset these counters.
Timers have a concrete merge/wait/expiry reason. There is no idle model polling.

Before committing final conversation text or a new action, Kernel checks that
the admitted input is still current. Private memory effects repeat this check
under their existing transaction lock. Completed effects are not undone or replayed.
New input does not cancel a Worker or approve a pending proposal. Approval/failure
notices are not ordinary drafts eligible for silent supersession.
The service/adapter prewrite check also reapplies current credential filters to
the stored delivery, so an old draft cannot bypass filtering after configuration changes.

Reconsideration rebuilds a model-context copy from preserved user messages and
fresh authorized evidence, excluding old private/source tool observations. Opaque
provider continuation is not reused across that new input boundary. Within an
unchanged request chain normal provider continuation remains intact. All private
draft previews are held; generated does not mean the QQ user saw the text.
Later private history includes confirmed outgoing text, not obsolete/uncertain
drafts, and withholds replies based on now-invalid inspected memories.

The latest request's actual selected memory IDs/revisions and bounded historical
source selections/fingerprints are also recorded on the private context, without
source bodies. Memory versions and re-projected source fingerprints are checked
at final-text/new-action commit and at delivery admission, in addition to response
checks. Forget, correction, source suppression, expiry, narrowing and revocation
cannot leave an old quotation eligible to send. Prepared-source audit and selected
fingerprints commit together. A scope-change notice has an explicit
failure purpose and carries no old quote. Reconsideration retains IDs-only receipts
of already committed memory effects rather than suggesting they never happened.

## Participation and expression

Silence is opt-in through a typed model decision, never random keyword filtering.
It is restricted to short social closures/acknowledgements with no substantive
tool action. Declared questions/requests/work/approval/faults require a response;
a conservative textual request/question guard can only *deny* silence. It cannot
establish permission or prove arbitrary natural-language intent. On denial the
Actor must answer or surface a failure within its existing budgets. Semantic
classification and naturalness require real-model review, not canned fixture text.

Separate private behavior and expression instructions retain Lain's core identity
and policy. Natural paraphrases may omit technical UUIDs; exact IDs/revisions and
provenance remain inspectable in existing receipts/APIs. Source history is still
historical evidence, not Active memory. No source grant or memory grant is enabled.

Migration 011 versions new chat snapshots and Superseded delivery semantics and
adds a batch-input index. It creates no old chat decisions, reads, memories or
confirmation timestamps. Personal instances are not migrated by development.
No proactive contact, schedule, experience subsystem or generic agent loop.
