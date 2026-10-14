# Phase 6.5 acceptance and conditional Phase 7 handoff

Status: implementation plus **native macOS real evidence verified across controlled attempts**.
Updated 2026-10-14. This was not one uninterrupted successful runner execution.
Windows/ConPTY and Linux are unavailable and unverified; cross-platform Phase 6
acceptance is incomplete. Phase 7 remains unstarted. No milestone completion
commit is claimed.

## Evidence by requirement

| Requirement | Current evidence | Remaining acceptance |
|---|---|---|
| Incremental output/parser | Fragmented UTF-8/multiline SSE, private tool deltas, completed-only fallback, malformed/failure/incomplete tests; gated fixture and actual MiMo CLI previews before completion | Other native platforms/providers |
| Safe preview contract | Bounded sanitizer/queues; turn/preview/message IDs and sequences; checkpoint reconnect, slow subscriber, wrong-generation/late-preview tests | Measured MiMo latency/omission caveats below; other providers unverified |
| CLI renderer/timing | Native Unicode/multiline/control escaping, final deduplication, measured timing, final JSON stdout | Native Windows terminal |
| Input ownership | Actual macOS controlling-PTY LF/CRLF, empty/slash/consecutive/delayed input, interrupts, EOF, approvals/reopen | Original owner incident may have additional triggers; native ConPTY |
| Reusable configuration | Private saved selections, literal env grammar, precedence, native permissions, spaces, export-free startup, settings atomicity | Windows ACL/drive-path native run |
| Startup lifecycle | Doctor revisions, no automatic interruption, active-work awareness, explicit graceful same-DB restart with actual owner-selected files, verified 2026-10-14 | Native Windows lifecycle |
| Read tools | All five real MiMo calls and fixture receipts verified; history required an isolated conversation | Provider-dependent tool selection remains a limitation |
| Confirmed delegation | Denial creates no work; separately approved exact action, real fixture WorkSession/turn IDs, duplicate decision rejection, atomic creation/receipt rollback tests and actual completed Codex 0.160.0 turn | Native Windows delegation |
| Conversation recovery | Actual combined CLI/release-daemon recovery retained real snapshots/IDs/actions without inference on macOS | Other native platforms |
| Types/invalid states | Named IDs, tagged previews, typed roles/tools/states, separate durable-message status, cross-field validation, strict current-record decoding | Further changes only if real acceptance exposes a defect |
| Regression | Native locked workspace checks and CLI/Kernel fixture acceptance; existing worker/profile/secret/migration tests | Native Windows and eventual Linux |
| Opt-in real acceptance | Explicit scope/exact approval received; 27 Actor requests and one Codex task; runner defaults to no quota | Native Windows real evidence |
| Phase 7 | Conditional inputs/provenance handoff below | Phase 6 evidence first; separate owner instruction |

## Required report

1. **Previous non-streaming cause.** The adapter requested streaming but returned
   only after `response.completed`; the Kernel committed/broadcast a complete
   snapshot, and the CLI rendered only those snapshots. Provider streaming alone
   did not make output visible.
2. **Architecture.** Private SSE parsing emits only bounded sanitized ephemeral
   previews. Validated final responses alone create durable messages/tool calls.
   Snapshot-first observation carries stable identities, revisions and preview
   sequences/checkpoints; the renderer reconciles final messages without reprint.
   Inference is Kernel-owned. Client disconnect never cancels it implicitly.
   [ADR 005](adr/005-sanitized-actor-previews.md) defines the contract.
3. **Redaction limits.** Known configured credentials and recognized sensitive
   patterns are withheld across chunk boundaries; ambiguous suffixes remain
   buffered. Sensitive/inconsistent/over-limit previews are discarded. Private
   frames/tool argument deltas never enter SQLite, ordinary logs or public events.
   This cannot detect every unknown or encoded secret. Already displayed safe
   preview cannot be recalled after a later failure and is explicitly unconfirmed.
4. **Duplicate prompt cause.** A real macOS PTY reproduced canonical CRLF becoming
   two newlines; a background stdin reader queued the empty line, producing
   adjacent prompts. One serialized raw-mode input owner now normalizes LF/CR and
   consumes paired LF; it restores terminal settings on exit. This establishes one
   reproducible cause, not proof of the owner's exact original incident.
5. **Configuration.** Both platforms use shared Rust `setup --local`, `config
   actor set --file`, `config actor env-file --file`, `doctor`, `up`. Selected
   files are never rewritten/migrated. Env values are literal allowlisted
   `KEY=VALUE`, never shell code. Process > trusted selected env file > saved
   non-secret settings > defaults. Unix secret files/directories require
   0600/0700; Windows uses owner-restricted ACLs, pending native acceptance.
   See [configuration](CONFIGURATION.md) and
   [ADR 006](adr/006-explicit-local-actor-files.md).
6. **Lifecycle.** Startup freezes validated configuration. Doctor compares
   observable non-secret JSON/persona revisions; credential values are not hashed
   or compared, so key rotation always needs explicit restart. Inspect open work,
   gracefully stop the owner's foreground launcher, then start with the same DB.
   Unknown/remote daemons are never killed or adopted. No hot reload was added.
7. **Type audit.** Wire DTOs stay distinct from Core/Provider models. Actor IDs,
   roles, states and tools are typed. Completed messages have a separate status
   dimension. Private `_message_format: 2` records reject active-only message
   status; versionless legacy metadata is normalized without losing IDs/bodies or
   action/turn uncertainty. Contradictory proposals, receipts, identities and
   preview generations have deterministic rejection tests.
8. **Real tool compatibility.** MiMo `mimo-v2.6-flash` emitted valid real
   function calls for all five reads. Two requests produced only prose rather than
   a mutation/history tool call; a further history request became Unknown without
   an action. Repeating a history read in the main conversation also omitted the
   call; the isolated new-conversation probe succeeded. This shows protocol
   compatibility with intermittent tool-selection/instruction-following limits,
   not reliable automatic delegation from every utterance. No omission was
   counted as execution and no synthetic receipts were supplied.
9. **Delegation.** Explicit owner rejection left zero workers. A separately
   proposed exact `codex/local`, `read_only` repository inspection was reviewed
   and explicitly approved by the owner. It created actual work
   `e51a67d9-18aa-49d3-9a04-21fb4be8f4ec` and turn
   `7592fbce-1b41-4788-bcc8-63e3932874e2`; the turn reached Completed with no
   terminal failure. Duplicate approval returned HTTP 409. Codex recorded four
   retryable provider-connection errors before recovering; several inspection
   commands returned nonzero status, so turn completion is not a claim that
   every command succeeded. No worker permission was auto-approved.
   Submission/decision ownership transfers before a cancellable storage await.
   Work creation and its Actor receipt commit atomically before backend launch;
   failure rolls both back. Bounded history advances only over retained records.
   Completed state is published only after the final body/timing metadata write;
   an injected failure leaves Unknown with its complete body preserved.
10. **Recovery.** Actual chat exit/reopen and graceful same-database daemon restart
    retained main-conversation snapshots/actions, the isolated read snapshot and
    terminal worker IDs/digests. The original partial attempt was also checked
    unchanged. The task-owned debug daemon stopped gracefully, and the installed
    release daemon started with saved Actor/env selections. Recovery used only
    read APIs; post-restart Actor model-call count stayed zero. Operational
    history remains separate from semantic memory.
11. **Checks.** `zsh scripts/check-native.zsh` runs locked workspace build, fmt,
    warnings-denied Clippy, Rust tests and native CLI/PTY fixtures. The runner
    itself is exercised with explicitly marked fake-provider evidence. Default
    `python scripts/actor_smoke.py` exits without model calls. Results/counts are
    recorded in [DEVLOG](../DEVLOG.md). Deterministic checks consume no quota.
    The separately authorized real run used
    27 Actor requests and one read-only Codex task across controlled attempts.
    Both local Rust binaries were updated; only the task-owned daemon was
    gracefully restarted.
12. **Platforms.** Native macOS is tested. Windows PowerShell/ACL/drive paths,
    Ctrl-Break and ConPTY remain unverified; run the native PowerShell entrypoint
    on Windows. Linux remains the eventual server target, currently unverified.
    Cross-compilation/emulation is not acceptance.
13. **Limitations.** No mid-inference provider resume, automatic mutation retry,
    worker auto-approval, short session aliases, conversation pruning, remote
    daemon management or public Internet deployment. Very fast responses may
    finish before the runner proves a preview preceded completion; report that as
    unverified. JSON ask remains final-only; work watch uses compact JSONL.
    Chat currently returns to input after the delegation receipt; later worker
    completion is not automatically delivered into the Actor conversation. An
    owner chat query creates another Actor turn to retrieve/summarize results;
    direct work/history observation does not require Actor inference. The owner's
    2026-10-14 walkthrough and desired result-delivery improvement are recorded
    in DEVLOG/ROADMAP, with no behavior change in that discussion.
14. **Readiness.** The native macOS real conversational/delegation/recovery
    evidence is available with the provider caveats above. Complete outstanding
    native platform checks before claiming cross-platform acceptance. Use ordinary
    commits while evidence is incomplete; reserve the
    Phase 6 milestone commit for actual acceptance. Phase 7 requires separate
    scope and must not begin automatically.

## Real latency and evidence (2026-10-14)

Two real self-introduction requests show why streaming and provider latency must
be measured separately. Times are monotonic measured milliseconds, not estimates.

| Measurement | First attempt | Second attempt |
|---|---:|---:|
| Provider first byte | 57 | 134 |
| Provider first useful text delta | 763 | 6734 |
| CLI first visible useful text | 785 | 6760 |
| Provider completion | 4726 | 16463 |
| Durable-body commit | 1 | 1 |
| CLI wall time | 4775 | 16522 |

The provider's first text was much later in the second request; the CLI still
showed it before completion. No raw provider frames, credentials or full
conversation content were retained in test summaries. Per-user evidence files:
`acceptance/phase-6-5-2026-10-14.json` (original partial run),
`acceptance/phase-6-5-2026-10-14-second.json` (main records plus explicit supplemental
reference and verified restart), and `acceptance/phase-6-5-2026-10-14-history.json`
(isolated real read). These live under the private local Lain directory, outside
Git. Evidence aggregates unique turn IDs rather than counting the supplemental
reference twice. Real acceptance is evidence across those controlled checks,
not a promise that the configured model will always choose a requested tool.

## Conditional Phase 7 inputs

This is a planning handoff, not an implementation or authorization to start.
[MEMORY.md](MEMORY.md) remains normative.

| Future input | Provenance and acceptance boundary |
|---|---|
| Explicit owner-authored memory | Authenticated owner statement; retain exact ConversationId/MessageId or manual source, time, scope and explicit origin; allow inspect/delete/disable |
| Conversation summary | Selected completed durable messages/actions with source IDs and revision; exclude ephemeral previews, secrets and unconfirmed statements; summary is not automatically memory |
| Session summary | Confirmed worker terminal result plus selected normalized operational evidence, WorkSessionId/TurnId/EventIds; distinguish failure and uncertainty from success |
| Project knowledge | Workspace/project scope, supporting artifact/commit/decision references and dates; conflicts/supersession explicit, no automatic global promotion |
| Memory candidate | Origin, content, scope, evidence, explicit/inferred distinction and review outcome; model inference remains a candidate until accepted |

Actor context is bounded inference input, not a durable knowledge base. Actor
conversation history and worker operational history remain independent sources.
No automatic transcript promotion, embeddings, vector database, autonomous
extraction, journals, World or background consolidation was implemented here.
