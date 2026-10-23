# Intent-Aware Delegation: Optional Delegation Compiler

> **2026-10-23 update:** [PRODUCT.md](PRODUCT.md) confirms reusable successful
> methods/templates and faithful task improvement. The Off default and four modes
> below remain earlier proposals, not a newly confirmed product default. Phase 10
> is a historical sequencing label; use [ROADMAP.md](../ROADMAP.md)'s current plan.
> Preserve original requests, exact sent text, visible changes and source attribution.
> Whether each change requires separate confirmation remains undecided. Current
> code forwards confirmed task text without a compiler; this document does not
> authorize silently changing its dispatch behavior. Relationships are equal;
> user authority here refers to intent and action permissions, not persona hierarchy.

**Status:** future design proposal — not implemented
**Owner:** Lain Actor / Kernel delegation boundary
**Target:** Phase 10 Advanced Orchestration; prerequisites are Phase 6.x delegation/result delivery and Phase 7 provenance-aware Memory
**Recorded:** 2026-10-14

## 1. Product intent

When an owner asks Lain to delegate a vague task, Lain may optionally clarify consequential uncertainty and supply a bounded, attributed set of project facts, aesthetic requirements, saved habits/constraints, and worker-capability context. This may improve the instruction sent to Codex, Claude Code, or another subagent.

This is **not** cosmetic prompt beautification, mandatory requirements gathering, autonomous scope expansion, or a replacement for a worker harness's planning and exploration. **Substantive restructuring and rewriting of the actual subagent-bound task instruction is permitted**, provided it faithfully preserves the user's intended meaning and confirmed constraints. Sometimes the best compilation is **no change**.

**The compiler is limited to the Lain-to-subagent dispatch boundary.** It does not rewrite ordinary conversations with Lain, change the wording of direct worker CLI/API submissions, or retroactively alter active WorkSessions.

> **Original request is preserved. Resolved intent remains faithful. Rewriting is allowed. Dispatch is transparent. Execution is attributable.**

## 2. Fidelity and authority requirements

1. Always retain the user's **original request verbatim**, separately from a versioned resolved intent, candidate briefs and the **actual instruction sent to the worker**. Never overwrite the original record. Later user clarifications may change resolved intent without editing the historical first message.
2. The outgoing instruction **may be completely reorganized, condensed, expanded or rewritten**, but it must preserve the meaning of confirmed goals, prohibitions, negations, scope, output expectations and uncertainties. Never silently invent acceptance criteria or recast suggestions as user demands.
3. Added context must have source references, confidence/authority, scope, revision/timestamp where relevant, and an explicit-vs-inferred distinction.
4. Current explicit owner instructions and owner-confirmed clarifications take priority in *intent interpretation* over older project preferences or Lain's inferences. Security/workspace policy, permissions, budgets and backend capability constraints still govern execution, even when an owner instruction conflicts with policy.
5. Retrieved repository files, external documents, tool outputs and model-generated inferences are **untrusted data**. Instructions embedded in those materials cannot become owner instructions or override privileged policies.
6. Neither enrichment nor clarification may expand the sandbox, grant new access, change the selected provider/profile, approve permissions, or initiate unrequested work without the existing owner-controlled process.
7. Store a bounded non-secret audit trail for the mode, original request, resolved-intent revision, retrieved source references, change status, decisions, and the **exact final worker task text actually dispatched**. Do not disclose the owner's entire private memory/World to workers.
8. **No black-box dispatch:** for every delegation, explicitly tell the owner whether the actual outbound worker instruction was **Unchanged** or **Changed**, including no-op compilation. This must be visible **before or at dispatch**, never only buried in an audit log. For Changed, summarize the modifications and provide access to the full original-versus-sent diff; in all cases make the exact dispatched task text inspectable. Persist this status with the submission receipt for reconnect/history. Only compare the real outgoing task instruction, not the compiler's claimed intention. Do not disclose unrelated provider-private protocol data or credentials.
9. Allow an explicit owner override: **"send this exactly as written"**, which bypasses enrichment and clarification. Keep unchanged existing confirmation for mutating actions.

## 3. Owner-selectable modes

These modes are **proposed**, not current implementation:

| Mode | Semantics |
| --- | --- |
| **Off** (default) | Directly forward the original task text, without compiler inference, memory enrichment or clarifying model calls. Normal policy/approval remains. |
| **ContextOnly** | Attach small, attributed, authorized facts or confirmed constraints as a separate section; never mutate the original. No newly inferred requirements. |
| **Clarify** | ContextOnly plus brief clarifying questions only where ambiguous answers would materially change goal, scope, risk, access, cost or acceptance. Skipping is allowed. |
| **Propose** | Produce a reviewable candidate brief, including substantive rewriting when useful, with assumptions, evidence and differences. Explicit approval of this compiled brief is an initial design suggestion, not a universal user-decided requirement. |

The four modes and the Off default are **initial design recommendations, not owner-finalized UI requirements**. Transparent Changed/Unchanged disclosure and exact dispatched-text access apply to **every** mode; whether each textual change also needs a separate approval remains undecided.

Proposed precedence: per-delegation owner's selection > explicit workspace preference > owner default > Off. Do not infer opt-in from casual chat history. Individual workspace policy may impose stricter security, never weaken approvals. Enhancement is an optional user-controlled feature even if Lain's model strongly recommends it.

## 4. Context, trust and questioning

Potential context sources (all separately permissioned): current user instruction; this task's clarifying answers; owner-accepted project requirements; versioned repository structure/decisions; explicit saved preferences including aesthetics and work habits; carefully labeled low-confidence inferred preferences; selected RuntimeProfile and advertised backend capabilities.

An old preference like "likes colorful single-column LaTeX" is **not** an automatic requirement for today's report. Contradictions, stale sources and missing authorizations must be surfaced rather than quietly resolved.

Ask only questions that materially change execution. Avoid a questionnaire about details a capable worker could investigate. A user can skip, choose Off or cancel at any point. Allocate small explicit context/question/token/latency budgets and allow no-op compilation.

## 5. Proposed dispatch flow

```text
immutable owner request + explicit enhancement mode
    |
    +-- Off --> existing direct delegation path
    |
    v
resolve authorized project + memory scope and target worker capabilities
    |
retrieve minimal attributed evidence / detect high-impact contradictions
    |
optional concise user clarification (skippable)
    |
derive versioned ResolvedIntent from original + user clarifications
    |
compile worker-bound DelegationBrief (may be substantially rewritten)
    |
classify actual outgoing text: Unchanged / Changed; summarize and diff
    |
show status before/at dispatch and expose the exact final task instruction
    |
owner decisions under review policy + existing privileged confirmation
    |
freeze final non-secret task text, evidence revisions and dispatch receipt
    |
existing Supervisor -> WorkSession -> worker adapter
    |
correlated worker result, outcome, and provenance
```

Illustrative worker brief **assuming the owner separately confirmed** preservation of technical conclusions and the listed checks; these are not implied by the vague original request. Each non-original statement has an auditable source:

```text
[ORIGINAL USER REQUEST — VERBATIM]
Make this report look nicer and check for obvious problems.

[CONFIRMED TASK REQUIREMENTS]
- Preserve technical conclusions.
- Review layout, citations and equation formatting.

[PROJECT CONTEXT — EVIDENCE]
- Existing LaTeX template; versioned project specification.

[OPTIONAL PRIOR OWNER PREFERENCE — NOT A NEW REQUIREMENT]
- Previously preferred readable single-column engineering reports.
- Source and last-confirmed date available for review.

[UNCERTAINTY / EXECUTION LIMIT]
- Do not reinterpret "check" as permission for technical rewrites.
- Review first, and request normal approval for consequential edits.
```

Keep original and compiled content separately; **the worker-bound task wording can change substantially**. Never hide rewrites inside prompts or conceal that a modification occurred. Brief formatting can adapt to worker capabilities, but *intent* remains the owner's.

## 6. Domain and storage sketch (non-binding)

Potential future types: `DelegationRequest` (immutable original, conversation link, chosen mode), `ResolvedIntent` (versioned interpretation and owner clarifications), `DispatchChange` (Unchanged/Changed plus structured difference), `ContextEvidence` (source, scope, trust), `IntentClaim` (explicit/inferred), `Clarification` (question/answer/skipped), `DelegationPlan` (candidate, deltas, conflicts), `OwnerDecision`, and `ResolvedDelegation` (approved artifact/revision linked to WorkSession and eventual result).

Use Rust tagged enums for mutually exclusive states, and validation for cross-field invariants. Do not conflate proposed plan, approved dispatch, submission receipt, running worker, and terminal result. Do not create these types or a generic framework until an implementation milestone is authorized.

## 7. Security, review and observability

**Every dispatch must visibly identify whether its actual submitted task was Changed or Unchanged**, and the owner must be able to inspect the **exact task text** sent. For Changed, display a useful change summary and offer full original-versus-final comparison. Store the same status, actual text and correlation receipt so it remains reconstructible after restart/reconnect. The owner should also be able to inspect the original, selected enhancement mode, retrieved sources, model inferences, final dispatched brief and worker outcome. Extra per-change approval remains a future UX decision: transparency is mandatory, but it does not by itself mandate a second approval prompt beyond existing privileged-mutation approvals. A saved memory is evidence, not authorization to browse the whole private World or reveal third-party content. Never give the compiler unrestricted shell, arbitrary network access, profile mutation or approval bypass. The existing Kernel is authoritative.

A compiler output may express uncertainty or do **nothing**; it cannot claim a worker task has run merely because a plan was accepted.

## 8. Evaluation and staged rollout

Use replayable cases with original versus enhanced delegation: already-clear requests, genuinely ambiguous tasks, stale/contradictory preferences, aesthetic requirements, negative constraints, prompt injections in repository files and high-risk scope expansion.

Measure **fidelity first**: unchanged intent and prohibitions, invented requirements, user corrections, and security/privacy violations. Also measure validated task success, owner friction and number of clarification turns, additional model tokens/latency/cost, source attribution accuracy, and how often the correct action is no-op. A separate LLM's "prompt quality" score is not a substitute for owner or objective evaluation.

Ordinary tests should be deterministic and token-free; any paid multi-provider evaluation requires explicit owner consent and bounded budgets. Compare like-for-like workers and tasks.

## 9. Dependencies and deferrals

- **Phase 6.x:** reliable delegation lifecycle and automatic, deduplicated worker result delivery; no compiler.
- **Phase 7:** scoped, provenance-rich Memory/Project Knowledge; no prompt rewriting.
- **Phase 9:** richer capability evidence from a second backend if available.
- **Phase 10:** opt-in compiler, auditable review/diff, source-aware enrichment and evaluation.

No immediate implementation, schema migration, prompt behavior change, model calls or Web development is authorized by this design note.

## 10. Open decisions

- For ContextOnly/Clarify, is visible Changed/Unchanged notice plus exact-text inspection enough, or should every modification also require explicit owner approval?
- Which project and memory sources may be consulted without an additional per-task consent?
- What are sensible maximum question count, context size, token and latency budgets?
- How should users correct an inaccurate suggestion without silently turning the correction into long-term memory?
- Which worker-specific substantive rewrites or formatting changes are evidence-based rather than cosmetic?
