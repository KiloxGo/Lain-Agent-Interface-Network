# ADR 017: Persistent Kernel-wide Lain inference ceiling

Date: 2026-10-26 (Asia/Shanghai). Status: accepted for D3b implementation.
This is a cost/admission restriction, not permission to call a provider, collect
data, write memory, contact a person, run a Worker or install a personal release.

## Scope and legacy compatibility

The D1b reading and D2 sharing ceilings remain enforced, but do not together cap
ordinary chat, recollection tools or reconsideration across many Actor turns.
Migration 016 adds one optional Kernel-wide ceiling over **all Lain Actor model
adapter invocations**: private/unbound conversation, reading and sharing. It does
not count Worker-provider requests or prove upstream billing. Model/credential/
frontend changes cannot reset the ledger in the same Kernel database.

An untouched migration has decision revision 0: ExistingLimitsOnly. Existing
reply/source/memory authority and turn/activity limits remain as before; no paid
campaign is invented. Once explicitly configured, stop, expiry and exhaustion
deny new reservations. There is no API to reset to the unconfigured compatibility
state or silently return to uncapped behavior. Operator-approved rollback still
requires restoring matching complete state/binaries and is not quota renewal.

## Explicit finite decisions

An InferenceLimit has an immutable BudgetId, explicit request ceiling (engineering
bound 1..100000) and UTC expiry. There is no product quota/default expiry. A
revision-checked administrator can enable a new limit, stop it, or reopen a
previous identical limit. Reopening an ID reuses all its reservations; changing
that ID's ceiling/expiry is rejected. A new ID is a new explicit administrator
budget decision, never a model capability or automatic midnight/restart refill.

Stop retains the last limit in the decision for truthful usage inspection. Enable
requires no unfinished prepared/attempting Actor requests, including legacy ones;
an old in-flight invocation must not be silently placed outside a new campaign.
Stop is allowed during a request, closes future admission and cannot promise
recall/cancellation of a request already entering its adapter. Late results may
still be recorded under the old reservation and existing data/action permissions.

## Reservation and lifecycle

The Actor counter increment and unique `(ActorTurnId, request_number)` reservation
share the same SQLite transaction as per-reading/per-sharing reservations. A
failed commit rolls them all back before any model call. Each row records only
IDs, decision revision, a typed reason, phase/times and normalized usage; no prompt,
source/outbound content, profile credential or secret is stored in this ledger.

Immediately before entering the model adapter, Kernel checks the saved reservation,
current decision/revision/expiry, total limit and Running Actor. A second attempt
of the same reservation is rejected. Phases:

- Prepared: counted, not yet entered the adapter.
- Attempting: the adapter may have started a request; not proof a server received it.
- Returned: an adapter reply returned; normalized token usage is recorded before
  later source/reconsideration/body checks can reject the response.
- NotAttempted: terminal/restart recovery ended a still-prepared request.
- Unknown: an attempted invocation ended without a recorded reply, including
  failure, cancellation, timeout or restart. No automatic retry exists.

All reservations consume the ceiling, including NotAttempted/Unknown. Failed,
cancelled and uncertain calls are never refunded. New turns, conversations,
source/share grants, rereads, reconsiderations and restart cannot bypass the
same limit. Counters use the full ledger, independently of latest-100 inspection.
Historical Actor snapshots are not retroactively converted into provider requests.

Returned usage is adapter-normalized; zero may mean missing usage, not a known
zero invoice. Unknown calls may have been billed. Ledger counts, model-reported
usage and currency charges stay distinct; no exact invoice is claimed. Current
Responses HTTP and public-text HTTP adapters explicitly disable reqwest retries
and redirects. Verified against local reqwest 0.13.5 source on 2026-10-26: its
default includes protocol-safe retries, which must not add hidden attempts here.
Any external proxy's/provider's behavior still requires separate live verification.

## Control and operation

Only authenticated management API/CLI exposes show/budgets/requests/enable/stop;
no Actor tool can change a budget. Stops do not approve/cancel Worker permissions
or discard operational results/approval notices. Existing data/grant freshness
and hard turn/tool/token/time limits remain intersections with this extra ceiling.

`lain inference show` reports admission, current decision/reservations/remaining
requests/normalized usage, all pending Actor requests and latest-100 ledger rows.
`inference budgets` preserves old limit totals; `inference requests [--budget ID]`
inspects latest-100 content-free rows. Both totals and active admission remain
durable when local frontends close. Timers/polls are not needed for budget expiry:
every actual reservation and adapter entry checks current UTC time.

## Acceptance and release preparation

See [D3b acceptance](../INFERENCE_BUDGET_ACCEPTANCE.md). A review-only release
packager copies hash-matched tested binaries and a non-secret scope proposal into
an owner-only workspace artifact; it never opens a personal instance, loads its
credentials, starts services or grants live permission. Separate instance budgets
are not magically account-global: any real campaign split must have fixed approved
partition caps whose sum stays inside the owner's total, with no automatic transfer.
Campaign controller state must be outside the instance rollback boundary; a real
rollback closes the campaign rather than continuing tests against an old daemon
without its ceiling. This controller/live runner is still D3c work.

D3c's bounded autonomous real-model runner, concrete live scope/authorization,
personal upgrade/contact and longer real-use acceptance remain open. W1–W4 are
still unfinished parts of the combined goal. No general agent framework, new
source collector, prompt compiler or Web redesign is introduced.

A live upgrade must bootstrap the migrated Kernel without model/QQ configuration,
set its approved finite ceiling over the authenticated local API, then restart
with the approved runtime configuration. Otherwise legacy-compatible startup could
admit an old reply before the new campaign is installed. The release proposal
does not execute any of these steps or assume current personal service state.
