# ADR040: Bounded cross-batch briefing reconciliation

Date: 2026-11-06. Status: implemented locally; deterministic/native fixture
acceptance passed, actual-model synthetic functional acceptance passed (25 requests); conversational quality limitations remain. Not deployed.

## Decision

Keep immutable extraction items and their original one-based numbers. After
extraction, a finite tool-free Actor activity sees a compact index of every
processed item (at most256), proposing at most one earlier candidate per item.
This index is explicitly truncated discovery metadata, not proof of semantic
completeness. Each proposed pair then gets a separate bounded tool-free judgment
over the complete stored item summaries and actual quoted source evidence.
Results are Different, Duplicate, ReportedCorrection or Conflict. Both sides must
be supported by exact supplied quotes; relations remain model interpretations,
not independently verified notifications. No unseen item can be dropped.

At most one index request and255 pair judgments, within the existing day deadline.
Every call uses the existing Actor/inference ledger, no Worker planner or generic
tool loop. Uncertain/restarted jobs are not replayed. Source and grant currency
gate preparation, admission, completion, report/original lookup, foreground
history and memory. Reconciliation is another stage of the same finite run.

Successful relations form groups rooted at the smallest original item number.
Duplicate notifications collapse in the ordinary list while retaining all member
numbers and source lookup. Corrections/conflicts show their separate statements
and source times; no unconditional last-message-wins rule or invented combined
fact. Original lookup still accepts every immutable original number. Pagination
uses those stable numbers, so hidden duplicates do not shift follow-up targets.

## Memory and visibility

A selected reconciled group derives a fixed cautious draft from actual members,
including correction/conflict labels and nonparticipation. A bounded immutable
reconciliation reference supplements legacy primary extraction lineage; complete
input versions remain in the original manifest and stored jobs, not copied into
model-visible semantic records. Saving a group rechecks that exact completed
group and all discovery/judgment inputs. Expiry cannot exceed any influencing
input. Suppression covers actual member extraction inputs; it does not erase
unrelated archives. Large drafts must request a specific original member instead
of silently truncating a merged claim. Current per-action review remains intact.

Partial/failed reconciliation retains original items with explicit limitations;
it must not masquerade as a complete semantic deduplication. Candidate discovery
may miss semantic matches; completion means the finite proposed comparisons were
processed, not proof every notification was understood or every duplicate found.

## Verification required

Cross-batch duplicates/corrections/conflicts, unrelated similarly named notices,
exact quotes/foreign IDs/duplicate edges, stable numbering and paginated originals,
finite calls, source withdrawal during index/judgment/review, restart/Unknown,
group memory evidence and expiry/suppression, ordinary TUI/QQ/model flow. Existing
cloud identity, source grants, installed clients and host remain unchanged.

## Local implementation checkpoint

Schema039 introduces only the operational relation-job table and reader gate.
Reports fold duplicate rows but keep original member numbers; correction/conflict
statements retain separate rows and actual correcting endpoints. Original-number
cursors avoid shifting follow-ups. include_related is explicit/default false.
Primary legacy lineage plus an immutable index/final-job reference keeps semantic
records bounded. Full discovery inputs constrain current access/expiry; suppression
covers actual members' extraction inputs. No original archive is erased.

Full Kernel340 passed/two existing ignored, client152/three existing ignored and
wire8 at the implementation checkpoint. Later targeted briefing45, private reply
classification and known no-write notice tests cover final hardening. Strict
all-target checks/builds pass. Native TUI/simulated QQ group workflow accounts48
requests, ordinary original/cursor/reconnect workflow24. Actual-model first run
completed grouping and save/correct/forget but a retention-phrase assertion treated
a negated sentence as an overclaim; its failed evidence remains. Final actual
recheck and screenshots are tracked in the external relations acceptance report.

A read-only review found and prompted repairs for lost correction direction and
fixed refusal replies missing the private Reply decision. The latter would bypass
normal QQ prewrite checks; a failing actual Store/delivery test reproduced it.
Fixed refusals now preserve all existing effects, set Reply, clear waiting and
handle new-input reconsideration/source changes through the normal paths.
