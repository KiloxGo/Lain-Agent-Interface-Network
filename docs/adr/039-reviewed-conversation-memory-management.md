# ADR039: Reviewed conversation-local memory correction and forgetting

Date: 2026-11-05; forget implementation continued 2026-11-06.
Status: Forget and Correct implemented and locally accepted on native Windows
through schema038; full daily-source goal and production acceptance remain open.
Not deployed.

## Current implemented slice

The tool schema/runtime now offer `operation: forget` and `operation: correct`.
Correction uses a real reviewed candidate, exact current-user quotation and a
ConversationCorrection source, inheriting visibility and finite expiry. The
replacement, predecessor supersession/scrubbing, suppression and metadata effect
commit together; generic candidate acceptance cannot bypass that transaction.
Management inspection returns metadata only. A distinct typed review projection
and content-free effect receipt supplement existing reviewed-selection receipts;
there is no fake candidate for deletion. Deletion, source suppression, candidate
scrubbing, effect and the fixed Actor completion acknowledgement commit together.
The closed MemoryEffectNotice path preserves invalidated lineage; it does not
authorize old model/tool bodies to be published again.

Core red-to-green tests cover missing inspection/forged quotation, source-stale
management, exact retries, transaction failure rollback, prior-body withdrawal,
restart receipt recovery and ordinary Actor tool dispatch. Both terminal review
entries share exact-quote validation. Full Kernel329, client151/wire8 and strict
checks pass. Native terminal/simulated QQ save-recall-withdraw-forget-restart uses
24 accounted fixture requests. Actual DeepSeek over four synthetic notices also
passes, including a new post-forget query: 13 requests, 82,905 reported tokens,
40.02 seconds. These are not real QQ source collection or production acceptance.

The inherited suppression policy is conservative at extraction-input granularity:
other selected memories depending on the same batch inputs can become unavailable.
Only the exact target record is deleted. This does not claim fine-grained semantic
claim suppression; improve that UX/precision before declaring the full goal mature.

Schema038 correction acceptance: implementation checkpoint Kernel334/client152/
wire8; final memory63 and strict Kernel checks after quoted-history repair.
Native TUI/simulated QQ final30 and actual DeepSeek final17 requests pass; actual
final112,518 reported tokens,43.25s. Earlier output-budget and User-quotation
leak failures are retained, not counted as passing. The latter required the
originating-turn version dependency above, not just prompt changes. Source
selection, cross-batch reconciliation and real QQ/production acceptance remain
open. See the external correction-acceptance.md referenced in DAILY_BRIEFING.

## Decision

Extend ADR038 through the existing Actor action review and semantic store.
`memory_selection_inspect` returns only management metadata for one exact owned
conversation-local selection: ID, actual revision/status, original expiry and
save time. It does not restore unavailable source bodies or confer read grants.
An executed inspection in the same turn is required before proposing a change.

One typed `memory_selection_change` proposes either correcting or forgetting
that inspected ID/revision. It references one real current-turn user MessageId
and exact bounded quotation. Activities cannot use it; another conversation,
channel identity or global/project memory cannot be substituted. Review displays
the operation, exact target/version, complete immutable current user message and
separately labelled excerpt. Correction also shows the actual replacement draft
and inherited expiry; excerpt authentication cannot hide user negation. No standing write
grant, conversational assent or new private-memory trial is introduced.

A correction reuses a real MemoryCandidate. Its entire replacement content is
the current user quotation, with separately attributed ConversationCorrection
provenance, not a rewrite of old source text. Actual review authorizes the
replacement. It stays in the same conversation, inherits the original finite
expiry, supersedes the exact previous revision and suppresses the previous
selection's source inputs. It cannot make a source report verified, copy the old
body into a fresh provenance, extend source-derived lifetime or widen visibility.
The originating Actor context depends on the new memory version. On forgetting,
supersession or expiry, ordinary/model history withholds that turn's quotation;
raw user audit remains immutable and content-free effect recovery is independent.
This prevents the forgotten fact from flowing back through historical User text.
ConversationCorrection current checks do not recurse into that Actor context.

Forgetting deletes semantic/index/candidate bodies using existing deletion and
source suppression within the same transaction as a content-free effect receipt.
Exact retries return committed metadata and never execute another mutation.
Reviewed receipts describe Remember/Correct/Forget independently of action ack,
source availability, current body and delivery. Forgotten content is not retained
in the effect receipt. Original conversation/audit retention remains separate
from semantic deletion; this is not secure disk erasure.

Once a source is unavailable, management may still inspect metadata and explicitly
forget or correct the exact owned record. No old body is disclosed. If previous
body evidence became stale during this turn, the existing source gates still
end that turn; a fresh metadata-only management turn must be used. A concurrent
target revision change invalidates pending review, which can safely close without
writing. Missing/expired records are not silently recreated.

## Required verification

Real-current quotation, inspection/review prerequisites, exact revision and
conversation isolation; no write before review; correction attribution/expiry;
forget suppression/FTS/candidate deletion; stale original source management;
concurrent revision changes; interrupted acknowledgements and exact retries;
ordinary QQ/terminal save-correct-forget-recall and restart. This ADR is not
implementation evidence. Source selection, cross-batch reconciliation and real
source/production acceptance remain required by the full daily-briefing goal.
