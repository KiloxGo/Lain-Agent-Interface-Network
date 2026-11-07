# L.A.I.N. Development Log

## 2026-11-07 — Prose instead of a final selection no longer fails the turn

Two real runs of the same 18-step scenario failed at different steps because the
model answered in prose instead of calling the final selection function, once on
an empty read and once on a saved record. The prose is still never published. The
Kernel now renders what the turn actually read, with no extra model request.
Malformed JSON and fenced attempts stay rejected, and a direct original request
without an inspected original still fails rather than substituting an excerpt.

A third run failed differently: the model's fourth request returned a valid final
selection, but crossing the turn cap discarded that already-paid-for answer. The
cap value is unchanged and still stops further requests and tools. Crossing it
during a source or memory read now answers from those reads with Kernel text;
ordinary replies are still withheld.

The fourth run passed, but its readback showed the model listing an old location
without the correction beside it. The Kernel now names any related statement left
out of a reply, so older details cannot silently look current.

## 2026-11-07 — Plainer default briefing and original views

Repeated per-quote labels are gone from the default and original views: the page
footer states the shared limits once (authorized archive, not a full chat record,
not independently verified). Differences that change how a quote should be read
stay on the item: fragment excerpts, receipt-time fallback, derived text, reported
corrections, conflicts and omitted paragraphs. An item fully shown as its original
is no longer listed again. Detailed expansion is unchanged, and the review notice
is now Chinese. `lain chat`'s English review prompts are left as they are because
that whole surface is English. Checked with native fixtures only; real-provider
wording review is pending.

## 2026-11-07 — Comparison output selects citations instead of copying UUIDs

The parser already accepted `citation_index`, but `RELATION_COMPARE` still told the
model to copy `record_id`, so a real run would have repeated the one-character
UUID failure. A new Actor-level regression simulates that miscopy: it ends Unknown
with the old instructions and Completed with the new ones. Stored judgments keep
real IDs, exact-UUID output stays valid, and the compare payload and its input
fingerprint are unchanged. Native fixtures now use the index form.

The maximum-group test takes about 112s alone in a debug build, so its 120s wait
timed out under parallel tests; the wait is now 300s. Personal paths, real QQ IDs
and a personal project name in docs are replaced with placeholders ahead of a
public release. Real-provider acceptance of this change is still pending.

## 2026-11-07 — Selected-memory evidence view and two actual reference failures

Current Windows25f414a0a0d8/d47/schema40 implements a checked selected-memory
final view: finite ephemeral read strings bind actual current-turn reads/records/
versions; full saved words, typed correction/source origin, query limits and
requested UTC deadline/capture basis are rendered without free fact invention.
Ordinary chat, other origins/scopes, mixed Work, mutations and bodyless management
retain their existing path. No schema/grant/TTL or reviewed-body change.

Real Actor red wrongly Completes known query/capture claims; new rejection retains
2requests/14tokens/1read and holds speech. R1preview assertion was initially hollow;
real streaming+counter+empty-page/no-old-dependency case fixes it, R2closes the gap.
First actual E4run35requests/264247tokens stops08recall: model copies saved-provenance
action ID instead of current read ID. Read succeeds; correct rejection retains
17768tokens/2requests/1read, no replay, cleanup passes. Finite-choice schema red→green
removes ambiguous model-supplied UUID tuples without accepting the bad reference.
Actual native image also exposed lazy Markdown continuation of Kernel labels;
Parser red→green plus two blank boundaries separates them from quoted saved words.
R3static review has no new finding on the changed interface/quote boundaries.

Final65Actor/105briefing filters overlap, fmt/strict Kernel checks/native build
pass. Initial maximum producer passes within the earlier105pass/1obsolete-fixture
failure; later filters correctly skip it. Legacy fixture is updated to actual
catalog selection and strengthens complete quoted-body checks. Native48fixture
requests pass both audiences with review/literal recall/correction/forget/restart/
isolation and cleanup. First native failure assumed a receipt fits current viewport;
exact persisted receipt plus bounded real PgUp/zero-model traversal preserves the
UI requirement. Native evidence is not actual-provider lifecycle acceptance.

Changed25candidate actual rerun stops before memory at source reconciliation:
5completed requests/22124tokens,6records processed in2batches; compare output copies
one source UUID character wrong while the quote matches the actual supplied
citation. Exact evidence validation rejects and run becomes Unknown/model_failed.
All Returned usage/accounting and cleanup pass; no guessed ID correction, hidden
retry or replay. Thus current actual full-memory tail is NOT accepted. Older ce43
18step evidence remains old-version only. Next fix explicit comparison citation
selection through exact item/citation indices while preserving stored real IDs,
input fingerprint, old exact-UUID compatibility and all quotation/currentness gates.

Known default-label verbosity/report splitting/English review notices remain.
No latest Linux/deployment/realQQ-source activation/trial/Git change. Unanswered
source proposal stays disabled; full goal remains active, not blocked. External
local evidence: configured visualizations/lain-memory-evidence-view-2026-11-07/.

## 2026-11-07 — Keep eligibility conditions beside checked short citations

Reproduce b801default missing first50/one-per-person despite an English summary
retaining them. Real Actor regression fixes only same-record paragraph context:
exact checked model quote stays; context is unique, bounded512UTF8bytes and gated
by full-record secrets/current authority/version. No other source fetch, model
call, schema or memory promotion. Default labels context and omitted paragraphs;
unavailable context warns. Original inspection remains an actual separate read.

R1finds an intermediate12KiB fitting interval that shortened old quotes beside
optional context; red proves it, drop context whole first. Initial actual maximum
group fails from extra metadata; extreme whole optional-metadata fallback restores
legacy citation semantics, preserves original group fields and warns. Specific
maximum rerun passes95.59s.4context/58Actor/103briefing filters overlap, final fmt/
strict/build pass. Native51source proves forced-short conditions in terminal and
simulatedQQ actual model inputs/default views;48memory retains reviewed save,
Chinese literal recall/correction/forget/restart. Cleanup passes, R2static clear.

Focused actual e1c5/d47/currentmedium uses unchanged6synthetic records/4questions:
15completed requests/99302reported tokens,max26864.4provider context observations
match exact welfare record/text; original correction consumed and displayed first
once, plus separately inspected base original. Actual quote includes conditions
this time, not causal short-quote proof. Exact paging/ledger/selected reads/end
scope checks/cleanup pass. Read4answers and actual ConPTY cells, not physical font
acceptance. Replies still repetitive/technical; selected-memory prose issues remain.
No latest actual full-memory tail/Linux/deploy/realQQ/source/trial/Git changes;
ce43actual18step memory stays scoped, full goal active. External evidence:
configured visualizations/lain-source-context-2026-11-07/.

## 2026-11-07 — Bind explicit original request to an actual original read

Preserve b801actual original failure; real Actor red falsely Completes report-only
selection. Add conservative direct Chinese current-User shortcut and final evidence
gate: fail original_required if no selected actual original/scoped empty read.
Catalog hides item-only choices until original exists, adds no authority/automatic
read/retry; private catalog still validates real handles and final checks remain.
Hold speech,2requests/14tokens/1read/no replay verified. Initial custom notice loses
category and is repaired. R1finds tilde/long-code fence misclassification; red confirms.
Reuse already-locked client Markdown parser in Kernel, same package versions; direct
prefix/quoted/narrated/negated/meta boundaries tested. First parser green loses现在
positive, restore it. Final57Actor/99briefing overlap, fmt/strict/build pass; initial
100test run includes unchanged maximum group (wrong skip string), no repeat of it.
Native51source/48memory fixture requests and cleanup pass; R2closes fence finding.

aa71/d47/medium on unchanged6synthetic notices/4focused questions:15completed
actual calls/97391tokens,max26399. Original turn3requests actually inspects correction,
provider consumes exact expected source body, renderer puts it first once/pointer.
Allaccounting/read references/scoped end checks/cleanup pass; all4answers/ConPTY
read, not physical fonts. Welfare limits complete this sample, but no relevant
condition code changed; prior omission stays. Full actual memory tail not rerun;
ce43 evidence not promoted. Next deterministic condition/citation coverage then
memory prose. General quality/Linux/deploy/realQQ/permission/trial/Git unchanged,
full goal active. External lain-original-request-2026-11-07/ preserves failures.

## 2026-11-07 — Follow actual read cursors; show inspected original first

Reproduce3renderer failures from ce43manual readback. Track only exact current-turn
same snapshot/query/run cursor chains after refresh; retain gaps/cycles/other-query
hints. Move the existing original renderer first and point only its same canonical
concise citation back, preserving other sources/details/bounds.55Actor, format,
strict checks/build pass. Native51briefing proves both audiences' exact continuation
and mixed original display;48memory reruns selection/recall/change/forget/restart.
First native observer cannot resolve sharedstatus;5tests red5failures→green strict
resolution. Then it assumes private source actions appear in ordinary API; retained
diagnostic proves sanitized actions empty/body correct. Audit only own isolatedDB,
close that connection (first leak blocks cleanup), remove only verified own residual
fixture. Final cleanup passes, no production privacy change. R1review clear.

Focused b801/d47/medium actual4questions:14completed requests/89918tokens,max25995.
Read-to-end output consumes exact0→4→terminal receipts and has no false hint, but
final original query reads reports twice then selects excerpts, never inspects.
Keep functional failure04and full trace; no unchanged paid rerun. Offline original
call-count assumption is corrected to actual two read calls without new model use.
All4answers/ConPTY read: checked welfare quote/default also drops first50/one-per-
person conditions. Actual memory tail not in this campaign; previous ce43 evidence
not promoted. End effect scan unreached; temporary cleanup/accounting pass. Next
original-intent/evidence constraint and condition coverage, then memory prose.
General quality/Linux/deploy/realQQ/full goal remain; no Git/grant/trial change.
External lain-briefing-paging-2026-11-07/ retains original and observer failures.

## 2026-11-07 — Keep selected source wording in reviewed memory

Reproduce English-note/Chinese-query miss, append only checked selected citation
fragments with source/origin/own clocks after full original draft/group. All fit
within4096bytes or retain the exact complete old draft; no truncating conditions,
group narrowing, new read or model call. Exact old pending candidates survive
restart; edited/stale drafts reject.3targeted tests,5-member group assertion,
52Actor/58briefing overlap, fmt/strict/native build pass. Native43/48 pass, but
independent R1finds QQ recall result discarded. Preserve that insufficient report,
reproduce7false-green negative cases in4tests, enforce actual query/result/reply
and reviewed-body equality, green/native48r2/R2review close the finding.

Actualce43c91b30b1/d47/medium on same8synthetic records plus one literal question:
18foreground steps,47requests/358040reported tokens,max28224. Save/review exact,
literal领取hit and exact provider-consumed body verified; source refresh/original,
restart recall/correct/expiry/forget/reconnect/accounting/end/cleanup pass. Read18
answers/ConPTY cells: current generated notes Chinese (forced-English repair is
proved separately), but memory prose still presumes unresolved older notice,
exceeds active query coverage and overstates capture-date reference. Paging,
original duplication, English review/verbosity remain. No general quality/Linux/
deployment/realQQ/source grant/trial/Git claim. Full goal remains active; next
receipt-based presentation/paging and bounded exact-memory recall, not more prompt
layers. Evidence: external lain-memory-source-wording-2026-11-07/.

## 2026-11-07 — One query declaration; current-candidate full actual path

Measure duplicated model-tool payload, decline the low-gain table-format prototype.
Offer existing memory_search once for null listing/literal filtering while keeping
the typed memory_list alias and all Kernel gates/restore/API/CLI paths. Group only
field-free offered global/internal kinds; selected IDs never cross-product. Reuse
shared source policy in concise briefing descriptors. Actual initial fixture tool
bytes15275→11997; intermediate over-target failures stay. Legacy equivalence/
foreign-scope/exact selected-ID/invalid pair tests pass. Final52Actor/55briefing
overlap, fmt/strict checks/build, independent R1/R2 and native43/48fixtures pass.

Currentd800/d47/medium on unchanged synthetic messages/questions completes17
foreground steps including3reviews:47actual calls/349869tokens,maxforeground29010.
Calendar/source clocks, late refresh/original, save/restart/recall/correct/expiry/
forget/reconnect and scoped end checks/cleanup pass. One incorrectly quoted active
status is refused without action; valid read recovers. Offline analyzer initially
fails on the nonJSON refusal; preserve failure, distinguish opaque control from
evidence and retain strict reference checks. Two observer fixture errors are fixed,
3tests pass; no paid replay. All17answers/final ConPTY cells read: English selected
content impairs Chinese literal recall, unrequested capture-date conversion and
verbose/paging/review-language UX remain. No general prose/reliability acceptance.
Next preserve checked source wording in memory drafts within current content/
scope/review/bounds, then constrain recall/paging. No Linux/deploy/realQQ/grant/
trial/Git change; full goal open. External lain-model-metadata-2026-11-07/.

## 2026-11-07 — Ordinary source rules compacted; complete-model path still open

One ordinary conversation policy replaces repeated source workflow/calendar/
quotation/memory/style blocks. Exact dynamic grant/clock, facts/corrections,
typed tools, internal extraction/comparison and all gates/budgets remain. Native
four-request red demonstrates12071-byte old initial instructions; final9609bytes
retain exact scope/clock/tool-allowance and source-receipt equality. Intermediate
tests incorrectly query active-only scope after completion and require ordinary
tools after their existing allowance ends; fix test observation, not production.
Independent review catches not_attempted falsely phrased as no target; actual
outbound regression red→green and R2 closes it. Final51Actor/55briefing overlap,
fmt/strict checks/build and native43briefing/48memory pass.

Real1942df955413/d47/unchanged medium on the same sources/questions completes7
foreground stages through late original. Later calendar query still fails33097
against32768 (30202input+2895output);30actual calls/228833tokens, all Returned,
valid read selections and temporary cleanup verified. No memory tail/end effect
scan reached, no blind replay or cap increase. Read seven replies/cell capture:
irrelevant lunch/injection item is included initially; original-first/duplicate
and continuation-hint UX remain. Static bytes decrease is not evidence of a
stable cumulative budget or useful item selection. Next attribution separates
tool/result description redundancy from facts and provider-owned continuation.
No Linux/deploy/realQQ/source authority/trial/Git change, full goal open.
External lain-source-guidance-2026-11-07/ retains all failures and evidence.

## 2026-11-07 — Short default with expandable evidence; retain actual cap failure

Source-only final replies default to exact source excerpts, essential clocks and
stable references. Generated notes remain explicitly expandable via detailed;
identical same-source/record/version quotations can share one body without losing
item numbers. Keep query/page/coverage warnings and separate original/empty footers.
Initial helper red catches English generated notes; actual original-footer red
and independent review close misleading footer and expanded-turn refresh bugs.
Native input remains single-submit: observe rendered draft before one Enter;
retain unsent-draft failure and improved redacted failure capture, not a claimed
product input fix. Two marker/7observer unit tests pass after native-interpreter
access correction. No terminal layout, schema or grant change.

First actual attempt9calls/58138tokens stops after valid final selection is emitted
as an undeclared function; normal invalid-tool handling adds a fourth completion,
foreground39216. Declare private briefing_answer only for eligible source reads;
normalize sole final choice into the same checked envelope, no Kernel Action or
extra roundtrip. Actual Actor red→green and rejection matrix preserve usage and
deny mixed calls/unknown fields/unread refs/reused IDs. Native51Actor/55briefing
filters overlap, fmt/strict checks/build pass, then43briefing/48memory fixture
requests pass; scoped independent source/protocol reviews close findings.

Actual4aa4f012c24c/d47/current medium next run completes7foreground stages through
short view, expansion, today-only reuse, late refresh and original lookup. Default
sample2221bytes; no controlled runtime/token savings claim. Calendar follow-up
then fails32982/32768 in3requests (30655input+2327output). All28provider completions
and215725reported tokens match Returned ledger, cleanup passes. Original analysis
incorrectly required originals-only footer on a mixed report+original reply;
retain first analyzer, repair offline without model replay. Read all7actual replies
and actual ConPTY short-view capture; no full memory tail or general prose/lifecycle
acceptance. Pagination hint and duplicate mixed view remain UX caveats. Next reduce
duplicated source guidance while retaining current intent/evidence/capabilities,
not raise limits or blindly remove provider reasoning. No Linux/deploy/realQQ/new
source/Git writes; full goal stays open. External lain-briefing-compact-2026-11-07/.
## 2026-11-07 — Bind final source speech to actual reads, retain UX gaps

Add a private source-only final selection envelope and bounded Kernel presenter
after actual briefing reads. It retains source provenance/time, original item
numbers, query/page scope, generated-reference labels and source-local speaker
labels. Unknown/duplicate/unread references fail with known failure categories;
returned usage stays accounted, previews/drafts held and publication checks stay.
No additional inference/read authority, schema or permissions change.

Initial actual9calls/51421tokens fail on one JSON fence surrounded by prose;
strict unique-fence compatibility discards drafts. Next actual9/50169 returns
valid JSON but selects3unread originals; correct Reference rejection is retained.
A real Actor red reproduces missing output choices. Per-request catalogs now
derive only individually renderable report/original/empty references, reusing
refresh checks and carrying no source body. Same strict final validation remains.
Earlier four static review findings and the shared-cache mutation restoration
are preserved in external evidence; no failure is relabelled as product success.

Final Windows49Actor/53briefing (overlap), fmt/strict checks/build pass. Native
24briefing and48memory fixture HTTP calls cover originals/paging/withdrawal and
reviewed save/correct/forget/restart/two audiences. Update obsolete memory fixture
reply to the selection contract, leaving lifecycle assertions intact. Actual
c53bacaddaac/d47/current medium three-question run:12calls/82471tokens, maximum
foreground30155, all terminal usage/Returned ledger agree, two7reference selections
bind actual reads. Today-only reuse, empty global Work/channel lists, empty current-
conversation active memory and cleanup pass; not a database-wide memory audit. Read all three
replies and actual ConPTY cells: no repeated Sunday/forwarding mistake, but English
generated summaries and verbose repeated blocks remain. Concise naturalness is
not accepted. Actual original/memory tail, native Linux/deployment/live sources
remain; cloudb9a+medium/full goal unchanged. See DAILY_BRIEFING_ACCEPTANCE and
external lain-briefing-presentation-2026-11-07/acceptance.md.

## 2026-11-07 — Make missing sender evidence explicit, retain new prose failures

Actual prior ordinary report lacks speaker fields while its answer claims the
same sender. Add static report attribution status and existing original lookup
pointer; source-local imported labels are not verified people or correction
authority. Preserve report12KiB, source/secret/publication checks, original DTO,
current-turn exact-receipt sharing and all authorization/schema/profile settings.
Red real lookup sees Null, red actual four-round input sees0shared fields versus2.
First green fixture fails because its report turn was never ended; complete it
through established fixture lifecycle and retain that failure. Final targeted3,
53briefing (including256maximum group,105.37s),4context, fmt/strict Clippy/build
pass. Scoped six-path independent static self-check has no evidenced blocker.
Native24HTTP/ConPTY/simulatedQQ calls pass;5report/original observations retain
explicit missing sender status and source-local labels in two isolated audiences.

One actual same-three-question run on729da4efc8e8/d47/current medium copy finishes
13requests/88276reported tokens; foreground13605/27700/28401<32768,8ordinary
clocks/6read metadata observations, all completions/Returned accounting agree.
No yesterday reprocessing for today's arrival, final noWork/channel/memory effect,
temporary cleanup pass. Analyzer initially leaves new field references unresolved;
preserve original KeyError,2red→green observer tests and7legacy negatives after
one-field whitelist update. Reconstruct unchanged raw requests offline; no paid
retry. Actual original briefing ConPTY cell capture viewed.

Manual all-three-reply readback does not show prior shared-sender speech, but
final prose calls the actual Friday source day Sunday and imported records group
forwarded notices. manual_check.py binds both to actual source texts/clock; full
prose remains failed. No new date prompt patch or extra paid run to obtain green.
Local-only, no Linux/deployment/newsource/QQ. Cloudb9a+medium, source proposal
and full goal unchanged. External lain-sender-attribution-2026-11-07/acceptance.md.

## 2026-11-07 — Verify a current reply frame without rewriting source time

The existing ordinary briefing scope now supplies a per-request epoch/local
time/today/tomorrow clock, moved into current Actor instructions on each round.
Old report calendars/source wording remain immutable; internal activities and
unavailable grants are excluded. No profile/schema/authority change. The earlier
wrong reply already had correct calendar dates, so do not claim sole causation.
Actual-four-round red→green plus fixed +8/-8 midnight/year-boundary tests cover
fresh request fields and unchanged historical calendars, not controlled-clock
Actor midnight E2E.43Actor/51briefing filters overlap;1clock/2calendar tests,
fmt/strict checks/build and independent five-path static review pass.

Native Windows HTTP/ConPTY/simulatedQQ24fixture calls pass12ordinary/12internal
clock separation and prior source/recovery/isolation checks. Initial observer
unittest invocation uses the wrong directory and imports no tests; preserve its
failure, then explicit discovery passes5tests. No product repair or paid retry is
attributed to that test-launch correction.

One actual current-medium copy run on631559bd33d3/d47 reuses the initial6notices
and first3questions, adding only the already-selected today-only notice.12model
requests/75120reported tokens, foreground6731/27647/27888 allbelow32768; every
provider completion matches Returned ledger.7ordinary clocks match their epochs;
same-correction item identities survive. No yesterday reprocessing on today's
arrival, no Work/channel/memory effect, all temporary state/processes cleaned.
Viewed original briefing ConPTY cell rendering, not real desktop/font evidence.

Manual readback no longer shows the old two-followups/source-day-as-today errors
in this run. It still asserts shared authorship without sender fields in the
ordinary read. Fixture truth is not model-visible evidence; preserve the finding
and avoid general prose acceptance. Late refresh/original/memory tails were not
rerun. Local-only candidate; cloudb9a+medium, private-source consent and full goal
remain. Evidence: external lain-reply-clock-2026-11-07/acceptance.md.

## 2026-11-07 — Observe the real clock chain, retain message-origin evidence

Use accepted local a717 and current medium8192/32768 on the same8synthetic notices
and first8questions, not the memory tail. A stale5eclient pin stops before SSH/
provider; actual d47 is accepted separately, corrected explicitly for a fresh
directory. Main run30requests/218365reported tokens reaches refreshed briefing,
original and both calendar/source-clock questions, maxforeground30964. All
provider completions match the returned ledger. Main exit1 is a collector KeyError:
private citations intentionally lacked record_id. Offline actual snapshot/item/
citation binding (with negative snapshot/index/quote/clock cases) recovers all
three matching receipt-clock paths; no paid replay.5observer+2binding tests pass.
Original failure and skipped final storage-wide no-effect scan remain explicit.

Static review catches manifest versus script bytes; prove original hash equals
current source normalized LF while actual file is CRLF. Record both, repair
metadata/diff without changing the live script; follow-up verifies resolution.
Native terminal capture viewed. Manual actual answers still fail message-count,
today/source-day, daytime/last-night and unsupported independent-new-notice claims.
These remain product issues even though explicit later clock questions are right.

One concrete information-loss fix follows: full report/source citations retained
the same ArchiveRecordId for a correction's two generated facts, but private
projection removed it. Keep that existing ID for equality, not chronology or
unprompted display; no source/permission/schema change. New regression first
failsNullversusUUID. Broader test catches obsolete omission assertion, changed to
exact retained equality. Next old+1000byte score fails at7921/7085; shorten only
the redundant projection note and retain that gate. Final43Actor/51briefing
filters (overlap; maxquote unchanged skip), fmt/strict Clippy/build and scoped
R2static self-check pass. No paid proof of corrected prose yet.

Native HTTP/ConPTY/simulatedQQ identity fixture first exposes stale historical
phrase assertion. Maintained script now checks actual assistant JSON terminal
metadata;2pure tests reject user/active/malformed/legacy-phrase cases. Local native
chain24fixture calls passes two explicitly authorized audiences,17source records,
12citation handles/8distinct originals, originals/cursors/reconnect/withdrawal/
noWorkMemory and cleanup. Final nonstring guard separately passes unit tests.
385185221be1 stays Windows-local, no Linux/deployment. Cloudb9a+medium, real-source
consent/full goal unchanged. External three acceptance directories retain scopes.

## 2026-11-07 — Separate internal evidence work from conversational instructions

Two bounded external low/medium attempts use the retained synthetic4record input,
same model/body except effort. The diagnostic observer raises AttributeError
before retaining terminal projection in both; no comparison or returned-usage
conclusion can be recovered. Preserve original script/report/log, correct the
misleading known-sum0 to actual-totalnull/two unreported attempts. Reproduce safe
nullable projection offline, then pass3observer tests; store known terminal
accounting before optional content checks, stop on diagnostic error and reject
rerunning an existing report. No third paid request or cloud profile change.

Independently confirmed unnecessary context: internal briefing still receives
companion voice and Work/memory policy. A real Actor request test first fails;
persisted briefing activities now choose a compact explicit untrusted-evidence,
no-tool/effect, credential/uncertainty base. Stage instructions, full source input,
output validation and Kernel gates remain. Ordinary chat/reading/sharing retain
their original base. No provider-specific core objects, profile, schema or grant.

Windows42Actor/51briefing/40reading-filter tests (overlap; prior same-tree max
quote evidence reused), format, strict all-target Clippy/build and scoped static
self-check pass. Actual native HTTP17records/5completed+1forged-rejected batches
preserve exact citations, restart identity, source invalidation and no tools.
Native HTTP+ConPTY2foreground/1failed extraction preserves ordinary instructions,
new internal base, failure/restore/no replay.9fixture calls total,0paid/realQQ in
runtime validation, temporary state/processes cleaned. Outgoing instruction hash
matches exact reconstruction from retained old stage suffix:7158to3738bytes,
3420redundant bytes removed. No token/speed/reliability improvement is inferred.

Binarya717c610baf6 remains Windows local; no deployment or actual-model acceptance
of new prompt. Prior8192reasoning/no-text failure and receipt-clock prose remain
unresolved. Cloudb9a+medium/identity/source consent remain; full goal open.
External lain-briefing-activity-context-2026-11-07 and extraction-diagnostic.

## 2026-11-07 — Report stopped preparation without another model or stale wait

Preserve two observed failures before implementation: corrected test reaches the
wrong terminal reply instead of factual failure feedback; a durable run-only stop
cannot wake the existing Actor-event wait in2seconds. The first test's8second
timeout collided with the actual8second readiness deadline; retain that false
failure and use20seconds for actor settlement without changing product limits.

Successful typed preparation status now directly gives pending or failed/stopped
feedback. Failed0processed differs from readable partial coverage; source-invalid
feedback omits counts. Full receipt and backgroundUnknown remain; foreground
completes with that truthful reply, no extra model/later tools/restart. Normal
completed/partial reads and private-input/memory/source publication checks stay.
The existing Kernel revision watch replaces Actor-only notifications, subscribing
before status reads; coordinator final commits wake it, with no polling or new
task/schema/configuration/authority.

Native Windows42Actor/51briefing-filter tests (overlap), formatting, strict
all-target Clippy and laind build pass. Unchanged maximum quote case is explicitly
skipped using previous same-tree evidence. Five-path independent static self-check
finds no evidenced blocker. Binaryca958b0215bb with real local Responses HTTP and
ConPTY passes initial failure, restored history and a fresh status question after
restart.2foreground+1extraction fixture calls,0paid/0realQQ,11079synthetic reported
tokens, no later Work action/Unknown replay, temporary processes/state cleaned.
Viewed02-restored-failure.png; actual cell-render evidence, not desktop/font QA.

No cloud install or Linux claim. The model's exhausted reasoning/output response
and receipt-clock prose remain unresolved; prior failed usage is not backfilled.
Cloudb9a+medium/source consent remain, full goal active. External
lain-briefing-terminal-status-2026-11-07/acceptance.md.

## 2026-11-07 — Account unsuccessful terminal responses without accepting them

Reproduce the observed11059usage drop through real ResponseStream parsing and
Actor admission: expected11059, actual0. The adapter now carries only valid
representable integer total_tokens alongside the existing allowlisted incomplete
cause. Actor records it once before returning that same failure, interrupts the
preview, persists turn usage/timing without source publication and never enters
the separate completed-format repair path. Missing/malformed/out-of-range usage
stays unknown. No schema, configuration, grant or historical backfill change.

Native Windows4target/41Actor/50briefing/11inference-filter tests (overlap), format,
strict all-target Kernel Clippy/build pass. Preserve intermediate test visibility
compilation failure and initial zero-match inference filter; tests move under the
adapter module and use only two test-fixture visibility changes, preserving
production privacy. Independent seven-path static self-check finds no evidenced
blocker. Actual native binary4bc443fff50a plus CLI and loopback streaming HTTP
passes known/missing/zero/other-reason and in-flight-revocation cases, restart and
duplicate-no-replay:4fixture calls,0paid/0realQQ,22118synthetic reported tokens,
cleaned temporary processes/state. Fixture usage is not a provider invoice.

Local only, no Linux/cutover. Earlier failed usage is not retrospectively repaired;
cloudb9a+medium/identity/data/source consent remain. Next address extraction's
reasoning/output failure and user feedback before another focused actual model
comparison. Full daily-briefing/selected-memory goal remains open. External
lain-failed-usage-2026-11-07/acceptance.md.

## 2026-11-07 — Readable archive receipt candidate and retained output-cap failure

Actual prior request24 formats source Friday14:00 but gives receipt Saturday02:17
only as milliseconds; speech wrongly places late intake on the source day.
Add archive_received_local_time to checked extraction/report/original views using
the existing fixed-offset formatter. Preserve raw values/text/source anchors and
all source/publication gates; invalid receipt yieldsnull without losing a valid
source time. Two real Actor/clock regressions fail before implementation and pass
after. This input improvement remains a hypothesis for prose, not proof of repair.

The first quote run finds a genuine new maximum-row regression:3pass/1fail,256
member conflict group cannot fit12KiB even with minimum nonempty quotes. Retain
failure; only in that extreme case omit redundant receipt labels with an explicit
notice, preserving raw milliseconds, full facts/relationships and original lookup.
Final4quote/3context projection, format, strict Windows Clippy and service build
pass. Independent five-path review plus fallback second round finds no evidenced
blocker, not deployment approval. No new schema/grant or page-cap increase.

One shortened actual-provider/nativeTUI comparison preserves the same8source
messages and first8questions, skipping unrelated memory mutations. It stops at
first extraction response.incomplete/max_output_tokens:8192output all reasoning,
0visible text,0published items,Unknown/no replay. Four checked cross-day receipt
labels reach actual extraction input, but report/original are not reached. Two
admitted calls: ledger6566reported tokens; raw terminal total17625 includes11059
known failed usage that adapter/ledger does not retain. Preserve both numbers and
cleanup, not a passing retry. Current output validator caps8192, so do not simply
write16384 or cancel32768 turn limits. Next inspect safe returned-error usage and
bounded extraction budget/failure feedback before another actual comparison.

Candidate7ac6015bc307 is local only; current cloudb9a+medium and private source
consent remain unchanged. Full goal active. External receipt-clock acceptance.

## 2026-11-07 — Protected medium-only adoption and current-profile full workflow

The cloud's low setting differed from successful isolated medium evidence.
Adapt the existing one-field profile operator to current schema40/four configs/
six-member backup and checked reader. Five conversion regressions first fail
against the old output-only transform, then pass;13local pure/16native guard
cases,3copy mutations and8actual isolated service faults pass. Independent static
review catches a missing public-verifier source manifest; supply its accepted
byte-identical copy and enforce manifest→plan identity before production. No
uploaded operator drift or policy/QA relaxation. Original low hash baseline stays;
new medium baseline is separately recorded for later maintenance.

Configuration operatione1edae96 is public-accepted. Actual pre-backup/current
JSON comparison proves reasoning_effort low→medium only, preserving8192/32768,
model/credential/timeout/vision. Same b9a executable/original40inode/identity,
other configs/reader/trial/native Codex host remain. Public Windows HTTPS/SSE/
loaded profile/CLI/installed ConPTY and independent six-member post-backup restore
pass;7backups8128346bytes,active timer,0builder/fixture. Maintenance invokes0model,
QQsend or newgrant; real personal-source proposal stays unanswered.

Then current cloud profile copy/accepted Windows binaries/8synthetic notices run
the full16foreground-turn chain:45actual requests/364893reported tokens, maximum
singleturn29754. Today reuse, late refresh/old immutable snapshot, one-question
original, date/source clocks, reviewed save/restart recall/exact User correction/
expiry/forget/reconnect pass. Paid-driver changes retain the same questions and
source text; only pins/current-profile requirement, safe diagnostics, exact intake
reference observation and a corrected Friday17:00 test expectation change. Two
observer tests are offline; an offline finalizer initially mistakes a trailing
Kernel identity annotation for User text, then readback repairs selection without
another paid run. Requests44/45 have no old502/503body. Temporary state cleaned.

Manual review of16answers retains a real failure: source Friday14:00 versus raw
Kernel receipt Saturday02:17 still yields “当天较晚才进入整理”. No general prose
acceptance; input clock-format asymmetry is observed, not proved sole cause.
Verbose page/group narration and underused User capture clock remain. Screenshots
are ConPTY cells, not desktop fonts; old default client still shows a missing-read
error, while the separate d47preview has a verified neutral branch. Full goal
remains active. Next: inspect checked receipt-clock projection/cross-day regression,
not another identical45-call paid loop. External medium-profile acceptance.

## 2026-11-07 — Preserve rejected extraction reasons and verify original follow-ups

Existing strict extraction gates now attach a closed static rejection reason;
the Actor publishes only safe diagnostics and preserves Unknown/no replay. No
raw parser error, rejected model body, schema or authority change. Real Actor
regressions cover seven categories and restart/duplicate recovery; two intended
red failures precede repair. Final Windows86briefing/39Actor filters (overlap),
format/strict checks/build pass; six-path static review is scoped separately from
intake semantics. Removing time_text's exact citation gate in a temporary copy
causes the actual Actor regression to fail;319 active inputs unchanged, copy gone.

Low actual four-question run reaches two originals but stops on an observer
assertion that incorrectly expects private source reads in normal action views.
Preserve its14requests/101522tokens and genuine last-night/last-chance prose errors;
no later withdrawal/restart credit. Offline observation instead binds the current
question to actual briefing_inspect outputs, source/text and call identity; four
negative cases pass without another model request or private-action exposure.

Same candidate/messages/questions with temporary medium finishes11requests/
76894tokens, both exact originals, withdrawal/restart/no replay and cleanup.
Manual readback confirms literal补收, attributed resend, relative November7 dates
and conditions; verbosity/numeric labels remain and first question returns
processing progress. This focused run creates no memory, realQQ, source grant or
cloud write, and is not full lifecycle/ordinary prose acceptance.25requests/
178416tokens total; earlier5/29483 remains separate. Screenshot is ConPTY cell
rendering, not native desktop/font evidence. Frozen319-input b9a217fa171a changes
eight existing Kernel/test paths versus cloud4b8, no additions/dependencies/schema.
This frozen source now passes native Linux86briefing/39Actor (overlap), all-target
checks with the existing1.89 style-only exception and release build.30guards,
3copy mutations and8native isolated fault cases pass; independent scoped static
review finds no evidenced new blocker. Three local preparation stops (fixture
marker count, generated set ordering, CRLF comparison) remain recorded; no gate
was weakened. Program-only operation3bd98b0b is public-accepted on original40/
inode/identity/config/reader/trial. Actual Windows HTTPS/SSE/CLI/installed preview
ConPTY restoration and independent six-member post-backup restore pass. Native
host identity unchanged;7backups7942554bytes,active timer,0builder/fixture. No model,
QQsend,newgrant or Gitwrite in maintenance. Medium adoption remains a separate
protected configuration operation. Full goal open, personal-source proposal
unanswered. External rejection and intake-release stages.

## 2026-11-07 — Separate intake clocks from publication claims, local candidate

Read the actual prior source/answer: literal补收 became补发. Extraction/report/
original now share interpretation-only intake_semantics: source/receipt clocks
alone do not prove publication history; ambiguous words stay quoted, an explicit
resend stays an unverified source report. Raw identity/text/clocks, gates and
Actor instructions are unchanged. The field joins existing exact-result/snapshot
sharing, not generic deduplication. Two intended red regressions precede repair;
intermediate case-sensitive assertion and unfinished test-turn mistakes are kept.
Final target tests,84briefing including maximum escaped conflict group,3projection,
format/strict checks/native build pass; filters overlap. Seven-path static self-
check finds no evidenced blocker, not actual-model or deployment acceptance.

Actual native TUI/cloud-low-copy attempt stops at extractionUnknown/model_failed:
4requests/25,482reported tokens, no original follow-up reached. Old trace omitted
visible extraction output, so the rejection reason is not recoverable or guessed.
A new observer plus one native batch-only diagnostic uses1request/4,001tokens and
accepts two items, preserving补收 and attributing explicit补发 as unverified. This
does not resolve the first failure or prove the full foreground scenario. All
5requests/29,483tokens and both cleanups remain. Future observer now preserves raw
visible output and failed batch/turn/budget before cleanup; it has not been used
to rerun the full scenario. Cloud4b8/low, sources/trial/host remain unchanged by
this local work. Concrete next task: retained safe extraction-rejection reasons,
then actual original follow-up. Full goal open; source consent still pending.
External: lain-intake-semantics-2026-11-07/acceptance.md.

## 2026-11-07 — A missing memory read is not a generic tool incident

The completed isolated model chain left a real Failed memory.inspect/not_found
receipt after forgetting. Keep that authoritative result but classify only the
exact read/false-success/not_found combination as not found or not visible. Do
not infer forgetting/deletion from a miss; other failures and turn errors remain.
Two red regressions precede repair; a test initially inspected body instead of
layout title, then required a mutable layout read. Both intermediate failures
are retained, and the final renderer/TUI regressions pass. Renderer8/desk43 cases
(overlap with the two targets), format, strict checks/build and scoped static
self-check pass. Native local Kernel/ConPTY with2simulated requests verifies
folded/expanded/restarted receipt and no replay; no paid model or realQQ.

Client d47f52f08e2b is installed in a new private preview directory using the
byte-identical previously reviewed installer. Existing default/profile/portrait/
credential files are never written and observed hashes remain. Actual installed
ConPTY restores the existing cloud test conversation, browses empty Sources and
returns/exits; not an actual missing-read branch on production. Full goal remains
open. Frozen319-file source4b8d15fa0954 has ten changed existing inputs and two new
test files versus cloud94922. Native Linux83briefing/39Actor/8sync/2client filters
(overlap), all-target checks with the existing1.89 style-only exception and release
build pass. Thirty guards, three temporary-copy mutations and eight actual isolated
native service fault cases pass; independent static self-check has no evidenced
blocker. Initial collector hash-basis/local mutation-row record ambiguities were
clarified before production; uploaded code stayed fixed.

Same40 program-only maintenance is now public-accepted on the original inode/
identity/config. Actual Windows HTTPS/SSE/profile/reader, existing CLI and new
installed preview ConPTY restoration pass. Post six-member backup independently
restores DB/configs; seven archives6,890,859bytes/timer active, no builder/fixture.
Local Codex host PID/start/path unchanged. Cloud remains low; this maintenance
invokes no model, realQQsend, source grant or trial extension. Earlier paid full
chain retains its Windows/synthetic/temporary-medium scope. External stages:
lain-memory-lookup-ui-2026-11-07 and lain-briefing-lifecycle-release-2026-11-07.

## 2026-11-07 — Reuse evidence instead of repeated framing and preparation

Trace the actual39562token failure: each historical source turn repeats its
explanation, and current reads repeat calendar/status/citation metadata. Keep
per-turn state/navigation with one bounded explanation. Share only equal known
metadata in the same request and snapshot, after exact matching against actual
executed results; preserve full data/input/audit and all normal read gates.
Tests first fail, then reconstruction/negative cases and real Actor-input fixture
pass. Initial test import error is retained separately.47briefing/39Actor-filter
checks overlap; one unchanged maximum-group case skipped. Format/strict checks/
build and scoped independent review pass. Removing the exact-match fence in an
external copy triggers its intended assertion; project source never mutated.

Actual low run uses the layout but fails source-versus-reply-day wording (23calls/
144915tokens). A single isolated medium comparison stops at an unread second page
(21/128549), not a token fault. Trace reveals guidance always re-prepared ready
work, spending an unnecessary model round. Correct only that workflow instruction:
checked latest read first on prepared follow-up, refresh when required, continue
authorized actual cursor pages. Keep both failures and unchanged scenario checks.

New pinned8a9c5d84b08c/native Windows plus isolated medium completes the original
full synthetic-message chain:16foreground turns,45requests/370647reported tokens,
largest foreground28968. Actual provider, terminal, Kernel and reviewed memory;
no liveQQ/cloud mutation. Read back correction/deadline/forget outputs and labelled
ConPTY screens; cleanup passes. Default cloud low profile is not changed. Verbose
speech, unsupported补发 phrasing and a post-forget NotFound error card remain
qualified; no general factual-quality, Linux or personal-source acceptance claim.
External: lain-briefing-context-2026-11-07/acceptance.md.

## 2026-11-06 — Progress is not another round of old-report reads

Reconcile97numbered requirements plus10normative clauses against31current/evidence
paths without declaring completion. Old missing correction-clock/follow-up gaps
are resolved by accepted94922; actual QQ intake still awaits the existing source
answer. Run one justified current full-chain model comparison after those repairs:
it fails at late refresh35078tokens. Preserve18requests/117160reported tokens and
cleanup; no later memory credit.

Actual trace identifies old report reads after new nonterminal preparation. Add an
Actor-entry regression that fails on2foreground requests, then a typed status
completion branch producing one factual progress sentence without another model
or later batch call. Background/old explicit history/current input/source gates
remain. Initial private-module test access compile error is kept; move only that
test to its owning module.46briefing/38Actor-filter cases (overlap; one unchanged
max-group case skipped), format/strict Clippy/build and independent static review
pass. Candidate is local only.

Same questions/data/assertions with the new pinned binary pass the refresh point
with1request/7294tokens, proceed through new original/calendar reads, then fail
source-clock follow-up39562tokens from repeated searches.30requests/215008tokens,
no memory/correction/forget steps executed in this run; cleanup verified. Known
prose errors and both unsuccessful full-chain runs remain. No cloud/source/QQsend/
trial/Git change. External: lain-briefing-pending-2026-11-06/acceptance.md and
lain-goal-audit-949-2026-11-06/requirements-audit.json with current addendum.

## 2026-11-06 — Source quota and service capacity are different stops

Fix a misleading Sources explanation: a finite plan can stop with unused quota
when the existing shared read-receipt ledger is full. Show the actual distinction
without increasing limits or discarding receipts. A view regression fails before
repair; an actual isolated Store test characterizes capacity, preserved history,
no spend and explicit revocation. Eight sync/four view tests, formatting, strict
all-target Kernel/client checks and native client build pass.

Static installation review catches a configuration check/replace race before
execution. Replace that unexecuted plan with a fresh preview directory and entry
that never writes existing configuration or launchers. Actual installation and
cloud-connected Windows ConPTY restoration/Sources/return/exit pass; manually
read the labelled ConPTY render. No new model call, QQ send, source grant, cloud
cutover, host restart or Git write. Default entry remains unchanged. Evidence:
external lain-source-capacity-2026-11-06/acceptance.md.

## 2026-11-06 — Checked source follow-up reaches the cloud

Deploy source94922bbab226through the existing same40 guarded maintenance on the
original database and identity. Exact six compiled-input compatibility, scoped
native checks,30guards,2copy mutations and8isolated systemd/SQLite/HTTP cases pass.
Independent review catches a stale collector derivation record; refresh evidence
before launch without changing uploaded code or weakening gates. Actual Windows
HTTPS and installed ConPTY restoration/source browsing pass; post six-member
backup independently restores DB/configs. No model, QQsend, source enablement or
Git write accompanies maintenance. Current configs/reader/trial/native host remain.
Personal-source consent and general prose quality still limit full-goal acceptance.
See CLOUD_OPERATIONS and external lain-reply-release-2026-11-06/acceptance.md.

## 2026-11-06 — Observed reference is not a resolved message, local

Follow the actual original-read evidence rather than treating a completed flow
as correct prose. Explicit unknown target chronology removes the earlier claim
but still permits a same-source assertion in one7request/48077token attempt.
Keep it. Reshape the private adapter's unverified references as observed markers,
raw reference IDs and null resolved targets; source identity of the containing
message is unchanged and no target is fetched. Plain fingerprints,4/128bounds,
8192byte total validation and current authorization remain. New HTTP shape test
first fails; final9knowledge/strict checks/native build pass.37Actor checks cover
the unchanged shared guidance; independent static second-round review is scoped.

Final actual service/nativeTUI/provider test completes original read, source
withdrawal and restart with7requests/47422tokens. Manual readback retains exact
quote/conditions, correct source clock and unknown event day, without claimed
target source/chronology/correction; still one unnecessary native ID. General
model compliance and realQQ remain unproven. Total14requests/95499tokens. Native
Linux6156228f82f1previous candidate now passes9knowledge/37Actor/8lookup/Clippy/
build and is hash-pinned; new94922bbab226is in isolated Linux staging. No cloud
cutover, source enablement, QQsend or Git write. Full goal remains active with
real-source consent pending. External: lain-reply-chronology-2026-11-06/acceptance.md.

## 2026-11-06 — Source interpretation and recoverable original follow-up, local

Clarify that a source clock cannot supply a missing event day and an observed
reply ID does not establish target existence/source. Refine the unreleased reply
metadata to separate lookup restriction from resolution. The unchanged actual
service/provider driver then finds a genuine follow-up failure: historical source
bodies were correctly omitted, but all navigation was also lost. Keep bounded
executed report/item/citation pointers in later Actor context, requiring ordinary
current-authority reads. No schema, permission, source or stored-body change.

New actual Actor-input regression fails before repair, then9lookup/37Actor,
format/strict checks/native Windows build pass. Independent static reviews cover
the interpretation and follow-up deltas separately. Same synthetic source with
actual WSGI/nativeTUI/model now returns the original in one question, withdraws it
and survives restart without restoration:7requests/47332reported tokens. Initial
5requests/30396token refusal and its unreached lifecycle steps remain recorded.
Model still calls an unresolved target earlier and uses unnecessary technical
metadata, so flow success is not general prose acceptance. Source6156228f82f1 is
now native-Linux checked and independently hash-pinned; cloudf65a and the original personal state remain.
No real QQ send/source enablement/Git write. Full goal awaits real-source consent.
External evidence: lain-briefing-interpretation-2026-11-06/acceptance.md.

## 2026-11-06 — Preserve observed reply references, local

Fix the explicit reply-association gap at the existing knowledge adapter, with
bounded4reference projection, no target fetch/new authority and unchanged plain-
message fingerprints. Missing-reference and later plain-fingerprint regressions
first fail, then Windows9knowledge/service contract/strict checks/build pass.
Temporary-source segment-bound mutation is killed/cleaned; static review clean
within scope. Actual independent service/nativeKernel/TUI uses6simulated calls;
future render observation repairs an early screenshot without product changes.
First fixture's fresh-admin-owned-root ACL refusal is retained; only newly created
empty temp-root ownership preparation changes, original guard stays. Real model
uses8requests/49428reported tokens, retains actual references/quote/conditions,
then source withdrawal/restart passes and state is cleaned. Manual undated-time
day inference and target-certainty overstatement remain, not semantic acceptance.
Source58dd has2compiled changes; Linux9knowledge/Clippy/build pass; candidate remains undeployed. Cloudf65a
unchanged, no realQQ/source consent/Git action. Full goal active. Evidence:
external lain-reply-references-2026-11-06/acceptance.md.

## 2026-11-06 — Correction capture and independent User recall deployed

Sourcef65a3f48d94b (317files, six compiled changes versus6ab) is public-accepted
through same-schema40 maintenance on the original database and identity. Checked
User-message capture shares the ordinary read lock; recall guidance preserves
current User words, and metadata explains the existing independent correction
dependency. No schema/grant/retention/config change. Native Linux66memory/37Actor
filters overlap, all-target checks with the existing1.89 style-only exception
and release build pass. Windows scoped checks/build and earlier clock evidence
retain their exact candidate scopes.

Actual Windows model/nativeTUI uses15requests/110397reported tokens: reviewed
save/exact correction/current recall, forget only the synthetic original, then
read the byte-identical active User record. Final speech correctly attributes
the words, distinguishes capture/expiry and denies automatic correction loss
after old-source withdrawal. Requests14/15contain no old502body. Verbosity and
omission of requested concrete storage timestamp remain; general prose/realQQ
acceptance is not claimed. Initial guidance-only false dependency warning stays.

Final30maintenance guards,2temporary-copy mutations and8actual isolated systemd/
SQLite/HTTP fault cases pass. Initial stale3path positive fixture is repaired to
the exact6 reviewed inputs without loosening production gates; failure retained.
The failed guard-only bundle is hash-archived locally and exact-filecleaned.
Actual publicHTTPS/auth/bootstrap/policy/profile/SSE/reader and installed native
CLI/ConPTY restore/source browse/return/exit pass. Post sixmemberbackup restores40/
integrity/identity/DBhash/fourconfig hashes. Original DBinode, configs, reader,
trial and native Codex host identity remain.7backups3427137bytes,timeractive,
0builders/0temporaryfixtures. No model/QQsend/newgrant/Gitwrite in maintenance.
Sources0/pending real-source consent keep the full goal active. Evidence: external
lain-correction-recall-release-2026-11-06/acceptance.md and sibling recall stage.

## 2026-11-06 — Checked correction-message capture clock, local

Ordinary current/scoped memory reads can resolve exact persisted correction User
message capture time under the same Kernel lock. Match conversation/message/role/
status/exact quote; expose only identity/capture UTC and explicit non-platform,
non-event/non-storage basis, with no timezone inference or record rewrite. Missing
match staysnull, original16KiB full-result fallback remains. No schema/grant change.
Cross-day delayed-review regression first fails, then passes; mismatched body/role
and foreign/duplicate identities are denied by the relevant existing/new gates.
Initial invalid negative-fixture expectation is retained/repaired without weakening
snapshot checks. Windows66memory/37Actor filters (overlap), format/strict checks/
build pass; temporary-source User-role mutation is killed by the actual regression
and cleaned. Independent static review finds no evidenced scoped defect.

Actual model/nativeTUI over one synthetic notice completes15requests/111728reported
tokens. Requests13/15receive unchanged record and exact original User-message clock,
distinct from storage/expiry. Explicit follow-up distinguishes clocks and retracts
prior timezone assumption; ordinary recall still speculates about an ambiguous day.
Keep that manual failure/verbosity: no general prose/date or realQQ acceptance.
Temporary state cleaned. New sourcea970fa81ec43 is in confirmed-live native Linux
staging, not deployed; accepted cloud remains6ab/schema40. Real-source consent/full
goal stay pending. Evidence: external lain-correction-clock-2026-11-06/acceptance.md.

## 2026-11-06 — Audit full goal and retain broader-context qualification

Read back ten actual evidence reports and map97numbered/bulleted requirements
without declaring full completion. Correct stale Current usable deployment39
text to actual40/6ab, source-setup remaining tasks and README demo status. Recheck
only the retained8notice context after the attribution change:44actual requests/
315933reported tokens reach reviewed correction/current recall and expiry; the
reply attributes replacement to User, but uses storage time as date corroboration.
A legal metadata-inspect expiry query is falsely refused by an annotation-only
harness gate. Preserve stopped run/cleanup; no later forget/restart pass is claimed.
Repair future observation only, offline actual-helper positive plus5identity/
state/tool negatives pass,0extra model calls. No product/cloud/grant/Git change.
Fresh metadata Sources0 and pending real consent remain. Checked correction
message-time projection is a concrete independent next gap; general prose and
long-term finite-sync operation stay qualified. Full goal active. Evidence:
external lain-goal-audit-2026-11-06/acceptance.md and requirements-audit.json.

## 2026-11-06 — Selected-memory deadlines and speaker attribution deployed

Snapshot 6ab1dbfbcc0e is public-accepted through same-schema40 maintenance on the
original database and identity. The three compiled changes add optional checked
UTC expiry/visibility and typed source-versus-user-correction interpretation;
full records, existing16KiB limit, permissions, deadlines and source scope remain.
Native Windows and Linux65memory/37Actor filter cases pass (overlap), with format,
strict Windows checks and Linux all-target checks using the existing1.89 style-only
exception. Actual-provider evidence is the earlier focused native Windows run:
15requests/110986reported tokens, reviewed save/correct/recall and attribution on
one synthetic notice; not real QQ or universal factual-quality acceptance.

Final maintenance30guards, two temporary-copy mutations and eight actual isolated
systemd/SQLite/HTTP fault cases pass. Initial inherited-fixture marker failure is
retained; all3markers and both exact uniqueness-error assertions are repaired.
Independent static review has no unresolved evidenced defect. Two obsolete guard
bundles are independently hash-archived locally and removed by exact file paths.

Actual Windows HTTPS/auth/boot/SSE/reader and installed native CLI/ConPTY restore,
source-page browse/return/exit pass. The post-upgrade six-member backup independently
restores40/integrity/identity/database hash and four config hashes. Original DB
inode, configurations, reader, trial and native Codex host identity remain.
Seven backups total2,066,831bytes; timer active, no builder/temporary fixture.
No model call, real QQ send, new source grant or Git write accompanies maintenance.
Sources remain0 and pending real-source consent leaves the full goal active.
See external lain-memory-attribution-release-2026-11-06/acceptance.md and sibling
lain-memory-attribution-2026-11-06/acceptance.md. Earlier local-only entries below
are historical; neither deployed annotation guarantees arbitrary model prose.

## 2026-11-06 — Attribute selected-memory words to their own speaker, local

Actual retained reply misattributes the exact User correction's today to the source
notice's tomorrow; stored record/provenance are correct. Optional checked memory
metadata now labels typed source selection versus verbatim User correction and
separates speaker-relative dates from storage/expiry clocks. No record/schema/
policy/source change; same16KiB whole-result fallback. Regression first fails,
then65memory/37Actor filters (overlap), format/strict checks/native Windows build
pass. Static independent review finds no evidenced defect. One native-terminal/
actual-provider synthetic notice completes15requests/110986reported tokens; both
current recall and attribution replies correctly name the User origin. Actual
record projection equals storage; temporary state is cleaned. This is focused,
verbose prose, not general factual or real QQ acceptance; old extraction inference
remains. Prior deadline-only dbfb snapshot now passes native Linux and pins its
own binary. New attribution source6ab1dbfbcc0e is in a distinct live Linux job;
no newer deployment. Cloud stays d769/schema40; real-source consent/full goal
remain pending. Evidence: external lain-memory-attribution-2026-11-06/acceptance.md.

## 2026-11-06 — Make selected-memory expiry explicit without changing its lifetime

Broader-context verification of the retained memory issue uses45actual requests:
workflow succeeds but generated summary speculates about an earlier notice and
corrected recall invents a today/window expiry. Root evidence separates stored
UnverifiedSelection inference from valid User correction and its real deadline.
Do not erase these manual failures. New actual checked-read regression first
fails, then projection adds exact stored expiry/UTC milliseconds and fixed scope
versus lifetime interpretation for selected/corrected records only. Original full
records stay equal; optional annotation cannot displace them at the same16KiB
bound. Empty/other origins remain unchanged. No schema/TTL/grant/config change.

Native Windows65memory-filter/37Actor-filter (overlap), format/strict checks/build
pass; UTC/range/bound/type-origin checks pass. Independent static review finds no
evidenced defect. Final47actual-request native-TUI/synthetic scenario checks the
actual emitted deadline and correct window explanation, then forget/restart with
both old/new body markers absent from actual requests. All3scenarios113actual
requests/833747reported tokens; cleaned. General wording is not accepted: another
answer attributes the User correction's today to the original notice and evades
its event date. Retain that qualification. Windows0af0c716ee51 is local; native
Linux sourcedbfb6fb4269e is staged, not deployed. Cloud/source grants/trial/QQ/Git
unchanged, full goal active. External deadline-fix.md holds exact evidence.

## 2026-11-06 — Verify exact-item memory content through real reviews and model inputs

Follow the retained wrong500/501/previous-notice examples with one isolated actual
source-backed memory scenario on the accepted d76923f31fc1 source, without changing
code or repeating broad refresh tests.21actual model requests/156198reported tokens
complete nine foreground interactions in53.02s. Native TUI all-page decisions
save one UnverifiedSelection record, restart/read it, correct via exact current-
user quote to503with Explicit provenance/same scope/expiry, read the replacement,
then forget and restart. Actual stored records/IDs/404/empty page agree with final
speech. Images and all replies are read;500/501 are absent in this case only.

Synthetic request capture binds phases to the exact current User MessageId.
Corrected recall has no old502body, and only gets503after native retrieval; both
post-forget requests lack either body. Initial analysis wrongly uses the last
wire user role (Kernel-attributed identity data), so phase count fails before any
body assertion. Offline reanalysis fixes that evidence query, not product code;
no extra inference. All temporary state/server/terminal are cleaned. No new Rust
checks/rebuild/deploy/source grant/QQsend/trial/Git action. Source metadata GET
still reports0. Retain old multi-notice failures and verbose/ambiguous organizer
wording qualifications; this is not universal factual quality or real-source QQ
acceptance. External evidence: lain-memory-fidelity-2026-11-06/acceptance.md.

## 2026-11-06 — Context/calendar/quotation repair deployed on the original40 database

Source snapshotd76923f31fc1 (317files/ten reviewed compiled changes) is now
public-accepted through same40 program maintenance. Native Linux75briefing
(including the maximum conflict group),34Actor,2API and11foundation cases,
all-target checks with the existing1.89 nonminimal_bool style exception and
service build pass. The independent pinned executable matches its unique build
log hash. Thirty native maintenance guards and eight actual isolated systemd/
SQLite/HTTP fault cases pass; two temporary-copy mutations prove schema and QA
candidate bindings are guarded. Static maintenance review finds no evidenced new
defect. No schema/config/source scope change is introduced.

The original database inode/identity, four configs, loaded reader, model limits
and trial expiry remain. Actual Windows HTTPS/boot/auth/SSE, existing installed
CLI and ConPTY reconnect/source browse/return/exit pass. The post-upgrade six-
member backup independently restores40/integrity/identity/database hash and all
four configs. Seven archives total2,066,845bytes; timer active, no builder or
cutover fixture remains. Original native Codex host is still observed running.
No model call, QQsend, source grant, Git write or publication accompanies this
maintenance. QQOnline is connection evidence only. Source0/pending consent and
qualified memory/prose quality leave the full daily briefing goal active.

Actual-provider evidence remains the prior native Windows/synthetic-source
32actual requests/205639reported tokens plus1injected17token reply, not paid Linux
or real QQ acceptance. See external lain-context-release-2026-11-06/acceptance.md
and sibling lain-briefing-context-2026-11-06/. The user walkthrough is now in
DAILY_BRIEFING.md. Earlier local-only entries below retain historical meaning.

## 2026-11-06 — Reduce repeated briefing metadata without weakening evidence

Actual prior failure totals34093turn tokens after one malformed argument. Keep
32768 and exact native parsing; project only the already checked model-bound
BriefingRead result. Full API/audit result persists. Omit redundant UUID/revision/
epoch fields, add actual citation positions and lift identical scope text once;
retain all facts, checked excerpts, own clocks, corrections, members, unknown
fields, dates, matching indices, cursors, coverage and omissions. Refresh/empty/
error/other tools remain unchanged. Shorten3descriptions without schema/policy
changes. Actual Actor recording regression first fails, then74briefing-filter and
34Actor-filter cases pass; sets overlap. Fix test-module position after initial
Clippy refusal; final format/strictKernel all-target checks/native build pass.
Static review finds no evidenced scoped defect. Fixture7057→5924bytes; initial
tool definitions16419→15275bytes. This is model context, not Worker compaction.

Actual HTTP/Responses/nativeTUI/simulatedQQ fixture passes10requests/100tokens with
one-question repair/original each and one QQ final. Actual provider over synthetic
sources: final32requests/205639reported actual tokens +1synthetic17token error,
121.70s, all8turns under32768 (max29277). Checks cover immutable late refresh,
unchanged-day reuse, same-question native original after injected bad format,
10:00/10:04/14:00 clocks,201→301, correct current-day dates and restart inspection.
Final images and all prose/source reports are read. No explicit read retry is used;
refresh_required notice is not observed because preparation is called directly.

Retain first attempt's measurement JSON assumption failure (one reserved request
not forwarded) and second's genuine HTTP/provider failure/Unknown/no automatic
retry. Three attempts total70actual attempts/421800reported actual tokens plus2
synthetic replies/34tokens; missing returned usage is not estimated. Textual error
measurement is repaired without modifying forwarded requests. All owned temporary
state/processes are cleaned. Prose remains verbose/overcautious and occasionally
confuses item/record counts; do not claim general factual/memory-prose acceptance.
No real QQ/source/trial/cloud/Git changes. Local2ffc6d4bc70e remains undeployed;
Linux/release/real-source proof remain. External evidence:
lain-briefing-context-2026-11-06/acceptance.md. Full goal active.

## 2026-11-06 — Reject text invocations before preview/completion and account one safe regeneration

The actual original-message failure is a whole DSML envelope returned as ordinary
assistant text, Completed with no native action. New native regressions first
reproduce completion and split-preview acceptance. The Responses adapter now
recognizes that whole-response prefix only, refuses it without parsing names or
arguments, and emits a body-free normalized format error with returned usage.
Ambiguous preview prefixes are held across deltas/item-done frames; literal code
examples and normal native calls remain usable. No provider envelope type leaks
into domain or frontend records.

Four Actor regressions first fail for missing regeneration/accounting. The
repaired path charges returned tokens/timing before deciding, records preparation,
and admits at most one regeneration through the normal loop and unchanged four-
request/four-tool/token/time limits. Only actual completed allowlisted reads (or
no actions) qualify; effects/uncertain/executing/preparation/wait/finish/background
activities do not. It neither parses text into a tool nor replays effects. Normal
network/incomplete errors retain no automatic retry. Exhaustion notice discloses
the actual one repair. Final32Actor/71briefing, format/strictKernel all-target
checks/build pass; unchanged costly max-group evidence is reused, not rerun.
Independent static review checks freshness, accounting and preview boundaries,
finds no evidenced defect and runs no tests/remote calls.

Real local Responses HTTP and native terminal plus simulated QQ prove the same
question recovers a malformed response, reads/inspects one actual fixture record,
answers within four requests and sends one QQ reply. Ten HTTP requests/100fixture
tokens match the ledger. Initial harness assertions wrongly inspect deliberately
hidden source actions and Tool notes in ordinary snapshots. Another expects an
unneeded singleton index and leaves a SQLite connection open (context manager
only commits). Read-only isolated ledger diagnosis proves two extraction plus
eight foreground calls. The exact owned residual is checked/removed; corrected
native-wire-4 passes with cleanup. Earlier failures stay separate from product
proof. This is simulated provider/QQ, not paid-model or personal-source acceptance.

The actual-provider attempt never reaches planned fault injection. It accounts
26calls/159,799reported tokens; refreshed-ready hits32768 after four requests total
34,093tokens. Trace shows page1 read, malformed unquoted UUID in page2 arguments,
corrected page2 and final prose; invalid JSON is not executed. The turn is Failed,
not Unknown despite the old driver's generic assertion label. No injection/repair
is claimed for this real run. Preserve it as context-budget evidence; reduce
redundant model-only context next without losing source/scope/currentness or
raising the bound. All actual and fixture services/terminals/state are cleaned.
Local service393f4a6c8a68 is not deployed; cloud, source authority, trial and Git
remain unchanged. Full goal active. Evidence: external lain-reply-format-2026-11-06/.

## 2026-11-06 — Make source-relative and answer-relative dates explicit

The retained real reply has correct explicit dates but calls the same current-day
arrangements tomorrow evening. Existing input contains local day bounds, but not
an explicit separation of the two relative-date frames. Add one bounded calendar
projection to report/original reads: source yesterday/today/tomorrow and next
Monday-to-Monday, versus this read's today/tomorrow, in the grant's fixed offset.
Only occurrence-based citations may use source anchors; no activity-date parser,
original/summary/memory rewrite, schema change or new authority is introduced.

The report regression fails before implementation. Two pure date cases then pass;
the first integration fixture incorrectly completes a still-Executing read and
is repaired without weakening Actor validation. Final72briefing/25Actor/2API,
format/strictKernel all-target checks/native build pass. Cases cover cross-year
positive/negative offsets, Sunday/Monday, historical source versus reply dates,
unrepresentable time and the actual maximum256-item conflict report. Static
independent review finds no evidenced defect and runs no tests/model requests.

Native Windows actual-provider synthetic workflow uses final a7340dccac70 service
and unchanged5e8835ee20d5 client:50accounted/protocol requests,373,508reported tokens,
145.83s;8synthetic messages/7selected yesterday records.24actual input observations
include calendar. Today/next-Monday and10:00/10:04/14:00 challenges, fresh snapshot,
reviewed selected memory, restart recall, correction and forget/reconnect pass.
Native terminal cells are viewed. Automated success remains qualified: original
first reply is Completed/noactions but contains DSML invocation text, so exact
inspection requires a second user turn. Manual source/store comparison also
finds invented prior vague notice and501/500 comparisons; actual original is502,
replacement503, and selected stored bodies contain none of those added stories.
No universal prose or one-turn original acceptance is claimed.

All isolated services/terminal/state are cleaned. No real QQ send/source grant,
private message read, cloud write, trial extension or Git publication occurs.
Cloud remains prior schema40 program. Continue the provider-format/completion
boundary before a consolidated deployment, keeping the source consent pending.
Evidence: external lain-calendar-frames-2026-11-06/ (native logs, actual protocol,
manual-model-review.json, source/memory comparisons and acceptance.md).

## 2026-11-06 — Ground foreground prose in checked quotations without breaking bounded reports

Retained actual-model output assigns an unrelated14:00 source clock to the
course original/correction despite correct Kernel clocks. Foreground reports
previously omitted the checked source words. Add one literal at-most128-byte
UTF-8 prefix per citation, an explicit truncation flag and data/clock semantics.
Full quotations use the current redactor before clipping, including owner HTTP
reads, so a newly configured secret spanning the prefix boundary is refused.
Original inspection, complete summaries, whole-manifest capture and currentness,
query numbering and memory authority remain; no schema/grant change.

Missing-excerpt and current-secret regressions fail before implementation. The
three first scoped tests and native68briefing/2API/25Actor/strict checks/build
pass. Independent static review then identifies a valid single-row overflow with
256conflict members, escaped maximum-length fields and8citations. The first
maximum fixture attempts fail for stale-grant correction and an oversized fake
extraction response; they are not product reproductions. The latter's panicked
isolated child is explicitly stopped after process identity checks. Corrected
64-batch extraction/actual relation processing completes, then report reading
fails (102.21s). Binary search now preserves the entire row while shortening only
marked excerpts against final serialized12KiB, retaining at least one character.
The same maximum case passes within the four scoped tests (100.13s);65other
briefing and2API tests, strict checks/format/build pass on the final code. Re-review
finds no new evidenced defect, without claiming runtime execution by reviewers.

Actual-provider/native-Windows first candidate ec690f9c9859 runs29accounted requests,
185,095reported tokens/98.05s over synthetic isolated messages. Foreground traces
contain29citation excerpts and11report-semantics observations. Refresh, unrelated
today reuse, correct source-clock challenge, one-turn late original and reconnected
correction original pass; viewed terminal cells show the factual challenge. Manual
readback also finds a real remaining "tomorrow evening" error after correct explicit
current-day dates. Automated pass is therefore not overall factual-quality approval.
Final7d8a936b5b13 byte-repaired binary is built locally but not model-retested or
deployed. Temporary model state/terminal/services are cleaned. No personal QQ,
source grant, cloud write, trial extension or Git/publication occurs. Full goal
remains active; continue current-day prose grounding and authorized real-source
acceptance. Evidence: external lain-quote-evidence-2026-11-06/.

## 2026-11-06 — Deploy phrase lookup with an explicit39→40 recovery boundary

Final317-file snapshot6a7fd0e92d9c passes native Linux11foundation/all-target
checks with the existing1.89 style-only exception and build. Earlier65briefing_
plus2API/25Actor evidence is reused only after whole-source comparison proves
the sole difference is two foundation current-version expectations. The accepted
executable is independently bound to its own build-log hash. A collection SSH
interruption and public timeout precede production admission; finite probes
distinguish reachability, then observation/collection resumes the same terminal
job without starting another compiler or replaying a service mutation.

New maintenance regressions first reject old39 publication/verification and old
reader startup against40. The repaired scoped derivative admits exactly ten
compiled changes/two additions, verifies40, and requires39 again before starting
the previous program. Full-row stopped39 equality remains the only previous
selection path; migrated40 or uncertain/live/new-data failures preserve state.
Initial test discovery imports a TestCase twice; that fixture issue is removed,
not counted as independent coverage.24native guards pass. Six additional local
tests execute the actual production entry's admission prefix: five expose missing
candidate/source/previous-runtime binding or duplicated fault coverage before
repair. Native final bundle runs all30 guards successfully; its local actual
production entry is SHA-bound to the tested copy. Static independent reviews find
no evidenced defect, without claiming to run those tests.

Ten actual isolated systemd/SQLite/HTTP cases pass with cleanup. They cover normal
upgrade, backup failure/missing reader, pre-start39 failure, live observation loss,
new accepted data, config/reader drift, failure after migration before verification
and stopped40 without new data. The happy case restores a six-member40 backup.
Production PID/inode are unchanged during these fixtures; no model/QQ is invoked.

One actual production operation is now public-accepted on original inode/identity
at40 with all four private configs, reader, limits and trial expiry retained.
Public HTTPS/auth/bootstrap/SSE/profile/reader and installed Windows client reads
pass. Actual ConPTY reconnects/restores the dedicated old test answer, browses
empty Sources, returns and exits; its cell render is viewed. Post-upgrade backup
restores40/integrity/identity/hash/configs. Seven archives total2,066,844bytes;
timer active. Three never-production guard-only bundles are hash-archived outside
Git, rechecked and removed by exact file paths, freeing316,690bytes. Current/
previous programs, backups and actual operation bundle remain. No builder or
temporary fixture remains. No commit/push/publication/source grant or trial extension.

Real sources remain0 and the existing specific consent is unanswered. QQOnline
is connection evidence only. Actual-model synthetic lookup proof is inherited
from the preceding entry; this maintenance adds0model calls/QQsend tests and does
not prove general prose quality or full personal briefing/memory acceptance.
Evidence: external lain-phrase-release-2026-11-06/ (native builds,30guards,
10cases, review, public/backup/UI/cleanup/inventory) and preceding lookup stage.

## 2026-11-06 — Locate an original by topic without paging through the entire report

Actual-provider traces previously show a four-request original follow-up ending
with summary-only evidence. The report had only cursor reads while inspection
required an item number. Extend the existing read with a nullable bounded literal
query over validated generated title/summary/time and checked citation quotations.
Filter before normal page/byte bounds, retain original IDs/numbers and expose
matching citation indices; matching folded originals are independently locatable.
Whole-report counts remain separate from match counts. Empty matches carry an
explicit non-absence limitation. Current/freshness/redaction/disclosure gates are
shared with ordinary reads; no extra source scan, model activity or permission.

Three new boundary tests first fail on old unknown arguments. Initial green work
uncovers fixture errors: a synthetic late item retains a time phrase absent from
its checked quote, and pagination leaves the previous foreground turn unfinished.
Diagnostics prove these causes; the fixtures are repaired. Windows67briefing and
25Actor tests, strict all-target Kernel/wire checks, format and native service
build pass. Eleven existing foundation tests additionally pass after changing two
post-migration current-version expectations to40. Historical1 and future99 stay.
Schema040 is only a reader gate; existing39 data/tables/legacy actions are retained.
Independent static review finds no evidenced defect, including those expectations.

Actual extraction over13explicitly synthetic notices places the target beyond
page1. A fresh foreground receives neither item number nor preloaded report.
Its real protocol trace shows query -> item13 -> exact original. Restart/reconnect
then reopens that same source. The passing scenario uses12actual requests and
63,016reported tokens/54.25seconds; foreground queries use4/6.12s and3/5.52s.
An earlier nine-call attempt also reads the correct original but its harness
mistakenly requires read-only calls in persisted UI action history; fixing the
assertion to actual protocol calls is recorded. Both attempts total21requests/
103,117reported tokens. Two actual ConPTY cell renders are viewed; temporary
databases/services are removed. These are not personal QQ/source or general
reliability acceptance. Evidence: external lain-phrase-lookup-2026-11-06/.

The317-file b11f7b9689bd Linux candidate passes65briefing_/25Actor, existing1.89
style-only-exception checks and native build. Exact Windows/Linux test-name
comparison identifies two excluded API cases; they pass separately on that same
frozen candidate. A final launch initially refuses the wrong67-in-one-log claim
before dispatch; its guard now requires65+2+25 actual summaries. Initial binary
cd6a66ce45049631 is pinned independently to its build-log hash. Actual installed
39 program also refuses a separate synthetic40 database before listening,
preserving its bytes/sentinel and the production PID; temporary state is removed.

Initial frozen pack predates the two final foundation expectation changes.
Final317-file snapshot6a7fd0e92d9c differs only in that test file; its guarded
foundation/check/build job is running, reusing no unrelated test evidence.
Read-only inventory proves the original inode/schema39/instance, four configs,
reader backup units and accepted32fd program remain, source0,7backups/2,066,327bytes,
timer active and no initial builder/fixture remains. The final builder is a
separate later live job, not a production service. Real-source
consent, factual prose quality and normal40 maintenance/live acceptance remain.
No Git write, source grant, real QQ send or trial extension occurs.

## 2026-11-06 — Prove the actual-model refresh path and repair its missing clocks/read guard

An expanded actual-provider test over explicitly synthetic messages reveals wrong
send-hour/date arithmetic in extraction. Foreground replies have canonical clocks
but extraction only receives milliseconds. Reuse the existing fixed-offset
formatter in the private model-page projection: source_local_time/time_basis/
original_relative_date_available and local day bounds, preserving original data,
typed page fingerprint, source gates and schema. Receipt fallback cannot establish
the sender's relative day. A recording Actor regression fails for missing input
fields, then passes; pure clock tests cover positive/negative offsets, fallback
and invalid timestamps. Progress guidance asks for short natural replies without
reciting polling rules, retaining the existing finite wait/no automatic follow-up.

A later actual run reveals that the model can read an old latest report instead
of calling the repaired preparation tool. A separate native regression first
fails while today-only/revocation behaviour passes. Default null-ID/after0 reads
now compare the same bounded current selection and return only a refresh_required
notice when meaningful inputs changed. No manifest, run or model is launched by
the read; a separate tool call owns processing. Explicit IDs and later cursor
pages retain historical semantics and normal source dependency/publication checks.
Final Windows61briefing/25Actor, strict checks/format/build pass. Separate static
clock/read reviews find no evidenced defect and execute no tests/model/server work.

Eight actual-provider attempts are retained, not relabelled as all green. Harness
mistakes include requiring preparation for a valid unchanged read, rejecting a
correct source date, reading only page1, a forget-boundary wording mismatch and
Chinese date whitespace. These are corrected by actual result/cursor/page evidence.
One real read needs four model requests after malformed JSON repair and cannot
inspect its original in that turn; the reply honestly says it has only a summary.
A bounded explicit read-only continuation is separately recorded in the final
scenario, never a replay of writes. Final run8 needs no continuation and passes
actual original/read/save/correct/forget/reconnect with39calls/260,444reported
tokens/106.39seconds. All attempts sum179calls/1,074,049reported tokens. Source
clock inputs are verified in actual outgoing requests; review pages and actual
ConPTY images are retained and viewed. Occasional source-time paraphrases and
historical batch-limit misstatements remain known quality gaps, not solved by the
passing flow. All data is synthetic; no personal QQ binding/source is activated.

Frozen315-input Linux snapshot32fdcd2ab363 passes61briefing/25Actor/all-target
checks with the recorded Rust1.89 style-only exception and service build. Only
six reviewed compiled paths differ and no files/schema/configs are added. Updated
nineteen maintenance guards and eight actual isolated fault cases pass with cleanup;
static derivative review finds no evidenced issue. The actual same39 upgrade is
public-accepted, preserving original database inode/instance, four private configs,
reader, model limits and trial expiry. Actual Windows HTTPS/auth/SSE/profile/reader/
client checks and installed ConPTY source-browse/return/exit pass. Six-member
post-backup restores identity/hash/integrity/configs;7archives total2,066,327bytes,
timer active. No builder/fixture/test directory remains; original Codex host is
observed running without arguments/restart. Maintenance makes0model/QQsend/grant
operations; the179test calls belong to isolated Windows ledgers, not cloud history.

Source0 and the existing concrete unanswered consent remain. Full real QQ
briefing/original/selected-memory/correct/forget/reconnect and general naturalness
are unproven. No Git write/publication. Evidence: external lain-refresh-model-
2026-11-06/ (all attempts, red/green, reviews, actual-run-8 and model-attempts.json)
and lain-clock-release-2026-11-06/ (build, maintenance, public/backup/inventory).

## 2026-11-06 — Release late-arrival refresh without losing the installed reader

Frozen snapshot840252830a0f has315 inputs. Compared with the running release,
compiled changes are the previously accepted client memory presentation, four
existing refresh-related Kernel files and one new refresh-test file; Cargo/SQL/
identity inputs remain fixed. Native Linux briefing57/Actor25, all-target checks
with the established Rust1.89 nonminimal_bool style exception and service build
pass. A build-log-bound independent executable is pinned before deployment.

The previous same39 operator assumes three configs/five backup members and no
reader. Native synthetic regressions expose that mismatch; its scoped derivative
now admits only the six reviewed changed paths/one new file, fences four configs
and exact loaded reader, and verifies six backup members plus reader manifest hash.
Original full-row/stopped-state/no-replay recovery is retained. Nineteen native
guards and eight actual isolated systemd/SQLite/HTTP cases pass and clean up.
The Windows guard attempt cannot import Linux fcntl and is not native evidence;
initial wrong exception and mismatched synthetic reader-body attempts are retained.

Independent static review identifies copied collector tests that do not guard the
actual dispatch code. Tests now extract that actual entry. A transferred in-memory
mutation of its hash fence fails before attestation as intended; actual candidate
code is unchanged. Re-review and separate launch/public-acceptance static review
find no new evidenced defect; reviewers execute no server or test actions.

The live one-shot operation is public-accepted on the original39 database/inode/
instance and unchanged four private configs/backup units. Actual Windows HTTPS
verifies the new boot, profile/reader/auth/SSE, existing conversation and installed
client. Actual ConPTY restores the prior test answer, browses empty Sources,
returns and exits; the viewed image is terminal-cell rendering, not desktop/font
acceptance. A UTF-8 script-copy preparation failure starts no terminal; the corrected
copy passes. Six-member post-backup independently restores database identity/hash/
integrity and all configs. Seven backups total2,072,289bytes; timer active. Seven
completed guard directories are hash-archived externally and removed, freeing
491,990bytes; no active builder/fixture remains. One existing native Codex host
is observed running without reading arguments or restarting it.

No paid model/QQsend, source grant, trial extension, Git write or publication
occurs. Source0 and the existing pending concrete consent remain; real-source
briefing/original/selected-memory/correct/forget/reconnect is still unverified.
External evidence: lain-refresh-release-2026-11-06/ build, guard/red/mutation,
native-maintenance-evidence, independent/launch-review, public/backup/inventory,
installed-client-cloud-native and acceptance.md. Full goal remains active.

## 2026-11-06 — Fix stale ordinary briefings after late receipt and preserve exact preparation

The ordinary yesterday cache only checks previously selected record versions.
A late-imported message whose source timestamp belongs to yesterday is therefore
missed by subsequent natural queries. Independent real SQLite/Store tests first
reproduce that failure while unchanged/today-only/foreign-source, revoked-source
and Unknown guards pass. The repair reuses existing bounded selection, compares
actual versions/omissions/fallback/coverage and preserves immutable old snapshots;
it ignores pure observation/cursor churn. No polling, new source authority,
protocol field, database table or memory-policy change is added.

Static review finds a second exact-action issue: B can reuse A's manifest, then
retrying still-Executing B after a late arrival silently selects new work. A
separate test initially fails to compile because SQLite count uses unsupported
u64; after correcting the test to i64 the target assertion fails. A closed,
content-free preparation receipt in existing Action.result binds creator/reuser
to exact manifest and grant. New manifest and receipt share a transaction; normal
completion replaces it with actual run status. Additional tests prove atomic
rollback, copied-receipt refusal, empty→nonempty, known selection cap reuse and
coverage normalization. Existing startup Unknown/no-replay and source gates stay.
Re-review confirms the P2 fix and finds no new evidenced issue, executing no tests.

Native Windows58 scoped briefing tests and one later original-correction/forget
refresh case pass, plus format, strict Kernel all-target checks and local service
build. The actual ConPTY/simulated QQ process flow starts with17 notices, reuses
yesterday after a today-only import without extra processing, then refreshes to18
after a backdated late import. It follows the eighteenth original, verifies the
old manifest stays unchanged, restores history, uses independently granted QQ
audience and withdraws source-dependent bodies after correction.39 accounted
HTTP model calls are all fixtures; private temp state/processes clean completely.
The viewed image renders actual terminal cells with synthetic input, not a desktop
or personal QQ/provider acceptance. No cloud/local installation, personal source,
QQ send/model request, host change, Git write or publication occurs.

This is a local repair, not a deployed fix; the existing cloud reader/source0 and
pending concrete consent are unchanged. Next is native Linux candidate acceptance
and normal reader-aware same39 upgrade, then the authorized real-source product
flow. External evidence: lain-briefing-refresh-2026-11-06/ refresh-red/green,
reuse-binding-red/qualified, binding-green, briefing-regressions, correction-refresh,
fmt/clippy/build and native-refresh/report/screens. Full goal remains active.

## 2026-11-06 — Make reviewed memory effects understandable in the daily terminal

The existing shared memory review puts actual saved/replacement content first,
then separates selected1record change, current-conversation visibility, inherited
expiry and source-input side effects. Forget now includes original expiry and
metadata status without rereading withdrawn bodies. The explanation distinguishes
other memories becoming unavailable from deleting their records, preserves full
user quotations, and makes no invented exact impact count. No wire/domain/policy,
source grant, confirmation semantics or cloud maintenance change is introduced.

The initial restricted tool environment cannot resolve the existing development
environment/Cargo, so no test runs there. In the actual native Windows user
environment Cargo1.99.1 runs three reviewed tests, the source-draft test and the
small-window/full-reading gate test. Formatting, strict client all-target checks
and native client build pass. The actual ConPTY/simulated QQ two-audience flow
then saves a cross-batch source group, withdraws uncited inputs, reviews exact
corrections, forgets corrected records and reconnects. It accounts48 fixture model
requests and cleans private test state/processes. The driver observes each actual
review page and checks the effect boundary before the read gate permits approval;
it no longer assumes five PageDown events. Screens are viewed actual terminal-cell
renders with synthetic data, not desktop/font or personal QQ acceptance.

Independent static review finds no evidenced issue and performs no tests/actions.
The frozen native client input manifest has265 files; owner-only existing installer
checks the manifest, accepted candidate hash, original profile and actual read-only
cloud proof. It retains old binary/profile backup, changes only binary selection,
and leaves open windows alone. Candidate and installed CLI reads verify the cloud
reader revision and QQOnline; installed actual ConPTY restores the old test chat,
browses empty Sources/returns/exits and cleans its cache. No model/QQsend, grant,
server restart/data replacement, worker-service change, Git write or publication.

Real-source consent still awaits the existing concrete question; reader configured
and sources0 remain. This closes the current review-wording gap, not the full
real QQ briefing/selected-memory/reconnect goal. External evidence:
lain-memory-review-2026-11-06/ review/source-review/review-gate tests, fmt/clippy/
build logs, native-flow/report/screens, source-manifest-client.json, client-plan/
installed and candidate/installed-public-check.json, installed-client-cloud-native/.

## 2026-11-06 — Install read-only knowledge connection with complete configuration recovery

The reader JSON, two private environment entries and an explicit backup drop-in
are installed through the existing same39/no-replay maintenance flow. Only the
existing read-role key is referenced; raw values stay in server memory/private
files. Original program/database inode/instance, Actor/NapCat settings,8192/32768
limits and private-memory trial expiry are retained. A stopped five-member pre-
backup and online six-member post-backup independently restore with database
integrity/instance/hash and all selected config digests. Timer active is observed.
Seven archives total2,076,236bytes at final acceptance; no database is replaced.

Seven document/protected-file cases first fail against unimplemented helpers,
then pass. Independent static review identifies recovery adopting a newly read
foreign file as its own replacement fence. A separately authored regression
proves exactly three foreign-file failures while normal recovery passes. One
failed patch attempt leaves source unchanged and a repeated red log is retained;
after the actual fix all nine tests pass. Before each restoration mutation the
recorded before/after states are revalidated. Re-review finds no new evidenced
issue; it performs no tests or remote actions. Nine actual isolated Linux systemd/
SQLite/HTTP cases also pass, including partial selection, startup failure, changed
data, live uncertainty, drift and lost timer responses. Fixtures clean completely
and the personal process/inode are unchanged during those tests. No model/QQ runs.

Actual Windows HTTPS proves the installed reader revision/new boot, auth/SSE,
existing conversation and installed-client reads. A native ConPTY test renders
empty Sources, returns to its original test chat, exits and removes its cache;
the image is real terminal cells, not a desktop/font screenshot. QQ reports Online
but no new QQ send/receive is claimed. Independent upstream HTTPS metadata checks
the read-role credential/store without message/change/image body GETs. Source0,
no new sharing/import/sync authority and unchanged trial expiry remain. Final
operation is public-accepted; the full real QQ briefing/memory/reconnect goal is
still incomplete. The existing concrete source-consent question remains pending.

External evidence: lain-reader-config-2026-11-06/ files-red/green/recoveryred/
recoverygreen/recoveryfixed logs, native-maintenance-evidence/, private-reader-
preflight/production job/status, reader-public-acceptance.json, reader-final-
acceptance.json and reader-client-cloud-native/. An obsolete unexecuted candidate
is archived locally and removed by exact path/file/hash checks, freeing96,922bytes;
accepted maintenance code/plan/receipts remain. No commit, push, public release,
third-party message, worker-service change or source activation occurs.

## 2026-11-06 — Install verified backup helper before reader configuration

The live backup task is read-only inventoried by loaded argv, fragment/drop-in
hashes and private-file permissions; it currently selects Actor and NapCat JSON
along with environment/database. The optional reader-capable helper is installed
atomically under the maintenance lock with owner/mode preservation. Only the backup
timer is paused; laind's PID/program and all three configs/backup arguments remain.
The real existing backup service produces an exact five-member archive; independent
restoration checks39/integrity/instance/database hash and all three config hashes.
The timer is observed active again. Seven archives total2,080,462bytes. No reader
file, collection permission, Kernel restart, database restore or model call occurs.

Before installation, independent static review finds three control defects: a
late input drift can leave a success flag true, lost timer-stop response can miss
recovery, and the launch marker directory is not fsynced. Initial test runs expose
a mock-module DEVNULL omission; their logs remain and are not target regression
evidence. After the fixture repair, exactly those three qualified regressions fail.
They then pass with durable pause intent, independent timer observation, refused
acceptance on drift and marker-directory persistence. Final nine native temporary
file/SQLite cases use simulated systemctl and clean up. They include one accepted
operation replay refusal and lost resume response without replay. Re-review finds
no new evidenced issue and performs no tests/service actions. Actual installation
is separately recorded as accepted, not inferred from those simulated cases.

The connection-only reader configuration is drafted with the existing HTTPS base,
store UUID, read-role SecretRef, exact future environment digest, root:lain0640
reader file and optional backup drop-in. Raw credentials stay in server memory;
no environment/file is written by drafting. Owner-auth metadata confirms source0
and readerunset. The reader installation still needs its own changed-state-safe
configuration/backup path; the draft is not authority to collect or share messages.
Existing source consent remains pending, full real QQ briefing/memory/reconnect
is unverified, and the trial expiry is unchanged. No commit/push/publication.

External evidence: lain-reader-ready-2026-11-06/private-unit-preflight.json,
install-red/red2/red3/green/final logs and hashes, private-helper-job/launch/status,
and private-reader-configuration-draft.json. Sensitive deployment details stay
outside Git. No actual-source data or raw credential is included in this log.

## 2026-11-06 — Ship unavailable-source handling and prepare recoverable reader setup

The Linux candidate uses a frozen314-file snapshot and differs from the current
compiled inputs only in Actor request handling and its tests. Briefing46/Actor25,
Kernel all-target checks (existing1.89 nonminimal_bool-only allowance) and service
build pass. Its own unique build-log hash binds an independent executable; twelve
guard tests and six native systemd/SQLite/HTTP cases preserve the real process and
database during isolated tests, clean fixtures and retain changed data/config on
failure. The one-shot live operation then preserves the original39 inode/identity,
three config hashes,8192/32768 limits and memory-trial expiry.

Actual Windows public HTTPS verifies the new boot, authentication, SSE, existing
conversation/client reads and unchanged loaded Actor profile. A separate cloud
model question completes with1request4563tokens and0actions. Installed native
ConPTY restores that test conversation, displays the actual answer, visits empty
Sources and returns, then exits and cleans its local cache. This is a rendered
real terminal session against the cloud, not a desktop/font or QQ send test.
Post-backup restoration checks the39 instance/database hash and all three config
members. Seven archives total2,082,699 bytes, all39; the32 executable now has no
backup/process/current/previous reference and is removed by exact three-file hashes,
freeing34,191,741 bytes. Current/previous39 plus the older39 referenced by two
backups remain. There are no active builders or synthetic fixture roots.

Independent reader readiness uses the existing HTTPS status endpoint and distinct
read-role credential, checking store identity with no message/change/image GET.
It discovers a concrete deployment prerequisite: the optional reader JSON is not
in the installed backup member set. New --knowledge-config stays explicit,16KiB,
ordinary/private-file bounded, independently restored/hashed before publication
or pruning, and never includes the separate knowledge DB/media. Tests first show
three real failures in19 native cases, then green19; final20 includes the actual
CLI with all three config flags. Tests leave no temporary code/state. New helper
is not installed or part of the already frozen Actor snapshot/operator bundle.
Two independent static reviews find no evidenced issue; reviewers execute no
tests/builds/remote actions. Source0/readerunset and real QQ product acceptance
remain under the full goal. No trial extension, third-party message or Git write.

External evidence: lain-empty-release-2026-11-06/ native-linux-build-summary/log,
native-maintenance-evidence/, empty-public-acceptance.json, empty-post-backup-poll-
finalize.json, empty-source-live-result/answer.json, empty-client-cloud-native/,
private-final-inventory.json, obsolete-release-plan/cleanup.json and reader-backup-
red/green/final logs. Private endpoints/credential values stay outside Git.

## 2026-11-06 — Adequate output profile and reproduced empty-source failure

The cloud profile changes only max_output_tokens1024→8192;32768 per-turn and
the existing unlimited aggregate decision remain. Seven native Linux guards and
six actual synthetic maintenance cases pass, preserving the real process/inode
during testing and cleaning all fixture state. The one-shot live change uses a
stopped full-row39 backup, atomic owner/mode-preserving profile replacement and
actual loaded-profile/new-boot readback. Program/release, database inode/identity,
kernel.env, NapCat config and trial expiry are retained. Actual Windows HTTPS,
auth/bootstrap/SSE/conversation/native-client checks and post-backup restoration
pass; seven archives total2,226,817 bytes at that checkpoint. No source grant,
reader activation or QQ send occurs.

A separate live-model cloud test in its own conversation fails after two requests
and three memory reads. The subsequent isolated native Windows/provider trace
confirms a three-memory-call response followed by two work lookups against one
remaining slot. The original failed turn is not replayed. Request-recording tests
first fail for missing explicit negative access, dynamic allowance and exhausted
tool definitions. The local repair keeps all caps/policy gates and whole-batch
refusal, adds fixed error classification and actual current-request context.
Six targeted tests then pass; Actor25/briefing46, native build and strict Kernel
all-target checks pass. Independent static review reports no evidenced issue and
runs no tests or remote actions. One actual-provider local retest completes with
one request/no tools; its too-technical phrasing motivates a final sentence-only
naturalness refinement. Final six targeted regressions, format, strict Kernel
all-target checks and native build pass. A fresh actual-provider/native-terminal
retest completes in one request/no tools with a brief honest limitation and no
internal terminology; readback records the remaining conditional memory suggestion
and does not generalize one sample to overall quality. Final local binary/source
hashes and the static-only review are retained. No conversational deployment is
claimed. Real-source/reader/QQ product acceptance remains pending.

Private evidence outside Git: lain-profile-ready-2026-11-06/ (operation/status,
profile-public-acceptance.json, profile-post-backup-poll-finalize.json,
native-maintenance-evidence/, empty-source-live-result.json, empty-local-before/,
empty-local-after/, empty-local-final/, empty-source-final-readback.json and
targeted/native validation logs). Original failed evidence
is retained; temporary local model services/state are cleaned. No commit or push.

## 2026-11-06 — Deliver grouped-recall interpretation to the cloud

Only Actor tools/policy and two tests differ among compiled inputs; migrations,
wire/domain parsers and clients are unchanged. Native Linux35 briefing and22
Actor/archive-Actor tests, Kernel all-target checks with the existing1.89
nonminimal_bool style allowance and service build pass;314 source files verify.
Static self-check finds and rechecks a repaired provenance defect: collecting
shared output later could misattribute another build. Collection now requires
its own unique build-log SHA and checks an independent candidate file, consumed
by QA/production. Four collector regressions plus eight SQLite/source guards pass.
The reviewer ran no tests or remote actions and reports no remaining evidenced blocker.

Six actual native synthetic maintenance cases pass: success, backup failure,
pre-start failure, running observation timeout, stopped candidate after new data,
and config drift. They retain data/configuration fences and clean fixture children,
units and private state without changing the real Kernel process/inode. The unique
live maintenance then changes only the executable on the original39 database.
Real Windows public HTTPS binds its new boot and compiled policy revision, owner
auth, bootstrap/SSE and installed client read. Native terminal browse/cancel/exit
passes. QQ reports Online; no actual receive/send or model call occurs. A post-
upgrade backup restores/integrity-checks independently; verification copy is removed,
retention seven/2,816,695 bytes. An initial local post-check wrapper import failed
before any network call; correcting its helper path did not replay a remote action.

The obsolete guard bundle's11 files are hash-archived locally then removed. One
unused release is removed after process, current/previous, backup-reference and
exact three-file checks, freeing34,122,886 bytes. Current/previous39 and the32
program needed by four backups remain. Personal state/config and other services
are not removed. Evidence is external lain-recall-release-2026-11-06/:
native-linux-build-summary.json, native-maintenance-evidence/, maintenance-self-check.md,
recall-public-acceptance.json, recall-post-backup-poll-finalize.json and cleanup/
inventory records. Sources remain zero, reader unset, output1024/per-turn32768;
real-source choice and live product acceptance remain pending. No trial extension,
commit or push. See CLOUD_OPERATIONS, SOURCE_SETUP and DAILY_BRIEFING_ACCEPTANCE.

## 2026-11-06 — Preserve historical uncertainty without presenting stale batch limits

Actual saved synthetic evidence showed an original notice and a later room
correction in one reviewed memory, while the correction's old extraction-page
uncertainty still said its original was absent. Recall repeated that historical
limitation as if it constrained the group. New fixed Actor tool-result notes are
derived from actual reconciliation provenance after the existing memory gates;
they do not parse body text, alter the saved draft or confer source authority.
Policy directs source-clock interpretation and concise current arrangements.

The exact grouped read first fails for missing interpretation metadata, then
passes with list/inspect agreement, unchanged body and isolation/deletion checks.
Single-item selection retains no grouped interpretation. An initial exact-name
test command matched zero tests and is not counted; its log is retained. Final
Windows35 selected tests, formatting, strict Kernel all-target Clippy and native
service build pass. No native Linux rebuild or deployment is claimed for this
small follow-up; current cloud/installed client stay on the accepted prior source.

Actual-model iteration1 makes16 accounted calls over12 synthetic messages:
functional checks pass, dates resolve and the original is acknowledged, but it
still narrates obsolete batch history at length. Readback marks it partial. The
final focused scenario guarantees the original/correction/old-limit reproduction
using6 fixture setup requests, then makes2 actual-model recall requests. It names
the corrected room and exact discussion/deadline dates, omits obsolete page-limit
narration and retains uncertainty/nonparticipation. Reviewed text is unchanged;
temporary state is cleaned. This is one verified synthetic example, not general
conversation-quality or live QQ acceptance. The first full-flow screens remain
evidence of that partial iteration, not illustrations of the final wording.

Formal behavior: MEMORY.md and DAILY_BRIEFING.md. External evidence under November6:
recall-semantics-red-actual.log, recall-targeted-final.log, recall-quality-live-1/
and recall-quality-focused-final/. Real source choice is still pending. No real
source activation, trial extension, production change, commit or push.

## 2026-11-06 — Schema39 live deployment verified without enabling sources

The previously accepted one-shot cutover finished successfully. Current process
hash matches the native candidate; the same cloud instance/database inode now
uses39, with unchanged Actor, QQ and runtime environment files. Four native Linux
synthetic cases cover normal upgrade, stopped-backup failure, pre-migration launch
failure and a running candidate accepting new data before an observation timeout.
Seven deterministic flow tests are separate from that native evidence.

Windows-to-Linux public HTTPS verifies certificates/hostname, owner-only access,
unsupported-protocol refusal, bootstrap and content-free SSE resnapshot. Existing
conversation restoration and the installed native client's channel query pass.
QQ is Online, and the trial expiry is unchanged; no actual message is sent, so
this does not prove post-upgrade QQ receive/reply or real-source briefing. Kernel
sources remain empty and the reader is unconfigured.

The maintained backup service produces an online39 archive. Independently opening
its private restored database verifies integrity/version/instance identity; that
copy is removed, and retention remains seven archives (3,406,508 bytes at this
checkpoint). The operation record reaches public_accepted, meaning deployment
acceptance only. Full product acceptance is still pending. Initial public-verifier
failures were local harness assumptions about relative binary paths, a keyed hash
manifest and channel-status wrappers; the deployed client was unchanged and its
actual hash matches. Failed outputs and final evidence are retained externally.

See DAILY_BRIEFING_ACCEPTANCE.md and CLOUD_OPERATIONS.md. No new source activation,
model invocation, QQ send, memory-trial extension, commit or push occurs.

The subsequent native Windows client build is installed through an independent
private version directory and atomic profile replacement. Accepted source hashes
and existing binary/profile/ACL checks precede selection; the old profile/binary,
conversation and credential remain. Both pre-install candidate and actual installed
binary pass live-cloud ConPTY source-list, registration-open/cancel and clean-exit
checks. Public HTTPS/read/SSE checks also pass with the new installed executable.
The first harness attempt used a Python environment missing websockets; the
existing complete private test environment is reused, with no new dependencies.
The screen is a real VT-cell render, not a Windows font screenshot. Three obsolete
synthetic cutover bundles (32 files) are archived/hash-checked locally and removed
only after terminal-unit/process/path checks. The current operator is retained.

## 2026-11-06 — Native Linux candidate verified; live cutover pending

The final native Linux service/client build and all-target checks pass, with the
already recorded Clippy1.89 nonminimal_bool style exception; no permission
predicate is relaxed. A separate rich-text collapsible branch is flattened with
the same behavior. Linux client172/one ignored and final rich9 pass; Windows
rich9 and strict client checks pass. The accepted binary-source snapshot is
21f437fd040f; native hashes and all earlier failed logs are retained externally.

An actual consistent private cloud snapshot passes eight isolated checks:32→39,
identity/entity retention, no new authority/effect, restart, old-binary rejection
and exact-baseline old-binary recovery. No model/QQ/source configuration is loaded;
all child processes, private database copies and temporary old binaries are gone.
The first old-binary test hit /run noexec; the successful retry uses the existing
program cache while retaining the mount policy and the failed evidence.

New read-only cloud_upgrade_guard.py rejects automatic restore after any changed
counter, old object/complete row or new-table data. Eleven Windows/Linux SQLite
tests and a killed same-count-edit mutation pass; independent static review finds
no evidenced issue. It also rejects a baseline with nonempty WAL/journal. An
additional actual-snapshot experiment disproves the assumption that idle boot
preserves all state: recovery updates sharing_runtime and the sync counter. The
guard correctly refuses it; do not waive those checks or call that failed positive
assertion a passing rollback. Live failure handling must preserve changed state.

Production remains schema32, with no source activation or Git write. Source choice,
the reviewed stopped-backup/cutover/failure path, public HTTPS/client acceptance,
recall phrasing and real QQ briefing remain under the full active goal. Evidence:
linux-candidate-build-summary.json, private-migration-result-2.json,
private-migration-cleanup-2.json, upgrade-guard-* and the actual refusal diagnostic
in the November6 external daily-brief directory.

### Earlier candidate checkpoint

The existing single build cache now holds an isolated candidate and the exact
locked missing dependencies, verified against Cargo.lock package checksums. No
production service, personal database or source authorization has changed.
Native Linux release library checks pass: Kernel347/two existing ignored, local2,
wire8. Initial foundation failures exposed stale schema32 assertions and a fixture
that deleted selected new tables to pretend it was schema1. It now builds a real
migration001 database separately and retains Unknown/recovery rollback checks;
production migrations remain strict. Native Windows foundation11/profiles5 and
the two tests' strict static checks pass; Linux foundation11 passes. Independent
read-only review finds no evidenced issue in those two test-file changes.

Candidate service/client binaries compile on native Linux; the first all-target
check stopped at the existing Clippy1.89 nonminimal_bool style diagnostic. The
existing server build's one-lint exception is retained; Windows strict checks
need no allowance. Linux profiles5 now pass, with terminal tests/final checks
still running. No permission predicate was changed to satisfy that lint. An
isolated current-cloud snapshot migration/old-binary restore rehearsal is prepared,
not executed or accepted. Real-source selection remains pending; staging does not
enable intake, reading, model activity or a trial extension. External evidence:
November6 daily-brief directory, linux-candidate-validation.md and private job/
snapshot manifests. The full daily briefing goal remains active.

## 2026-11-06 — Explicit source registration, retained import and finite sync

The local /sources workflow now offers N to register an exact QQ account/group or
private peer, E to review Kernel import retention/expiry/count, and S to review
finite existing-service GET synchronization. Each confirmation applies exactly one
existing API decision; G conversation disclosure and memory review stay separate.
Sync explicitly includes retained upstream fragments, not a from-now filter.
A metadata-only reader endpoint reports configuration/credential availability and
a hash; new reviewed sync requests fence that hash before the existing transaction.
Old explicit calls remain compatible, and stopping does not require a live reader.
No new archive store, channel runtime, model activity or schema table is added.

Lost responses recover only by reading source/version/scope records. The older
APIs lack request IDs, so matching configuration is not claimed as original-request
identity or current authority. Absent records after an unknown POST remain Unknown;
a fully read warning can abandon local tracking without cancelling the request.
Independent review caught pending keyboard paging and late GET reopening after
Esc; both reproduce with failing tests and are repaired. Additional failing
regressions cover background metadata erasing unsent fields and result navigation
being lost on sync. Only local input drafts survive revalidation; old source/reader
snapshots and approval gates do not. Explicit navigation/close cancels result focus.

Final client173 passed/3existing ignored, standing-sync7 and reader-API1 pass;
strict Kernel/client/wire all-target checks and native builds pass. Native Windows
ConPTY goes from an empty registry through N/E/S, copies one actual synthetic HTTP
projection, resumes the committed cursor on the second read, stops and restarts
without replay:10 cases,2 source GETs,0 model calls. Existing G controls pass14
native cases on the same build. Private state/temporary services are cleaned.
A one-page narrow-details assumption in the old driver was replaced with bounded
page-to-visible-content handling after metadata grew; its failed evidence stays.

Formal operation/limits: docs/SOURCE_SETUP.md. External evidence under November6:
source-setup-native-final/, source-setup-existing-access-final/,
validation-source-setup/ and source-setup-acceptance.md. Initial native registration
navigation failure and all earlier evidence remain. No cloud deployment, source
authorization change, installed-client replacement or Git write. Actual-source
selection is now requested with a concrete private scope proposal; unselected
sources remain off. Cloud reader/staging preparation and actual QQ acceptance,
finer suppression and grouped-recall phrasing remain under the full active goal.


## 2026-11-06 — Reviewed source-disclosure controls in the terminal

The local /sources entry now offers G to select existing authorized QQ sources,
set the current conversation's date window/offset/record limit and review the
complete old/new scope before Ctrl-Y. F8 reviews revocation. This never enables
intake, synchronizers, automatic memory or another audience. Dates, source epochs,
finite expiry and complete reading are checked; source/CAS checks repeat before
the Kernel mutation. Clipboard delivery binds editor/field as well as connection,
conversation and draft identity.

Exact content-free grant receipts reuse existing persistence and owner-auth APIs;
history survives revocation/restart without restoring authority. A lost response
is recovered with GET only. Independent review found definite409 rejection left
the editor permanently pending; a failing real HTTP regression and classification
repair cover409, committed500, committed400 and rejected400 (400 first checks the
exact receipt because response validation can occur after commit).

Native final revalidation found an additional race: creating another conversation
before G invalidates global sync, canceling the load or routing stale g into Chat.
Two regressions now preserve only the requested metadata-opening intent, withdraw
old editor/selection/read gates, resume GET after sync and cancel that intent on
navigation. Pending writes never replay. The failed native-final artifact remains.

Final client162 passed/3existing ignored, receipt persistence1 and briefing API2,
strict Kernel/client/wire all-target checks and native builds pass. Final native
Windows ConPTY flow passes14 cases with0 model requests, no real QQ, and temporary
state cleaned. Review, finite scope, independent audience and revocation were
checked against actual Kernel state. Screenshots are synthetic ConPTY renders,
not user font or real QQ evidence. Independent read-only re-review finds no scoped
blocking issue. No cloud/installed-client upgrade, source activation or Git write.

Evidence: source-access-native-final-evidence/, validation-source-access/,
source-access-acceptance.md and source-manifest-source-access.json under the
November6 external daily-brief directory. Next: understandable source registration
and actual QQ intake configuration, finer suppression/recall quality and explicitly
authorized real-source/cloud acceptance. The full goal remains active.


## 2026-11-06 — Finite cross-batch relationships and reviewed grouped memory

Local schema039 keeps immutable original item numbers, adds bounded index/pair
activities and folds duplicates while retaining correction/conflict direction and
original lookup. Group selection uses existing reviewed memory, full-input
provenance, finite earliest expiry and source-dependent disclosure/suppression.
Oversize or unfinished groups return truthful no-write notices through normal QQ
freshness checks. No source collection, cloud/installed-client upgrade or Git write.

Checkpoint Kernel340/client152/wire8; final briefing45, refusal2 and private-delivery1
regressions plus strict checks/native builds pass. Native two-audience final flow
uses48 fixture requests. Actual DeepSeek final synthetic 12-message/three-batch
flow passes25 accounted requests,164000 reported tokens,92.23s. These prove selected
functional paths, not real QQ sources or daily conversational quality. Manual
readback still finds batch-local uncertainty repeated as a whole-group limitation
and unnecessarily technical narration; this remains a quality follow-up. Earlier
failed/intermediate evidence is preserved; the first paid run failed a retention
assertion that incorrectly matched a negated claim. Final post-forget reply has no
old body and distinguishes semantic deletion from retained audit.

Evidence: November6 external daily-brief directory, relations-native-final/,
relations-live-final/, validation-schema039/ and relations-acceptance.md.
Source-management configuration, real-source selection, finer suppression UX and
production acceptance remain under the full active goal.


## 2026-11-06 — Exact reviewed user correction on the existing memory store

Schema038 adds ConversationCorrection provenance for one exact current-user
quotation and its real MessageId, conversation, review action, predecessor
revision and inherited expiry. Both terminal entries show the complete user
request, separately labelled excerpt and actual new draft. Explicit attribution
means the user supplied the correction; it never verifies the old source report.
Fresh metadata-only management can correct a withdrawn selection without copying
its old body or reopening old authorization.

Existing candidate acceptance, new semantic record, predecessor supersession/body
scrubbing, FTS update, old input suppression and content-free replacement receipt
commit with the fixed operational acknowledgement. Generic candidate acceptance
cannot skip the predecessor/effect transaction. Expiry also prunes correction
records and candidate copies; restart/retry recovers metadata without another
mutation. No new semantic store, standing grant or private-memory trial extension.

Tests first reproduced unsupported correction, then covered exact quote/scope/
expiry, rollback, predecessor/index scrubbing, concurrent target changes,
replacement forgetting and restart. Kernel334 passed/two existing ignored,
client152/three existing ignored, wire8; strict checks/builds pass. A later focused
normal ActorModel tool-dispatch test now exercises both Correct and Forget via
actual inspect/change definitions and real message identity. Read-only independent
review found no scoped blocking issue; its noted ordinary-dispatch coverage gap
has been covered by this additional test and the native chain.

Native TUI and simulated QQ, two independent audiences: 30 accounted fixture
requests pass review-before-write, correction after source withdrawal, exact new
text, attributed predecessor, restart, forgetting and final absence. First attempt
failed because the test fixture did not recognize the new correction intent and
sent the old yesterday request; it was a driver error, not a product pass.
Actual DeepSeek functional acceptance uses four synthetic messages, 17 requests,
114,056 reported tokens and 51.19 seconds. Manual final-answer review found an
overclaim that no trace remained, although only semantic/index/candidate bodies
were deleted. Guidance now explicitly distinguishes conversation/archive/audit
retention. A follow-up failed at the pre-correction extraction output limit;
a separate 8192-output run then exposed genuine history leakage: old User correction
text remained visible after semantic forgetting. A regression reproduced it;
creation now captures a version dependency on the new memory in the same tx.
Existing gates withdraw that historical quotation after forget/supersede/expiry,
without rewriting raw User audit or losing effect metadata. Independent follow-up
review confirms no cycle or transaction gap. Final memory63 tests and strict
Kernel checks pass. Final native30 and actual-model17 requests pass; actual final
usage112,518 reported tokens,43.25 seconds. Its post-forget reply contains no old
arrangement and correctly distinguishes retained audit. This is not general
naturalness or real QQ source acceptance.

Artifacts are under the November 6 external daily-brief directory:
`correction-native-final/`, `correction-live-final/`, `validation-schema038/` and
`correction-acceptance.md`. All earlier failed/intermediate evidence is retained. Cloud identity/database/QQ login,
installed client and original Codex host remain unchanged. No commit/push/release.
Cross-batch reconciliation, fine-grained suppression, real-source configuration
and production acceptance remain open under the full active goal.

## 2026-11-06 — Reviewed forgetting without restoring withdrawn sources

Schema037 implements the Forget part of ADR039 using existing Actor review and
semantic stores. Metadata inspection binds one real conversation-owned selection
and revision without disclosing its body. A change proposal requires an executed
same-turn inspection plus exact text and MessageId from the current user. The
model receives bounded current-message identities, not forged private incoming
IDs. The tool currently offers only Forget; correction remains planned.

Semantic/index deletion, candidate scrubbing, inherited source-input suppression,
content-free receipt and the fixed operational completion commit together.
Exact retry recovers committed metadata. Earlier invalidated read lineage is
retained, so ordinary restore cannot republish deleted evidence; a separate
typed memory-change receipt survives source withdrawal and restart. Both terminal
entries validate and display the operation, exact target/version and user intent
before independent approval. No autonomous write grant or trial extension.

Red-to-green tests cover missing inspection, forged quotation, normal Actor
dispatch/review, source-stale metadata management, transaction failure rollback,
prior-body withdrawal and restart. Independent read-only review finds no scoped
blocking issue. Native whole-page acceptance discovered metadata-only rendering
had cleared User messages required by shared quote validation; a failing whole
Model snapshot test reproduced it, and rendering now retains only User records
for validation while emitting no duplicate user text. Follow-up review confirms
no body/visibility widening. Two later harness failures were early F4 input and
an exact-title mismatch; their evidence remains separate from successful v4.

Native Windows TUI/simulated QQ passes save/recall, source withdrawal, reviewed
forget of both independent audiences, actual 404 after deletion and restart
without resurrecting memory, using 24 accounted fixture calls. Actual DeepSeek
on four synthetic messages passes the corresponding native path, including
post-restart post-forget readback: 13 accounted calls, 82,905 reported tokens,
40.02 seconds. The model confirms the real Forget receipt and absent record,
without repeating the old body. It still overexplains IDs and lookup details;
this is not general naturalness approval or real QQ source collection.

Final validation: Kernel329 passed/two existing ignored; client151 passed/three
existing ignored; wire8 passed; strict all-target checks and native builds pass.
Evidence: `forget-acceptance.md`, `forget-native-v4/`, `forget-live/` and
`validation-schema037/` under the documented November 6 external directory.
All successful fixtures clean their private temporary state/services.

Natural correction and cross-batch reconciliation remain open. Suppression still
uses complete extraction inputs, so other selections sharing those inputs may
become unavailable; only the chosen record is deleted. Fine-grained UX/semantics,
real source choices and personal deployment acceptance remain required by the
active full goal. Cloud identity/database/QQ login, installed client and original
Codex service are unchanged. No Git commit, push or public release.

## 2026-11-05 — Reviewed briefing selection and truthful save recovery

Local schema036 connects actual numbered briefing items to existing reviewed
memory candidates, records and FTS. A real Conversation scope avoids fabricated
channel identities and accidental Global sharing. The Kernel derives a fixed
cautious episodic draft, retains every extraction input version (including
uncited inputs) and keeps UnverifiedSelection authority after actual review.
Ordinary Actor chat can read the report, propose a selection, wait for review
and later retrieve the saved record in that same conversation. Both desk and
retained chat display the same checked draft, visibility and finite UTC expiry.
CLI administration gains `memory list --conversation`; `/stop` can end pending
Actor review. No standing source-memory grant is added.

Independent scoped review found three gaps: suppression was not part of pending
review currency, new candidate deadlines did not wake retention, and ordinary
restore lost committed saves after source withdrawal or interrupted ack. Each
was reproduced and repaired. Candidate creation wakes existing maintenance;
approve/decline/cancel of stale pending selections closes without writing.
Typed reviewed-memory receipts derive from accepted candidates and actual
review times independently of action-result publication. They retain only IDs,
time and real conversation, never withdrawn bodies or permission to replay.
Follow-up review found retained chat could confirm a selection without showing
its draft; shared validation/rendering fixes that too. Scoped re-review finds
no remaining finding in these fixes.

Native terminal acceptance then found a distinct real history reconstruction
bug: a later withheld read cleared an earlier restored save card. A failing
regression reproduced it; checked history now rebuilds only metadata cards
after body withdrawal. Final native Windows TUI/simulated QQ uses four records
per audience, two independently granted conversations and 18 accounted HTTP
requests. Exact F4 review, approval-before-save, recall, restart, audience
isolation, uncited-input correction and metadata-only recovery pass. An initial
test-driver error assumed nested rather than existing flattened MemoryRecord
JSON; its failed evidence is retained separately.

Actual DeepSeek over four synthetic messages also passes natural briefing,
first-item selection, native review, one conversation-local save, actual semantic
memory readback, restart and source withdrawal: nine accounted calls, 54,226
reported tokens, 30.88 seconds. Manual inspection confirms the selected room
correction and uncertainty/nonparticipation language. Recall is still verbose
and exposes internal IDs/import labels; this is not general naturalness approval.
No live QQ source is collected. Screenshots are ConPTY renders, not native GUI
font acceptance. External evidence is in `memory-acceptance.md`,
`memory-native-v3/`, `memory-live/` and `validation-schema036/` under the existing
daily-brief visualizations directory documented in DAILY_BRIEFING.md.

Final native Kernel library: 325 passed/two existing ignored. Final client:
150 passed/three existing ignored; the earlier same-stage CLI integration also
passes, wire8 passes. Formatting, strict all-target checks and native builds
pass. A full-suite legacy scope assertion was extended to check the exact new
conversation ID and deny foreign IDs. A restart test now waits for the owning
Actor task to release after Completed, rather than racing its final snapshot
publication; its original lock failure was separately reproduced.

Natural reviewed correction/forget and cross-batch semantic reconciliation are
still required, alongside real source selection and production acceptance.
The goal remains active. Cloud identity/database/QQ login, installed daily client
and original Codex host remain unchanged. No commit, push or public release.

## 2026-11-05 — Finite daily coordination and natural original lookup

Local schema035 adds durable run progress and per-foreground-turn manifest
disclosure dependencies. One explicit run reuses existing tool-free Actor
batches, with at most 64 batches/20 minutes and earlier manifest expiry. It
subscribes before starting/reading terminal events, remains independent of HTTP
callers, and never relaunches Unknown coordination on reopen. Normal yesterday/
read/inspect tools require the actual conversation's independent source grant;
source changes gate later inference, publication, QQ prewrite and restored text.
Generic/source-influenced memory writes remain unavailable pending a properly
typed selection path.

Independent review found a stopped run could still admit prepared input at the
actual inference transition. A failing regression reproduced it; the final gate
keeps that ledger entry Prepared. Late results cannot extend a stopped report.
A second fix keeps the closed source-change notice usable without clearing its
invalid disclosure lineage or authorizing another call. Scoped re-review finds
no remaining finding. Reports now expose stable item numbers, readable QQ source
identities and a complete-response 12 KiB bound; whole-item trimming recalculates
the cursor so no item is skipped. A dense regression first reproduces the gap.

Actual model trials exposed the cloud-derived 1024-output budget, repeated
not-ready queries and unnecessary per-item original lookups exhausting tool
limits. Isolated tests use 4096 output within the unchanged 32768 turn limit;
production configuration is unchanged. Readable attribution and truthful
progress guidance reduce these calls. A successful actual run then exposed a
wrong model timezone conversion on manual readback. Kernel now supplies checked
fixed-offset source clocks and explicit half-open day bounds, marking receipt
fallback separately. It reuses the existing time crate, adds no host timezone or
general natural-language calendar engine, and retains all failure evidence.

Native Windows clock-base validation: Kernel316 passed/two existing ignored; client146
plus one integration passed/three existing ignored, wire7 passed; formatting,
strict all-target Clippy and native binaries pass. Nineteen focused briefing
tests include restart, revoke, stop, late completion, source withdrawal, original
lookup, audience denial, byte paging and clock boundaries. Native TUI/simulated
OneBot uses two independently authorized conversations, 17 records each, ten
batch requests and 22 total accounted requests. Reconnect, original follow-up,
continuation and visible source withdrawal pass. Screenshots are inspected.
Final history-continuity and initial-status date changes then pass four context
and nineteen briefing checks plus fresh client/wire, strict checks and builds.
Earlier ended source turns retain only a fixed ephemeral state marker, not their
answer, delivery claims or authority. A native regression first reproduces the
missing marker. Initial scope/status now also carry explicit dates; a failing
regression reproduces that earlier omission after actual progress misdated the day.

Final synthetic-source actual DeepSeek terminal run passes natural yesterday
query, completed report, second-item original and API/UI source withdrawal. Seven
requests (six foreground/one extraction), 35663 reported tokens, 24.49 seconds;
the answer correctly anchors the sample's relative dates. This proves one
selected example, not general semantic completeness or real QQ collection.
The final terminal waits for source words to disappear and a recovered connection
before its withdrawal screenshot. One earlier clock-base screenshot raced the UI
and is not accepted evidence. Metadata-only progress is allowed to remain; an
overstrict harness assertion was corrected without weakening source withdrawal. Earlier
attempts remain failed/Unknown without replay. All temporary test state/services
are cleaned. External local evidence is indexed by
`lain-daily-brief-2026-11-05/conversation-acceptance.md` under the documented
visualizations root in DAILY_BRIEFING.md.

Full goal remains active: selected-memory save/recall/correct/forget, cross-batch
reconciliation/date ambiguity, explicit live sources and production acceptance
are unfinished. Cloud identity/database/QQ login, the installed client and
original Codex host are unchanged; no commit, push or public release occurred.

## 2026-11-05 — Cited model items through the existing Actor

Added local schema034 page-to-Actor bindings and independent finite extraction
activities. Each model page carries up to four complete records/16KiB and one
accounted request; source revisions, retention, grants and knowledge freshness
gate input, admission, completion and ordinary Actor restores. Generated items
retain stable IDs, exact original substrings, Kernel-owned versions/timestamps/
text origin, model summary origin, uncertainty and explicit omissions. The
internal processing origin cannot carry tools, private context, memory or Worker
actions. Repeating a page returns the original turn, including Unknown; it does
not resume provider work. There is no whole-day runner or natural query tool yet.

Independent read-only review found rejected provider JSON could survive the
ordinary two-phase Assistant-body/completion path. A regression first fails on
the actual Actor snapshot leak. Briefing now commits body, citation validation,
result and Completed together, and finishes held preview only after success.
The regression passes and scoped re-review confirms the finding resolved. A test
reopen race was also corrected by waiting for the actual owning Actor task to
release its Kernel reference before opening another process-level DB owner.

Four focused Actor tests pass, including revoke-during-call, fabricated quote/
Worker tool rejection and shutdown/restart Unknown with retained reservation.
The complete Kernel library now reports 308 passed/two existing ignored, wire
seven passed, client 146 plus one integration passed/three ignored. Strict
Kernel/client all-target Clippy, formatting and native build pass. Seventeen
historical schema cases pass after extending their explicit downgrade fixtures.

A native HTTP Responses fixture processes all 17 synthetic notices in five
batches, adds a rejected sixth batch, verifies restart without replay/stable item
IDs, exact citations, common inference accounting, rejected-body isolation and
all-record invalidation. Temporary state is cleaned; this is simulated model
output over real HTTP, not paid-provider evidence. Separately, one actual DeepSeek
batch accepts two items from four synthetic messages in 5.95 seconds with 2645
reported tokens. Human inspection confirms the 201-to-301 room correction and
resource deadline/eligibility, retains relative-time uncertainty and ignores
chatter/injection. An earlier actual request ends Unknown before a returned
usage record; cause remains unproven. A metadata-only connection check succeeds;
the successful independent experiment does not erase or replay that first call.

External artifacts: `lain-daily-brief-2026-11-05/model-http-native`,
`model-live-native` (Unknown), `model-live-diagnostic` (accepted). Both actual tests
use isolated native storage and private in-memory credentials, no live QQ, no
production mutation. Cloud schema32, original services/data, existing grants and
installed daily client stay unchanged. Full goal remains active for day
coordination, ordinary interaction, original follow-up and selected memory.

## 2026-11-05 — Daily evidence: source gaps, scoped consent and complete pages

The independent QQ service now includes at most 20 source-scoped historical gap
reports in its changes feed; Kernel persists them and the terminal distinguishes
report receipt time, expired snapshots and unknown live presence. Six bridge
checks, one wire bound/order/time check, client rendering and all 16 independent
service/observer checks pass. No live source is enabled. An administrative group
candidate probe fails to establish a usable metadata WebSocket; no group bodies
are retained and temporary SSH tunnels are closed. This does not block local work.

ADR037 and additive schema033 introduce an independent finite conversation grant,
not an extension of existing private recall/memory trials. Day preparation stores
IDs/versions/coverage metadata only; complete-text pages recheck all selected
versions and current grants/freshness. Exact retries preserve the original day
and sources across restart. Seven targeted core/API cases cover full-day selection,
source/conversation isolation, complete UTF-8 text, bounded pages, date boundaries,
receipt-time fallback, correction/forgetting/revocation and exact recovery.

The schema change exposes stale historical-version fixture construction: newly
added tables remained while fixtures manually downgraded user_version. The
fixtures now remove schema033 tables before pretending to be older databases;
their content-preservation/default-off assertions remain. A dedicated schema32
upgrade checks original archive snapshot bytes and invalidation triggers. The
17 migration regressions pass. The earlier complete Kernel run reports 290 passed,
14 fixture failures and two ignored; the 14 failures are then repaired and covered
by the targeted 17-case migration rerun, not misreported as a new full-suite run.

Client 146 unit checks (three existing ignored), one CLI integration check, strict
Kernel/client all-target Clippy, formatting and native binary build pass. Actual
isolated Windows CLI/API startup reads 43 synthetic records over six pages,
reopens the Kernel, recovers the exact manifest and rejects stale/cross-conversation
reads. The eight-case native source TUI fixture passes again on schema033. Both
fixtures make zero model calls, use no live QQ and clean temporary state.
Scoped independent read-only review finds no evidence-backed defect in the day
manifest/grant/API/CLI changes. Evidence stays outside Git under
`lain-daily-brief-2026-11-05/day-evidence-native` and `sources-terminal-schema33`.

This is evidence preparation, not generated briefing or a completed daily demo.
Next work remains model processing with the shared inference ledger, persisted
items/citations, fresh original follow-up and memory integration. Cloud schema32,
personal data, installed binaries, source authorizations and QQ login are unchanged.

## 2026-11-05 — Daily briefing foundation: context and truthful source status

Started the user's full authorized daily-briefing goal. Revalidated existing
archives, independent QQ knowledge, private recall and source-derived memory:
ordinary three-record recall cannot substantiate a whole day, and ordinary TUI
turns do not inherit private-channel grants. Read-only cloud inventory finds no
registered archive source and one reply binding; no source or grant is enabled.

Two new history regressions fail on full old tool payload replay and history
size. The projection now retains exact current input, recent user referents and
operation state/IDs without old task/output bodies, with labelled assistant
excerpts and a 12 KiB history soft cap under the existing 48 KiB hard cap. Existing
source/memory guards and full persisted audit remain. Twenty-two selected Actor
and 28 archive regressions pass at this checkpoint; scoped independent read-only
review finds no evidence-backed correctness defect.

Added owner-only source status GETs, CLI `archive status` and TUI `/sources`.
Status separates import authority, current-epoch actual receipt time, retention,
last reader result and finite synchronization; live collection presence remains
unknown without evidence. Duplicate imports do not update receipt time and old
grant epochs do not appear as new receiving activity. Three core/API checks and
two UI unit checks pass. Eight native Windows ConPTY cases pass for empty/selected
sources, actual receipt/duplicate, no body leakage, narrow paging, revocation and
zero-model navigation. Strict Kernel/client Clippy and native build pass before
the final detail-paging adjustment; final checks remain separately recorded.

Implementation is local only, with no cloud/installed-client upgrade. The next
work is actual collection-gap propagation, explicit briefing audience/grants,
bounded day processing/citations and selected-memory integration. No real QQ
source or full briefing acceptance is claimed. External evidence is under the
session artifact directory `lain-daily-brief-2026-11-05`.

## 2026-11-05 — A real web task, continued through Lain

The user requests a complete small-webpage test. An isolated writable project,
separate cloud conversation and finite dedicated native host exercise actual
Codex/provider execution. Three turns produce and refine a working static task
page. Work creates the first task, F6 adds filtering/clearing, then Lain Chat
finds the latest parent and proposes a final fix, explicitly reviewed in TUI.
All three complete in one native session. Parallel Chat status lookup, closing
and reopening the frontend during execution, and complete three-round history
are verified against actual cloud receipts and ConPTY views.

Chrome independently verifies adding/completing/counting/blank rejection,
localStorage persistence, filtering without deletion, clearing only completed
tasks, keyboard Enter, accessible input and no horizontal overflow at actual
390/1440 px viewports. A favicon 404 is corrected by Codex through the real Chat
proposal path; the final fresh browser context has only three successful local
resource requests and no console errors/warnings. The tester independently runs
the final script's real Node syntax check, exit 0. The first turn's PATH-related
check failure is retained, not presented as a completely error-free run.

The temporary host stops normally, its registration is revoked and dedicated
credential removed. The original host/policy and cloud release remain unchanged.
The generated webpage, local loopback preview and evidence are retained outside
Git. This is actual selected Windows workflow evidence, not a claim of universal
reliability, automatic setup or Kernel artifact verification. See
[acceptance details](docs/WORK_WEB_ACCEPTANCE.md).

## 2026-11-05 — Devices becomes a computer and project view

Replaced the raw device-ID table/task-list default with named computer rows and
an explicit detail page: OS, authority expiry, historical version/report time,
project selection and full registered directory. Runtime presence and local
policy remain unknown without evidence. C only rereads cloud inventory; I shows
startup/recovery help, F7 exposes non-secret diagnostics and T retains the
device-wide task inventory behind a labelled secondary route.

N enters the existing Work form with a freshly checked exact device/workspace.
Refresh preserves selection by ID; removed destinations retain drafts without
selecting another computer/project. Task review, revision checks and uncertain
request recovery remain enforced. Added bounded detail wheel scrolling and
narrow-screen hints, retaining the selected portrait and Chat/Work text behavior.

Native Windows validation: 143 client unit tests plus one CLI integration pass
(three existing explicit native/manual cases ignored), strict client all-target
Clippy, format and build. The synthetic actual-ConPTY smoke passes eight grouped
checks including narrow layout/wheel, exact task destination and preserved Chat
draft, with no task/model/Worker calls. Artifacts remain outside Git under the
session directory `lain-devices-ux-2026-11-05/terminal-final`. This increment changes
no Kernel/host protocol or cloud database; Linux/macOS UI remain unverified.

The existing protected Windows daily launcher now selects the new client, with
the previous binary retained. Its actual cloud-connected read-only terminal check
reaches computer details, the registered project, startup help and preselected
Work form, then exits without submission. No personal transcript/screenshot or
clipboard content is saved; neither the host nor cloud service is restarted.

## 2026-11-05 — Explicit global unlimited quota enabled

The user explicitly removes the finite aggregate model-request allowance. The
existing management API supports finite limits and one-private-channel daily use,
so this change adds a real `unlimited:true` global mode rather than a fabricated
large finite ceiling. The authenticated revision-checked API/CLI selects exactly
one mode. Unlimited preserves per-turn/activity limits, permissions, source grants,
Stop, atomic reservations and the historical finite ledger; it enables no new
activity or model credential. Missing legacy fields default false. ADR036 records
the policy extension and old-binary rollback's conservative stopped behavior.

The regression fails before implementation on the unknown unlimited field. Native
Windows passes 13 related Kernel tests and 138 client tests plus one CLI
integration case, strict Kernel/client all-target Clippy and format. Native Linux
passes the same 13 related tests, ten Actor integration cases, the final eleven
inference contract cases and library Clippy with the existing documented
nonminimal_bool allowance. Checks cover exhaustion removal, persistence, old
totals, mode exclusion, pending requests and stop-before-entry. Independent
read-only review finds no scoped blocker. No real model call is needed for this
configuration change.

Cloud release `/srv/lain/releases/unlimited-b9d6092df4af` is selected, binary
SHA-256 `985b13262edc151f7b29d24cbfc710429ec123dadc8f97e3e630b27e4804a612`.
After idle checks, stopped backup, atomic selection and public HTTPS/auth/receipt/
event checks, the owner API writes decision revision 2. Readback confirms enabled
global unlimited, null limit/remaining requests and no aggregate expiry. The old
finite budget still exists with its spent totals; model/environment, identity,
database inode, QQ binding and grants are retained. A verified subsequent backup
contains the enabled decision. Two releases and seven backups remain.

The local CLI includes `inference unlimited --revision N`; the selected client
SHA-256 is `448a6f0a46d8306a4cc2b985537b14bf37267c95e7fb4ae9bceb114fe2459150`.
The old running client remains valid; no local host restart or task dispatch is
performed. Local metadata-only evidence and operation records:
`<local-evidence>/lain-unlimited-inference-2026-11-05/`.
Aggregate cost authorization does not remove the separate long-context/per-turn
budget concern recorded below or establish upstream provider billing capacity.

## 2026-11-05 — Cloud Actor multi-call compatibility and failure causes

The user reports intermittent failures while listing sessions and requesting a
follow-up through Chat. Read-only evidence shows both selected failures stop in
the first model request with no recorded action, while a nearby query succeeds;
the original provider cause was discarded by the generic error handler. A
separate deterministic incompatibility is verified: DeepSeek documents that
parallel_tool_calls=false is ignored, but the adapter rejects complete responses
with multiple calls and Actor consumes only the first. Both regression cases fail
before the change. This does not retroactively prove both historical failures had
the same cause.

The adapter now accepts up to four distinct call identities. Actor preflights the
whole batch against remaining tool slots, executes serially through existing typed
guards and pairs every original call with its result in the next Responses input.
The first review-required action stops with exactly one current proposal; later
calls do not execute. Private source/input fences, correction/forget refresh,
request/token/time ceilings and no-replay rules remain. Allowlisted provider
errors now distinguish timeout/connection, HTTP rejection, interrupted or
incomplete output, output-length truncation and protocol failure without provider
bodies, credentials or raw URLs.

Native Windows passes 33 Actor-related library tests, ten Actor integration tests,
the final three-case batch suite and strict Kernel all-target Clippy/format.
Native Linux independently passes the same 33+10 cases, the final three-case
contract run and library Clippy (retaining the documented unchanged-code
nonminimal_bool compatibility allowance). Tests include two/four-call reads,
read-before-confirmation, stopping after a proposal, no partial oversized batch,
remaining-slot enforcement, duplicate IDs, request/result pairing and secret-free
failure notices. Independent read-only review reports no scoped blocker.

The schema32 cloud release is now `/srv/lain/releases/chat-ae6ae3610a1f`, binary
SHA-256 `2c6500145d7985d28ebe63c5f0d237865f9372de4f00f8866d82902e8b2145b9`.
Publication checks idle chat/device work, takes a stopped verified backup and uses
the existing atomic selection/rollback path. Database inode/instance, model and
environment configuration, QQ binding/grant and budget decision are unchanged.
Public HTTPS/auth, missing-request receipt and SSE checks pass. A single new
read-only query in the actual conversation completes with work.list and
device.work.list, both executed, in two model requests. Its report-parser failure
is recovered by GET only; no second POST or task control is issued. The trial
allowance changes from ten to eight requests for that validation, with no refill.
Two releases and seven backups remain; QQ is online.

Known next concern: this long-conversation validation uses 31,733 of the existing
32,768 cumulative turn-token ceiling. Budget-aware context slimming remains
pending; neither the ceiling nor the current low-reasoning/1024-output setting is
silently changed. One successful query is not general reliability acceptance or
verification of a real follow-up's result. Local metadata-only evidence:
`<local-evidence>/lain-chat-reliability-2026-11-05/`.

## 2026-11-05 — Stable scroll and drag-selection coordinates

The user reproduces stuck scrolling and large viewport jumps during selection in
real long Chat history. Two deterministic red cases identify unconditional redraw
reanchoring as the shared cause: an empty row is treated as source offset zero,
and every code caption can match the message heading. A requested row 2 becomes
row 1; starting selection at row 203 jumps to row 1. Prior short-text selection
checks do not cover these long blank-separated messages.

The reader now honors explicit scroll movement and relocates a source anchor only
when the actual layout changes without new user scrolling. Empty rows retain real
parsed offsets and code captions retain distinct block identities. Mouse hit tests
use the last presented scroll position; dragging inside the first/last visible row
does not auto-scroll. Direction compares source anchors, and changed source
versions invalidate even a zero-width pending selection. Chat and Work retain
separate View state and the existing clipboard/source generation checks.

Native Windows verification passes 138 client unit tests and one CLI integration
test, with three existing explicit manual checks ignored, strict all-target
Clippy, format and build. Four focused regressions cover exact scroll positions,
blank-top selection, background layout changes and presented-frame/boundary hits.
The extended native ConPTY/private-window-station test traverses a long synthetic
history, confirms exact plain/Markdown/code/drag clipboard contents, and preserves
the user's interactive clipboard; only a simulated provider is used. Independent
read-only review finds no blocking issue within this scope.

The side-installed client SHA-256 is
`5f878cb7a1681d586c8c036952f4c4d5ad3a339c510c8a73d026aeffc3d8382f`;
the prior version and all running services remain. The actual daily launcher then
passes twelve distinct upward-scroll frames and a drag with unchanged visible
text in the existing cloud conversation. This read-only check submits no message
or task, accesses no clipboard, and saves no personal transcript or screenshot.
Local evidence is under
`<local-evidence>/lain-scroll-selection-2026-11-05/`.
Linux/macOS acceptance remains unverified; this does not claim arbitrary terminal
emulators or all pointer timings have been manually accepted.

## 2026-11-05 — Readable Work context and retained turn history

The user reports that UUID-only Work rows hide task purpose, time and directory,
and that follow-ups appear to replace prior results. The previous reader renders
only the selected command's output_tail even though each continuation command
retains its own request and receipt. Work now shows original-request summaries,
local creation/latest-receipt timestamps, device labels and registered directories.
Selection includes the latest instruction; the sidebar uses the declared path.
The default detail pairs every loaded instruction with its own Markdown result.
Missing ancestors use the existing conversation-scoped GET; mismatched families
are refused and incomplete reads are explicit. No schema/server/Worker change or
new task/model invocation is needed for the product behavior.

The original two-turn regression fails before the change and passes after it.
Native Windows checks pass 134 client unit tests and one CLI integration test,
with three existing explicit manual checks ignored, strict all-target client
Clippy, format and build. Additional tests cover stale/missing recent ancestors,
scope mismatch, authentication failure and bounded source copying. Independent
review identifies a fixed-eight-round scroll overflow; adaptive display-budget
pagination and a legal 8192-byte multi-line request regression resolve it. At most
64 rounds are loaded and each page holds at most eight complete pairs. F5 and
Shift-F5 change the reading window, not the task targeted by control operations.

Native ConPTY checks cover both results after same-session continuation and client
restart, task context, Markdown/copy menu, explicit diagnostics, draft retention
and direct new-task exact text. Fixtures use no paid model or real QQ and leave
the user's clipboard untouched. The side-installed client SHA-256 is
`95c32b10c3d8890d65c5890f8ab0b8c2977ab96dd978f3de7ccc3aebf7a45ec7`;
the previous client remains available. Actual installed-launcher HTTPS navigation
then confirms the existing selected family has two retained output-bearing rounds
and the timeline shows both; no task IDs change during the observation. No private
transcript or screenshot is saved, and no host/cloud restart occurs. Local evidence:
`<local-evidence>/lain-work-history-2026-11-05/`.
Each output remains the existing bounded device receipt, not a full native log or
independently verified artifact. Linux/macOS terminal acceptance is not claimed.

## 2026-11-05 — Compact activity cards and Work result reader

After the user confirms real read-only Codex results, the selected UI change
replaces repeated long System dumps with stable compact cards. Device tasks update
in place; their result action opens the same Work task. F7 offers keyboard activity
selection in Chat and result/diagnostic switching in Work. Tools and retained
Kernel Worker receipts default to folded details. Work reuses Markdown rendering
and source-based plain/Markdown/code copying with its own document and selection;
bounded device output remains labelled as an excerpt when incomplete.

The cold-start replay regression is red before implementation. Native Windows
validation passes 128 client unit cases and one CLI integration case, with three
existing checks ignored, strict client all-target Clippy, format and build. The
native ConPTY script covers quiet historical snapshots, live compact cards,
keyboard entry, Markdown, copy-menu availability, explicit diagnostics, preserved
Chat draft and quiet reopen. It uses structured fake Workers and zero model/paid
calls, never the interactive clipboard; synthetic temporary state is cleaned.

Independent review identifies full synchronization retiring the first card, a
single-stream reconnect baseline gap, temporary-message anchors disappearing and
hidden notices entering /seen. These are corrected with bounded metadata-only
identity/version/position checkpoints, stable message anchors, separate stream
baselines and filtering before the displayed-notice set. Actual HTTP/SSE tests
cover Changed → empty/retired view → checked replacement → restored card before
later conversation. Retirement still clears body/menus/selection/copy epochs;
neither plaintext nor signatures survive in the presentation checkpoint. Hidden
receipts are not acknowledged. No task replay, approval bypass or extra inference.

The Windows daily launcher now selects the checked side-installed client; actual
launcher status and read-only ConPTY navigation against the existing HTTPS Kernel
pass without task or chat submission. Existing client binary/configuration are
retained; host/cloud service, credentials, database and portrait are unchanged.
The user only needs to reopen the Lain interface. External evidence stays in the
session's `lain-activity-cards-2026-11-05` directory. Linux/macOS behavior was not
newly exercised. No commit or push.

## 2026-11-05 — Codex command output burst disconnect

The user started the prepared device host and submitted the selected read-only
task. It reached a real Codex turn, opening message and command start, then became
Unknown with `worker protocol stream closed`, without a final report. Read-only
inspection of this command's local journal/Worker history and its exact native
Codex thread/process shows interrupted work and a command outputDelta burst;
no other personal transcript, login credential or raw protocol was retained.

A 12,000-notification subprocess regression fails against the previous transport.
The fix drains only well-formed request-ID-free command progress notifications
before the 512-entry lifecycle queue. Final aggregated output, assistant text,
permissions and lifecycle events remain on their original path and retain their
existing bounds/redaction. Fixed transport-close reasons replace opaque receive
errors without exposing original frames/stderr. The native seven-test Worker
suite passes, including exact final output and later same-session approval;
genuine overflow, bad framing, cancellation and recovery retain their protection.
The original Unknown command is not resent, edited or marked complete.

The classifier unit test and all 17 native Windows device-service scenarios pass,
including final-summary reporting after the output burst, with zero paid calls
and normal synthetic state cleanup. The intentional network-disconnect fixture
also emits its existing WinError10053 server traceback; it does not fail a check.
Strict Kernel all-target Clippy and workspace build pass. Independent source
review finds no code or installer blockers; the stale event mapping is corrected
in the same change. Native Linux/macOS are not newly verified by this Windows fix.

The selected PC has a side-installed checked executable behind a protected
manifest and stable `Start-Codex-Host.cmd`. Its prerequisite check passes. The
original running binary is retained; no policy, credential, cloud service or
execution database is replaced. The user is asked to stop the old foreground host
normally and use the new entry. Installation is not a running-version or real-task
success claim. External `lain-codex-output-fix-2026-11-05` artifacts retain the red/
green logs, selected tests and installation evidence. No commit or push.

## 2026-11-05 — Direct Work desk and native Codex preparation

The user selected Work-first development and explicitly deferred Claude. Work now
has a direct N/new-task flow: declared computer/workspace selection, an independent
unchanged task draft, finite deadline and sandbox, full exact review and explicit
submission. It uses existing device commands and starts no conversational model.
Chat remains usable with its own draft; task details retain their originating tab.

A read-only conversation/device/request receipt route reuses persisted links,
without schema changes. Target preflight and route support checks precede POST;
uncertain responses retain the original request and recover by GET only. Native
loopback regressions cover response loss, changed targets, older servers and scope
isolation. This is not permission to replay an uncertain Worker mutation.

Installed Codex 0.166.0-alpha.5.1 passes selected protocol checks and both existing
isolated native Rust adapter probes, including same-thread resume without login
or inference. The exact allowlist adds this alpha; adjacent unknown versions stay
rejected. Device initialization can explicitly declare `--codex-work`, independently
of pairing, host policy/start and actual provider access.

Native ConPTY testing exposed LF loss in ordinary Windows key-event paste. F9 now
explicitly reads local clipboard text into an unchanged editable draft, with view/
target binding and whole-input bounds. A test-only private window station verifies
exact multiline text, one queued task, preserved Chat draft and task persistence
after exit, without touching the interactive clipboard. The latest desk suite
passes 91 tests (two existing checks ignored); the native host fixture passes 16
scenarios. All use synthetic state: no paid Worker task or cloud rollout is implied.
Independent review found and resolved paste routing and tab-origin issues. Formal
scope and remaining real-device acceptance are recorded in docs/WORK_DESK.md.

The follow-up selected installation updates the cloud with a seven-file schema32
maintenance snapshot and the native daily client. Native Linux passes the five
receipt-store tests, six device API tests and three Codex adapter tests; Clippy
uses the previously documented nonminimal_bool compatibility exception. An
initial overly narrow adapter filter selected zero tests; the actual current
test binary subsequently runs the three adapter tests, with its hash/source
snapshot recorded separately before publication. Windows final checks pass 122
client unit tests, one CLI integration test, strict client/Kernel all-target
Clippy, format and workspace build. The native host fixture remains 16 passing.

Publication uses a verified stopped backup, exact source manifest and actual
release selection, retaining the original database/identity/configuration/QQ/
memory/budget. Native Linux synthetic fault injection confirms recovery after a
post-replace fsync failure and refusal of a foreign release. Review catches
outer deadlines shorter than allowed rollback/finalization paths; both are
increased to 600 seconds before use. Public HTTPS verifies owner receipt/null,
device/unauthenticated denial, readiness and snapshot SSE. Final retention leaves
two releases and seven backups. No model/task requests are submitted by these
operator checks.

The actual Windows launcher now selects the new client; real ConPTY navigation
sees the paired PC/project and opens Work's task editor, then exits normally
without submission or changed task IDs. No production screenshot/transcript or
interactive clipboard is retained. Native pairing prepares a seven-day device
registration and read-only host policy; prerequisite checks pass, but the host
is not started and real provider access remains unverified. A PowerShell null
conversion initially prevents the client-profile replacement; original/stage/
backup hashes confirm no selection change. A synthetic replacement verifies
NullString, then exact content/ACL/hash checks complete that same staged switch,
without replaying pairing or deployment. Operational evidence is outside Git in
the session's `lain-work-entry-2026-11-05` artifact directory. No commit or push.

## 2026-11-05 — Cloud picture restoration and temporary image retention

The user selected continued development without a multi-day testing prerequisite.
The independently authorized seven-day private-memory scope/expiry remains intact.
Cloud QQ image reception was enabled on the existing image-capable Actor, with
the same database inode, model/configuration, reply binding, memory grant and
finite inference decision. No operator model submissions or input replay occurred.

The renewed real-image check failed before decoding. Read-only metadata diagnosis
confirmed an allowed official HTTPS QQ image URL and 16 public A plus nine public
AAAA records; the downloader rejected that legitimate answer count above 16.
A native Windows regression reproduces UnsafeLocation before the correction.
The narrow fix bounds answers at 64, still rejecting every private/over-limit
answer set before pinning; exact CDN, body, proxy/redirect/retry and timeout
restrictions remain. Both native Windows and Linux pass 50 channel tests.
Windows strict Kernel-library Clippy passes; Linux's scoped check requires an
explicit existing-code nonminimal_bool compatibility exception. Removing the
public-IP guard from a separate extracted-code copy fails the new regression.
The Linux maintenance binary is deployed on the original schema32 database with
unchanged identity/inode/configuration/budget/grants. Actual public HTTPS auth,
readiness and SSE pass. Two new real user images produce one and three pixel
samples, appear in actual viewed_media dependencies and receive Completed model
turns/Confirmed sends. The user confirms understanding and replies. No failed
image input was replayed. See `docs/VISUAL_CHAT_ACCEPTANCE.md`; broader visual
quality is not implied by this selected-sample confirmation.

The maintained `scripts/napcat_cache.py` has eight native non-root Linux safety
tests, initially red before implementation, then green. Removing its hardlink
guard in a separate copy makes the protection test fail. Its installed root
oneshot passes under filesystem/network restrictions and an hourly timer is
active. Only aged top-level temporary pictures qualify, with minimum-age,
scan/deletion limits, soft byte target and quarantined identity revalidation.
The first real inventory was empty; no disk-reclamation claim is made. Native
QQ account/media storage and login backups remain separate concerns.

Independent review found a stale idle snapshot after backup and two partial
publication/durability gaps. Rechecking activity, inspecting actual release
selection during rollback and syncing the release parent fix these. A native
Linux synthetic post-replace fsync fault verifies old-binary recovery and refusal
of a foreign target. No database overwrite/recovery replay is introduced.
Nine Windows visual process checks also pass using fake providers; their final
schema assertion now reflects schema32. The private `lain-cloud-visual-2026-11-05`
artifact directory retains execution metadata outside Git. No commit or push.
Final verified backup/retention leaves two releases and seven archives totaling
655153 bytes at the checkpoint. The known older compatible release and two
failed-build input scripts were removed with exact path/hash/process checks;
the immediate previous binary and final build recipe remain. Cleanup holds the
same upgrade lock as publication, and no temporary diagnostic directories remain.

## 2026-11-05 — Cloud QQ cutover and bounded private memory trial

Authenticated server management confirms the intended Lain account is logged in.
The previous local binding had no queued/in-flight deliveries, proactive scope or
Worker sessions; its reply grant was revoked and its Kernel stopped gracefully,
retaining all 27 previously confirmed exchanges and its database. The same cloud
Kernel/database/model/budget now owns the QQ adapter. Replies start disabled,
enable only after the exact account is Online, and persist across an additional
idle Kernel restart with the same binding/conversation/grant. Initial post-start
process-image checking had not yet observed the expected executable; subsequent
read-only inspection confirmed that image was Online with replies still disabled.
The reviewed recovery continued that verified process without another restart
or effect replay. The existing finite inference decision was never reset.

The user confirms real private receipt. Durable records show ordinary completion,
QQ confirmed sends and real remember/correct operations after a separately
approved seven-day, current-binding, new-input-only memory grant. The old preference
is Superseded and the corrected preference Active with inferred ChannelSelection
provenance; private contents remain outside Git. The user used a real preference,
so it was preserved rather than deleted as a synthetic fixture. Live forget and
cross-conversation sharing are unverified/off. One early real turn ended Unknown
without returned token usage; it was not replayed and later turns completed. Its
specific cause remains unproven. Topic carryover also shows naturalness still
needs daily-use evaluation. Cloud image input/group collection/proactive contact
are not enabled by this narrow cutover.

Backup now optionally includes the exact 8 KiB-bounded NapCat adapter JSON and
restore/hash verification, alongside the existing environment/model/database.
New archive/link/size regressions first fail; 16 synthetic native Linux non-root
tests then pass, and removing the restore hash guard in a separate copy fails the
new corruption case. The exact tested helper was installed. Actual pre-cutover,
staged, enabled and final backups pass; the final five-entry archive is restored/
checked, with seven archives totaling 316331 bytes and no restore scratch paths.
QQ login/media state remains a separate backup/retention concern. Independent
review caught boot-time reply revocation and missing parent-directory fsync in
operation fences; both were fixed before cutover. Operator syntax/targeted
whitespace checks pass. No Rust binary rebuild, new public listener, personal DB
import, automatic model-test submission, commit or push. Evidence is the private
local `lain-cloud-qq-2026-11-05` artifact directory.

## 2026-11-05 — Server-hosted QQ transport prepared for login

The user explicitly selected server-side QQ login for the next daily-use slice.
A separate NapCat container is deployed beside the existing cloud Kernel using
an inspected immutable official image digest. Its QQ processes run as a dedicated
non-root account, host ports bind only to literal loopback, and private QQ/config/
plugin state is separated from the Kernel database. Memory/swap is limited to
1536 MiB, CPU to 1.5 cores, processes to 512 and core dumps disabled. The maintained
optional deployment template/runbook is in `deploy/napcat`; connection/account/
credential values and operation evidence remain outside Git.

Native Linux runtime checks verify the pinned image, mounts, numeric QQ UID,
resource/log configuration, authenticated management status and all original
12 containers plus the unchanged Kernel process/binary. A real startup check
exposed that NapCat prints auth-bearing management/QR URLs before per-account
log settings take effect even with console/file logging disabled. Before any
account login, only the new container was stopped/removed, its logging data
removed, management/OneBot credentials rotated and the same private state retained.
The rebuilt container uses Docker's `none` log driver. Startup process validation
also exposed Docker's mandatory PID column and Linux username truncation; the
operator now validates the numeric UID. No weaker runtime permission was needed.
Independent static deployment review and a follow-up cover this boundary; final
template validation occurs before a destructive service-replacement step.

The authenticated service is now in `waiting_qrcode`, not logged in or connected
to the Actor. This is deployment preparation, not acceptance of QQ chat, memory
continuity/correction or multi-day companionship. Local QQ replies/history are
preserved; no database migration, group collection, proactive contact, Worker,
paid model request, Kernel restart, commit or push occurred. Actual login requires
the user's phone confirmation; reply cutover and real-use checks follow it.
QQ media/session disk retention and backup are separate from existing bounded
Kernel backups and are not claimed complete by container resource limits.

## 2026-11-05 — Source-backed terminal text and clean local copying

The portrait/sidebar remains intact while Chat now renders saved Assistant
Markdown without a surrounding transcript border. Headings, emphasis, lists,
quotes, inline code and syntax-highlighted fenced code have separate source and
display representations. Prose is bounded to 96 columns and prefers whole words;
code uses the available width. User input, operational receipts and transient
previews remain plain text. Complex tables, math, Mermaid and dedicated HTML
rendering are outside this increment; links are styled but never opened.

F2 or `/copy` offers plain reply, canonical source Markdown and individual literal
code blocks. Ctrl-O copies the latest saved reply. App-owned mouse selection plus
Ctrl-C copies body text without roles, sidebar or visual soft wraps; original code
tabs, line breaks and pipe characters survive. F3 or `/read` opens a wider read-only
view and restores the draft/scroll state on return. Copy targets retain message
identity/version rather than following a changing menu index. Source retirement
removes text, anchors and menu targets, and invalidates pending clipboard setup;
an already-started system clipboard write is not revocable. Clipboard access is
local, asynchronous and explicit, with success shown only after native delivery.
No OSC 52/SSH clipboard forwarding or archive pagination is claimed.

Independent read-only review found empty-start preview forwarding, reference-link
display amplification, outside-origin drag continuation and zero-width cell
mapping defects. Failing regressions preceded their fixes. Additional regressions
cover reverse selection, frozen menu targets, changed-anchor highlighting,
word-aware wraps and oversized source replacement. Text/cache bounds are explicit;
an oversized authoritative message replaces stale copyable text with an honest
noncopyable limit notice rather than disappearing or leaving old text available.

Validation on native Windows: 109 client tests and one CLI integration pass,
formatting and strict affected-target Clippy pass. Three explicit/manual tests
remain ignored by the default suite; the private-window-station clipboard test
was run explicitly, and the isolated desk entry was exercised through ConPTY.
The latter verifies actual native clipboard round trips for plain/Markdown/two
code blocks and drag selection, plus reader draft retention. The 13 existing
terminal cases also pass. Final acceptance uses seven local fake model requests,
zero paid calls, and no access to the interactive desktop clipboard. Tests clean
their temporary state and services before reporting success; a fixture shutdown
omission and a mouse-coordinate frame race were fixed in the acceptance script.
Evidence is external to Git under the local `lain-rich-chat-2026-11-05` artifact
directory. Images reconstruct actual ConPTY output with synthetic content.
Native Linux/macOS clipboard/UI and Rust 1.89 execution remain unverified for
this increment; declared new dependency minimums do not exceed 1.89.
The versioned daily cloud client entry now points to the tested executable
`6bb54f1f7e4b…`; its hash and owner-only ACL passed the launcher's read-only check.
Prior client binaries/windows remain intact. No server restart, model invocation,
credential change, commit or push was performed for this installation.

## 2026-11-05 — Chat wheel scrolling and local terminal sidebar

Real terminal feedback exposed that the alternate-screen client ignored mouse
wheel events. A native ConPTY reproduction failed on the previous executable;
the repaired client captures/releases mouse input and scrolls only the Chat body,
preserving drafts and pausing follow mode while reading. Ctrl-End restores latest
output. Mouse restoration precedes raw-mode restoration on Windows. Existing
recent-history and transcript byte bounds are unchanged; full archive pagination
is not part of this increment.

The portrait remains verbatim. Its wide-screen sidebar now shows OS/architecture,
logical CPU availability, physical memory, local cwd, selected task workspace and
real operational task states. Local metrics stay local; Worker state comes from
the existing checked event stream and linked device tasks. Kernel/global tasks
are labelled, disconnection is not a running claim, and no task or model is
started by observing. Independent review caught default-height task clipping and
approval states labelled as responding; both are fixed and covered, including a
two-line cwd/workspace fixture that fails under the original layout.

Native Windows validation: 92 client tests pass (one existing manual check
ignored), one CLI integration, formatting, strict affected-target Clippy and one
test-decoder regression. Thirteen native ConPTY cases pass with six local fake
model requests and zero paid calls. The pyte wide-cell stub issue was separately
reproduced and handled only in the test display helper. Windows telemetry and
actual wheel input are verified; native Linux/macOS this increment are unverified.
The separate versioned daily client entry was updated without closing prior
windows, changing credentials or restarting the cloud service. No commit/push.

## 2026-11-05 — Multi-step chat budgets and truthful memory retrieval

Real terminal feedback exposed two distinct failures: an exact multi-word search
missed a saved preference, and the following model request crossed the per-turn
token ceiling. The latter rolled back its turn usage accounting while the durable
provider ledger already recorded both returned requests. The saved memory itself
was intact. New regressions reproduce 5085 + 5261 tokens: the lower cap now records
10346, preserves the completed read receipt and fails without publishing the
over-budget reply; a 32768 cap completes the same two-step fixture.

Memory tool schemas and receipts now explain literal substring matching and
filtered-page coverage. The Actor policy distinguishes a historically committed
ordinary proposal from current visibility and an empty search from a failed save.
No search scope, memory authority or approval requirement was widened. Known
token exhaustion has a specific error; generic interruptions remain uncertain.
The terminal labels runtime failures as System output, translates the exact legacy
generic error on display and no longer advertises an unavailable desk `/history`
command. Existing stored history is unchanged.

Ctrl-J is the primary newline key in Chat and Worker continuation input. Modified
Enter is retained when the terminal forwards it; Windows Terminal's default
Alt-Enter fullscreen binding is respected. Native Windows tests pass: Kernel
library 282 (two manual probes ignored), client 86 (one ignored), CLI integration
one, strict affected-target Clippy and formatting. Ten native ConPTY checks pass,
including multiline editing while idle/running, retained drafts, restart and
approval handling; five local simulated model calls, zero paid calls.

The user explicitly approved a cloud per-turn cap of 32768. That configuration
change is backed up and active on the existing identity/database, preserving the
original total request budget and expiration. A separate versioned native client
update preserves the old running window. Native Linux selected validation passes
308 tests (282 library, ten Actor, seven reliability, nine memory; two manual
probes ignored), followed by a native release build. The exact completed build
receipt binds the source snapshot, unit, tests and executable hash. A durable
idle-guarded cutover and actual HTTPS bootstrap/memory/budget checks verify the
same schema-32 database, identity and saved active memory. No paid model call was
made by diagnosis, tests, configuration change or deployment. A post-upgrade
backup includes the Actor configuration; retention leaves two releases and the
existing bounded backup set. No commit or push was performed.
Markdown remains plain text; real QQ collection
and model interpretation/naturalness are not claimed by deterministic checks.

## 2026-11-05 — Explicit cloud chat activation and separate native entry

Enabled the existing selected DeepSeek Flash configuration on the same cloud
Kernel after explicit user authorization to reuse its protected credential.
An independent Windows cloud entry keeps current-user-only connection files,
the verified native client and the selected portrait, while preserving the local
instance/database and global environment. An initial persistent ceiling allows
100 model requests/seven days without refill; no QQ, background activity or
device execution was enabled.

One short real HTTPS conversation completed with one provider request and
3423 reported total tokens. Checked bootstrap restores its reply in the native
terminal. Opening the actual cloud window leaves that count at one. Its 1.234s
observed completion is only this short connection check, not a direct-Codex
latency benchmark or broad real-provider acceptance. Original cloud identity,
database inode and twelve existing containers are preserved; no personal data
was imported and no local Kernel was started.

Extended the backup helper with an explicit bounded Actor JSON input and checked
restore hash. Twelve native Linux regressions pass; an isolated size-guard removal
fails its regression. The Lain-only backup override includes the selected model
file, and actual post-activation/first-chat archives verify database and Actor
configuration while retaining seven archives within the one-GiB cap.

Independent review found a configuration-recovery scope gap, fixed before use.
Actual guarded activation then exposed the systemd pre-exec image race; the
operator now waits within a finite readiness deadline while requiring the exact
Kernel executable hash. Two configuration-failure cases and the startup-image
case pass synthetic native Linux checks. Failed attempts restored configuration,
kept provider calls at zero and reused the same budget without refill. Windows
provisioning also handles the Administrators creation owner only for this new
exclusive installation; final files remain protected current-user-only.

Private receipts, source hashes, failures/reviews and the activation handoff stay
outside Git. No commit/push. Real Worker tasks, QQ and multi-day experience remain
unverified and separately scoped.

## 2026-11-05 — Cloud continuity maintenance, retention and complete evidence handoff

Deployed the exact selected-scope tool-schema executable accepted by native Linux
328 tests and seven strengthened real-HTTPS memory cases. The same-schema update
took and independently restored a stopped backup, retained the cloud identity,
database inode and private configuration, and verified the actual running process
hash. Native Windows verified public certificate/authentication/bootstrap/SSE and
the new boot generation. The latest twelve HTTPS Worker cases pass again on this
executable with the matched native Windows binaries and exact no-replay counts.

A further online backup passed isolated restore. Retention keeps the current and
previous schema32 releases and seven archives within the one-GiB cap. The first
retention attempt refused an incorrect three-file assumption before deletion;
the actual two-file legacy release layout and unknown-extra-file refusal now
have regressions and independent recheck. The obsolete schema28 binary is removed;
its historical isolated rollback proof is not a currently retained rollback target.

All twelve temporary fixture instances are cleaned. Thirty-six exact completed
operator inputs were archived outside Git, revalidated and individually removed;
the production PID remained fixed. Final inspection records about4.28MiB runtime
memory,0.161 cumulativeCPU seconds, two tasks, two releases, seven archives totaling
130583bytes and one2.73GiB retained build cache. Original twelve containers remain
running. These empty-instance measurements are not long-term workload benchmarks.

The full original eight-part goal is mapped to direct evidence and explicit
limits in the private final audit and maintained [acceptance](docs/CLOUD_ACCEPTANCE.md).
Architecture, operation/removal/recovery and minimum real-use configuration are
documented. The cloud Actor stays unconfigured; no personal endpoint/DB migration,
paid inference, realQQ effects, global cleanup, commit or push occurred. Real
provider latency, QQ companionship, native macOS, offsite disaster recovery and
multi-day reliability/naturalness remain separate unverified work.

## 2026-11-05 — Candidate HTTPS scoped-memory and live withdrawal acceptance

Seven strengthened actual Linux–Windows HTTPS cases pass on the tested candidate
with 38 bounded local fake-model calls. Real native CLI equal-path registrations
remain isolated; explicit mappings share project knowledge while device/channel
bodies remain restricted. Every offered memory tool is checked for exact scope
branches and strict closed objects; successful reads bind exact source ID/revision/
scope and independently captured model input. Model-generated selected memory
retains inferred provenance and explicit candidate review.

Correction and deletion of a separately active source invalidate derived replies
in model history, startup snapshots, the complete scrolled terminal, private
metadata cache checks and reopened client. A real short-lived device bounds the
snapshot lease and causes idle SSE expiry; a fresh active source proves revocation
withholding. Independent review found three weak assertions and one nested schema
gap; all were strengthened and final raw-input evidence was rechecked. Six guard
tests and five model contract tests pass. Temporary cloud route/unit/candidate/
data and Windows state are cleaned; production identity/PID/binary remain fixed.
The candidate is accepted, not yet deployed. No paid/QQ effects, personal-data
migration, commit or push. Maintenance release and final full-stage audit remain.

## 2026-11-05 — Context-aware model tool schema follow-up

The stage-wide audit found that stable device/project memory scopes were enforced
by Kernel retrieval but missing from the strict schemas actually offered to the
model. A regression captures real Actor model requests and fails on the omitted
device branch. Tool definitions now use the exact Kernel-selected ID enums for
search/list/inspect/propose, exclude daemon-local paths in a selected remote
context, and retain the legacy unselected Workspace path contract without leaking
another conversation's IDs. Kernel source/authority/review checks remain separate.

Native Windows Kernel280 and Actor/reliability17 tests pass (two existing
manual/live ignored), with strict targeted Clippy and independent scoped review.
The same 228-input source passes 328 selected native Linux tests (two existing
manual/live ignored). This follow-up is not yet deployed. The full
stage still needs the remaining memory/sync audit and final maintenance inventory.

## 2026-11-05 — Actual HTTPS native Worker fault/recovery chain

Ran the real Linux Kernel in a temporary synthetic instance behind the existing
verified HTTPS proxy, with native Windows terminal/device execution and the
structured local Worker fixture. Twelve checks pass: pairing/credential isolation,
local scope rejection, terminal dispatch and exact denied permission/application,
allowed permission and same-session continuation, parallel chat/client exit,
Kernel restart/cancellation, host crash recovery, native/HTTP missing receipts,
unconfirmed permission, partition and revocation. Ten work sessions create ten
empty seed threads plus ten durable forks and twelve accepted turns; recovery
does not add submissions. Unknown outcomes remain unknown. No paid/QQ calls.

The first native run exposed raw Win32 ACL opens failing on long execution paths.
A failing Unicode-path test reproduces error3. Lexical absolute/extended-path
handling fixes it without resolving reparse points or weakening ACL checks; six
native local tests, 84 terminal tests and strict local/client Clippy pass. An
independent scoped security review reports no blocking findings. Two test-model
history mistakes were separately reproduced and fixed; three model contract
tests pass. Actual provider-thread counting now includes the adapter's existing
empty seed instead of incorrectly calling it duplicate work.

Successful Windows temporary state and the isolated Linux unit/route/data are
cleaned; the production Kernel retains its PID, identity, schema32 and binary.
Failed-run reports and exact binary hashes remain outside Git. Final memory/sync
coverage audit, operations inventory and architecture delivery remain required;
this is not daily-use, real-provider, QQ or multi-day acceptance. No commit/push.

## 2026-11-05 — Backed-up cloud upgrade and stopped-WAL backup repair

Published the native schema32 release while retaining the previous binary and
matching schema28 backup. The first attempt stopped before migration because the
old backup helper could not open a closed WAL-mode source in its read-only mount;
the original service was restored. A low-privilege reproduction confirmed the
cause. The maintained helper now takes the actual Kernel ownership lock for a
quiescent DB/WAL copy into private scratch, retaining live SQLite backup when the
Kernel owns that lock. No source-directory write permission was added.

Nine native Linux backup regressions pass, including committed crash WAL and
aggregate archive-byte retention. Removing that byte guard in an independent
copy fails the intended assertion. Independent review also fixed release traversal
permissions and an unsafe six-count rollback assumption; eight isolated rollback
guard tests pass. Both review reports retain findings and fix rechecks.

The resumed upgrade used a verified stopped backup, preserved the original
database inode/configuration and left existing applications running. Native
Windows verified real HTTPS certificate/authentication, protocol negotiation and
SSE; after a real service restart, identity persisted and boot generation changed.
An actual schema32 online backup also passes. Current and previous releases remain;
private resource/evidence inventories are outside Git. Model/QQ remain unconfigured,
and no personal database, paid inference, Git commit or push was involved. Actual
HTTPS two-machine Worker work and the final stage audit are still required.

## 2026-11-05 — Terminal live synchronization and private metadata cache

Connected the terminal to bootstrap/invalidation with a conservative monotonic
lease, connection generations and bounded read-only retry. Invalidation retires
rendered bodies and permission previews and cancels old readers. In-flight effects
retain exact receipts independently, without replay or stale-body restoration.
HTTP authentication/protocol failures stop retries until explicit Ctrl-R, including
ordinary reads, receipt recovery and notices. A 16 KiB private atomic cache stores
only endpoint/identity/resource hints and a checked-on-resume conversation ID.

Failing regressions exposed ordinary-read auth retirement, stale submit receipt
loss, diagnostic refresh and effect-recovery auth gaps. Independent static review
also found the notice error path; fixes have been rechecked. Native Windows passes
89 selected tests (one existing manual ignored), strict workspace/all-target
Clippy and format. Eleven HTTP/SSE lifecycle cases and native negative ACL cases
exercise these changes. The same 228-input source passes all 429 native Linux
workspace tests (three existing manual ignored); Linux Clippy was not rerun.
Ten Windows ConPTY interaction cases preserve the selected portrait, independent
Worker inspection, restart and repaired configuration. Two isolated-copy guard
removals fail their intended regressions; their copies/targets are cleaned.

On the actual Linux server, an isolated copy of the empty schema28 cloud database
opens with the new binary as schema32. The old binary explicitly refuses it;
restoring the verified baseline database lets the old binary become ready again.
All rehearsal processes/directories are cleaned, and production PID, inode and
schema remain unchanged. This proves a database restore rehearsal, not a live
upgrade/rollback operation. Real HTTPS Worker work, service upgrade, final
operations inventory and the full stage audit remain open. No paid/QQ calls,
personal migration, commit or push. See [operations](docs/CLOUD_OPERATIONS.md).

## 2026-11-05 — Versioned startup and invalidation protocol

Added schema032 with persistent operational instance identity, per-open boot
generation and a transactional invalidation counter. Source/grant deadlines have
an indexed set; expired entries cannot hide another future deadline in the same
snapshot. Existing Kernel lock ownership signals committed changes without a
polling service or new model call. New authenticated bounded bootstrap and SSE
invalidation routes close the snapshot/subscription gap and require resnapshot
for expired, foreign, future or old cursors.

Terminal startup negotiates before conversation restoration; manual Chat refresh
uses the same checked snapshot. Rendering is rebuilt while drafts/uncertain IDs
remain. Red-first regressions exposed same-revision withdrawn text surviving and
late history overwriting a newer live completion. Both are fixed; the latter was
found in independent scoped review. Windows selected regression and strict lint
pass (397 tests, three existing manual ignored); the same 225-input source passes
327 native Linux tests (two existing manual ignored). Two independent-copy
mutations removing clock updates and live/history generation fencing fail their
expected regressions. The automatic runtime subscriber,
local expiry and private metadata cache remain unfinished. No service/schema
upgrade, paid provider/QQ call, personal migration, commit or push occurred.

## 2026-11-04 — Private lineage and publication boundaries

Added schema031 as a serialization gate without rewriting legacy rows. Private
evidence keeps cumulative direct and inherited source leases, original input
fingerprints and monotonic influence. Generic proposals cannot promote private
source material. Complete input rebuilds retire excluded message/action IDs from
publication and future model history while preserving raw audit records.

Meaningful failing regressions reproduced same-turn A-to-B recall losing A,
withdrawal between body save and final completion, stale restore/duplicate-submit
and queued SSE responses, inherited QQ source changes before send, and discarded
audit speech returning after a rebuild. Corresponding focused tests now pass.
Normal HTTP/SSE responses use checked projections; QQ delivery checks full
lineage. Fixed operational notices and exact committed-effect metadata stay
separate from provider prose. Native Windows passes 389 selected tests (three
existing manual ignored), Linux passes 322 (two existing manual ignored) for the
same 218-input uncommitted source snapshot. Windows strict workspace/all-target
Clippy and formatting pass. Four isolated-copy guard-removal mutations fail the
expected regressions; source/copy hashes were checked and temporary mutation
outputs removed. Independent static review found the races above and verified
their fixes; it did not run the tests or review client synchronization.
Startup/live client invalidation, actual HTTPS device fixtures and
database rollback remain pending. No provider/QQ calls, personal migration,
runtime upgrade, commit or push accompanies this increment.

## 2026-11-04 — Cloud continuity: contextual semantic memory

Added schema030 without rewriting legacy memory JSON. Project, Device and
DeviceWorkspace scope use stable IDs and explicit reviewed registry context,
not Linux canonicalization of Windows paths. Owner-selected conversation context
has CAS and exact historical receipts; generic Actor tools cannot borrow transport
administrator power. New candidate/record origin distinguishes owner API,
selected device/workspace context, actual channel and summary provenance while
keeping legacy origin unknown.

Memory reads persist bounded source versions before model disclosure. Compatible
current history carries its restrictions forward; corrected/deleted or differently
scoped generated history is excluded, and derived records check their lineage.
Untracked legacy generated memory stays owner-inspectable but is not model
evidence. Inference admission and publication recheck sources, and influenced
previews are held. A real Actor fixture exposed denial/cancellation being treated
as fresh generated speech after revocation; operational Tool receipts keep those
cleanup paths available while approval remains blocked.

The final maintained source passes Windows Kernel259/wire6 plus foundation11,
memory9 and profiles5 (290 distinct; two existing manual ignored), and actual
Linux Kernel259/wire6 (265; two manual ignored). Windows strict workspace/all-target
Clippy and format pass. Two external source mutations fail at the intended
privacy assertions. Their first cache reuse exposed a Cargo artifact collision
between identical package clones; only three goal-owned local package artifacts
were cleaned, then maintained-source tests were rebuilt and passed. The temporary
source copy was removed; evidence and the separate-target requirement remain
outside Git. No Linux Clippy pass is claimed for this increment.

Full stage acceptance remains open: explicitly verify channel-to-generic
promotion/inherited-evidence paths, connect startup/live synchronization and
private disposable caches, run actual HTTPS Linux–Windows Worker fixtures, then
prove upgrade/database rollback and final resource retention. The running cloud
instance remains the original empty schema28 deployment; no personal database,
QQ scope, model budget, commit or push changed. [Contract](docs/CLOUD_CONTINUITY.md).

## 2026-11-04 — Cloud continuity: project registry foundation

Started the full cloud/memory/device stage while retaining the existing dirty
W2–W4 tree. Added schema029 and owner-only project/assignment APIs with stable
Lain IDs, explicit device/workspace membership, exact reviewed revisions and
atomic historical receipts. Same names/paths/workspace UUIDs never infer a shared
project; unbinding retains its revision. Existing semantic-memory JSON is preserved
across the schema28 fixture migration. There is no new Actor administration tool
or Worker authority.

Seven focused native Windows tests pass after expected failing tests on actual
Linux (missing domain) and Windows HTTP (missing route). A fresh Windows build
also exposed incomplete cached GCC SDK files; restored only missing files from
the SHA-256-verified official matching GCC/MSVCRT release, preserving existing
files and removing the trial header-selection flag. Broad validation now covers
251 Windows Kernel library tests plus foundation11/memory9/profiles5 (276 distinct,
two existing manual ignored), native Linux library251 (two manual ignored),
strict Windows workspace/all-target Clippy and format. Removing the admitted
workspace guard in an external copy makes its test fail while production source
stays unchanged. Older version fixtures were updated to remove the new project
tables when constructing actual old schemas; no migration checks were weakened.
Source and evidence remain separate; actual server
profiles/logs are outside Git. No existing instance upgrade, paid model, QQ change,
personal data migration, commit or push. Full stage remains active; memory context,
bootstrap/live sync, real cross-machine fixtures and rollback are still required.

## 2026-11-04 — Native visual acceptance and usable chat-configuration feedback

The user provided a real Windows Terminal screenshot and explicitly accepted the
selected A portrait/sidebar appearance. Retained its original bytes and source
record outside Git; visible Chinese/borders/ASCII cursor confirm wide native
rendering, while the shown disconnection establishes no live-chat result. Exact
font family/complex IME/all DPI variants are not claimed. Updated W4's acceptance
matrix and current roadmap, retaining fixture versus human evidence distinctions.

Continued development on the screenshot's confusing unavailable-chat path. The
terminal reads existing allowlisted diagnostics at startup/explicit Chat refresh
with a three-second bound. Known unconfigured model/missing credential/invalid
configuration preserves input with a concrete reason and no submit; Work/Devices
remain usable. Unknown diagnostics preserve existing server authority. Repeated
Enter during uncertain delivery keeps the original identity/text and precise
refresh/retry guidance. Configuration recovery is read-only and never sends a
draft automatically. No new server schema, model call or service startup.

Added four relevant client tests; all67 unit +1 CLI passed (one existing manual
ignored), strict client Clippy and native build passed. An ordinary final link
was blocked by in-use preview files; built the same source offline in independent
target-w4f, preserving the user's two running previews. Same-binary terminal10
and work11 passed with cleanup and5/3 fake model calls, zero paid/real QQ. New
process cases prove absent-model/no submit and explicit refresh/then-send. Earlier
fixture failures exposed a wrong compact-label expectation and a synthetic marker
matching existing credential redaction; fixed fixture inputs/expectations only.
Redaction and product guards were retained.

Full368 earlier workspace +11 added client cases means379 distinct coverage,
not a full379 invocation. Kernel/device-host/fixture binaries are unchanged from
host16; no extra inference/QQ was used for configuration checks. Source/evidence
and latest hashes are in [W4 acceptance](docs/W4_ACCEPTANCE.md). External artifacts:
session `lain-w4f-2026-11-04/{native-bin,terminal-accepted,work-final}` and user
screenshot/metadata; logs `%TEMP%/lain-q1-tools/w4f-*`. The manual Python preview
now selects that explicit binary set and passed its headless check. Local W4 is
accepted; broader live environments remain separate. No commit/push/deployment,
personal database migration or system execution-policy change.

## 2026-11-03 — Make manual terminal review independent of PowerShell script policy

The user attempted the prepared GUI review but Windows refused its generated
`manual-client.ps1` before Lain started. Replaced that disposable launcher with
`manual-client.py`, run explicitly by the user in their existing terminal. Its
child environment drops Lain/provider overrides case-insensitively, selects only
the protected synthetic connection and passes program arguments without a shell.
No system execution-policy change, installed service or automatic GUI action.

The host's `--check-only` now executes the actual generated Python entry in
read-only mode rather than checking an independent command. Native rehearsal
passed isolated saved-connection access with zero model/QQ and complete cleanup.
Updated the local manual instructions and maintained runbook. External evidence:
`%TEMP%/lain-q1-tools/w4e-manual-python-launcher-check.log`. The desktop/font
acceptance is still pending; the execution-policy failure did not run Lain and
therefore supplies no GUI evidence. Core Rust code and its prior validation are
unchanged.

## 2026-11-03 — W4 acceptance follow-up: current cancellation status and cross-machine handoff

Reopened the local audit while inspecting actual terminal effect images. Task
details correctly reported Cancelled, but the bottom message could still say
native termination was unconfirmed. A second red regression showed a late cloud
acknowledgement could hide an already known completion. Added one bounded typed
frontend notice over the original task identity/revision; fresh remote/detail
snapshots update terminal status, and late receipts use the newest known matching
facts. This adds no Kernel record, request, model wake or backend mutation.

Both regressions passed, as did all63 client unit tests and one CLI integration
test (one existing manual test ignored), strict client Clippy/build and format.
The new same-binary native11 work and8 terminal runs passed with cleanup and zero
paid/real QQ calls. The work test now checks the actual device-cancellation status
text before saving its image. Full368 prior workspace plus seven added client
tests gives375 distinct coverage, not a claimed full375 invocation. Server/host/
fixture binaries are unchanged from the native16 host acceptance.

Expanded [the runbook](docs/W4_RUNBOOK.md#conditions-for-the-next-real-cross-machine-acceptance)
with concrete two-machine/native-build, authenticated HTTPS/streaming, isolated
state/rollback, selected execution scope, independent credentials, Actor versus
Worker spending authority and evidence/cleanup prerequisites. Seven explicit
cross-machine scenarios are prepared, not executed or newly authorized. Updated
[acceptance](docs/W4_ACCEPTANCE.md) with the new client hash, images and logs.
External evidence: session artifact `lain-w4e-2026-11-03/{native-status-final,terminal-status-final}`;
logs `%TEMP%/lain-q1-tools/w4e-cancel-status-*.log`. Actual Windows Terminal font
confirmation remains pending. No commit/push, personal migration or deployment.

## 2026-11-03 — W4d/e: reported permission application and explicit task cancellation

Added backward-compatible cumulative `approval_reports` with exact original owner
decision and Received/Applied observations. The cloud requires a real immutable
decision for that command, rejects lost/changed/regressing evidence, and grants no
new permission. Native observation/final-result paths derive receipt from the
protected response fence and application only from matching Supervisor resolution.
Pending/responding/expired/unknown stays unconfirmed, including after a completed
turn. Scope is rechecked before writing the response fence, without extra pre-RPC
cloud reporting or replay. Records/HTTP reads remain explicitly bounded at128/256
KiB for the finite256 reports and duplicated current/last-event evidence.

The terminal displays three exact stages. F8 now opens a full task cancellation
review; Ctrl-K requests a stop only after full reading and fresh exact-revision
GET. Esc/navigation does nothing, Lain draft stays separate, and uncertain outcomes
use current facts or an explicit original-revision retry. A flag is not attributed
to this client and is not native termination/rollback; completion winning the race
is shown as the actual result. Loading/stale/unread targets cannot be cancelled.

TDD guards first reproduced missing report support and missing UI stages, then
passed exact-decision/monotonic storage and native scope/choice/resolution checks.
New structured fixture mode omits the permission-resolution event while completing
the turn: native acceptance confirms Received, never Applied. A later cancel
rehearsal failed because its second F8 preceded the task-detail GET; the client
correctly refused. The fixture now waits for the exact target and a loading-state
unit guard covers it. No production assertion was weakened.

Permission final: full368 Rust/zero failures/three existing manual-live ignored,
strict workspace checks/build; native16 host,11 work,8 terminal. Cancel increment:
61 client unit +1 CLI integration passed (one manual ignored), strict client
checks/build and final11 work/8 terminal passed with cleaned synthetic state.
Together373 distinct Rust tests are covered, not a claimed full373 invocation.
Native tests made zero paid/real QQ calls; work/terminal used3/4 fake requests.
An isolated manual GUI host's headless check passed with zero model/QQ and cleanup;
it opens no terminal itself. Human Windows Terminal font review remains pending.

Added [W4 runbook](docs/W4_RUNBOOK.md) and [full acceptance matrix](docs/W4_ACCEPTANCE.md).
External logs are `%TEMP%/lain-q1-tools/w4d-*` and `w4e-*`; images/reports/manual
steps are the session artifact directory's `lain-w4d-2026-11-03/` and
`lain-w4e-2026-11-03/`. Reports identify actual binary hashes and distinguish VT
renders from desktop captures. Full W4 remains active pending manual font evidence;
the current local completion audit is recorded. No personal DB migration, paid-provider/TLS deployment, installed
service, commit or push is implied; unavailable native OSs remain unverified.

## 2026-11-03 — W4c: continue the original remote session and remove orphan ACL helpers

Added schema028 linear continuation families, atomic origin/child/head/receipt
admission, typed Actor `device_send`, owner follow-up/receipt APIs and CLI. The
terminal F6 composer keeps Lain and Worker drafts separate, previews actual
target/text and recovers uncertain cloud receipts without replaying native RPCs.
The native service requires protected original parent scope and actual provider
binding, reuses the original session/thread with new turn IDs, and rejects absent
bindings rather than silently creating replacement work.

Native rehearsal exposed a real recovery error: a verified idle parent's existing
binding was classified as an unexplained root. A minimal red regression and the
narrow verified-parent branch now preserve both continuation and no-replay rules.
Client regressions also protect newer snapshots from late historical queue
receipts and preserve drafts while refreshing pre-submit scope.

A subsequent host regression passed its functional assertions but failed cleanup:
an orphan PowerShell ACL helper retained the synthetic log. Restart Manager and
PID/parent/creation/ACL-helper checks established the exact holder before targeted
cleanup. Shared Rust security now uses handle-based Win32 APIs, preserving current
SID/owner-only policy, check-only existing paths and link/type restrictions, with
no ACL child process. Three native tests cover protected ACLs and rejection of
foreign/inherited/null/empty ACLs, hard links and wrong types.

The ACL increment passed full364 Rust tests (zero failures, three existing manual/
live ignored), format, strict all-target workspace Clippy and build. Its same
binaries passed11 native work,15 host and8 terminal cases with normal cleanup,
zero paid inference/real QQ. No-PowerShell device status is a new host regression.
Evidence is outside Git in `%TEMP%/lain-q1-tools/w4c-native-acl-*.log` and the
session artifact directory's `lain-w4c-2026-11-03/{native-acl-final,terminal-acl-final}`.
Each report contains its actual binary hashes. GUI font fallback, final W4 audit,
unified task cancel and separately tested permission-application visibility remain.
No commit/push, personal migration, installed service or public deployment.

## 2026-11-03 — W4b: inspect actual dispatch text and diagnose native prerequisites

The terminal now compares the exact current-turn user records with actual
DeviceQueue/WorkCreate/WorkSend text. It shows Changed/Unchanged, separate message
IDs, whitespace-preserving JSON strings, the full replacement diff and complete
tool payload; remote ActionId-based RequestId remains visible. This enables no
prompt compiler. Missing/mismatched originals fail closed; confirmation recomputes
the whole view so changed originals cannot reuse a read/approved old scope.

Added `lain-device-agent --check --directory`: same private startup settings,
native executable/version-only probe, canonical workspace checks and device-only
cloud GET, with no operational Kernel/database/lock/Worker creation. It reports
prerequisites, not live-service presence or paid-provider availability. Unreachable
cloud is explicitly not ready. Structured start arguments avoid shell quoting
assumptions; foreground shutdown remains explicit. Startup reuses settings checks.

Three regression tests first failed for the missing comparison/stale-original
guard, then all 22 desk tests passed. Current-source full workspace validation:
350 passed, 0 failed, 3 pre-existing manual/live ignored. Workspace format, strict
all-target Clippy and native build passed. Two Python AST checks and 515 relative
documentation file-link targets passed (different broader scanner than the older
324-link entry, not a count of newly added links); tracked whitespace checks passed.

Expanded native W4 acceptance passed all seven isolated cases, including actual
Changed preview/full-scope reading, exact payload/identity, explicit allow/deny,
parallel chat, close/restore/retry, Kernel restart/cancel, pre-start diagnostics
without execution state and unavailable-cloud diagnostics. Three fake Responses
requests, zero paid calls/real QQ effects and cleaned temporary state. Images
render actual ConPTY VT with synthetic inputs, not native Windows Terminal GUI
font evidence. Report/images: local-only
`<local-evidence>/lain-w4b-2026-11-03/fidelity-diagnostics-attempt5/`.
Its binary hashes are in report.json; subsequent cargo test relinked regular
binaries, so those hashes must not be presented as the later regression hashes.

The native rehearsal exposed fixture mistakes: Windows service/Store exclusively
lock host.lock and worker.db.lock, so the test observes the identity/size/mtime of
these two exact files rather than bypassing their locks. Another attempt precisely
identified `.receipt-2.json.pending-*` becoming receipt-2.json after the cloud
already exposed its WaitingForApproval event. The whole-tree no-write assertion
now waits for the matching native receipt to be durably published before taking
a quiescent snapshot; all other file bytes remain compared. No production lock,
permission gate or Worker-application assertion was weakened. Earlier failed
attempt artifacts/logs are retained beside the final report.

Local logs: `%TEMP%/lain-q1-tools/w4b-fidelity-red.log`, `w4b-fidelity-green.log`,
`w4b-diagnostics-red.log`, `w4b-diagnostics-unit.log`,
`w4b-fidelity-diagnostics-{workspace,clippy-final,native-attempt5}.log`.
Existing host/terminal native regressions then passed all 14 and eight cases on
the cargo-test-relinked binaries, using one and four fake model requests and
cleaning temporary state. Their actual hashes are in
`w4b-fidelity-diagnostics-{host,terminal}.log`; terminal images/report are in
`fidelity-diagnostics-terminal/` beside the seven-case report. No Rust source
changed between these validation builds; do not merge their binary identities.
Full W4 remains ACTIVE: actual same-session
remote follow-up, GUI font evidence and final goal-wide audit remain required.
No commit/push, personal migration, paid calls, real QQ sends, installation or
deployment occurred.

## 2026-11-03 — W4b: conversation-bound dispatch, remote permission and result return

Continued the full W4 goal beyond its visual increment. Schema027 atomically links
each new device command to its actual originating conversation and, for confirmed
Actor dispatch, the exact Actor/action partial receipt. The action UUID becomes
the stable request identity. Unrelated existing work cannot be adopted/moved;
failed link insertion leaves neither an executable command nor a device change.
No cloud-local WorkSession is fabricated for remote execution.

Added typed device list/queue/conversation work reads; ordinary on-demand Lain
requests receive bounded last-known evidence without another model wake. Owner
`queue --conversation` and authenticated conversation device-work snapshot APIs
reuse existing device notifications. Terminal Work joins distinct local/remote
rows; Enter on remote work routes to its device details. Meaningful reported
state/results arrive while chat and drafts remain independent. Reconnect dedupes
versions and restores recent facts rather than repeating work.

Remote F4 review now displays complete actual scope/fingerprint/offered choices,
requires contiguous reading and fresh current-task preflight, and retains its
exact decision identity on uncertainty. Ctrl-R reads receipts; explicit
`/decision-retry` retains the same body/RequestId. Truncated scope cannot approve,
stale/unread views cannot send, and a cloud recorded choice is not presented as
device receipt or backend application. Existing device/Supervisor fences remain.
Contract: [DEVICE_WORK.md](docs/DEVICE_WORK.md), [ADR032](docs/adr/032-conversation-bound-device-work.md).

Tests exposed missing wire ToolCall registration for the new typed action and
outdated old-version migration fixtures. Fixed the actual missing wire boundary;
updated 11 fixture table-drop sequences and four version expectations instead of
weakening production migrations with IF NOT EXISTS. Full workspace then passed
346 Rust tests, 0 failures, 3 pre-existing manual/live ignored. Strict workspace
Clippy and native build passed. A later client-only restoration fix added one
regression and passed all 19 desk tests plus strict client Clippy/build: combined
coverage is 347 distinct tests, not a claimed full rerun after that last edit.

New native Windows ConPTY acceptance passed five complete isolated scenarios:
confirmed natural Actor dispatch, exact deny/factual chat result, explicit allow
verified in the real device Supervisor store, parallel chat/frontend close/
restore/exact retry, Kernel restart with unchanged remote session plus actual
cancel evidence. Three fake Responses requests, zero paid calls and QQ sends;
owned temporary state cleaned. First rehearsal found ConPTY drops legacy CSI14
F4; a separate temporary console probe established SS3S input, fixing the fixture
without changing approval assertions. Next rehearsal exposed a full ID wrapping
inside the long update title and reversed restore display. Added a failing
regression, gave full IDs their own lines and restored newer tasks last in chat
while Work stays newest first. Final native rehearsal passed all five.

Final native binaries also passed the 14-case existing host regression (one fake
request) and eight-case existing desk/portrait regression (four fake requests),
with no paid/real QQ effects and cleaned temporary state. The host report now
queries actual schema27 rather than printing stale26. Python syntax and 324
relative documentation links passed; tracked diff whitespace checks passed.

Local-only evidence: `%TEMP%/lain-q1-tools/w4b-workspace-tests.log`,
`w4b-clippy.log`, `w4b-build.log`, `w4b-device-work-*.log`,
`w4b-device-review-*.log`, `w4b-restored-id-*.log`,
`w4b-work-process-attempt3.log`, `w4b-host-regression.log`,
`w4b-terminal-regression.log`. Images/report:
`<local-evidence>/lain-w4b-2026-11-03/work-attempt-3/`
and `terminal-regression/` beside it. Images render actual VT output with synthetic
content; native GUI font fallback is still unverified. Earlier failed process
attempt images remain in `work/`; console key probe is outside Git.

| Final native binary | SHA-256 |
| --- | --- |
| lain | `23dd5683d2947a6e35ff51b9e695fd9a52901a475d820bb77307cc13635ac728` |
| laind | `ab4bc6123b3952c3d5b98b71d0a2e2c089e3ec748193863776c36134515b0c8a` |
| lain-device-agent | `321071c335de7da696a4ce132be8878ebd45cbcbec38029c44eb2fae050927f8` |
| lain-test-app-server | `29c34c4a667f2b2bf04995be50d8b1c5e2d5b3d7bc1315d809898db4f0ec25cd` |

Full W4 remains ACTIVE. Same-session remote follow-up, service diagnostics,
explicit Changed/Unchanged outbound presentation, GUI font evidence and the
goal-wide completion audit remain. The real cross-device TLS/paid-provider/
Linux/macOS/multi-day checks retain their independent status. No commit/push,
personal runtime migration, real source collection, service installation or
public deployment occurred.

## 2026-11-03 — W4b begins: selected portrait and terminal continuity

Continued the selected W4 goal in the existing dirty checkout without replacing
the W2/W3/W4a baseline. Removed the rejected handmade portraits; an explicit
`desk --portrait <file>` or `LAIN_PORTRAIT_FILE` loads a bounded local 29×18 UTF-8
asset verbatim. The selected third-party A remains outside Git, with source and
unverified redistribution terms recorded. No automatic file discovery, download,
persona change or font installation. Malformed/control/double-width assets fail
before creating a new conversation. No configured asset means a functional
text-only identity, never the rejected face.

Wide windows retain the full portrait rail; 80×24 empty Chat shows all 18 rows,
then yields the space to messages on submission. Smaller windows keep controls
without clipping artwork. Bounded frontend page positions preserve chat reading,
follow state and Worker selection across navigation. Draft ownership is unchanged.

Added regression tests first: the initial compile failure exposed the absent
resource contract; later rendering assertions found that TestBackend wide-cell
padding must be skipped when reconstructing Chinese text. Corrected that test
reader, retaining the actual draft assertion and complete-portrait row counts.
Final native Windows GNU client tests: 40 passed/1 existing manual ignored;
CLI integration: 1 passed. Client all-target strict Clippy, format and build passed.
Eight native ConPTY cases passed, including the added selected-art/welcome/resize
case; 4 fake Responses requests, 0 paid/real QQ calls, owned temporary state cleaned.
The test image renderer selects Segoe UI Symbol for Braille. These images render
actual native VT output with synthetic content, not desktop captures or proof of
Windows Terminal font fallback. Other native platforms remain unverified.

Tested client SHA-256:
`1890609f0aaeb754726f37adb554e3ab9cff1c92279a694b70377e28c3bd3ca3`.
Local logs: `%TEMP%/lain-q1-tools/w4b-portrait-red.log`,
`w4b-portrait-green.log`, `w4b-client-tests.log`, `w4b-terminal.log` in that directory.
Images/report:
`<local-evidence>/lain-w4b-2026-11-03/terminal/`.

This is a visual/continuity increment, not complete W4. Remote conversational
dispatch, exact remote permission UI, supported session follow-up, factual
conversation results and service diagnostics remain. Actual GUI font fallback,
cross-device TLS and real-provider acceptance still need evidence. No personal
instance upgrade, deployment, commit or push.

## 2026-11-02 — Portrait rejected; source-based character-art research

The user rejected the refined handmade portrait and requested faithful reproduction,
research into better methods and existing artwork. Inspected real public Lain
character-art entries rather than drawing another face from memory. Found a
29-column/18-row Lain head and a 34-column/22-row bear-onesie figure, both Unicode
Braille text; preserved their original characters and verified the local research
copies against browser-read text fingerprints. Compared them with the current
draft using fixed-cell font specimens. These are existing third-party artworks,
not original project assets; original artist/republication terms remain unverified.

Research distinguishes structure/edge-to-glyph matching from brightness-only
conversion, citing the authors' 2010 structure-based ASCII project and the 2022
text-placement paper. Chafa's official docs provide a practical offline generator
reference. Also inspected a MIT Lain CLI logo: it is a Copland symbol, not a
character portrait; the Hermes Lain skin describes NAVI computer art. Neither is
misrepresented as a ready-made human avatar.

Font-file checks found missing-glyph output for tested Braille characters in
Consolas/Courier New under Pillow and distinct glyphs in Segoe UI Symbol. This is
not native Windows Terminal fallback/width acceptance. No font, tool package or
third-party installer was installed. No application code or personal runtime
changed. Updated terminal design to mark the rejected visual draft accurately;
passing render tests never meant the user accepted its likeness.

Local-only sources, original text candidates, comparison image, rendering script
and detailed method/source notes:
`<local-evidence>/lain-art-research-2026-11-02/`.
The comparison is a font-layout specimen, not a screenshot of a newly implemented
terminal. New visual assets, expression frames and native font acceptance remain
next steps; no claim of new implementation or user approval.

## 2026-11-02 — Refined character portrait direction

The user selected detailed character art after comparing Lain visual references.
Replaced the eight-line placeholder with an original 18-line ASCII portrait:
asymmetrical fringe, crossed hairclip, one long sidelock, outlined eyes and collar.
Idle looks straight ahead rather than winking. Existing quiet/responding/review/
unavailable states alter only the eyes. Independently drawn eight- and five-line
versions preserve the silhouette in smaller windows; no bitmap/image protocol,
font dependency, runtime image fetch or new timer was introduced.

At >=110 columns and >=32 rows, a 34-column portrait rail leaves the conversation
its full vertical space. Medium windows use an eight-row header; smaller ones use
five rows. Input and exact-scope review keep their existing behavior. Updated the
[terminal design](docs/TERMINAL_DESIGN.md) with these layout thresholds. This is
a reviewable artwork iteration, not a claim that the user approved this drawing.

Validation: both existing render/resize/approval-reading tests passed, client
all-target strict Clippy, workspace format check and locked/offline client build
passed. All seven native Windows ConPTY scenarios passed again with four fake
model requests, zero paid/real QQ calls and cleaned temporary state. The final
tested `lain` SHA-256 is
`d7180f1561e3139e689014ac7a72e0d58463a4a49abcc6d0d651dbabef3e6462`;
the Kernel/Worker-fixture binaries are unchanged from W4a. Other native platforms
remain unverified. No personal runtime upgrade or Git commit/push.

Local-only logs: `%TEMP%/lain-q1-tools/portrait-*.log`.
Character study, actual ConPTY-output renderings and final `report.json`:
`<local-evidence>/lain-character-portrait-2026-11-02/`.
The study is a font-rendered art draft; terminal images reconstruct the actual
native VT stream with synthetic conversations, not desktop screenshots.

## 2026-11-02 — W4a: Lain's conversational terminal

Implemented the already selected Claude Code-like terminal direction as a pure
Rust Chat/Work/Devices shell. `lain` / `lain desk` show Lain ASCII status expressions,
an editable Unicode/multiline draft, authoritative Actor rendering, exact-action
review, existing Worker details and read-only device task lists/details. Explicit
conversation restoration, legacy line chat and dedicated Worker entrances remain.
No second persona, prompt rewriting, remote dispatch UI, service installation,
automatic approval or native Worker-terminal emulation was added. Full interaction
contract: [TERMINAL_DESIGN.md](docs/TERMINAL_DESIGN.md), [ADR 031](docs/adr/031-conversational-terminal.md).

Added an owner-only bounded device-command metadata projection, with no schema or
executor change. Tests verify principal isolation, empty/nonempty lists, no task
body/output in summaries and no Worker launch. Terminal regressions cover original
request identity, busy Enter preserving the next draft, late page responses,
device-cache isolation, reading recovery and stale-action suppression. Failing
tests exposed and guarded trailing-newline cursor placement, skipped approval
scope in tiny windows, and leaving a loading page blocking chat refresh. Actual
Windows ConPTY exposed stale alternate-buffer characters after returning from a
Worker; explicit clears fix re-entry on both surfaces. A failed history read now
marks data unavailable; reconnect can observe the same unchanged Actor revision
without replaying a submission.

Validation on native Windows, Rust 1.99.1 GNU/GCC 16.2 (external caches):

- Workspace run: **328 passed, 0 failed, 3 manual/live ignored**. After the remaining
  client fixes/tests, targeted final client run: **37 passed, 0 failed, 1 ignored**;
  final owner-summary API test: **1 passed**. These runs cover **333 distinct Rust
  tests**; this count combines the workspace baseline and final targeted reruns,
  rather than claiming another full workspace run after every UI edit.
- `cargo fmt --all --check`, strict workspace/all-target Clippy and locked/offline
  workspace build passed. Changed terminal Python scripts parse; 22 changed/new
  Markdown documents have 330 valid relative links; `git diff --check` passed.
- `scripts/desk_terminal_smoke.py`: **7 native ConPTY cases passed**, 4 fake Responses
  requests, zero paid/real QQ calls. Covers draft editing during generation,
  Chat/Work/Devices navigation, native resize, Kernel restart/read recovery,
  Worker inspection/return, exact Lain-action refusal, enrolled-device task list/
  detail observation, parallel chat and exit leaving Worker approval pending.
  All owned temporary state cleaned. Fixture corrections were expected HTTP
  201/202 statuses, a valid-length fake credential and a temporary directory name
  that accidentally matched the existing secret-shaped-output filter; production
  credential filtering was preserved.
- `scripts/terminal_smoke.py`: native error/caught-panic raw-mode restoration passed;
  dedicated Worker attach without resume, resize, detached follow-up, offered
  approval denial and clean terminal exit passed. Removed its incidental `rtk`
  dependency; it invokes the native Cargo executable directly, locked/offline.

Final binaries used by the seven-case acceptance:

| Binary | SHA-256 |
| --- | --- |
| lain | `55f26cb9d7470c6eaf358f93a088526c6aee6f3b6975f66f7cc342bacfce5a33` |
| laind | `1204985bd7a4ac6dbbae43d469e4d49c2c3cd135c8821b041b7c448b03b487e1` |
| lain-test-app-server | `5c4d5ba52e7d39ddc31dd2eab8a6d2c786ab98c51b2bd715acb2afc1173d20ad` |

Local-only evidence, outside Git: logs under
`%TEMP%/lain-q1-tools/w4a-*.log`; accepted images
and `report.json` under
`<local-evidence>/lain-w4a-2026-11-02/`.
Images render the actual native ConPTY VT stream with synthetic fixture content;
they are not desktop screenshots or evidence of real-model chat quality.

No personal QQ/runtime upgrade, cloud deployment, commit or push occurred. macOS,
Linux and real paid/cross-device behavior remain unverified here. W4 is still in
progress: remote dispatch/permission interaction, device-service management,
capability-supported native terminal entrances and authorized follow-up remain.

## 2026-10-28 — Actual QQ picture/sticker conversation accepted

The owner sent three actual image-only picture stickers and an ordinary photograph
to the upgraded personal instance. All four prepared pixels, reached Ready rev2,
entered real Actor viewed_media context, completed one deepseek-flash request each
and received Confirmed QQ replies. Real follow-up messages also used recent image
context. Two selected samples were compared with delivered responses: visible
photo details and sticker action agreed. No autonomous paid replay or forged
owner input was used. The existing private decision and permission boundaries
remain unchanged.

The first four media later expired at their original ten-minute deadline and had
no remaining frame BLOB rows. Temporary private QA image copies were deleted;
only a protected metadata receipt is retained outside Git. No private picture,
message body or credential is added to the repository. The selected ordinary
QQ picture/sticker goal is accepted on native Windows; this is not a claim of
universal OCR accuracy, humanlike chat, live animation/market variation acceptance,
macOS/Linux acceptance or completion of the broader W1–W4 stages. See
[visual acceptance](docs/VISUAL_CHAT_ACCEPTANCE.md).

## 2026-10-28 — Authorized personal visual upgrade

After explicit owner approval, normally stopped the owned hidden console with
no active work/inference/queued delivery, verified the released listener and DB
lock, and made an ACL-preserving full stopped backup of all 43 instance files.
A separate backup copy booted schema24 without model/QQ configuration; eleven
core tables and the selected-private inference decision remained unchanged.
Installed the previously accepted immutable 2d98c70 binary pair, enabled only
the selected NapCat/Actor image capabilities, and restarted in a hidden console
outside the Codex tool job. Actual Ready/Online, schema24 integrity, bound
identity/conversation, existing credential availability, unchanged private budget,
disabled memory/history and no enrolled devices were verified. Full rollback
state is retained privately. No autonomous paid replay or forged input was sent.

Added synthetic APNG-with-poster, animated WebP and overlong GIF fixtures and
tests for actual sampled pixels/times and the explicit 64-frame prefix limit;
292 Rust tests pass (3 manual ignored), with format and strict Clippy passing.
These are test-only additions to the already installed runtime behavior. Real
QQ picture/reply evidence has not arrived yet; the goal remains active.

## 2026-10-28 — Current private visual conversation

Continued from pushed 1d33927 after the user selected picture/QQ expression
support in ordinary conversation. NapCat now preserves image-only/mixed/multiple
picture, market sticker, QQ-face and quote-marker order. Actual bounded decoded
JPEG/PNG/GIF/WebP frames (sampled animation included) enter the normal Responses
request rather than a separate caption/planner call. Model image support and
received-image preparation are explicit default-off configuration capabilities.

Migration 024 adds private short-lived operational frame BLOBs and media metadata;
original text, signed transport locators, archives and semantic memory stay
separate. Receipt/order precedes owned bounded preparation; failures are explicit,
restart never refetches interrupted media, and withdraw/revoke/expiry invalidate
stale drafts, sends and visual speech/tool descriptions in future model history.
Viewed social stickers retain natural closing; unavailable pictures cannot be
silently discarded. Media versions and actual viewing are separately recorded.
Actual visual influence cannot silently promote automatic semantic memory.

Final Windows/MSVC: 290 Rust pass, 3 manual ignored; format, strict Clippy and
locked build pass. Python unit checks: 9 pass, 5 platform skips. Native visual
process: 9 checks/12 fake requests/8 reads; full chat: 36/83; source memory: 5/16;
bridge: 6/10 GETs/jobs; devices: 5/3 reports; normal private policy: 4/4. Exact
installed schema16→24 upgrade/whole-state rollback: 6 checks/3 fake requests,
including retained selected-private unlimited policy; schema23→24: 5/3. All
reports agree on final hashes and clean up; no paid call or real QQ test send.
See ADR 027 and [acceptance/upgrade boundary](docs/VISUAL_CHAT_ACCEPTANCE.md).

The user's real ordinary QQ text message was independently confirmed through
completed inference and Confirmed send receipt after restarting its existing
schema16 release in a hidden console outside the Codex tool job. The current
personal instance remains Ready/Online on that release; this development has not
migrated it or widened source/memory/contact/Worker permissions. Real picture/
sticker quality, personal installation and native macOS/Linux remain pending.
The visual goal and broader W1–W4 exit criteria are not declared complete.

## 2026-10-28 — W2a explicit device enrollment and inventory

Continued from pushed `67bb620`. Added Kernel-owned Pending/Active/Revoked device
identity, exact administrator admission plus independent device credential/manifest
proof, immutable workspace/capability declarations and finite expiry. Dedicated
device routes cannot use administrator credentials or access other principals.
The reserved key namespace is rejected as a Kernel administrator token and covered
by complete-record/streaming redaction. Migration 023 enables no device.

Native CLI generates protected private credentials outside Git, exports only a
separate registration and bypasses owner connection loading for agent operations.
It reports actual selected workspace availability without inventing Worker support.
Monotonic request/sequence receipts survive restart; revoke/expiry prevent late
reports, old claim/re-enrollment and scope expansion. Device paths are not opened
by Kernel. No command/executor/heartbeat subsystem or new dependency was added.

Native Windows/MSVC: 267 Rust tests pass, 3 manual ignored; format, strict Clippy,
locked build and 14 script tests pass. Device process acceptance passes 5 checks,
3 reports, no model. Schema22→23 stopped backup/upgrade/refusal/full rollback:
5 checks/3 fake calls. Full chat: 36/83; source memory: 5/16; bridge/sync: 6 checks,
10 GETs/jobs, no model. Reports match final binary hashes and clean up temporary
instances. Zero paid calls, real QQ sends or real device installation were initiated.
See [acceptance](docs/DEVICE_PAIRING_ACCEPTANCE.md) and ADR 026.

At the user's request, regenerated the protected NapCat login QR. After phone
confirmation, actual existing account <bot-qq> → peer <owner-qq> was verified
Online with no connection failure; personal Kernel remained Ready on its separate
schema16 maintenance release and authorized private-daily inference decision.
This development did not migrate it or enable new personal source/memory/Worker
authority. W2 command/event/resync, W3 native execution, W4 integration and the
combined goal's live/long-term acceptance remain open.

## 2026-10-28 — W1f selected source-derived memory

Continued from pushed `d959871`. Added independent finite binding/source-epoch
write policy, typed one-selection-per-turn archive memory, inferred provenance
with flattened source/memory/input lineage, atomic effects/allowance and existing
freshness/retention gates. Migration 022 changes no old memory or authority.
Source withdrawal/correction and parent changes withhold derived bodies/drafts;
quoted private correction/forget can manage an exact unavailable ID without its
body. Successful correction rebuilds model input and uses the fresh user source.
Generic source-influenced promotion and Worker review remain unchanged.

Native Windows/MSVC: 254 Rust tests pass, 3 manual ignored; format, strict Clippy,
locked native build and 14 script tests pass. New native source-memory fixture:
5 checks/16 fake requests. Full process regression: 36/83. Source bridge: 6 checks,
10 GETs/jobs, no model requests. Actual schema21→22 upgrade/full rollback: 5 checks,
3 fake requests. Final binary hashes agree and temporary instances clean up.
Intermediate failures and their fixes remain documented in
[acceptance](docs/ARCHIVE_MEMORY_ACCEPTANCE.md); zero paid calls/real QQ sends.

Personal daily instance remains the separately installed schema16 maintenance
build; this milestone does not migrate it or enable new personal memory/sources.
Real multi-day continuity, broader summaries/cross-account lineage and W2–W4
remain active work in the combined goal. See ADR 025.

## 2026-10-28 — Explicit private daily inference

The user requested local startup and then removed the proposed count/time limit
for the existing owner private conversation. Added an explicit binding-scoped
daily mode, retaining per-turn limits, durable usage, finite campaign history and
stop/restart gates. No model, memory, contact, Worker or source authority expansion.
This maintenance backport targets the installed schema16 release; validation and
installation evidence will be appended after execution. Native Windows/MSVC: 211
Rust tests passed, 3 manual ignored; format, strict Clippy and locked build passed.
The native daily-mode process smoke passed 4 checks / 4 fake model requests,
including auth, exhausted finite history, actual private replies, restart,
unbound denial and stop; cleanup succeeded. No paid test call or real QQ send.
See ADR 024.

The schema16 maintenance build was installed locally after targeted normal console
shutdown and verified complete ACL-preserving backup. Personal database stayed
schema16 and credentials were unchanged. Runtime reached Ready; owner-authorized
private-daily decision revision 12 selects the existing private binding. QQ login
was still awaiting phone QR confirmation at this check; no real model/QQ success
is inferred. The mainline integration separately passes 244 Rust tests (3 manual
ignored), format, strict Clippy and locked native build.

This file records implementation history, milestone outcomes, important decisions, problems found during development, and the handoff state for the next coding session.

It is intentionally different from the architecture documents:

- `docs/*.md` describe what the system **should be**;
- `ROADMAP.md` describes what we **plan to build**;
- `DEVLOG.md` records what **actually happened**.

The Git history remains the authoritative record of exact commits.

## 2026-10-27 — W1e finite selected-source synchronization

Continued from pushed `666d1fc`. Added separately enabled source/archive-epoch
network scopes, retained revision/usage snapshots, atomic attempt/Prepared read
admission, persisted nearest-deadline scheduling and existing source cursor/update/
withdrawal handling. Restart preserves spent attempts; failed/unknown reads stop,
and revoke/expiry block late commits. Migration 021 enables nothing by default.
No generic scheduler, model calls or personal-instance upgrade is introduced.

Validation and current limits: [KNOWLEDGE_SYNC_ACCEPTANCE](docs/KNOWLEDGE_SYNC_ACCEPTANCE.md).
Durable derived continuity and the rest of the combined goal remain active work.

## 2026-10-27 — W1d explicit independent QQ source bridge

Continued from pushed `d954c80`. Added persistent independent store identity and
source-filtered current change export; deployed the additive protocol after
native Windows/server Linux service tests, with a stopped protected full backup
and compatible-container rollback that never rewinds spent vision reservations.

Kernel adds explicit private/group knowledge pulls through the existing durable
read receipt/cancel/Unknown lifecycle. Migration 020 stores source/event linkage,
atomic projections/updates/tombstones and short caught-up freshness leases. Raw
image bytes stay independent; original text and model captions retain separate
attribution. Exact same-perspective private suppression prevents a mirrored old
message from bypassing correction/forget. No automatic collection/model/Worker,
personal-instance upgrade or old budget reuse.

Evidence and remaining scope: [KNOWLEDGE_BRIDGE_ACCEPTANCE](docs/KNOWLEDGE_BRIDGE_ACCEPTANCE.md).
Continuous ingestion/finite standing network authority, durable derived memory,
real QQ images/long-term naturalness and W2–W4 remain active work.

## 2026-10-27 — W1c selected archive conversation recall

Continued after pushing W1b (`2f7a3eb`) to both development branch and main. Added
an independent default-off finite private-binding archive recall decision,
selected source/import epochs, typed archive search/inspect and fresh attributed
excerpts. Migration 019 enables nothing on existing instances. Source archive
storage is still independent of semantic memory, actual experiences and control.

Lookup selections/audit commit atomically; version checks cover source/recall
revoke/expiry, record correction/forget/retention and model adapter admission,
response, completion and delivery. Empty results retain authorization dependency.
New-input reconsideration clears selection without resetting budgets; later
model/source history cannot replay derived answers. Archive-influenced memory
proposal/remember/correct is blocked pending durable derived-memory provenance.

Seven new domain/Actor regressions and an extended native CLI/HTTP/model-SSE/
OneBot fixture exercise three-day recall, restart, correction, forget and revoke.
Final validation evidence/remaining limits are in
[ARCHIVE_RECALL_ACCEPTANCE](docs/ARCHIVE_RECALL_ACCEPTANCE.md). All this slice's
model responses are fixtures; no old paid budget or personal migration is reused.
Independent QQ ingestion into Kernel, persistent derived summaries/recognition,
actual source collection, long-term naturalness and W2–W4 remain active work.

## 2026-10-27 — W1b explicit native visible Worker archive reads

Added source/revision-scoped read receipts and explicit CLI/API operations for
local Codex visible history, separately verified for installed 0.160.1. Managed
Worker control remains gated to 0.160.0. Metadata/terminal-turn/item pagination
does not resume/control a source, run inference or create managed work. Private
native cursors continue through source/grant/version-bound Lain receipt IDs.
Reasoning/raw/unknown/diff bodies are omitted. Import/dedup and Completed cursor
commit atomically with adapter_observation origin; cancel/revoke/shutdown await
owned-reader closure, while boot recovers unfinished reads Unknown without replay.

Migration 018 appends one table/index. Existing owner imports and retention/
correction/forget remain intact. Native Windows workspace 223 passed/3 manual
ignored; strict Clippy, format and locked build passed. Six new contract/storage
tests cover pagination, gates, sensitive/mismatched sources, blocked-reader
cancellation/exit, receipt-write rollback and interrupted recovery. Fixed an old
30 ms expiry-test timing assumption using deterministic persisted expiry.
Fake process suite: 33 checks/76 fake calls; synthetic 17→18 full upgrade/old
refusal/rollback: 5 checks/3 fake calls; 0 paid and cleanup true.

An actual isolated native Codex turn used one local fake Responses request.
After restart, the final actual Lain CLI archived its user/assistant items through
read-only APIs; source rows remained unchanged, revoke withheld archived text,
and no new Worker/Lain model turn ran. Personal history/database were untouched;
schema16 personal release and hosted QQ store were not upgraded by this slice.
See ADR 020 and ARCHIVE_READ_ACCEPTANCE.md. W1 continuity/derived invalidation,
managed 0.160.1 control, W2–W4 and complete-stage/live quality remain open.

## 2026-10-27 — Independent QQ message/image knowledge service deployed

Final lifecycle follow-up: direct native-interpreter task ownership and an OS
outbox lock prevent duplicate observers. The original PowerShell wrapper left
Python children after task stop; only positively identified observers were
terminated. Actual Stop→zero processes→Start→one authenticated process passed.
The final independent service suite has 14 tests on native Windows and server
Linux, with the same 1/10 real vision reservation retained across deployments.

The user extended the active goal with decoupled QQ messages/images, DeepSeek
vision and a keyed hosted API for Utakera, explicitly authorizing server deployment.
Added a separately runnable Python source store and read-only NapCat observer:
source revisions/allowlists, ordered segments/media provenance, source-scoped image
hash/cache, bounded direct vision, durable conservative budget, retention/tombstones,
search/change feed, role-specific API keys and a durable bounded upload outbox.
No Kernel/persona/semantic-memory/Worker runtime dependency or historical DB upload.

Actual Ubuntu/Caddy/Docker deployment serves a loopback-only unprivileged container
through the private deployment's verified public HTTPS `/lain-knowledge` route.
The actual address is retained outside Git. A real synthetic
image was stored/downloaded with equal SHA-256 and described by DeepSeek Flash:
one request, 322 reported tokens, independent lifetime cap 10. Duplicate event
delivery did not spend another call; synthetic source subsequently revoked.
No-auth 401/read 200/read-key mutation 403 passed. Three API roles/model key are
protected outside Git. Existing unrelated services were not restarted.

Native Windows and actual server Linux Python each passed 13 deterministic tests,
including an actual local WebSocket observer fixture. Ruff check/format passed.
The current-user hidden observer task is Running, actual NapCat identity verified,
outbox drained; only established bot/owner private pair enabled, groups empty.
MSIX AppData redirection initially broke scheduled-task paths; installation now
uses the physical resolved path and checks runtime health, not just registration.
Original Rust W1+D3 merge passes 217 tests/3 manual ignored and strict Clippy;
personal schema16 grants/budget remain closed. MaiBot mechanism research and
Utakera reference commits, API/operating limits and remaining actual-image/client/
Kernel-continuity evidence are recorded in ADR 019 and QQ_KNOWLEDGE_ACCEPTANCE.md.
The complete expanded goal remains unfinished; no multi-day or Worker acceptance.

## 2026-10-26 — W1a selected visible-source archive imports

Implemented the Kernel-owned archive foundation for declared QQ-group/Codex-session
sources: default-off revisioned import/disclosure scopes, bounded atomic owner
imports, exact per-source event identity, commit-order cursor windows, finite body
retention, correction/forget tombstones and content-free edit receipts. No source
import creates a conversation, model call, semantic memory, experience or managed
Worker. Owner declarations are explicitly not authenticated provider observation,
complete coverage or participation. Actual read adapters/derived continuity remain W1.

Schema 017 appends four tables/indexes and preserves prior grants/records/control.
Replays neither renew retention nor refund admission. Same-ID changed payloads
conflict atomically; different source accounts/events remain distinct. Corrections
label owner text; original-event replay cannot undo correction/forget. Grant expiry,
explicit revoke, body expiry and previous grant epochs have separate projections.
Startup/fresh-read checks and one nearest-deadline timer enforce expiry without
idle inference. CLI/API expose bounded selected JSON operations and audits.

Native Windows/MSVC: 215 Rust tests passed, 0 failed, 3 manual ignored; fmt/check,
strict workspace/all-target Clippy and locked native build passed. Eight archive
tests include credential-rotation response filtering and actual timer-driven body
clearing. Native fake HTTP/OneBot: 33 checks, 76 fake model calls, 0 paid calls;
archive cases add no inference. Whole-backup/schema16→17/old-version refusal/full
rollback: 5 checks, 3 fake calls, 0 paid, no personal state. Fourteen Python
replay/staging tests passed. Cleanup and binary hashes match; ARCHIVE_ACCEPTANCE.md
and ignored target/w1a/ retain evidence, including early compilation and stale
schema-expectation failures. No actual group or Worker history was collected.

The owner subsequently authorized the separate D3 live plan: a new fixed 48+12
request split and 6-hour deadline over the selected schema-16 release. Its protected
controller and exact source/binary snapshots are outside personal rollback state;
W1 is excluded from that installation/collection authority. Real D3 operations
and outcomes are tracked separately. The complete two-stage goal remains active.

## 2026-10-26 — D3 targeted real follow-up and reading receipt retention

Four fresh isolated memory scenes now commit/recall correction and forget using
the same original DB and budget: eight new calls, cumulative 23/48. The final
absence response remains verbose. The last allowed RFC acquisition completed with
nine form feeds and one bounded model summary (personal 1/12); quiet hours kept
sharing Prepared with zero QQ sends. All temporary scopes/budgets closed.
Post-revoke checks exposed a summary duplicate in the root Actor receipt. Cleanup
now removes that content too, including old invalidated snapshots, without losing
operation/usage facts. Native schema16 workspace 209 passed/3 manual ignored,
strict Clippy and locked build passed. Full-stage quality/live send remain open.
See docs/D3_LIVE_ACCEPTANCE.md; no personal data or credentials committed.

## 2026-10-26 — D3 live regression repair on schema 16

The new authorized campaign used 15/48 isolated real-model requests before a
private-memory correction failed safely: prefix-only automatic terms omitted
late topic words, generic global memory inspection could not inspect a private
ID, and correction lacked a valid inspection. No correction effect committed;
temporary scope/budget closed. Actual source acquisition failed UnsupportedContent,
with 0 personal model calls/0 sends. The personal target was backed up completely
and upgraded 11→16 headlessly before its finite 12-request budget was installed.

Repair keeps schema 16 and existing permission/revision guards. Private recall
supports exact receipt IDs within the active binding and bounded prefix/suffix
terms; generic inspect remains general-scope only. Bounded plain-text acquisition
retains benign ASCII form-feed page separators with JSON escaping while still
rejecting ESC/other forbidden controls. The first source failure's exact cause is
not asserted without the remaining actual acquisition. No extra URL or allowance.

Native Windows: 208 passed, 0 failed, 3 manual ignored; format/check, strict Clippy
and locked build passed. Native fake protocol: 28 checks/76 fake calls; synthetic
schema11→16 full rollback: 5 checks/3 fake calls; 0 paid/cleanup true. Real follow-up
must retain the existing DBs, budget IDs, caps and deadline, inspect failed effects
before fresh cases, and respect the now-active 23:00–09:00 contact quiet period.
Raw operational data/credentials remain outside Git. See D3_LIVE_ACCEPTANCE.md.

## 2026-10-26 — D3c isolated autonomous replay runner

Added an automatic local OneBot replay over 16 synthetic chat/continuity/memory
scenes. It records actual incoming/batch/Actor IDs, normalized model outputs,
memory effects, literal sends and simulated receipts rather than using user-sent
QQ messages. Mechanical format measurements do not certify naturalness. Real
mode creates no fake Responses server; free rehearsal is separately labeled.

The runner binds a new decision to exact binaries/config/SecretRef, immutable
campaign/budget IDs, output directory, request cap, duration and upstream-attempt
verification reference. It boots headlessly to install the budget before enabling
model/transport, expires/revokes synthetic memory independently, never approves
work, and leaves historical source collection off. Controller state is outside
the instance subtree; intent is flushed before input. Existing or interrupted
campaign output cannot be replayed. Exit closes grants/budget, stops the owned
daemon and reconciles the same database in a second headless boot; an attempted
but interrupted request remains spent and Unknown.

Ten native Windows Python boundary/process tests passed, including a complete
16-scene fake corpus, finite exhaustion, no-overwrite, tamper/proposal refusal,
secret filtering and interruption both before and after an actual local HTTP
attempt. Four release-staging tests passed; its manifest now fingerprints the
three repository runner dependencies without copying extra files/credentials.
All calls were fake/0 paid; native schema-16 Rust binaries are unchanged from D3b.
Details and retained early failures: DAILY_REPLAY_ACCEPTANCE.md, target/d3c/.

Fixed a Windows acceptance-directory ACL helper: verify the existing owner and
set only the DACL, avoiding an unnecessary WRITE_OWNER request that failed on the
workspace volume. Current-owner-only validation remains enforced. Early launcher
setup-in-Git and channel-list shape mistakes failed before model use and were
corrected in the runner, without weakening runtime credential location checks.

No personal credentials/state were read, no new real model/QQ calls or personal
upgrade occurred. D3 real-model quality, actual source→share→recall and multi-day
evidence remain pending; W1–W4 and the combined goal are not complete. Protected
credential-source access, actual upstream bridge inspection and one concrete
consolidated live decision still precede paid or personal operations.

## 2026-10-26 — D3b persistent global inference ceiling

Added a Kernel-wide optional Lain Actor inference limit over conversation,
private chat, reading, sharing, tool continuations and reconsideration. Immutable
budget IDs, revisioned enable/stop decisions and transactional Actor/request
reservations prevent turn/grant/restart refills. Adapter entry rechecks the limit;
returned normalized usage is recorded before later source/body rejection. Failure,
cancellation, timeout and restart preserve spent reservations and honest
Prepared/Attempting/Returned/NotAttempted/Unknown phases. No prompt, source body,
outbound words or credentials are stored in the ledger. Worker costs and actual
upstream invoices are explicitly outside its coverage.

Migration 016 preserves old limits until explicit opt-in and never invents paid
authority. After opt-in, stop/expiry/exhaustion stay closed until a finite owner
decision; reopening the same ID retains all usage. A budget cannot enable source,
memory, contact, Worker permissions or model access. CLI/API provide show/budgets/
requests/enable/stop without adding an Actor grant tool. Local reqwest 0.13.5
inspection revealed default protocol-safe retries; own model and text adapters now
explicitly disable retries as well as redirects.

Prepared a review-only release packager: exact evidence-matched binaries, non-secret
scope proposal and five-step full backup/upgrade/rollback note under owner-only
workspace storage. It never opens a target instance, loads credentials, starts a
service or authorizes live calls. Existing native ACL protection is shared with
the synthetic upgrade fixture. A real upgrade must bootstrap without model/QQ
configuration, install the finite campaign first, then restart approved settings.

Native Windows/MSVC (Rust/Cargo 1.98.1): 207 tests passed, 0 failed, 3 manual ignored;
format/check, strict workspace/all-target Clippy and locked build passed. Native
fake HTTP/OneBot: 28 checks, 76 fake requests, 0 paid calls. Full synthetic schema
15→16 and 11→16 backup/upgrade/refusal/rollback each passed 5 checks/3 fake calls;
no personal state access, cleanup passed. Four native Python package-staging tests
passed. Evidence and final hashes are in INFERENCE_BUDGET_ACCEPTANCE.md; local logs
and unapproved packages remain ignored under target/d3b.

No real provider, QQ contact or personal upgrade was performed. D3c still needs
the bounded autonomous real-model replay runner and concrete consolidated live
authorization, including source/retention/memory/contact/stop scope and verification
of any upstream bridge. Naturalness/multi-day evidence and W1–W4 remain open; the
combined goal is active and not complete.

## 2026-10-26 — D3a fresh sharing receipts in conversation

Connected private conversation to the actual D2 share outbox without pretending
its draft was user-visible speech. One recent content-free terminal receipt can
support follow-up; typed sharing search/inspect supplies freshly checked outbound
words and source evidence only while the original experience/disclosure remains
current. Confirmed/Unknown/NotSent/Rejected and user-read uncertainty stay distinct.
Source correction/withdrawal/revocation withholds content without denying an
actual earlier contact. Revoking contact does not erase its historical receipt.

Private ShareVersions participate in existing response/body/completion/Ready/
prewrite freshness guards and are reset on bounded reconsideration. Past share-
source-influenced assistant/tool copies do not reenter model history. Source
words cannot silently promote memory; automatic metadata does not block ordinary
reviewed/current-source memory. Migration 015 adds content-free transactional read
audits; CLI/API provide receipt recall/inspection/audit without new send authority.

Native Windows/MSVC (Rust/Cargo 1.98.1): 199 tests passed, 0 failed, 3 manual tests
ignored; formatting/check, strict workspace/all-target Clippy and locked build
passed. Final native HTTP + fake Responses/OneBot: 26 checks, 75 fake requests,
0 paid calls. Synthetic schema 14→15 backup/upgrade/refusal/full rollback: 5 checks,
3 fake requests, no personal-state access. Both report successful cleanup and
match final binary hashes in SHARING_RECALL_ACCEPTANCE.md. Actual acquisition for
that process case is seeded synthetic data, not a real public fetch.

Retained failures: omitted closed Wire ToolCall registration; invalid Running/
Proposed fixture state (replaced with actual reviewed-tool flow); unused fixture
variable; parallel global diagnostics warning capture (isolated before adapter
events). Topic search was tightened to permitted values rather than JSON schema
keys before final full-suite, lint/build and process reruns. Logs remain ignored
under target/d3, including all intermediate failures.

Personal installation/credentials/QQ remain untouched. This does not complete D3:
the concrete new live campaign budget/authorization/upgrade package and longer
real experience still need work. W1–W4 and the complete combined goal remain open.
See ADR 016 and SHARING_RECALL_ACCEPTANCE.md for exact limits and operating steps.

## 2026-10-26 — D2 independent proactive sharing

Implemented the next slice of the combined daily-companion/shared-work goal:
default-off per-binding ShareGrant, explicit revisioned attention/DND, fixed-offset
quiet windows, cooldown/day/grant limits, source-linked expiring candidates and
no-response control. New reading completions may generate a separate no-tool
Actor Share/Skip draft. Drafts, model reservations, attempts and confirmed/uncertain
QQ outcomes remain distinct; no user Batch or semantic memory is fabricated.

Migration 014 preserves old grants/records and starts contact disabled. Fresh
checks cover source/grant/experience/input/attention before disclosure and send.
Enforcement covers all historical attempts, even beyond the inspection page.
New user input takes priority; service-owned bounded send tasks keep receipt
waiting off chat inference. Restart expires unsent work, marks in-flight contact
Unknown and advances the boot watermark without catch-up or automatic retry.
CLI/API provide enable/revoke/show/consider and explicit available/DND controls.

Native Windows/MSVC (Rust/Cargo 1.98.1): 192 tests passed, 0 failed, 3 manual tests
ignored; formatting, strict workspace/all-target Clippy and locked build passed.
Actual native HTTP + fake Responses/OneBot suite: 25 checks, 69 fake requests,
0 paid calls, cleanup passed. Synthetic schema 11→14 backup/upgrade/refusal/full
rollback: 5 checks, 3 fake requests, personal state not accessed, cleanup passed.
The installed old executables were only copied as binary inputs to the fixture.
Schema 13→14 unit evidence also preserves original snapshot bytes/default-off state.

Failures retained: initial SQLite count return-type error; service timeout exposed
a missing Kernel wake-up for experience publication by another Actor; Clippy found
test-module ordering. Each was corrected before the final validation. Local logs
and hashes remain ignored under target/d2; reviewable evidence and operating limits
are in SHARING_ACCEPTANCE.md and ADR 015.

No real-model, public-source, QQ-contact or personal-upgrade operation was made.
Natural-language DND/calendar/DST, real timing/quality/multi-day evidence, complete
sharing-receipt recall integration (D3), and W1–W4 remain open. This milestone does
not complete either large stage or the active combined goal.

---

## Legacy note

The pre-reboot implementation is preserved on the `legacy/v0` branch.

The old repository also contained a historical `DEVLOG.md` describing the earlier Rust + React rewrite, WebSocket connectivity, and file-based memory experiments. That log recorded dates as 2025-04-15, while the corresponding Git history is from 2026. The legacy branch is preserved unchanged; this v1 log uses the current Git history and reboot timeline.

Important legacy milestones include:

- Python + Live2D / ASR / TTS prototype;
- first Rust + React rewrite;
- WebSocket chat;
- direct LLM adapter;
- file-based profile/history memory;
- topic extraction and consolidation.

Those experiments are useful historical evidence, but they are not architectural precedent for v1.

---

## 2026-10-04 — Architecture reboot

### Milestone

`reboot: "Reset the Wired"`

### Why the reboot happened

The previous implementation had begun to reproduce responsibilities already owned by modern agent runtimes:

- LLM client/runtime logic;
- conversation history;
- memory extraction;
- retrieval;
- topic consolidation;
- parts of a generic agent loop.

The project direction was changed from "build another agent runtime" to:

> Lain is a persistent resident agent and an agent control plane for existing worker agents.

### Completed

- archived the previous implementation to `legacy/v0`;
- removed the old Rust kernel, React frontend, memory implementation, and chat loop from active `main`;
- established documentation-first architecture;
- added:
  - `AGENTS.md`
  - `README.md`
  - `ROADMAP.md`
  - `docs/PRODUCT.md`
  - `docs/ARCHITECTURE.md`
  - `docs/DOMAIN_MODEL.md`
  - `docs/RUNTIME_MODEL.md`
  - `docs/AGENT_BACKENDS.md`
  - `docs/MEMORY.md`
  - `docs/CONFIGURATION.md`
  - `docs/DEVELOPMENT.md`

### Important decisions

- Kernel is the server-side source of truth.
- Web/TUI/future integrations are clients of the Kernel.
- Worker-agent intelligence stays inside external harnesses.
- Lain owns orchestration, continuity, memory, and her private world.
- Worker context/transcripts are not Lain memory.
- Provider-specific protocol objects must not leak into the core domain.
- Runtime profiles will replace global configuration mutation.
- 24-hour residency is event-driven, not continuous model inference.
- GitHub Actions should remain lightweight and deterministic.
- Milestone commits may keep the project's short atmospheric naming style.

### Next step at the time

Build Phase 1: a durable headless Kernel foundation before integrating any real worker.

---

## 2026-10-04 — Phase 1: Kernel foundation

### Milestone

`foundation: "A Place to Remain"`

### Goal

Create a durable headless `laind` process with provider-neutral domain state, persistence, recovery, a minimal authenticated API, and no worker-agent implementation.

### Completed

#### Rust workspace

Created a minimal workspace:

```text
Cargo.toml
Cargo.lock
crates/
  lain-kernel/
```

The initial implementation intentionally remains a single crate rather than splitting into speculative micro-crates.

#### Domain model

Implemented strong domain identifiers:

- `WorkSessionId`
- `TurnId`
- `EventId`
- `ApprovalId`
- `RuntimeProfileId`
- provider-neutral `BackendKind`

Implemented `WorkSession` and a concrete `WorkSessionState` transition matrix.

Important semantics:

- `Idle` means a reusable session with no active turn;
- `Completed` means a confirmed terminal session state;
- `Cancelled` means confirmed cancellation, not merely a cancellation request;
- `Disconnected` and `Unknown` do not imply failure.

#### Operational persistence

Added SQLite persistence with:

- transactional schema migration;
- committed `Cargo.lock`;
- WAL mode;
- durable WorkSession snapshots;
- restart recovery;
- database ownership lock;
- fail-closed behavior for corrupt/unsupported persisted state.

On restart, previously active states that cannot be confirmed without a live backend are reconciled to `Unknown` rather than guessed.

#### Kernel ownership

The Kernel owns:

- lifecycle state;
- committed in-memory WorkSession projection;
- storage mutation ordering;
- event publication ordering.

Session changes are persisted before the in-memory projection/event notification is committed.

#### Event foundation

Added a bounded Tokio broadcast channel.

Initial normalized events:

- `SessionCreated`
- `SessionStateChanged`

The channel is a live notification mechanism, not a durable audit log.

Slow subscribers receive lag and must resnapshot authoritative state.

#### Local API

Added an Axum HTTP API:

- `GET /health`
- `GET /ready`
- `GET /v1/work-sessions`
- `GET /v1/work-sessions/{id}`

Phase 1 intentionally keeps the API read-only.

All routes require a bearer token.

The daemon binds to loopback only in Phase 1.

#### Secret handling

`LAIN_API_TOKEN`:

- is required;
- is validated;
- is never serialized;
- is redacted from Debug/errors;
- is compared using constant-time byte comparison;
- is not stored in SQLite or Lain domain state.

#### Lifecycle

Implemented:

- boot;
- recovery;
- ready state;
- SIGINT/SIGTERM and Windows Ctrl-C/Ctrl-Break handling;
- bounded graceful HTTP shutdown;
- storage close/reopen behavior.

### Validation

The Phase 1 test suite covers:

- domain IDs;
- complete WorkSession transition matrix;
- configuration validation/redaction;
- workspace validation;
- persistence round trips;
- restart reconciliation;
- corrupt-state fail-closed behavior;
- database single ownership;
- commit-before-event ordering;
- storage failure without false event publication;
- subscriber lag behavior;
- API authentication/status/errors;
- real TCP shutdown and database reopen.

Documented validation commands:

```sh
cargo fmt --all --check
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
```

No GitHub Actions workflow was added.

### Known limitations / deferred work

- no worker runtime is started;
- no Codex protocol exists yet;
- no public session-create/prompt/cancel API exists yet;
- no Turn or Approval entity is implemented yet;
- no provider-native session identity is persisted yet;
- no durable event timeline exists;
- no live WebSocket/SSE client event API exists;
- no RuntimeProfile registry/SecretRef store exists yet;
- no Lain Actor, memory, World, scheduler, Web, or TUI exists yet;
- native Linux signal behavior still deserves direct Linux validation.

### Phase 1 review

The implementation matches the intended architecture well enough to proceed.

Notable good choices:

- conservative restart semantics instead of pretending worker state is known;
- no speculative backend framework;
- no fake worker implementation just to satisfy APIs;
- provider-neutral domain names;
- strict separation between operational state and future Lain memory;
- event publication only after durable mutation;
- explicit lag/resnapshot semantics.

### Handoff to Phase 2

Phase 2 should integrate **Codex App Server** as the first real worker.

Before locking the adapter contract, verify the installed/current Codex version and protocol behavior.

The next milestone must settle, with evidence:

- App Server process ownership and restart behavior;
- initialize/initialized handshake;
- thread creation and resume;
- turn lifecycle;
- provider thread/turn identity persistence;
- event normalization;
- cancellation acknowledgement;
- approval request/response mapping;
- disconnect/reconciliation behavior;
- multiple concurrent WorkSessions;
- how live worker events are exposed to future clients.

Do not begin Claude Code, DeepSeek Harness, Lain memory, or UI work as part of Phase 2.

---

## 2026-10-04 — Phase 2: Codex worker control

### Milestone

`contact: "Across the Wired"`

Lain now supervises real Codex work through authenticated domain APIs. Phase 3
has not started.

### Implemented behavior and decisions

- Verified installed `codex-cli 0.160.0`, its generated schemas, official App
  Server documentation/source and the current ACP adapter. Selected direct
  App Server: the ACP adapter itself starts App Server, so an extra translation
  does not help this first backend. Volatile facts are in `docs/backends/CODEX.md`.
- Kept one Rust crate. Added a small supervisor plus private Codex/JSONL/process
  modules; no generic worker framework, coding loop or mandatory ACP layer.
- One owned process/task per WorkSession isolates failures and cancellation.
  Processes have allowlisted environments, drained private stderr, bounded
  frames/queues/RPC waits, and owned process-tree cleanup. Windows Job Objects
  were exercised; Unix process-group cleanup still needs native validation.
- Added Turn, Approval and explicit cancellation evidence. A completed, failed
  or interrupted turn leaves the reusable WorkSession Idle. RPC acknowledgement
  never substitutes for observed terminal state; a completed turn can coexist
  with an acknowledged cancellation. Rejected cancellation can be retried.
- Migration 002 preserves opaque provider thread/turn identities, selected
  session options, turns and approvals separately from public Lain IDs. Atomic
  recovery and mutations cover both session and worker metadata. No prompts or
  worker transcript history are stored in Lain SQLite.
- SQLite/projection access from async paths uses blocking tasks, including reads
  that might wait behind a writer. The independent reader routes RPC responses
  without SQLite waits. Output fragments publish counts without durable writes;
  completed output is sanitized and bounded. Queue overflow loses certainty and
  stops the affected runtime instead of dropping lifecycle observations silently.
- Owner decisions support approve-once, deny and applicable cancel. Permission
  requests grant only the requested subset for one turn. Unknown requests are
  rejected; unresolved connection-local approvals expire/become Unknown on loss.
- Added create/prompt/follow-up/cancel/resume/approval control, backend reporting,
  turn/approval inspection and authenticated SSE. Snapshot and subscription are
  established atomically against durable mutations. Lag requires a new snapshot;
  EventId is not a replay cursor. Client disconnect does not stop the worker.
- Restart restores the same Lain/provider identities and reconciles provider
  history. Missing rollouts, lost submission identities and uncertain outcomes
  remain explicit; no replacement thread is created silently.

### Validation and problems resolved

Local validation: 14 deterministic tests passed; `cargo fmt --all --check`,
`cargo test --locked --workspace`, and
`cargo clippy --locked --workspace --all-targets -- -D warnings` passed.
Tests use a local Rust fake App Server; they do not require login or model tokens.
Coverage includes migration/recovery rollback, notification-before-response
ordering, numeric/string approval IDs, command/file/permission approvals, unknown
requests, cancellation rejection/retry and completion races, two sessions,
foreign-session events, secret fragments, malformed frames, bounded-queue overflow,
crash/restart, API controls and SSE snapshot/lag/detachment.

Real Windows acceptance used `scripts/codex_smoke.py --run-real`, the local
ChatGPT login and `gpt-6-astra/high`. A complete successful run demonstrated:

1. Two distinct real provider sessions in separate temporary workspaces.
2. Initial turns, normalized progress/completion and same-session follow-up.
3. Cancellation of one active session without stopping the other.
4. SSE client disconnect/reconnect with continued worker ownership and snapshots.
5. A real command approval routed through the owner API and resolved.
6. Forced termination of one direct child of the test daemon, followed by
   conservative disconnect and explicit resume of the same identity.
7. Graceful daemon shutdown/restart, both provider identities unchanged, followed
   by a new successful real turn. Shutdown exited 0; the API token was absent
   from captured daemon logs. Temporary workspaces/databases were cleaned up.

After moving asynchronous projection reads to blocking tasks, the final binary
also passed a real HTTP/SSE prompt check: exact expected assistant text, correctly
correlated Lain session/turn IDs, persisted Completed turn and Idle session.

Implementation checks found and fixed a resume race after publishing disconnect,
a snapshot/subscription stale-event window, non-atomic worker/session recovery,
and protocol replies that could be interrupted by command selection. Test cleanup
initially held its own SQLite connection open on Windows; the smoke now closes it
explicitly and runs a private executable copy to avoid Cargo file-lock contention.
One real run failed an immediate interrupt; active-work acceptance now waits for
actual progress. The adapter still reports rejection/uncertainty rather than
claiming cancellation succeeded.

### Limitations and Phase 3 handoff

- Supported protocol version is explicitly 0.160.0. Capability reporting reflects
  verified adapter support, not a general server method negotiation mechanism.
- An unused thread may have no resumable rollout. A turn whose native ID was lost
  cannot safely be matched by text/order. Approval request IDs never survive a
  process connection; external side effects may remain uncertain after loss.
- Windows is the tested host. Linux needs native lifecycle tests and service
  cgroup cleanup for hard daemon crashes; managed shutdown uses process groups.
- Completed output is live-only and bounded; reconnect recovers operational
  state, not a transcript. Known/pattern-based secret filtering cannot recognize
  arbitrary unknown or encoded secrets. Credential retrieval remains prohibited.
- Metadata uses whole-session snapshots; indexed turn/approval rows are deferred
  until history size warrants them. Full profiles/SecretRefs, additional backends,
  durable event replay, archive semantics, memory/world and UI remain deferred.
- Phase 3 clients must handle Unknown, cancellation acknowledgement separately
  from completion, approval expiry, SSE resync and refresh after BackendReconnected.
  Decide transcript retention and profile configuration explicitly when needed;
  do not turn these operational records into Lain memory.

## 2026-10-05 — Phase 3: presence

Phase 3 is implemented in its required order: durable WorkSession and detachable
WorkerRuntime, persistent operational history, then an independent CLI/TUI.
Phase 4 has not begun. The architecture decision is recorded in ADR 001.

- Added separate Detached/Starting/Attached/Stopping/Lost runtime observations.
  An event-driven 300-second default idle timer releases only safe idle workers;
  active turns, connection-local approvals and serialized operations prevent it.
  Lazy prompt/resume uses the existing provider identity. Known idle boot does
  not spawn workers; uncertain work selectively reconciles. Planned idle/shutdown
  keeps Idle + Detached; unexpected idle loss keeps Idle + Lost; active unconfirmed
  loss/shutdown makes the session/turn Unknown. Processes remain Kernel-owned.
- Verified that an unused Codex 0.160.0 thread is not resumable by thread/start
  alone. A full read initializes this version's default history (including its
  -32601 list_turns limitation), and a persistent fork creates a resumable empty
  rollout. Bind only the successful fork at initial creation, never replace an
  existing binding, and never submit synthetic model work. This empirical,
  version-gated behavior and native seed-retention tradeoff are explicit in CODEX.md.
- Migration 003 stores normalized history in indexed rows separate from worker
  snapshots. Stable EventIds and global monotonic order unify live and stored
  events. State/history commit together before publication, including reconciliation
  and recovery turn changes. Only OutputProgress stays ephemeral. Byte-bounded
  completed output and safe metadata are retained indefinitely initially; raw
  prompts/protocol/reasoning and semantic memory remain excluded. Future pruning
  is explicit maintenance, not implicit summarization.
- Added authenticated forward/backward history pagination and snapshot watermark.
  Reconnect combines current snapshots, frozen history and live EventIds without
  an exactly-once promise. Historical output cannot roll newer projections back.
- Added `lain-client` with no Kernel/backend/SQLite dependency: HTTP/SSE only,
  environment URL/token, full control command surface and JSON output. The Ratatui
  0.30.2/Crossterm 0.29 TUI has list/detail views, current turn, recent/older history,
  visible offered approvals with scrollable details, UTF-8 prompt editing, scoped
  cancel/resume, resize and reconnect/resync. Terminal escape/bidi controls render
  as text; guards/panic hooks restore terminal modes. Its view window is capped
  at 1000 records, with CLI/API pagination for older history.

Validation: 26 deterministic Rust tests passed. Two native checks (Codex
no-inference persistence and terminal error/panic restoration) are ignored by
default and passed when explicitly run. Formatting and
Clippy with warnings denied passed. Tests cover idle/active/approval lifetimes,
lazy binding continuity, known-idle boot, active shutdown and idle loss, history
durability/order/isolation, state/history rollback without broadcast, UTF-8/output
bounds, whole-record redaction beyond truncation limits, API pagination/auth,
remote path preservation, offered approval decisions, fragmented SSE/reconnect,
snapshot ordering/dedup, deterministic resize/rendering and restoration commands.
No expensive CI or model automation was added.

Focused real Windows acceptance (`codex_smoke.py --run-real --phase3`) passed
unused-thread idle detach, CLI inspection without attachment, two short successful
model turns through lazy resume, stable provider identity, history surviving two
daemon restarts and Idle + Detached boot. Completed output/IDs/order were unchanged
after reboot and the daemon token was absent from captured output/logs.
Windows ConPTY acceptance with the deterministic subprocess passed attach without
worker launch, actual keyboard follow-up on detached work, resize, approval denial,
and alternate-screen restoration on exit. A native terminal guard test also
confirmed raw-mode restoration after an error return and a caught panic.
The helper dependency was installed only
in an ignored isolated test environment. No global worker config was changed.

Native Linux was not claimed: the available WSL Ubuntu has a C compiler but no
Rust toolchain. Unix signal/process-group and terminal acceptance remain host
validation work, and SIGKILL requires service-manager cgroup cleanup/terminal
reset. Other limits: exact Codex version gating and provider-owned rollout storage,
unknown/unbound submission outcomes, unknown/encoded secrets, pre-Phase-3 output
not backfilled, whole turn/approval snapshot write cost, bounded client response
and view sizes, indefinite total history growth until pruning, and loopback daemon
binding. Full profiles/SecretRefs, Web, additional backends, Lain memory/personality
and background cognition remain later phases.

## 2026-10-05 — Phase 4: Web control plane

Milestone: `window: "Present Day, Present Time"`. Phase 5 has not started.

- Added a fresh React 19 / TypeScript 5 / Vite 8 client in `web/`; no legacy
  frontend, Redux, WebSocket, browser orchestration or transcript database.
  Connection, list, New Work, detail, timeline, composer and approval cards use
  the same public HTTP/SSE API as the terminal. Backend model selection is now
  explicitly advertised for the already-supported session-options API.
- Chose explicit page-memory-only bearer entry and optional same-origin built
  assets from `LAIN_WEB_DIR`. All API routes retain authentication; public assets
  use first-party CSP, no-store, nosniff and no-referrer. Loopback binding remains.
  Refresh requires re-entry; only a public selected ID persists in the URL hash.
  ADR 002 records the interim choice and future TLS/session/Origin/CSRF boundary.
- Browser state is disposable. Fresh snapshots replace authoritative projections;
  paginated history merges only timeline entries by stable EventId/server order.
  Reconnect catches up from the last fully fetched history watermark, not the
  largest live event. Generation/abort guards reject stale history work. The
  recent 200-record view can page older within a 1000-record window.
- Logical state, runtime attachment and browser connectivity have separate labels.
  Unknown is not Failed; detached is healthy; cancellation acknowledgement is not
  confirmation. Creation/follow-up/cancel/reconcile and only actually offered
  approval decisions go through Kernel APIs. No concurrent prompt queue exists.
- Desktop sidebar/detail and phone list/detail layouts prioritize operational
  information. React text rendering, escaped terminal/bidi controls, bounded
  text/scroll containers and semantic focused controls protect worker output.

Validation: 28 deterministic Rust tests passed (2 optional checks ignored by
default), including new static/auth/config boundary tests. Clippy with warnings
denied passed. Ten frontend deterministic tests, TypeScript and production build
passed. Browser acceptance uses a private real daemon and deterministic worker:
creation, commands/files, model option, deny/approve-once, cancellation, two
observers, closure during active work, refresh, daemon restart, idle detach/lazy
follow-up, historical pagination, network loss/catch-up, Unknown/reconcile, safe
text and desktop/phone screenshots. Windows ConPTY and native terminal error/panic
guard regression passed. No real Codex/model call was made in Phase 4; no CI
workflow was added. Final audit: all three browser acceptance scenarios passed,
including independent unreachable/readiness/backend-availability UI states.
Formatting passed, and locked dependency metadata retains the Rust 1.89 floor.
Browser layouts were checked at desktop, laptop, tablet and phone widths without
horizontal overflow; desktop and phone screenshots were visually inspected.

The completion audit found a selection gap: a newly created or previously
unselected session could have durable activity after the initial stream snapshot.
Initial history now freezes the highest observed server order, retains live events
during selection, and advances coverage only after the frozen fetch completes.
Browser acceptance explicitly checks creation history and work while unselected.

The in-app Browser had no connected browser instance. Local Chromium/Playwright
provided opt-in acceptance and screenshots, which were visually inspected. An
existing user daemon locked the default Windows executable; validation used
`target/phase4` and private copies without stopping that service. Acceptance scripts
accept `LAIN_TEST_BIN_DIR` for this normal concurrent-development case.

Limitations: root-origin Web hosting, explicit credential re-entry, no public
Internet/TLS/cookie-session/CSRF design implemented, JavaScript safe-integer cursor
ceiling, and a bounded view rather than the full worker transcript. Native Linux
worker/terminal lifecycle remains unverified. Before Phase 5, settle profile
capability/resolved-view contracts and remote authentication/rotation/security if
needed. See `docs/WEB.md` for commands, recovery and actual deployment limits.

### 2026-10-05 — Web worker error explanations

A real Codex turn emitted retryable errors and subsequently completed, but the
Web parser discarded the Kernel's safe `worker error; backend will retry`
explanation. The timeline now preserves and displays normalized failure messages
for error observations, disconnects and turn failures; the latest turn's failure
also remains visible after snapshot reconstruction. Messages use the existing
bounded text renderer. Error observations do not alter session/turn state, and raw
provider diagnostics remain private. A focused frontend regression covers retry
semantics, snapshot failure parsing, malformed messages and safe text rendering.
Validation: all 11 frontend tests, TypeScript and the production build passed;
`git diff --check` passed. No Kernel or worker lifecycle change was required.

## 2026-10-05 — Phase 5: Runtime profiles, SecretRefs and provider switching

Milestone: `identity: "Who Are You?"`. Phase 6 has not started.

- Added the Kernel-owned SQLite profile registry and canonical workspace default/
  allowlist policy. Migration 004 is transactional, seeds `codex/local`, and adds
  revision-0 compatibility snapshots to existing Codex workers without changing
  public IDs, native thread/turn identities, approvals or history. Invalid legacy
  worker metadata rolls back the migration. No profile deletion cascades exist.
- Definitions start at revision 1; owner replacements require the expected
  revision. Disabled profiles reject new creation while existing work continues
  with its persisted snapshot. Selection uses explicit ID, workspace default,
  then Kernel default; policy denial is explicit with no fallback. Policy is
  checked on future prompts and attachment, including already attached workers.
- Non-secret resolved configuration is committed atomically with session creation
  in `worker_state.resolved_profile`: profile/revision, backend/driver, model,
  provider/endpoint, effort, sandbox, approvals and SecretRef. Defaults precede
  profile values and narrow model/sandbox overrides. Initial structured responses
  capture unspecified local model/effort. Registry edits never rewrite snapshots,
  restart workers or alter history. Managed official endpoints resolve explicitly.
- The first Secret Store permits only `secret://env/<lowercase-hyphen-name>` to
  `LAIN_SECRET_<UPPER_UNDERSCORE_NAME>`. No arbitrary env/argv public surface,
  credential persistence or plaintext CLI argument exists. Missing credentials
  fail before session creation. Values live in non-serializable zeroizing launch
  and runtime redaction buffers; each cleared/allowlisted child receives only its
  required `LAIN_PROFILE_CREDENTIAL`. Ordinary Codex tool environments exclude it.
- Verified the existing exact Codex 0.160.0 gate with installed help, regenerated
  structured schemas and native probes. Adapter-owned launch specs use process
  `-c` provider/model/effort settings and thread start/fork/resume model, provider,
  sandbox and owner approval settings. Managed providers speak Responses over
  HTTP. Global Codex config/authentication is never rewritten. Existing local
  authentication and unmanaged tool/home/endpoint settings remain externally owned.
- Profile inspection/validation reports safe local prerequisites and credential
  status, consuming no provider/model quota. Owner CRUD, workspace policy and
  frozen-session inspection APIs distinguish current registry and actual session
  revision. Invalid extractor values return safe JSON without echoing secrets.
  CLI adds profile list/show/validate and work start --profile. Web selection uses
  Kernel profiles, policy and advertised overrides; detail shows the frozen
  revision. TUI still renders old sessions and safe reconstructed turn failures.
- Worker failures now distinguish reliable structured authentication, rate limit,
  permission, provider connection/server and execution evidence, with unknown as
  fallback. Malformed known variants also remain unknown. Optional retryable
  decodes absent fields in old records; intermediate observations never terminate
  a turn. Raw provider prose is discarded. Redaction includes known daemon secrets
  and the runtime credential before commands, outputs, approvals, history and SSE.

Validation: 34 deterministic Rust tests passed, including Phase 4 migration and
rollback fixtures, new revision versus frozen sessions, disable/resume, concurrent
profiles with distinct endpoints/models/secrets/effort/approvals, redaction and
safe error reconstruction. Workspace formatting and all-target Clippy with
warnings denied passed. TypeScript, 12 frontend tests and production build passed;
all four private-daemon browser acceptance scenarios passed. Desktop and phone
screenshots were inspected. Native Windows ConPTY/terminal restoration passed.
The deterministic API/CLI smoke passed owner CRUD, safe errors, workspace policy,
detach/lazy resume, daemon restart and credential rotation under the same handle,
checking SQLite, CLI/API output and logs for fixture-secret leakage.

Two opt-in native Codex checks passed without inference: simultaneous App Server
config/read isolation and unused durable thread fork/resume. No second real
gateway credential was supplied, so paid two-provider/model access is unverified
and unclaimed. Ordinary tests consume zero model quota; no CI workflow was added.
Validation used `target/phase4` and private executable copies because an existing
user daemon can lock the default Windows binary; that daemon was not stopped.

Limits: one implemented worker family; namespaced environment secrets require a
daemon restart for rotation; configuration isolation is not an OS secret vault.
Local Codex auth/tool/endpoint defaults are not a full copied/frozen Codex home.
No arbitrary headers/auth commands, Chat Completions provider, live session
reconfiguration or automatic provider selection. Before Phase 6, decide the Lain
Actor's model/auth/budget boundary independently from worker profiles; retain
explicit owner-only profile mutation and no raw credentials in Actor/memory.
Remote browser authentication/TLS remains the separate ADR 002 decision.
ADR 003 records mutable definitions, frozen resolved snapshots and runtime refs.

## 2026-10-12 — Phase 6: CLI-first Actor implementation

Priority changed: Web is retained and frozen. Rust Kernel/CLI and Lain's own
capabilities now lead development. Phase 7 has not begun. This is implementation
progress, **not a completed cross-platform milestone**: native Windows acceptance
and real-provider acceptance are still outstanding. A final atmospheric milestone
such as `voice: "I Am Here"` should wait for the required evidence.

### Implemented

- `lain setup --local` creates per-user settings and a separate 256-bit OS-random
  token, exclusively and outside Git. A setup lock serializes repeated setup;
  existing credentials/settings are validated/reused, never silently overwritten.
  Unix validates owner/single-link ordinary files and 0600/0700; Windows uses
  native PowerShell/.NET current-SID owner-only DACLs, rejecting other access and
  reparse/linked files. Environment token overrides and explicit remote client
  mode remain available; local credentials are not forwarded to another URL.
- Foreground `lain up` launches the existing native laind beside the CLI or from
  LAIN_DAEMON_PATH. It supplies saved local token/listener/absolute database and
  waits for native console/process-group graceful shutdown. Existing authenticated
  compatible daemon returns already_running; mismatched auth/version or unrelated
  port occupancy fails without killing any process. No embedded Kernel/service.
- `doctor` reports CLI/PATH/executables, authenticated readiness, SQLite quick_check,
  backend availability/version, Runtime Profiles, missing SecretRefs and independent
  Actor setup, without model/provider calls. Explicit commands and remote HTTP/SSE
  remain compatible. Added watch, chat/ask, conversations/conversation inspection,
  actor show/cancel/approve/deny; JSON result/error semantics are documented.
- Kernel `actor/` separates domain, direct Responses model adapter and closed typed
  tool registry. Config is explicitly independent of Codex worker profiles/login;
  SecretRef resolves credentials only for the provider header. Persona is the
  versioned identity/persona.md with optional owner startup override. No coding
  tools, shell/file/HTTP/SQL access, worker auto-approval or profile-mutation tool.
- Migration 005 transactionally adds indexed Lain conversation and ActorTurn
  records, with distinct conversation/turn/message/action UUIDs, timestamps,
  request deduplication, revisioned states and tool provenance. Accepted user text
  commits before inference. Same conversation/request ID/text returns the original
  turn; changed payload conflicts. Restart marks unfinished execution Unknown and
  never replays inference/mutations. Pending proposals remain reviewable.
- A deterministic recent context window bounds history to 48 KiB/the latest 20
  turns, plus stable persona and typed tools. The runtime bounds model requests,
  tools/errors, active time, output/request/context/result/frame sizes and reported
  usage. Genuine provider SSE is consumed, but only complete redacted committed
  text/actions become public snapshots. No raw deltas, hidden reasoning or secrets
  in storage/history/logs. Mid-generation continuation is not claimed.
- Read tools reuse Kernel state; every mutating tool persists Proposed and requires
  explicit owner confirmation. A duplicate decision conflicts. Existing Supervisor
  validates workspace/profile policy and drives delegation. Real WorkSession ID
  commits as a partial receipt before prompt submission; acceptance never means
  worker completion. Partial/uncertain outcomes retain provenance and cannot be
  replayed under the same action. Owner denial performs no mutation.
- Documentation and ADR 004 describe commands, model/secret/persona boundaries,
  storage/context/confirmation, native acceptance and deferred work. Web source
  is untouched; existing Rust browser/API compatibility tests remain enabled.

### Validation evidence

Native macOS (Darwin, Rust 1.92.0), `zsh scripts/check-native.zsh`:
locked workspace build, formatting, all-target Clippy with warnings denied,
49 deterministic Rust tests passed; 3 existing opt-in tests remained ignored.
Native CLI smoke passed local permissions/reuse, startup/already-running,
auth mismatch/unrelated port, no-call doctor/PATH diagnostics, real local fixture
Responses streaming, typed reads/proposals, restart-preserved confirmation,
confirmed real fake-Codex WorkSession, session/history continuity, duplicate request,
secret-free SQLite/logs and native graceful shutdown/reopen. macOS PTY acceptance
also exercised chat resumption, exact-action owner confirmation, exit and Ctrl-C
at an input prompt without hanging on stdin.

The real-provider smoke entrypoint was verified to make no calls without its
explicit `--run-real` flag. No real Actor credential was supplied/configured for
acceptance, so paid Actor/Codex acceptance was **not run and is unverified**.
`native_smoke.py --platform windows` correctly refuses a macOS host; this is an
anti-emulation check, not Windows acceptance.

| Platform / acceptance | Result |
|---|---|
| Native macOS build/test/CLI/PTY/permissions/signals | Passed, deterministic fixtures |
| Native Windows PowerShell build/test/CLI/ACLs/Ctrl-Break | Unverified; no Windows host available |
| Native Linux server build/test/process lifecycle | Unverified; no Linux host used |
| Real Actor model + real Codex delegation/chat/restart | Unverified; explicit paid acceptance not run |
| Cross-platform Phase 6 Definition of Done | Pending native Windows and real acceptance evidence |

### Handoff / limits before Phase 7

Run `powershell -NoProfile -File scripts/check-native.ps1` on Windows; do not
substitute cross-compilation/emulation. Linux can run the locked Cargo checks and
native_smoke.py --platform linux. Run the opt-in actor_smoke.py only with deliberate
quota authorization and real model/Codex setup, then complete its documented live
chat, independent worker execution and daemon-restart checks. No automatic CI or
paid test was introduced.

Complete-record streaming instead of token-by-token display, no true mid-inference
resumption, environment-secret restart rotation, indefinite conversation retention,
latest-100 conversation discovery, exact UUID work references and foreground-only
launcher are current limits. Existing project databases need an intentional saved
local database-path selection; setup does not silently move/import sessions.
No semantic memory, MemoryCandidates, SessionSummary extraction, World, scheduler,
automatic routing or other Phase 7 work was started. No milestone commit was made.

### Local Codex refresh and additional macOS validation — 2026-10-12

- Updated the owner's standalone Codex from 0.154.0 to the adapter's verified
  0.160.0 using curl and the official OpenAI GitHub macOS ARM package, without npm.
  Verified the package SHA-256 against release metadata before installing and
  atomically switching the existing standalone current symlink. Retained 0.154.0.
- Re-ran `zsh scripts/check-native.zsh` on native macOS with Rust 1.92.0:
  locked build, formatting, warnings-denied Clippy, all 49 deterministic Rust
  tests and native CLI/PTY smoke passed. Three opt-in tests remained ignored.
- Explicitly ran `probe_profile_launch_configuration -- --ignored`: the real
  Codex 0.160.0 App Server passed the two-profile configuration isolation check
  without inference. A fresh authenticated Kernel reports this backend available.
- Ran `python3 scripts/codex_smoke.py --run-real --phase3` with the owner's
  existing ChatGPT login: passed unused-thread idle detach, two short real model
  replies, same-session lazy follow-up, durable history, retained provider identity
  and two daemon restart/recovery cycles. This is Worker acceptance only, not
  combined Actor/Worker acceptance. The first run exposed a Python 3.14 SSE reader
  close race after each successful turn; fixed the test helper to join its reader
  after socket shutdown and before closing the HTTP connection. Verified the fix
  with 30 local HTTP/SSE cycles and 30 real Kernel SSE/repeated-close cycles,
  without reader failures or additional model calls.
- An already-running daemon retains its startup-resolved old executable and
  backend health until restarted; updating the standalone symlink alone does not
  refresh that daemon. No owner daemon was interrupted by these private tests.
- Real Actor credentials/configuration remain absent, so real Actor chat and
  confirmed Actor-to-Codex delegation remain unverified. Native Windows and Linux
  also remain unverified; cross-platform Phase 6 acceptance is still pending.

### Owner-reported real Actor conversation — 2026-10-13

The owner supplied a native macOS `lain chat` transcript after restarting the
Kernel with explicit Actor configuration. Doctor reports configured, and the
conversation diagnostics identify the `openai-responses` driver and
`mimo-v2.6-flash` model with an available namespaced credential. Two ordinary
Chinese messages received real assistant responses. This establishes a manual,
owner-reported ordinary Actor conversation check; the startup "You're here."
greeting itself is static CLI text and is not inference evidence. No credential
or raw provider traffic was requested or recorded, and the coding agent did not
repeat these paid calls.

The transcript also exercises `/help`, `/status`, `/conversations` and the empty
initial `/history`. `/conversation` is unsupported; `/exit` must occupy the
whole input line. It does not yet demonstrate typed model tool calls, confirmed
Actor-to-Codex delegation, independent worker completion, or conversation recovery
after client/Kernel restart. Combined real acceptance and native Windows/Linux
acceptance remain pending.

The owner reports slow replies. The adapter requests and consumes provider SSE
but currently exposes only complete, redacted response records to the CLI;
provider token deltas are discarded. This explains the absence of incremental
display, not the provider's total latency: no timing measurements were supplied.
The transcript also contains a repeated `You>` prompt; its precise trigger has
not been reproduced. Incremental display and input prompt handling remain UX
follow-up work, not completed fixes.

### Actor streaming foundation checkpoint — 2026-10-13

- Added the small shared `lain-wire` crate with typed Actor IDs, states, tool
  calls, snapshots and ephemeral preview events. Durable Actor records are
  validated before writes and after reads; legacy complete message bodies with
  active turn metadata are normalized without changing identity or content.
- The Responses adapter now parses multiline SSE data and fragmented UTF-8,
  forwarding assistant text through bounded incremental redaction. Ambiguous
  credential suffixes are withheld, including Unicode case-fold equivalents of
  sensitive patterns. Tool argument deltas and reasoning remain private.
- Actor event streams retain snapshot-first revision semantics and add ordered
  preview identities/checkpoints. Previews are transient and may be discarded;
  validated complete responses remain the source of durable messages and tools.
  Added measured queue/provider/commit timings where available.
- Pre-commit checks found and fixed a flattened preview DTO deserialization
  failure, with a regression test covering every preview variant.
- Native macOS `zsh scripts/check-native.zsh` passed: locked workspace build,
  formatting, warnings-denied Clippy, 52 deterministic Rust tests and the native
  CLI/PTY fixture smoke. Three opt-in tests remained ignored. No paid inference
  was run, and no owner daemon or ignored Actor configuration was changed.

This is a foundation checkpoint, not Phase 6.5 acceptance. The CLI still renders
committed snapshots; incremental terminal rendering, end-to-end display timing,
the repeated `You>` reproduction/fix, reusable local Actor configuration and the
expanded real delegation/restart checks remain pending. Native Windows and Linux
are unverified. Real provider preview behavior and combined Actor/worker
acceptance are unverified. Web remains frozen and Phase 7 remains deferred.

### Incremental CLI and prompt regression — 2026-10-13

The CLI now decodes validated typed Actor snapshots/previews, displays sanitized
text during inference, and reconciles checkpoints/finals by identity and prefix.
Duplicate/stale preview sequences are ignored; gaps/discards/failures are marked
unconfirmed. Terminal controls are escaped. JSON ask keeps final-only stdout;
optional `--timing` diagnostics use stderr and contain measured durations only.

A native controlling-PTY reproduction established one concrete `You> You>`
trigger: canonical terminal CRLF became two newlines, and the background stdin
thread queued the empty second line, causing two adjacent prompts. It does not
prove the owner's original trigger was identical. Removed that stdin thread in
favor of one serialized, scoped raw line read with terminal restoration. LF
(Ctrl-J in Crossterm raw mode) and CR are handled explicitly, and paired LF is
consumed rather than printing another prompt or answering a later confirmation.
Empty ordinary input retains the current prompt; empty confirmation denies.

Native macOS locked build, fmt, warnings-denied Clippy and 59 deterministic Rust
tests passed, with three opt-in tests ignored. Extended native fixture/PTY checks
passed, including gated Chinese preview visible while the turn remains Running,
long/multiline output, final deduplication, no-preview completion, partial failure,
LF/CRLF/slash/consecutive inputs, JSON/timing separation, denial, existing approval,
Ctrl-C/EOF at input and confirmation, generation cancellation, exact terminal
restoration, persisted conversation reopen, same-generation preview checkpoint
reconnect and continued Kernel inference after client loss. Parser tests cover
multiline/byte fragmentation/private argument deltas/failure/incomplete/empty
output; slow-subscriber and contradictory durable-state tests were added.

No paid provider was called and no owner daemon/configuration was changed.
Native Windows/ConPTY and Linux remain unverified. Real provider incremental
output, typed reads, denial/confirmed delegation and combined recovery acceptance
remain pending. Local configuration reuse is the next implementation item.
See ADR 005 for the preview contract and the actual redaction limitations.

### Reusable local Actor files and startup diagnostics — 2026-10-13

Added `config actor set --file`, `config actor env-file --file`, `config actor
[show]` and `clear`, plus one-launch `up --env-file` and matching doctor selection.
Saved paths live in existing private per-user settings and are replaced atomically
under the setup lock. Selected files, including the owner's ignored Actor JSON,
are never copied, overwritten or migrated. Explicit process environment overrides
selected literal env-file values, then saved non-secret settings, then defaults.
Only Actor/persona paths and namespaced secrets are accepted; no shell execution,
interpolation, arbitrary-key injection or workspace/cwd discovery is introduced.
Credential files and parents require owner-only native permissions/ACLs outside Git.

Doctor compares validated non-secret Actor JSON/persona startup revisions and
reports reliable mismatches, unobservable old daemons or excluded remote mode.
Credentials are not hashed or compared; key rotation requires explicit restart.
`up` reports mismatches/open work instead of interrupting an existing process.
The documented workflow inspects active workers, stops the owner's foreground
launcher gracefully and starts it again with the same database. No hot reload,
guessed-PID killing or remote daemon control was added.

Native macOS `zsh scripts/check-native.zsh` passed locked build, fmt,
warnings-denied Clippy, 62 deterministic Rust tests and the extended native
CLI/PTY fixture smoke; three opt-in tests stayed ignored. New evidence includes
export-free saved configuration, protected credential sources, spaces in paths,
literal LF/CRLF parsing, precedence, rejected unsupported keys without secret
echo/settings changes, changed-config detection/no interruption, explicit env-file
selection, restart application and retained Actor/WorkSession IDs without model
regeneration. Public fixture streaming also checks character-split known-secret
withholding and no sensitive durable assistant message. Windows-specific ACL tests
are implemented but Windows/ConPTY and Linux remain unverified. No paid provider
was called and no owner daemon/configuration was changed. See ADR 006.

Remaining Phase 6.5 work includes the final wire-state audit, expanded opt-in real
provider/delegation/worker-terminal/restart acceptance and a conditional Phase 7
handoff. Real acceptance still requires explicit owner provider/workspace approval;
no Phase 6 completion or cross-platform acceptance is claimed.

### Typed durable messages and reviewable real acceptance — 2026-10-13

Separated complete message-body status from ActorTurn state and typed all message
constructors. New private message-format metadata rejects contradictory current
records; versionless Phase 6 bodies preserve identity/content while normalizing
legacy turn-status metadata. Recovery revision increments are checked. Provider
completion timing is captured on the completed SSE event, before final validation.

Added read-only `lain work turns`, compact flushed JSONL for machine work watch,
and shared genuine Unix/Windows signal registration for launcher/chat/watch.
Extended the existing opt-in Actor runner with Chinese conversation, measured
preview-before-completion, all five read tools with receipts, owner denial,
separate exact approval, duplicate-decision rejection, actual worker terminal
observation, macOS chat exit/reopen and read-only post-restart hash/ID recovery.
Safe evidence contains IDs/hashes/states/durations, not prose or credentials.
Fixture execution is explicitly labelled and does not establish real acceptance.

Native macOS `zsh scripts/check-native.zsh` passed locked build, formatting,
warnings-denied Clippy, 63 deterministic Rust tests and all CLI/PTY/native fixture
checks, including the expanded runner itself; three opt-in tests remain ignored.
Default real-runner invocation makes no model calls. Native Windows/ConPTY/Linux
and combined real-provider acceptance remain unverified. No paid calls or owner
daemon/configuration changes occurred. The acceptance record and conditional
Phase 7 inputs are in `docs/PHASE_6_5_ACCEPTANCE.md`; memory remains unimplemented.

Additional cancellation/partial-creation/history/worker-invariant probes found
in the workspace are being reviewed before real acceptance. This checkpoint is
not final Phase 6.5 acceptance and does not close those review items.

### Cancellation ownership and atomic delegation receipts — 2026-10-13

Reproduced four concrete defects from the workspace's independent architecture
probes before fixing them. A caller dropped while storage was queued could leave
Running/Executing Actor state without an execution owner. Backend startup could
fail after durable session creation but before the Actor learned its ID. Bounded
Actor history could retain the original page cursor after dropping tail events.
Worker recovery accepted confirmed cancellation on a Completed turn or a resolved
approval without a valid owner decision.

Submission and confirmation now transfer to Kernel-owned tasks before storage
awaits; supervisor creation also owns its handoff. WorkSession, worker metadata,
creation event and the exact confirmed Action's partial receipt commit atomically
before backend startup. A receipt-commit failure rolls creation back; startup
failure keeps its ID. Bounded history advances only through returned records and
marks additional pages; worker validation rejects the contradictory states before
recovery writes. The original untracked review probes were preserved unchanged;
new permanent regressions assert the corrected behavior.

Native macOS `zsh scripts/check-native.zsh` passed locked build, fmt,
warnings-denied Clippy, 69 deterministic Rust tests and full CLI/PTY/native
fixtures, including the opt-in runner against fake providers. Three opt-in Rust
tests remain ignored. No paid model calls, owner daemon restart or owner
configuration edits occurred. Native Windows/ConPTY and Linux remain unverified;
real provider/tool/delegation/restart acceptance remains pending explicit owner
provider/workspace authorization and an available protected credential source.

### Final Actor completion commit boundary — 2026-10-13

A final timing audit found a second-write failure window: the complete body could
commit Completed before its measured timing metadata, leaving false terminal
success if that final write failed. The body now commits while Running; terminal
state and metadata commit together before publication. Injected final-write
failure preserves the complete body with an Unknown turn rather than publishing
premature completion. Reconnection therefore cannot observe the intermediate
write as an authoritative completed turn.

Native macOS `zsh scripts/check-native.zsh` passed again after this change: locked
build, fmt, warnings-denied Clippy, 70 deterministic Rust tests and all native
CLI/PTY fixtures. Three opt-in tests remain ignored. No paid calls were made.
Real acceptance is awaiting the requested owner provider/workspace authorization
and protected env-file path; Windows/ConPTY and Linux remain unverified.

### Authorized native macOS Actor/Codex evidence — 2026-10-14

The owner explicitly authorized up to 36 Actor requests plus one read-only Codex
task in this repository, then separately approved the exact real work.create
Action. Added a tracked credential-file placeholder example; created the actual
owner file outside Git with 0600 permissions in the existing 0700 Lain directory.
The owner filled it. Saved its explicit selection and the existing ignored Actor
JSON without rewriting either file. No credential value entered Git, summaries,
ordinary logs or model prompts.

MiMo `mimo-v2.6-flash` provided actual Chinese incremental text while Running,
real work.list/profile.list/profile.inspect/work.inspect receipts, and a real
work.history receipt in an isolated conversation. A first attempt described a
second proposal without calling its function. The main-conversation history
request also omitted the tool call; one explicit follow-up ended Unknown without
an action and another still returned prose. Those failures remain recorded. A
clear separate-proposal request succeeded; the isolated history probe succeeded.
These observations establish protocol compatibility with intermittent model/tool
selection limits, not one uninterrupted successful run or universally reliable
natural-language delegation. No prose was counted as execution.

Denial left zero workers. The separately reviewed approval created WorkSession
`e51a67d9-18aa-49d3-9a04-21fb4be8f4ec`, worker turn
`7592fbce-1b41-4788-bcc8-63e3932874e2`, which reached Completed with no terminal
failure. Duplicate approval returned HTTP 409. Codex 0.160.0 recorded four
retryable provider-connection errors before recovering; some inspection commands
returned nonzero status, so successful turn completion does not mean every
command succeeded. No worker permission was auto-approved and no mutation was
replayed. Total actual quota use: 27 Actor provider requests and one Codex task.

Measured first useful CLI text was 785 ms / 6760 ms in two requests; provider
first text was 763 ms / 6734 ms and full CLI wall time 4775 ms / 16522 ms. Streaming
worked in both; the larger delay preceded the provider's first useful text.
Safe summaries contain only IDs/hashes/states/durations in the private per-user
acceptance directory, outside Git. The original partial attempt, main evidence
and isolated history probe are retained and cross-referenced without double
counting quota. The runner now supports a smaller per-run request budget,
explicit function prompts and supplemental read-snapshot recovery checks.

Actual PTY chat exit/reopen preserved real conversation records. After verifying
no running turns/pending approvals, the task-owned debug daemon stopped gracefully.
Both local Rust binaries were updated with locked Cargo installation; the
installed release launcher/daemon restarted with saved files and the same DB.
Main/partial/supplemental Actor snapshots and completed worker digests retained
identity/content. Recovery queries made no inference; the new daemon's Actor
model-call count remained zero. The installed daemon is left ready for local use.

Final `zsh scripts/check-native.zsh` passed locked build, formatting,
warnings-denied Clippy, 70 deterministic Rust tests and full native CLI/PTY/runner
fixtures. Three opt-in tests remained ignored. Budget-boundary checks consumed no
quota; supplemental-digest contradiction/recovery was tested with fixtures.
Windows/ConPTY and Linux remain unavailable/unverified; cross-platform Phase 6
acceptance is not claimed. Web remains frozen, Phase 7 is unstarted, and its
conditional handoff stays in `docs/PHASE_6_5_ACCEPTANCE.md`. Ordinary commits are
used while native platform acceptance is incomplete.

### Owner chat walkthrough and delegated-result delivery gap — 2026-10-14

After the implementation and authorized real acceptance recorded above (latest
pushed checkpoint `e883425`), the owner supplied a normal `lain chat` walkthrough
asking Lain to read and introduce a separate local project, `example-automation`.
This follow-up was explicitly scoped to explaining the observed behavior. No
runtime/CLI behavior was changed and no new implementation was authorized by
that discussion.

Observed and verified flow:

- Conversation `dc8294fc-bc24-4ddb-9be2-8ba65f068061` queried profile.list, then
  requested an explicit daemon-host workspace rather than inventing a path.
- A bare absolute path was rejected as an unknown slash command. Prefixing it
  with ordinary text passed it to the Actor. Source inspection confirmed the
  CLI's broad `starts_with('/')` fallback, which currently conflates path input
  and commands.
- The owner reviewed and confirmed an exact work.create proposal using
  `codex/local`, `read_only`, and a project-introduction task. The Kernel returned
  the actual WorkSession `db707cf0-17d3-4088-b3f4-8c91151d6132` and worker turn
  `997fe844-1490-4466-a316-5e5d807820db`.
- At initial inspection the worker was Running. Later read-only Kernel
  observation confirmed Completed and retrieved its actual completed report.
  The worker reported a Python CLI project overview and that it had neither
  modified files nor run the application. Its full report/project contents are
  not copied into this development log or promoted to semantic memory.
- No additional Actor inference, worker creation, permission approval, mutation
  replay or daemon restart was performed for this explanation.

The distinction behind the visible transcript is important: Action Executed
means the confirmed delegation operation ran; `stage: submitted` means a worker
turn was accepted; `worker_completion: not_observed` explicitly does not claim
that worker execution finished. The confirmation continues the same ActorTurn;
it does not itself require a new model turn. The CLI returns `You>` when that
ActorTurn finishes, while the independent worker can continue.

Current chat has no worker-terminal-result bridge back into the conversation.
Even after the worker completes, no automatic Actor follow-up or chat result is
produced. Asking Lain to inspect/summarize the result therefore normally creates
another ActorTurn/model request. Direct `work watch`, `work turns` and
`work history` read operational state without invoking the Actor. Retrieving or
summarizing a result is also distinct from verifying that the work is correct.

Three UX gaps were identified, not fixed:

1. Absolute paths can be misclassified as slash commands.
2. Human chat prints full JSON on successive tool-state changes, making the
   receipt noisy; exact mutation arguments must remain reviewable in any future
   presentation change.
3. A submitted task's completed result does not return to chat automatically.

The owner's feedback is that requiring a manual second message to obtain the
result is cumbersome. The desired follow-up experience is one request plus
explicit mutation confirmation, then automatic delivery of actual completion or
failure. Internal retrieval/summary may still need a separate bounded Actor turn;
the owner should not have to send another message just to trigger it. Event
ownership, durable correlation, reconnect/deduplication, output coordination and
any additional model-call budget remain design work, not an accepted or deployed
implementation. These are unchecked ROADMAP follow-ups. Existing confirmation,
worker independence and operational-history/semantic-memory boundaries still
apply. Phase 7 remains unstarted.

Validation for this recording: re-read the current CLI/Actor implementation,
checked the real WorkSession/turn and completed normalized report through read
APIs, and ran documentation diff/consistency checks. No code changed, so no new
build or model-based test was run. Earlier native/real acceptance evidence and
its platform/provider limitations remain as recorded above.

### 2026-10-14 — Future intent-aware subagent delegation proposal

Recorded an owner-requested **optional Delegation Compiler** for future Phase 10
orchestration in `docs/INTENT_AWARE_DELEGATION.md`. The goal is to improve handoffs
of vague user requests using authorized project context, confirmed preferences,
constraints and targeted clarifications, **without silently changing intent**.
Proposed modes: Off (default), ContextOnly, Clarify, Propose. The exact original
request stays separate from attributed contextual additions and inferred claims;
ordinary Lain chat and a worker's own planning are unaffected. Approvals, scope,
secrets and profile policy remain Kernel-owned. Updated Roadmap, Product,
Architecture, Memory and AGENTS.md guidance accordingly.

Documentation-only future proposal. No dispatch/runtime behavior, migrations,
model calls, implementation tests or Phase 7 work were introduced. Phase 6.x
completion/result delivery and native-platform acceptance remain the priority.

### 2026-10-14 — Clarified owner intent: faithful rewriting, fully transparent dispatch

The owner confirmed that the future compiler **may materially rewrite the task
instruction sent to a subagent**, provided it remains faithful to the user's
resolved intent. The owner must always know **whether the actual submitted text
was modified or unchanged**, and be able to inspect what was really sent. This
is not black-box optimization. Retain the verbatim original as a separate
historical record; track clarifications/resolved intent, final worker-bound text,
Changed/Unchanged classification and the persisted dispatch receipt. The
previous absolute-sounding prohibition of "rewriting" was corrected across the
design, Product, Architecture, Roadmap and AGENTS.md.

Mode names, Off default and any **additional** per-text-change approval flow
remain proposed interaction designs, not owner-finalized requirements. Existing
Kernel mutation/permission approvals continue. Documentation only: no runtime
or data migration, no inference or implementation tests, no Phase 7 start.

### 2026-10-18 — Phase 6.6 preparation checkpoint

The owner supplied the next goal, **The Thread Continues: Durable Delegation &
Result Delivery**. Its central requirement follows the chat walkthrough above:
one owner request and one explicit delegation confirmation must lead to automatic
delivery of the actual worker outcome in the originating conversation. The owner
should not need another message just to retrieve the result. Retrieving a report
still does not establish that its conclusions or every command/test succeeded.

Preparation completed so far:

- fetched origin and fast-forwarded local main from `0a30c1f` to `ee0c954`,
  retaining the existing untracked `docs/reviews/` material;
- read the supplied goal and the Product, Architecture, Domain Model and Runtime
  Model documents; re-read the updated repository instructions;
- recorded the scope and unchecked acceptance work in ROADMAP. Remaining required
  document/source inspection and the concrete design are still pending.

The known gap is between confirmed delegation and later worker-result delivery:
the Actor receipt identifies a submitted operation, while the independent worker
can finish after the original ActorTurn and its observation stream have ended.
`Action Executed`, a submitted worker Turn, worker completion and owner-visible
delivery are separate facts. The next implementation must establish durable
conversation/action/session/worker-Turn correlation and conversation-scoped
notices using authoritative normalized evidence. A completed ActorTurn must not
be reopened to inject a result. Exact schema, API and delivery/acknowledgement
semantics have not yet been selected.

Required follow-through includes persisted exact instructions/receipts; bounded,
sanitized useful result excerpts; restart/reconnect recovery, ordering and
deduplication; concurrent and later worker turns; honest failed/cancelled/unknown
outcomes; and no mutation replay or model call merely to announce completion.
The absolute-path parser, noisy human receipts and asynchronous terminal output
are part of the same scope. Exact approval arguments and machine JSON contracts
must remain inspectable, and notices must preserve typed input and raw-terminal
restoration. Optional model interpretation is separate from factual delivery.

No Phase 6.6 Rust code, migration, runtime configuration or installed binary has
changed at this checkpoint. No new provider calls, worker tasks or daemon restart
were performed. This recording makes no new build/test or acceptance claim;
previous Phase 6.5 evidence remains historical. Native Windows/ConPTY and Linux
acceptance remain unverified; Phase 6.6 native macOS checks are also pending.
New real-provider acceptance must be opt-in with workspace/action authorization;
the earlier completed acceptance budget is not reused. Web stays frozen and
Phase 7, Scheduler and Delegation Compiler remain deferred.

### 2026-10-18 — Durable delegation results reach the original conversation

Implemented Phase 6.6 against `ee0c954`, following the preparation checkpoint.
The Kernel now owns a durable link from conversation/ActorTurn/confirmed Action
to WorkSession and the specific worker Turn, exact submitted instruction,
workspace, profile and receipt. Migration 006 adds delegation and notice tables.
Work creation retains atomic session/receipt semantics before backend launch;
worker Turn allocation and its source link/partial receipt commit before prompt
RPC. Work.send correlates a new Turn in the same session. Exact confirmed task
text is forwarded without rewriting. Approval, submission, execution and display
state remain distinct; no second authoritative worker-state machine was added.

Store projects meaningful normalized worker transitions in the same transaction
as worker metadata/history. Completed, Failed, Cancelled and Unknown produce
separate immutable factual notices; pre-Turn partial startup retains the created
session with SubmissionUnknown. A lost prompt acknowledgement preserves its
Starting/Unknown Turn and blocks action replay. Repeated action/outcome events
deduplicate. Completion never reopens the ActorTurn or calls a model. Evidence
contains an available actual final report, nonzero-command counts/excerpts, safe
errors, changed paths and truncation with IDs/full-history guidance. Completed
does not mean every command/test passed. Excerpts are sanitized and bounded.

Authenticated conversation notice pages/SSE reuse existing Actor/worker broadcasts
as wakeups and SQLite cursors for durable catch-up. No event bus, Scheduler,
semantic Memory or model continuation was added. Notice creation survives death
after commit and before publication. Eight-page bounded SSE delivery disconnects
slow consumers after a ten-second enqueue timeout; reconnect reads committed
pages. Notice IDs/cursors remain stable across restart. Delivery is at least once:
CLI cursors suppress ordinary reconnect duplicates, and owner display acks are
persisted after output flush. A render-before-ack crash, failed or unqueued ack
can repeat a notice on reopening; exactly-once rendering is not claimed. Acks
change presentation metadata only and are shared by owner clients.

Chat now displays independent `Worker>` notices while retaining typed characters,
wide-character wrapping, CRLF behavior and guarded terminal restoration. Other
Actor previews continue across interleaved notices and reconcile once with final
messages. Renderer dedup/error/timing state resets between ActorTurns. Background
observer/ack tasks never write the terminal. Human tool receipts are compact;
full exact proposed arguments still appear before confirmation, while existing
machine Actor JSON and complete receipts remain inspectable. Added quota-free
`lain notices <conversation-id>` and `lain delegations <conversation-id>`.
Multi-component Unix paths, drive/UNC paths and spaces pass through as ordinary
text. Ambiguous one-component slash words retain command feedback; use a trailing
slash or `path: /name` for such a directory. No daemon-host path is resolved by
the client. Existing Phase 6.5 conversations remain readable, without guessing
historical receipts into new delegation links.

Validation: `zsh scripts/check-native.zsh` passed locked workspace build, fmt,
warnings-denied Clippy, **83 Rust tests and 3 explicitly ignored opt-in tests**,
and full native macOS CLI/controlling-PTY fixtures, consuming zero provider quota.
Coverage includes confirmed/denied/duplicate dispatch, same-session follow-ups,
concurrent/out-of-order completion and conversation isolation, command failure in
a Completed Turn, rejection/crash/cancellation/lost acknowledgement, bounded
redaction, transaction rollback, duplicate projection, commit-before-publication
recovery, cursor replay/ack loss, slow consumers and older database migration.
Native PTYs demonstrated automatic delivery without another owner message or
model call, partially typed wrapped Chinese input retained exactly, frontend exit
before completion, pending notice replay, acknowledged reopen, and stable notices
after actual daemon restart. The PTY fixture now drains output while waiting for
exit, avoiding a harness backpressure hang when a notice arrives after `You>`.

Updated architecture/domain/Actor/CLI/configuration/README, ROADMAP and
[ADR 007](docs/adr/007-durable-delegation-result-delivery.md). The new
[acceptance matrix and opt-in real procedure](docs/PHASE_6_6_ACCEPTANCE.md) separate
this deterministic evidence from historical Phase 6.5 real runs. Native
Windows/ConPTY and Linux remain unavailable/unverified. No new paid model calls,
real Codex work, owner daemon restart, installed binary/configuration changes or
owner database migration were performed. The existing untracked review material
was retained. No Web/Memory/World/Scheduler/compiler work was introduced. Further
native acceptance and any newly authorized real-provider result-delivery check
are the next validation steps; Phase 7 remains deferred.

### 2026-10-19 — Independent owner-reviewed memory and project knowledge

Fetched `origin/main` before editing: latest HEAD was still `17a676d`. The owner's
Phase 7.1 instruction explicitly authorized deterministic memory implementation
while preserving pending Phase 6.6 real-provider/native Windows/Linux acceptance.
The existing untracked `docs/reviews/` material was left untouched. No subagents,
real model calls, actual Codex work, installed binary changes, owner daemon restart
or owner database migration were performed.

Reused the Kernel mutex/Store, SQLite migrations, authenticated HTTP boundary,
persisted Actor action confirmation, typed wire crate, CLI and Phase 6.6 result
projection. No new crate, persistence architecture, vector database, worker harness
or generic retrieval framework. Lain's independent personal continuity and her
agent-control role remain equally important; persona stays separately versioned.

New typed IDs/records distinguish Memory, Candidate and SessionSummary. Episodic,
ProjectKnowledge, Preference and Reflection have explicit scope and lifecycle.
Direct exact owner text gets Explicit provenance; Actor suggestions retain
LainGenerated/Inferred conversation/turn/action provenance after confirmation;
summary selections retain SummaryId/UnverifiedSelection. Kind/scope and
source/authority contradictions are rejected. Acceptance never claims independent
technical verification. Known credentials/patterns and terminal controls are
rejected before persistence; unknown/encoded secrets retain existing detection
limits. No raw transcript is automatically promoted.

Migration 007 adds indexed bounded memory snapshots, transactional FTS5 trigram
search, request tombstones, independent review candidates and factual summaries.
Literal search supports Unicode/CJK substrings and short-query fallback; one
explicit scope per page, increasing sequence order, bounded records/bytes and
cursors over retained rows. Repeated request IDs deduplicate or conflict on changed
canonical drafts. Expected revisions protect enable/disable/supersede/delete.
Deletion removes semantic/FTS data and scrubs candidate copies; independent Actor,
worker, notice and summary audit remains, with no secure-erasure claim. Unix
storage creation now uses 0600 database/lock files and 0700 new directories;
Windows uses the existing protected per-user directory/owner-only inherited ACL
boundary and still needs native verification.

Actor search/list/inspect returns explicit bounded evidence without whole-store
prompt injection; proposals atomically persist with Proposed actions. Existing
owner decision is the sole review gate. Denial/cancellation rejects; approval
atomically accepts memory/candidate without inference. Generic candidate review
cannot bypass an Actor action. An Unknown receipt after a committed acceptance
requires inspection, never automatic replay. Native CLI supports memory CRUD,
status/supersession, candidates/review/summary selection and factual summary
create/show/list, including stable JSON and typed IDs, without browser/LLM access.

SessionSummary explicitly freezes one worker Turn's normalized evidence through a
history watermark, source update time and current outcome. Reuses sanitized
Phase 6.6 evidence limits, preserves failed commands under a Completed Turn and
labels worker reports unverified. A summary is never automatically Memory or a
Candidate; selected project knowledge needs a separate owner review in the same
workspace. Phase 6.6 result notices/receipts remain independent.

Validation results follow in the acceptance record below. World, journals,
automatic extraction/personality evolution, reflection, consolidation, Scheduler,
new backends, browser changes and Delegation Compiler remain unimplemented.
Suggested Phase 7.2 is a small owner-inspectable World/notes foundation with
explicit evidence links, requiring new scope; no automatic phase advance.

Phase 7.1 validation: `zsh scripts/check-native.zsh` passed on native macOS with
`cargo build --locked --workspace`, `cargo fmt --all --check`,
`cargo clippy --locked --workspace --all-targets -- -D warnings`,
`cargo test --locked --workspace`: **96 passed, 0 failed, 3 opt-in tests ignored**,
plus full native CLI/controlling-PTY smoke. The extension exercised authenticated
memory add/idempotent retry/list/search/show, project isolation, actual fixture
daemon restart then Responses-adapter retrieval using the saved MemoryId/content,
Actor candidate denial/approval, refusal of generic action bypass, status changes,
deletion and independent summary creation/selection/acceptance. Existing Phase
6.6 PTY notice delivery, wrapped Unicode input, acknowledgement and restart
continuity still pass. All inference/work used local fixtures; provider quota was
zero. Targeted review added corrupted authority/index rejection, candidate lifecycle
validation, rollback, byte-bounded pagination and deletion of now-sensitive content.

[Phase 7.1 acceptance](docs/PHASE_7_1_ACCEPTANCE.md) maps requirements to tests and
records open obligations. Native Windows/PowerShell/ACL/ConPTY and Linux hosts were
unavailable and remain unverified. No real-provider Phase 7.1 memory conversation
was run, so that definition-of-done acceptance item is still open pending explicit
provider/budget authorization. Phase 6.6 real-provider delivery also remains
pending. Deterministic implementation completion does not imply final cross-platform
or real-provider acceptance. Ordinary reviewable commits are used; no atmospheric
milestone/Phase 7.2/Phase 8 completion is claimed.


Phase 7.1 real-provider follow-up (separately owner-authorized): one Actor turn,
maximum four model requests, current `mimo-v2.6-flash` provider, non-secret Global
preference in a task-owned temporary database/daemon, with no Codex work or changes
to the owner's existing instance/database. Direct creation and exact record equality
after a native test-daemon restart passed. The Actor CLI returned nonzero; this
is **not** a successful real retrieval acceptance. The temporary wrapper wrongly
raised before retaining the CLI failed-turn snapshot. Its generic RuntimeError
alone cannot establish the actual model-request count, terminal state or tool
receipts. The test daemon stopped, temporary database was removed, and no second
real attempt was automatically made. Provider-specific cause remains unknown.

Safe private evidence is `acceptance/phase-7-1-memory-2026-10-19-7748512f.json` under
the per-user Lain directory, with model/scope/limits, successful persistence checks,
MemoryId/content digest and generic failure only. The wrapper was fixed and tested
without inference to retain structured failed-turn snapshots/counters even on a
nonzero CLI exit. A further real turn requires additional authorization because
unused allowance from the first run cannot be proven. The 96-test native macOS
deterministic result remains valid; only this temporary acceptance script changed.


A separately authorized additional real turn used the corrected wrapper under the
same isolated scope. It **did not pass**: `mimo-v2.6-flash` made **2 requests**,
reported **4,838 usage tokens**, and ended Unknown with no executed memory-tool
receipt and no actual MemoryId citation. Native save/restart/exact-record equality
passed; no WorkSession was created; inspection retained an identical Actor snapshot;
the task-owned daemon stopped. The owner instance/database and Codex remained
untouched. Private safe evidence:
`acceptance/phase-7-1-memory-2026-10-19-aa23914a.json`, ActorTurn
`df2e27eb-3c97-4f10-9c76-0001214c3a37`, Conversation
`cc04e3c9-3293-42b1-a406-e82952ad7379`, Memory
`ca698129-487f-4bc5-8620-b44f7d7a3e12`. The provider-specific cause is unverified;
no rejected-call text or raw provider frame was preserved. The first run's request
count is still unknown, at most four; the two-run upper bound is six, with usage
known only for the second. No third run was made or automatically authorized.

Phase 7.1 implementation and native macOS deterministic acceptance remain complete,
but **final Phase 7.1 acceptance remains incomplete** because a real Actor retrieval
conversation has not passed. Native Windows/Linux and Phase 6.6 real-provider
acceptance also remain pending. Do not advance phases or use a milestone completion
commit to conceal these gaps. Further provider/tool-call diagnosis should be based
on safe evidence and any new paid probe requires its own concrete authorization.

## 2026-10-23 — QQ-first cloud residency and equal companionship: documentation

The user revisited the product boundary in detail and explicitly prioritized
daily QQ companionship. Lain should have one cloud identity with shared memory
and real experiences, interact as an equal, read selected social sources,
autonomously remember and privately share within agreed limits, and connect
local PCs for Worker execution and history. The user rejected master/servant
framing. Worker archives should include independently created sessions; archive
access does not itself authorize control. Faithful reuse of successful report
methods and bounded follow-up supervision remain product directions, with exact
interaction and resource defaults unresolved.

Rewrote PRODUCT.md as the detailed Chinese product contract, explicitly separating
confirmed decisions, provisional directions, proposals, current implementation and
open questions. Added RESIDENT_DESIGN.md and CHAT_DESIGN.md for cloud/device state
ownership, dual-account QQ, scoped authorization, provenance, correction, outgoing
delivery uncertainty, conversational timing and acceptance scenarios.
MAIBOT_REFERENCE.md records the user-selected upstream reference, observed
documentation/source mechanisms, Lain-specific adaptations and unverified limits.
The research read public main/online pages without a pinned SHA; no upstream
runtime or paid model evaluation was performed.

ADR 009 records the replacement of CLI-first ordering and universal per-item
confirmation as future product policy. ADRs 004/008 retain their historical and
current-code meanings with explicit supersession notices. Updated README,
AGENTS, ROADMAP, architecture/domain/runtime/memory and related operating/design
guides so future development sees the new direction before older phase plans.
The roadmap proposes Q1–Q3 for QQ interaction, selected observations/continuity
and bounded daily activity, followed by connected devices and terminal work.
The stages are proposals, not implementation completion or automatic authority.

This task changes documentation only. No Rust, frontend, SQL migration, compiled
persona, runtime configuration, installed binary or user database changed.
Existing per-action confirmation and Phase 7.1 candidate review still apply in
code until a separately scoped grant model and migration replace them. The
compiled persona's historical owner language is explicitly recorded as pending
behavioral work. No QQ account login, real chat collection, external messages,
production deployment or new paid inference occurred. Native Windows/Linux and
Phase 6.6/7.1 real-provider gaps remain open.

Validation: checked all 21 changed/new Markdown documents, 124 relative file
links and balanced code fences; no broken relative file links or fence imbalance
were found. `git diff --check` passed after whitespace normalization. Reviewed
status/supersession wording and confirmed no runtime/configuration/source or
acceptance-record files changed. Rust/frontend tests were not run because this
change is documentation-only.

## 2026-10-23 — Q1 decoupled private conversation implementation

Implemented the selected QQ reply-only slice on the existing Rust Kernel and Actor.
The optional OneBot 11 forward-WebSocket adapter verifies the logged-in Lain
account and accepts only the bound user's friend-private text. Group, self,
wrong-account/peer and unsupported mixed-media events cannot become Actor input.
Native IDs and protocol objects remain in the adapter; the shared conversation
service has a fixture transport and no Utakera runtime/database dependency.
Read Utakera's Bridge/source drafts at `8e30fd2` as interoperability proposals,
without adopting them as an approved source schema or implementing an observer.

Schema 008 adds durable bindings, incoming messages, bounded batches and delivery
intents/receipts. A stable batch request recovers exactly the committed Actor turn.
Queued input survives generation; send results distinguish platform confirmation,
definite non-submission/rejection and uncertainty. Interrupted claimed sends are
never automatically replayed. Fixed merge, backlog, lifetime, write/receipt timeout
and send-spacing limits bound the live path without idle model polling.

Reply permission has a persisted revision and compare-and-swap administrator API.
Revocation invalidates old queued/generated work and does not silently reset on
restart. Authorization and expiry are checked again at Actor admission and send.
Existing tool mutation, candidate review and Worker permission checks remain.
QQ “同意” is not an approval; proposals direct the user to the authenticated local
review flow. Compiled persona v2 uses equal companionship, calm/offbeat expression
and truthful experience claims. The same Actor serves the terminal and channel.

Added configuration example, operation/deployment/recovery guide and a requirement
to evidence map in `docs/Q1_IMPLEMENTATION.md`. Updated current product, architecture,
domain, runtime, configuration, Actor and roadmap descriptions; older acceptance
records retain their historical meaning. No real QQ/model/deployment acceptance,
Q2/Q3 work or personal data migration is included. The user subsequently requested
committing and pushing this implementation, with further testing handled separately.

Validation is recorded in the Q1 acceptance document. The full native Windows
workspace run passed 111 tests with zero failures and three manual/live tests
ignored; format, strict workspace Clippy and workspace build also passed.
Failure testing found and fixed a shutdown
admission race and bounded previously unbounded socket writes. Migration assertions
were updated for schema 008. The memory boundary test correctly checks that a
candidate is not active memory; proposal persistence itself is existing behavior.
Small lint fixes included an equivalent Windows input match guard. Native CLI
testing also fixed inherited PowerShell 7 module paths breaking the Windows
PowerShell ACL helper and ownership of exclusively created paths under elevated
tokens. Existing directories remain check-only and are never taken over.
No CI workflow or remote model automation was added.

Subsequent protocol review corrected outgoing OneBot `user_id` to a JSON number
and updated its fixture assertion. At the user's request, no build or test was
rerun after that correction. The pushed revision therefore retains a final
verification gap despite the preceding successful full run.

The initial portable LLVM-MinGW build passed Kernel tests but its CLI executable
failed before test entry with native access violation `0xc0000005`. A separate
checksum-verified GCC 16.2.0 / MinGW-w64 14.0.0 MSVCRT toolchain completed the full
run with `RUSTFLAGS=-C link-self-contained=no`. Tools, build output and test logs stay in the task-owned external
directory `%TEMP%/lain-q1-tools/`; no system PATH or
user runtime configuration was changed. Native MSVC, Linux/macOS, interactive
terminal acceptance and existing Phase 6.6/7.1 real-provider gaps remain separate.

## 2026-10-24 — Q1 final-revision recheck and Windows fixture repairs

Rechecked the Rust tree at `560ddf9`, including the final outgoing numeric
OneBot `user_id` correction, on native x86_64 Windows/MSVC with Rust/Cargo 1.98.1.
Format, strict all-target workspace Clippy, locked workspace tests and workspace
build passed: **111 tests passed, zero failed, three opt-in tests ignored**.
The initial crates.io timeout was resolved using the existing Windows proxy only
for the Cargo retry; tests/build then ran offline. No lockfile, global toolchain,
proxy configuration, personal database or running QQ process was changed.

The additional native CLI smoke exposed fixture portability problems: inherited
PowerShell 7 modules, Windows canonical-path string comparison, a speed-dependent
streaming probe, a stale schema-7 assertion, and a SQLite inspection connection
left open during Windows temporary-directory cleanup. The fixture now uses
Windows PowerShell's bundled modules, file identity, an actual Running-snapshot
gate, schema 8 and an explicitly closed connection. A focused Windows check of
the actual SQLite inspection block passed immediate file deletion. The Windows
CLI subprocess bound is 90 seconds because repeated native ACL checks exceeded
the previous 30-second setup bound; other hosts retain 30 seconds.
The Windows launcher-readiness bound likewise rises from 20 to 90 seconds after
an env-file startup exceeded it; other hosts retain 20 seconds. This affects only
the fixture's waiting budget, not production startup, inference or shutdown policy.
Production Rust, channel and authorization behavior is unchanged.

The final `python -X utf8 scripts/native_smoke.py --platform windows` run exited
0/PASS, including setup/ACLs, signals/restart, doctor, streamed Actor, confirmation,
fake Worker, scoped memory/summary/Actor retrieval and persisted recovery, with
successful temporary-state cleanup and zero model quota. The embedded runner's
native interactive/ConPTY and real-provider acceptance flags remain incomplete;
this is deterministic native evidence only.

Added a Windows hands-on live acceptance walkthrough to `docs/QQ_CHANNEL.md`:
fresh owner-only local setup on port 18787, explicit Actor/NapCat files, hidden
credential entry, forward WS settings, receipt inspection, restart/reconnect and
revocation/re-grant. Its four PowerShell blocks passed native parser checks.
Official NapCat 4.18.41 Shell and Windows OneKey archives were prepared separately
outside the checkout, with hashes matched to GitHub release asset digests; they
were not launched or logged in by the coding agent. Real QQ/model/deployment,
ConPTY interaction, other native hosts and existing real-provider memory/result
delivery gaps remain unaccepted. See `docs/Q1_IMPLEMENTATION.md` for evidence.

## 2026-10-24–25 — Q1 diagnostics and Q2-A private-memory boundary

Recorded the user's manually confirmed real Chinese private QQ exchanges as
ordinary-chat evidence only. Full live reliability, memory experience and cloud
deployment, and historical Phase 6.6/7.1 real-provider failures remain open.

Added ephemeral Kernel-owned connection phase/failure observations to authenticated
channel API/CLI. Account verification gates Online; restart never restores an old
Online flag, and shutdown/task exit closes observations. Failures use safe closed
categories and repeated identical failures suppress duplicate warnings. Receive,
reply grants, send receipts and Unknown/no-replay rules remain unchanged.

Q2-A adds an explicit administrator-only, default-off, revisioned private-binding
memory grant with persistence, revocation and optional expiry. New authorization
captures an input sequence watermark; queued pre-grant sources cannot backfill.
Existing Actor turns gain bounded remember/recall/correct/forget capabilities only
while the grant is current, retaining all existing model/tool/time/token budgets.
No extraction loop, observer, scheduler or general Worker approval is introduced.

Schema 009 reuses Memory/FTS and adds grant revisions, effect receipts, source
suppression and inspected-target revisions. Autonomous selections retain actual
conversation/input/batch/Actor/action and grant provenance, Inferred authority and
no fabricated individual confirmation time. Old snapshots and confirmations keep
their exact meaning. Each request refreshes bounded relevant Active memory; target
correction/forget requires inspection and revision. Semantic effects, relations,
tombstones and receipts commit transactionally, with idempotent action IDs.
Dialogue, superseded versions, audit and backups remain separate from forgetting.

Added deterministic transport/API/CLI, grant/race/scope/expiry/migration/rollback
and actual MemoryId retrieval tests. The independent-memory Actor fixture removes
the original source from recent dialogue using temporary Kernel-only filler turns,
reopens storage and proves the selected record reached model context before
correction/forgetting. Design contracts, ADR 010, roadmap, operating paths and
staged manual acceptance are updated; see `docs/Q2_A_ACCEPTANCE.md` for final
commands/results and live gates.

All development uses isolated `target/q2a` outputs and temporary data. MSVC debug
symbols exhausted the available F: space; only that task-owned build cache was
cleaned, then dev/test symbols and incremental output were disabled for the local
validation commands. No running QQ executable, service, personal database, private
credential or real memory grant was changed. No paid call or real QQ send was made.

Final native Windows validation on 2026-10-25 passed format, strict all-target
workspace Clippy and workspace build. The final sequential workspace test run
passed **126 tests, zero failed, three manual tests ignored** across 15 suites.
The parallel Kernel executable passed
45 tests with two manual probes ignored. The native CLI smoke passed exit 0 with
clean temporary-state removal, including ACLs, process restart, fake-model/Worker
flows and existing memory/candidate/summary retrieval; no paid quota was used.
Windows PowerShell parsed all nine documented operating blocks without executing
them. A warning-capture fixture was isolated by unique ChannelId; a Cargo/native
smoke overlap was corrected after Windows locked the active test exe. Real
QQ/provider memory, ConPTY and native macOS/Linux remain unverified.

## 2026-10-25 — Q2-A process-level acceptance closeout

Reviewed current Q1/Q2-A transport/service/Kernel/Actor/persistence/API/CLI paths
against ADR 010 and the acceptance requirements. No additional product defect was
reproduced within that scope; this task makes no Rust product behavior, schema,
milestone or deployment change. Existing uncommitted work and historical failed
real-provider evidence remain intact; HEAD is still `560ddf9`.

Added the focused `scripts/qq_memory_smoke.py` entry point rather than extending
the broad older CLI/Worker smoke with a new optional dependency. It uses the
already-installed Python 3.13.5 / websockets 15.0.1, ephemeral loopback fake OneBot
and Responses servers, actual native laind/lain and the existing fake Codex process.
An owner-private temporary instance and generated credentials are independent of
all personal state. No package/framework, product dependency or CI was added.

The 12 normal checks cover identity-gated Online/reconnect, management auth,
default-off candidate review, sensitive-input rejection, source-linked MemoryId,
21 real fake-protocol exchanges evicting original dialogue, restart and exact
record retrieval in actual model HTTP input, correction/forget refresh and later
empty recall, old-source replay, in-flight revocation/expiry, unchanged Work review
and an actual pending fake Worker permission, and persisted receipts/tombstones/
semantic/FTS deletion/secret redaction. The provider uses actual source/record IDs
and revisions from request data. No memory is pre-seeded in SQLite.

All 12 passed, with 42 localhost fake model requests and zero paid calls. Separate
startup and provider-failure injections each returned expected exit 1 and cleaned
the daemon, fake services, background threads and temporary state. Operations and
cleanup are bounded; failure output reports a safe stage/assertion, not credentials
or provider bodies. The local ignored report is
`target/q2a/qq-memory-closeout.json`, including tested binary SHA-256 hashes.

The current native Windows rerun passed format, strict workspace/all-target Clippy,
workspace tests (**126 passed, zero failed, three manual tests ignored**, 15 suites,
46.98 seconds), workspace build and Python compilation. Cargo steps and process
acceptance ran sequentially. The previous broad native smoke remains baseline
evidence; it was not repeated without a relevant change. The operating guide now
has five manual upgrade/test stages, including consistent stopped-state backup,
schema-9 rollback requirements and explicit short-lived memory authorization.

This goal added the script and updated DEVLOG/Q2-A evidence only. No commit/push,
personal credentials/database access, real QQ/model call, grant enablement, service
restart or deployment occurred. Real memory quality, live reliability, ConPTY and
native macOS/Linux remain unverified.

## 2026-10-25 — Q2-B1: bounded private-history source lookup

Implemented the separately selected source-read slice on top of Q1 and Q2-A.
A default-off administrative SourceGrant fixes one private binding, a half-open
received-UTC window (at most 31 days), captured incoming high-water, CAS revision
and optional expiry. Reply/memory grants do not imply this permission. No future
collection, Worker approval, automatic memory restoration, experience summary,
proactive contact or deployment was added.

SourceRecord projects existing Incoming/Batch/ActorTurn/Delivery. Migration 010
adds decision/read ledgers and a receive-window index without copying raw chat
or changing old confirmation meaning. Search seeks the receipt-time/sequence
index directly, processes at most 512 candidate rows and returns at most 10 admin
or 3 Actor records / 8 KiB. Cursor scope, time bounds, ID route, high-water,
truncation, partial coverage, generation and actual delivery/Unknown stay explicit.
Every original input/reply is checked using current known credential filters
before excerpting, including opaque configured credentials crossing clip bounds.

The two typed readonly Actor capabilities use IDs-only receipts. All prior source
selections are freshly projected/fingerprinted before each further request and
after responses; revoke/narrow/expiry/suppression stops stale text/call dispatch.
Source-influenced streaming previews are held. Later model history excludes
assistant/action observations from source-reading turns while retaining original
user requests and operational snapshots. Q2-A suppressed inputs are excluded;
merged out-of-window inputs, changed/deleted inspected memory and source-derived
replies withhold their bodies. Audits say prepared, not comprehended/received.
Opaque provider continuation is retained only while every selection remains valid.
Existing request/tool/token/wall budgets and Work/Worker confirmations remain.

Added authenticated source grant/history/search/inspect/read-audit API and CLI,
ADR 011, the design/domain/runtime/CLI/QQ contracts and Q2_B1_ACCEPTANCE.md with
requirements-to-test mapping and five separately authorized manual upgrade/test
steps. Schema 010 rollback needs a consistent stopped pre-upgrade backup. Earlier
schema-9/Q2-A counts and real-provider failure records remain historical evidence.

Native Windows/MSVC verification passed format, strict all-target workspace
Clippy, offline locked workspace tests (138 passed, zero failed, three manual
ignored, 15 suites), and workspace build in target/q2a. New fixtures cover default
off/CAS/persistence, binding/time/ID/cursors, high-water with clock rollback,
index seek without temporary sort, count/byte/UTF-8 limits, suppression,
credential filtering, truthful generation/confirmed/Unknown sends, merged input
scope, stale inspected memory, atomic audit failure, no migration backfill,
actual IDs after eviction/restart, in-flight revoke/narrow/expiry/disable/forget,
preview privacy, source-derived history isolation and unchanged Work review.

Extended the existing qq_memory_smoke.py rather than adding a test framework.
All 18 actual process/HTTP/WS checks passed with 53 local fake model requests,
zero paid calls and complete cleanup. Source IDs come from real query results,
with original test inputs delivered over fake OneBot, not inserted into SQLite.
The normal provider timeout test requires Unknown, two attempted requests, no
further call or cached quote and successful restart. Startup/model/source failure
injections each returned expected exit 1 and cleaned services/temporary state.
The first timeout fixture wrongly expected Failed and did not handle Windows
connection abort; fixed the fixture, preserving the failed log without changing
Actor lifecycle semantics to satisfy it.

The initial broad native CLI regression was externally interrupted at six minutes
while still making progress through native configuration/ACL checks. Its owned
launcher/daemon were gracefully stopped via their console group and only that
verified temporary fixture was removed. Its safe daemon log is retained. The
full rerun passed in 496.34 seconds with native setup/reuse/ACLs, launcher/doctor/
shutdown/restart, streamed Actor, reviewed fake Worker delegation, independent
scoped memory/summary retrieval and restart persistence. Native terminal/ConPTY
and real-provider acceptance remain unverified.

Ignored evidence is under target/q2a/q2b1-*: gates/logs, successful process JSON
with binary SHA-256, failure results, and the earlier timeout assertion/wrapper
logs. Existing 45 uncommitted paths were preserved; the combined worktree now has
56 paths (40 modified, 16 untracked), including eight Q2-B1 files and three extra
tracked files touched for source preview/SSE snapshot sizing. HEAD remains
560ddf9. No commit/push, paid model, real QQ, personal credentials/database access,
F:\Lain-Q1-Acceptance migration/restart or real grant enablement occurred. Real
provider/QQ experience, ConPTY and native macOS/Linux for this slice remain
unverified. This stage stops before Q2 experiences or Q3.

## 2026-10-25 — Select C1 natural private conversation as next priority

The user clarified that the desired outcome is MaiBot-like lifelike conversation
and explicitly requires learning from its chat design. Recorded C1 as the next
selected priority in PRODUCT, ROADMAP and CHAT_DESIGN: natural everyday expression,
message/turn timing, graceful closure and naturally used correctable memory.
Q1/Q2-A/Q2-B1 infrastructure remains implemented, not proof of naturalness.

Rechecked the current local persona, channel dispatch and merge bound, and the
upstream official Bot configuration document. The current private channel still
submits merged input to Actor; full participation/wait/no-reply and draft
reconsideration remain unimplemented. Online upstream documentation is recorded
separately from historical source notes; Git remote lookup failed with connection
reset, so no fixed-source review or code reuse is claimed. Fixed-revision source
mapping remains a C1 implementation prerequisite. No new runtime behavior, goal,
model/QQ call, deployment or commit was made; this is a product-priority/doc change.

## 2026-10-25 — C1: bounded natural private conversation

Completed the selected C1 implementation and deterministic native Windows slice,
without claiming real-model naturalness. Git transport had failed earlier; the
official GitHub API and fixed files succeeded at MaiBot revision
062d45cf8bd4272ac57d7ec03fcf405e5a458211 (2026-10-23T17:46:39Z). Read the actual
debouncer, wait/stop, optional message steering, private/group gate, context and
personality/behavior/reply-style assembly. MAIBOT_REFERENCE.md records immutable
links and Lain's choices. No upstream source or prompt text was copied into product;
25 downloaded reference files, hashes and LICENSE remain in ignored target data.

Identity v3, always-appended Actor policy, private behavior and private expression
are assembled separately. Ordinary private recall need not display UUIDs in every
reply; exact IDs/revisions/provenance remain inspectable. Configured diagnostics
add private_chat_revision for this bundle. Stable personality, truthful experiences,
typed tool policy and explicit Work/Worker review remain enforced. No extra Planner,
Replyer, background extraction, random personality or paid evaluation was introduced.

The private channel now uses a trailing 750ms quiet window, capped at three seconds,
with the original 16-input/8KiB/128-queued bounds and ten-minute expiry. chat.wait is
once per turn for 1–5 seconds and wakes on new input/deadline; chat.finish records
an inferred social closure with no delivery. A conservative guard refuses silence
for protected requests; rejection removes finish for the remaining chain. Semantic
classification and naturalness still require real feedback, not fixture text.

In-flight supplements retain original IDs/text/times and associate atomically with
the current batch/Actor; root request and Batch.text remain unchanged. A returned
stale response is accounted but its text/calls are discarded. At most two
reconsiderations use the existing four-request/four-tool/reported-token/120-second
budgets without resetting counters. Over-cap input stays queued for a fresh turn,
with an honest capacity notice replacing the obsolete reply. Opaque continuation
is rebuilt across new input, with IDs-only receipts of already committed memory
effects. Private draft previews are held. No Worker is cancelled by reconsideration.

Final conversation text/new actions and private memory effects recheck current
input. Latest selected memory IDs/revisions and bounded source selections/hashes
are revalidated at commit/send boundaries. The final review found and fixed the
Ready historical-quote gap: revoke/expiry/suppression after generation now makes
that unsent quote ineligible, with prepared audit/fingerprints committed together.
The service also reapplies current credential filters immediately before writing
stored drafts. Ready ordinary drafts become Superseded; prewrite refusal is known
NotSent. Confirmed/Unknown outcomes and approval/failure notices retain their truth.
Later model history uses confirmed speech and withholds stale/uncertain drafts or
changed memory/source-derived replies. Migration 011 adds the batch-input index;
legacy snapshots are preserved without inventing old chat decisions or memory.

Native Windows gates passed format, strict all-target workspace Clippy, locked/
offline workspace tests (157 passed, zero failed, three manual ignored, 15 suites),
and workspace build. Nineteen added C1 tests cover quiet/cap/rollback, atomic input
adoption, bounded interruption/cumulative usage, protected requests, wait/deadline/
cancel/restart, proposal isolation, delivery truth, memory/source changes, private
preview/history, schema-10 compatibility and stored-credential prewrite filtering.
The original identity/dedup/receipt-loss fixtures remain; a separate post-confirmation
input now exercises the uncertain second send after an in-flight supplement is
merged into the first turn. Early old fixed-window/two-reply expectations and an
invalid approval fixture were corrected, not recorded as passes. One transient
Windows mapped-source formatting error was resolved by retrying without killing
user processes.

Extended the existing qq_memory_smoke.py: 22 actual native process/HTTP/WS checks
passed with 61 local fake model requests, zero paid calls and complete cleanup.
All original Q2-A/Q2-B1 scenes remain. Added C1 merge/original IDs, silent closure/
question protection, wait/new-input wake and in-flight negation/stale proposal
checks. Startup/model/source injections each returned expected exit 1 and cleaned
owned state/processes. Current binary SHA-256 values match the successful report.
Final broad native CLI regression passed in 237.12 seconds, including owner ACLs,
launcher/shutdown/restart, doctor, streamed Actor, reviewed fake Worker submission,
independent memory/summary retrieval and persistence. No concurrent Cargo build
ran against those process fixtures.

Updated ADR 012, product/chat/architecture/domain/runtime/memory/Actor/CLI/QQ docs,
README and ROADMAP. C1_ACCEPTANCE.md contains requirements-to-evidence mapping,
the repeatable offline command, twelve multi-turn experience scenes/rating criteria
and five separately authorized manual upgrade/test steps with schema-11 rollback.
Evidence is ignored under target/q2a/c1-*: gates, process/failure/native logs and
reports, reference hashes and preserved baseline fingerprints. Prior failed
real-provider records and historical Q2-B1 counts retain their original meaning.

Preserved all 57 pre-existing dirty paths; the combined tree has 69 paths (42
modified, 27 untracked), including the new C1 files and existing Q1/Q2 work. HEAD
remains 560ddf9. No commit/push/deploy, personal credentials/database read, real
QQ/model call, real grant enablement or F:\Lain-Q1-Acceptance upgrade/restart occurred.
The real instance still needs a separately authorized upgrade and experience review.
Naturalness, intent misclassification, real latency/cost, ConPTY and native
macOS/Linux remain unverified. Stop this bounded C1 here; no Q2 experiences/Q3 work.


## 2026-10-25 — C1.1: upgrade rehearsal, live acceptance pending

Prepared a concrete per-instance plan and a hash-checked launcher candidate under
ignored target/c1-1. Read only file/process metadata, startup AST and selected
non-secret settings. The existing entry selects target/debug; accepted C1 binaries
are in target/q2a/debug. No personal credentials, SQLite content or authenticated
personal API was accessed. No personal service was stopped or upgraded.

Extended the existing QQ process smoke script with an isolated --upgrade-from
mode. Native Windows passed five checks: old schema-8 state creation, stopped
whole-state backup with owner DACLs, migration to schema 11 preserving identity/
memory/receipts, old-binary refusal of the future schema, and complete rollback
with no input/delivery replay. Three local fake requests, zero paid calls and
complete cleanup. Earlier ACL failures remain in evidence; a DACL-only .NET
setter avoided the unavailable SeSecurityPrivilege while retaining owner checks.

Normal regression again passed all 22 process checks with 61 local fake requests,
zero paid calls and cleanup true. Python AST validation and positive/tampered
launcher inspection passed. Evidence is target/c1-1/upgrade-rehearsal.log and
normal-regression.log. No Rust/persona behavior changed; the earlier 157-test C1
result was not rerun or relabeled as C1.1 live evidence. Native macOS/Linux and
real naturalness, latency, cost and personal migration remain unverified.

Updated C1_1_ACCEPTANCE.md, QQ_CHANNEL.md schema documentation and ROADMAP.
Pending: consolidated authorization for stopped full backup/upgrade/rollback,
existing SecretRefs, a bounded model-call budget, the existing private binding,
and optional short-lived test-memory permission. Then user-led scene feedback
can support targeted fixes. No commit/push/deployment or Q2/Q3 expansion.


## 2026-10-25 — C1.1: authorized upgrade and first real expression feedback

The user explicitly authorized the prepared personal-instance operation, at most
60 real model requests, the existing private QQ binding, and optional one-hour
test memory permission with source access off. The user gracefully stopped the
old launcher; no Work sessions or unfinished channel batches were present.

Backed up all eleven stopped instance files with byte and DACL verification,
archived exact old binaries, staged only the accepted real CLI/daemon under a
versioned release, and retained the original entry/settings. Owner-plus-DACL
initialization of the backup parent failed before credential copy; a DACL-only
setter verified the existing owner and succeeded without elevation. Extended-path
spelling also required normalization in the path preflight. Personal migration
8 to 11 preserved all 38 old incoming/batch/delivery records in their old columns.
The new foreground entry is task-controlled; doctor paths/configuration agree,
QQ is identity-gated Online and memory/source grants remain off. Personal rollback
is prepared but not exercised.

Six actual user-sent greeting/joke/venting inputs used six requests and 21,516
reported tokens, with all turns completed and deliveries confirmed. User feedback:
content was generally acceptable but every short reply felt written, repeatedly
two sentences separated by a blank line with deliberate punctuation. Preserved
before-change evidence and feedback in the protected backup outside Git. Added
only short-message expression guidance; no output postprocessing, fixed responses
or permission/runtime behavior changed. Retest remains pending. Formatting and
strict workspace all-target Clippy passed; relevant Actor tests/build are underway.
The first-batch ledger leaves 54 requests. Read-only capture uses persisted Actor
counters, not the diagnostics endpoint literal model_calls zero. Worker version
compatibility is unavailable and outside this chat task. No auto commit/push.


## 2026-10-26 — C1.1: autonomous real-model repair and restored QQ service

After the user requested debugging without further manual message sending, paused
the owned personal foreground service and reused the existing native OneBot/process
fixture for isolated real-provider replay. Synthetic inputs were not inserted into
the personal conversation and no automated QQ probe was sent. Combined actual
request usage is 57/60: 28 user-led real QQ requests and 29 automatic real-model/
local-transport requests, reporting 255,754 aggregate tokens; monetary cost and
provider-resolved model revision remain unavailable. Automatic inference has stopped.

The first expression revision failed its actual QQ retest. Explicit user format
instructions worked, then firmer short-message defaults were tested independently.
The final cold-start sample, without personal history or style instruction, replied
in one short line without a final period. Some other replies still use periods or
formal explanation; broad naturalness is not declared solved.

Actual QQ remember/correct/forget effects succeeded, but the model falsely said
the preference had never been saved. Instruction clarification plus historical
IDs-only action projection alone also failed real replay. Added the binding latest
three committed MemoryEffect receipts to fresh, authorized memory context as Kernel
operation metadata. They contain no recalled text and grant no capabilities.
Historical private action observations omit old arguments/result bodies and retain
only correlated successful effect metadata/state. Failed/Unknown operations cannot
become committed truth; canonical audit stays unchanged. The identical real-model
post-forget question and a causal follow-up then correctly acknowledged save,
correction and deletion and corrected the earlier denial. Inferred provenance and
ordinary candidate review remain enforced; no memory/approval check was bypassed.

Real-model/local-transport timing: three inputs 101ms apart became one batch and
one request; successive in-flight corrections reached two reconsiderations in
three requests, then retained the excess final input for a fresh one-request turn.
The superseded turn emitted an honest capacity notice; obsolete drafts were not
sent. Closure followed by an explicit question answered normally. Existing actual
QQ screenshots and all failed replies remain separately preserved.

Validation passed formatting, strict all-target workspace Clippy, 77 Kernel unit
tests (two manual ignored), relevant Actor integration tests, workspace build and
22 cross-process checks with 61 fake requests/zero paid calls/cleanup true. New
context test covers operation truth without forgotten arguments; existing lifecycle
test now verifies three same-binding, content-free committed receipts after deletion.
Disk-full and later PDB LIMIT failures were preserved; only verified regenerable
q2a incremental/debug-symbol caches were cleared with compilers stopped, then gates
rerun successfully. A SQLite-handle temporary cleanup failure was repaired and
verified; a zero-call old-event-ID replay received no acceptance credit, and new
input/Actor checks precede the valid retest.

Final personal release c1-1-0c5048d155ee started at 00:00 Asia/Shanghai, using the
unchanged Actor config/identity/binding and schema 11. QQ Online, reply permission
retained, temporary memory grant revoked at revision 2, source grant never enabled.
The full stopped schema-8 backup/old binaries remain protected; personal rollback
was not needed. The new start-c1-qq-test.ps1 selects the verified release; original
entry and prior releases remain retained. No original dirty work was reset, and no
commit/push/cloud deployment/Q2/Q3 expansion occurred. C1_1_ACCEPTANCE.md has actual scene
outcomes, artifact paths, startup/recovery instructions and remaining quality limits.


## 2026-10-26 — D1a: bounded text acquisition in the combined companionship/work goal

The user selected both large stages together. Read the full goal and current
architecture/evidence; baseline main/origin main was clean at 3bfb855. Added an
explicit D1a–D3/W1–W4 delivery plan without declaring either complete. Corrected
PRODUCT current-state statements to reflect C1.1 real QQ/model replay and the
installed schema-11 release with temporary memory revoked and source access off.
Historical failed/live records retain their original meaning.

Implemented a separate default-off CAS ReadingGrant selecting exact HTTPS URLs,
expiry, retention and attempt/body limits. Migration 012 appends grant/activity/
result tables; no old authority is changed. One unfinished acquisition, durable
attempt charging, immutable request dedup, actual timestamps and safe closed
failures survive restart. Restart marks unfinished work Unknown without fetching;
cancel/revoke/expiry prevent late content commits. Completed content/state commit
atomically. Grant replacement removes old content without inheriting its authority.

The asynchronous reader pins validated public DNS addresses, disables proxies,
redirects and ambient auth, accepts only bounded complete UTF-8 plain text, checks
known credentials and has finite connect/overall deadlines. Stored text remains
untrusted administrator-inspectable data. AcquiredTextOnly never claims reading,
comprehension, summarization or verification. A next-retention-deadline timer is
rebuilt from storage and deletes bodies without model polling. Operation metadata,
WAL/backups and semantic erasure remain distinct. Added authenticated API and
Rust reading CLI; no Actor tool, model job, experience or contact is yet added.

Native Windows workspace run: 167 passed, zero failed, three manual ignored;
fmt, strict all-target Clippy and workspace build passed. Added one further atomic
rollback test afterward; focused reading suite passed all ten, including timer
expiry inspected directly through SQLite. Actual native QQ/Worker regression
passed 23 cross-process checks with 61 fake requests, zero paid and full cleanup.
Independent full backup/migration/rollback rehearsal passed five checks from
schema 8 to 12, three fake requests, no paid calls/personal data and full cleanup.
Schema 11 preservation is separately covered at the storage unit level.

Preserved failures: initial SQLite unsigned conversion/test-constructor compile
errors fixed; a later workspace compile exhausted disk and was rerun successfully
after cleaning only verified generated target artifacts with compilers stopped.
Process-scoped no-incremental/no-debug-symbol build settings limited disk use.
No global tool config, personal runtime, credential, database or grant was changed.
Evidence and binary hashes are under ignored target/d1a and READING_ACCEPTANCE.md.
No new real-model/public-source/QQ/device acceptance is claimed; old real-call
budget remains closed. D1b experiences/Actor processing, D2 contact, D3 full daily
chain and W1–W4 remain outstanding. This is a verified implementation milestone
for commit/push, not completion of the active combined goal.


## 2026-10-26 — D1b: source-linked reading experiences and private recall

Continued the combined goal from clean f4ec0d4. Added optional, default-off model
processing/disclosure scope to exact-URL reading grants: one private binding,
a cumulative prepared-request cap and an explicit fixed UTC offset. Schema 013
adds transactional reading jobs, model reservations, experiences, content-free
read/edit/source-decision audits and withdrawal/suppression records. Existing
schema-12 grants decode processing-off without rewriting their raw bytes/bodies.

A successful acquisition event or explicit process action reuses the existing
Actor in a Kernel-originated reading mode. It has no tools and cannot acquire
private contexts, memory writes or Worker actions. Source text enters only fresh
bounded untrusted inference context, not the immutable activity request. Actual
input/original sizes and truncation are recorded. Source/cancel/expiry/fingerprint
checks protect response and commit; returned usage is retained before discarding
a stale or malicious response. Completed Actor and source-linked summary publish
in one transaction. Idempotent/Unknown jobs never regenerate automatically, and
budget reservations survive failure, cancellation and restart.

Added typed private experience search/inspect with fresh evidence rather than
cached tool bodies. Revision/fingerprint and grant checks persist through model
response, action/final commit and send admission/prewrite. Correction is explicitly
owner-originated, not retroactively model interpretation or verified fact. Withdraw
suppresses the exact URL, removes its source bodies and invalidates derivatives;
explicit restore permits only new acquisitions and does not restore old jobs,
content or spent requests. Experience-influenced turns cannot silently promote
unlinked semantic memory; original ordinary memory and Worker review remain.

Closed the analogous C1.1 empty-lookup trap with at most three content-free committed
summary-operation receipts. No URL, forgotten original, old summary or correction
enters them. Operation-only reads also retain an admitted grant revision; revocation
before another request/response/commit/send cannot bypass policy just because the
selected body list is empty. A withdrawn/expired summary remains an actual past
operation, while active retrieval correctly returns no content. Real-model quality
is not inferred from the deterministic model outputs.

Added Rust CLI/API inspection, processing, date/topic search, correction/withdraw,
source restore, budget and audit. Updated architecture/domain/runtime/memory/chat/
Actor/CLI/QQ/product docs, ADR 014, combined plan and EXPERIENCE_ACCEPTANCE.md.
D1b is an implementation milestone; D2 sharing, D3 live daily-chain acceptance
and W1–W4 sources/devices/work/terminal remain outstanding.

Native Windows/MSVC Rust/Cargo 1.98.1 final gates: fmt, strict workspace/all-target
Clippy, build and full workspace tests passed (180 passed, zero failed, three
manual ignored). Reading tests total 22, including source extent, malicious tool
refusal, durable budget/restart, atomic terminal/experience failure, private scope,
correction/withdraw/expiry, stale drafts, operation-only truth/revocation, midnight
and schema-12 body/grant-byte preservation. The new source/model process mode uses
explicitly seeded synthetic acquisition, actual native HTTP Responses/OneBot and
fresh Actor identities; it is not claimed as a production public fetch.

Final process regression passed 24 checks with 68 fake requests and cleanup true.
Schema 11 to 13 native binary-only full backup/migration/refusal/rollback passed
five checks with three fake requests and cleanup true. Copied only the installed
C1.1 executable pair into synthetic state; no personal DB, settings or credential
was read, migrated or restarted. Final binary hashes match both reports under
ignored target/d1b. All calls were fake/local, zero paid; the old real-call campaign
remains closed. No real source, QQ contact, device connection or deployment.

Preserved failures and intermediate evidence: a test sliced Chinese text at an
invalid UTF-8 byte and was fixed; Clippy required lexical lock scopes; the first
schema-11 rehearsal exposed an old fixture assumption that historical turns had
no C1 decision. It now compares exact original/migrated fields and checks no reading
origin/grant/request was invented. Full rebuild/test and final process reruns cover
the subsequent operation-truth extension; earlier 179-test/66-request reports
are retained without promoting them to final-extension evidence. Real public
TLS/source/provider quality, multi-day continuity and native macOS/Linux remain
unverified. Commit/push this verified milestone; the complete goal stays active.


## 2026-11-02 — W2b durable device command and receipt protocol

Continued the user-selected server/device mainline on codex/d3-live-repair,
based on 9b40de0, without restarting the architecture or adding a Worker runtime.
Added immutable selected-workspace work intents, per-command device event
receipts, transactional per-device change cursors and authenticated bounded waits.
Administrator queue/cancel and device fetch/inspect/report remain separate.
Native CLI journals an event before HTTP and retains its matching receipt; exact
replay recovers uncertain acknowledgements without a replacement work identity.

Cancellation is separate from an observed stop. Queued cancellation prevents
acceptance; accepted/running cancellation requests retain uncertainty until actual
device observations arrive. Deadline/revoke cannot claim execution stopped.
Late ordered observations can reconcile uncertainty without authorizing a new
start. Server restart never requeues accepted work. Device revocation and command
invalidation commit together. Additive migration 025 changes no existing grant,
conversation, model selection or media. Actor tool/Worker approval remains intact.
No native executor, paid inference, real QQ sends, device installation, cloud
Kernel deployment or personal-instance migration is part of this increment.

Native Windows Rust 1.99.1 GNU/GCC MSVCRT validation: 304 tests passed, zero failed,
three existing manual/interactive probes ignored; format, strict all-target
workspace Clippy and locked build passed. Device process acceptance passed six
checks with no model/Worker execution. Visual process regression passed nine
checks with twelve fake requests/eight fixture media reads. Real old/new binary
schema24→25/full-rollback rehearsal over synthetic state passed five checks with
three fake requests. Python script checks ran fourteen tests: nine passed and
five existing opt-in/platform cases skipped. All process reports match the binary
hashes recorded in docs/DEVICE_COMMANDS.md; temporary services/state cleaned up.
Native MSVC, Linux/macOS, cross-device TLS and actual remote Codex remain unverified.

Validation exposed old migration fixtures that left new tables behind, an old
upgrade-version upper bound, and a paused-clock/real-handshake race under Windows.
Updated only the affected fixtures/schema assertions; the timeout test now pauses
after its real handshake while preserving all identity/timeout assertions. Native
elevated fixture setup used an external fresh-path owner adapter and DATSO backup
copy; it never repaired existing user paths or relaxed private-file checks.
Original task line endings are preserved, including Windows CRLF; terminal control
characters, credential-shaped observations and traversal-like profile identifiers
are rejected. Local build/test reports remain outside Git under
%TEMP%/lain-q1-tools/w2b-*.

Contract, operational commands, limits and deferred W3 execution obligations are
in docs/DEVICE_COMMANDS.md and ADR 028. Updated product/architecture/domain/runtime,
CLI/configuration, resident design, roadmap and development instructions together.
Changes are local implementation work; no commit, push or deployment is claimed.

## 2026-11-02 — W3a typed execution evidence and explicit permission receipts

Continued the selected cloud/device mainline with a reviewable permission and
evidence increment, before introducing native effects. Device events now retain
typed Lain execution/session/turn bindings, one bounded pending approval and
factual result feedback. Correlation cannot be removed, changed or shared with
another command on the same device. Original work intent remains immutable;
reports neither create cloud-local Supervisor sessions nor establish that a
native Worker ran. Output tails preserve failed-command, withheld-reference and
history-truncation facts; relative paths are reported references, not verified or
uploaded artifact bytes.

Added owner-only exact-scope decisions and device-only receipt reads to the
existing HTTP/CLI surface. A versioned fingerprint binds execution/session/turn,
approval identity, visible kind/description, truncation and explicit choices.
Truncated scope cannot offer approval; new choices require an active, unexpired,
uncancelled Running command with that exact pending request. Changed/recycled
approval contents and reused decision identities conflict. Recording a choice
publishes its revision atomically but does not claim backend application. Exact
receipt recovery remains possible after resolution without granting permission
to replay uncertain Worker mutations. No Actor, QQ or model decision can call
this owner authority implicitly.

Schema026 adds execution-correlation and approval-decision ledgers. Legacy W2b
snapshots/receipts omit the optional evidence and retain their original replay
identity. Secret-bearing evidence is rejected before local journaling and server
persistence. Updated earlier migration fixtures and expected target schemas;
also corrected an existing deadline test to project past the later of its two
command deadlines instead of assuming their wall-clock timestamps coincide.

Final native Windows GNU workspace validation: 316 passed, zero failed, three
existing manual/real-service tests ignored; formatting, all-target Clippy and
workspace build passed. Python helper suite: 14 tests, nine passed/five skipped.
Six approval and six legacy-command process checks, nine visual checks and five
schema24 → 26 upgrade/full-rollback checks passed on the same final binaries.
Protocol runs started no Worker/model; visual/upgrade runs used 12/3 fake model
requests, zero paid calls and zero real QQ sends. Temporary services/state were
cleaned up. Process and upgrade evidence is recorded in docs/DEVICE_EXECUTION.md.
All build, test and process logs stay outside Git in
%TEMP%/lain-q1-tools/w3a-*.

W3 remains in progress. This increment starts no native host or Worker, applies
no backend approval and does not enable QQ/Actor dispatch to a real device. W3b
must implement native policy/journal, launch/submission fences, actual permission
application, cancellation and restart reconciliation around the existing
Supervisor. W4 retains terminal/chat/dashboard integration. Updated product,
architecture, domain/runtime, CLI/configuration, resident design, roadmap,
development instructions and ADR029 together. No paid invocation, deployment,
personal database upgrade, commit or push is part of this work.


## 2026-11-02 — W3b native device Worker service and effect recovery

Added the explicit native lain-device-agent service and device host-configure
entry. It uses only protected device credentials, a separate Worker operational
store and the existing Supervisor/structured Codex adapter. It creates no Actor,
channel, memory service, administrator listener or second Lain persona. Local
policy selects the canonical executable, read-only or explicitly workspace-write
scope and finite network grace. The first supported path is serial codex/local;
capability/version probes, the exact registered workspace and current cloud
scope gate actual effects. Configuration starts no service and edits no global
Worker settings. The shared lain-local crate extracts the existing native ACL/
private-file helpers without adding another runtime or identity registry.

Persisted original intent and deterministic session/turn IDs precede native work.
Separate launch, submission, response, cancel and stop fences prevent uncertainty
from becoming replacement tasks. Stable Kernel metadata/prompt interfaces reuse
existing supervision instead of adding another harness. Restart resumes only the
original bound session/turn; a claimed launch without a binding or submitted
unmapped turn stays Unknown. Owner decisions must match the current native and
cloud approval fingerprints and choices, and response intent is persisted before
RPC. Truncated/redacted/incomplete scope cannot offer approval. Cancellation,
revocation, expiry, changed policy and prolonged network loss stop new effects;
only an observed terminal event confirms cancellation. Pending result facts can
be delivered after reconnection without restarting work. Recovered status marks
history incomplete; original bounded output, failures and relative reported paths
remain facts rather than an LLM evaluation or verified artifact upload.

Two actual native process failures tightened the implementation. Abrupt death
while a receipt file was being created left an incomplete record: journal writes
now fully write/sync unique private staging files and atomically publish them
under the exclusive host lock. Interrupted staging is not an effect fence.
A regression test first demonstrated that a mismatched receipt could advance the
outbox; advancement now checks the full immutable prior event. The other failure
was a locked fixture log held by an orphan Windows ACL helper during shutdown/
crash cases. A read-only Windows resource query identified that helper and its
exited parent; it was cleaned up only after verifying ownership of the synthetic
fixture. ACL helpers now use the same CREATE_NO_WINDOW console isolation as
owned Workers. The final native service rehearsal passed all fourteen scenarios
and normal cleanup without weakening owner checks or hiding cleanup failures.
The lookup followed [Microsoft's RmGetList contract](https://learn.microsoft.com/en-us/windows/win32/api/restartmanager/nf-restartmanager-rmgetlist);
no unrelated application was stopped or restarted.

Final Windows GNU checks: 322 Rust tests passed, zero failed, three existing
manual/live cases ignored; formatting, all-target Clippy, locked offline build
passed. Six host-specific tests cover recovery plans, hidden-scope redaction,
metadata identity, immutable scope/fences, receipt validation and interrupted
staging. Native process evidence and compatibility regressions are recorded in
docs/DEVICE_HOST.md. The process Worker is the structured fixture only; the
parallel-chat case uses one fake Responses request and fake OneBot, no paid
provider or real QQ message. Python helper tests: 14 total, nine passed/five
skipped. External local evidence stays under
%TEMP%/lain-q1-tools/w3b-*; no temporary debug
script, fixture state, log, credential or personal content is added to Git.

Final compatibility gates on the same native binaries: six W3a permission checks,
nine visual checks (12 fake requests/8 fixture media reads) and five actual
schema24 → 26 executable upgrade/full-rollback checks (three fake requests).
Each final process report confirms cleanup and zero paid requests; common binary
hashes and the explicit native-host hash are in docs/DEVICE_HOST.md. Actual
synthetic backup copy preserved ownership with DATSO through the external fresh-
path harness; the existing report's DATS label is not the actual copy flag.

Product/architecture/domain/runtime, backend/configuration, resident design,
CLI, roadmap, development instructions and ADR030 document the same ownership
and lifecycle boundaries. W3's real paid-provider, cross-device TLS and other
native platform acceptance remain separate. W4 owns service management and
terminal/chat/dashboard dispatch. No actual device installation, cloud deployment,
personal QQ upgrade, commit or push occurred.
