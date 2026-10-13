# Phase 6.5 acceptance and conditional Phase 7 handoff

Status: implementation/fixture acceptance; **real combined acceptance incomplete**.
Recorded 2026-10-13 on native macOS. Windows/ConPTY and Linux are unavailable and
unverified. Phase 7 remains unstarted. No milestone completion is claimed.

## Evidence by requirement

| Requirement | Current evidence | Remaining acceptance |
|---|---|---|
| Incremental output/parser | Fragmented UTF-8/multiline SSE, private tool deltas, completed-only fallback, malformed/failure/incomplete tests; gated native CLI preview before completion | Real configured provider preview |
| Safe preview contract | Bounded sanitizer/queues; turn/preview/message IDs and sequences; checkpoint reconnect, slow subscriber, wrong-generation/late-preview tests | Real-provider latency/compatibility |
| CLI renderer/timing | Native Unicode/multiline/control escaping, final deduplication, measured timing, final JSON stdout | Native Windows terminal |
| Input ownership | Actual macOS controlling-PTY LF/CRLF, empty/slash/consecutive/delayed input, interrupts, EOF, approvals/reopen | Original owner incident may have additional triggers; native ConPTY |
| Reusable configuration | Private saved selections, literal env grammar, precedence, native permissions, spaces, export-free startup, settings atomicity | Windows ACL/drive-path native run |
| Startup lifecycle | Doctor revisions, no automatic interruption, active-work awareness, explicit graceful same-DB restart | Owner's actual configured daemon restart |
| Read tools | All five tools verified through local Responses/Codex fixtures and real Kernel receipts | Real provider structured function calling |
| Confirmed delegation | Denial creates no work; separately approved exact action, real fixture WorkSession/turn IDs, duplicate decision rejection, atomic creation/receipt rollback and startup-failure tests | Authorized actual Codex turn and terminal result |
| Conversation recovery | Actual CLI exit/reopen and daemon restart retain snapshots/IDs/actions without inference | Combined real provider/worker recovery |
| Types/invalid states | Named IDs, tagged previews, typed roles/tools/states, separate durable-message status, cross-field validation, strict current-record decoding | Further changes only if real acceptance exposes a defect |
| Regression | Native locked workspace checks and CLI/Kernel fixture acceptance; existing worker/profile/secret/migration tests | Native Windows and eventual Linux |
| Opt-in real acceptance | Existing runner extended and exercised with explicitly marked fixtures; no quota in routine tests | Owner provider/workspace authorization and exact proposal decisions |
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
8. **Real tool compatibility.** Unverified. The owner previously reported ordinary
   Chinese dialogue with `mimo-v2.6-flash`; that establishes neither function-call
   schema support nor current preview behavior. The opt-in runner verifies all
   five read tools against actual Kernel results and reports incompatibility.
9. **Delegation.** Local fixture denial/approval, independent worker turn,
   duplicate rejection and uncertain partial receipts pass. Real combined Actor
   and Codex delegation is pending authorization and exact owner confirmation.
   Submitted work is never described as a completed worker turn.
   Submission/decision ownership transfers before a cancellable storage await.
   Work creation and its Actor receipt commit atomically before backend launch;
   failure rolls both back. Bounded history advances only over retained records.
   Completed state is published only after the final body/timing metadata write;
   an injected failure leaves Unknown with its already complete body preserved.
10. **Recovery.** Deterministic native CLI and daemon restart tests retain Actor
    snapshots/actions and terminal worker IDs without model regeneration. The
    real runner compares IDs and snapshot hashes after an owner-attested graceful
    restart; this real stage remains pending. Operational history is not memory.
11. **Checks.** `zsh scripts/check-native.zsh` runs locked workspace build, fmt,
    warnings-denied Clippy, Rust tests and native CLI/PTY fixtures. The runner
    itself is exercised with explicitly marked fake-provider evidence. Default
    `python scripts/actor_smoke.py` exits without model calls. Results/counts are
    recorded in [DEVLOG](../DEVLOG.md). No paid calls or owner daemon changes were
    made for these implementation checks.
12. **Platforms.** Native macOS is tested. Windows PowerShell/ACL/drive paths,
    Ctrl-Break and ConPTY remain unverified; run the native PowerShell entrypoint
    on Windows. Linux remains the eventual server target, currently unverified.
    Cross-compilation/emulation is not acceptance.
13. **Limitations.** No mid-inference provider resume, automatic mutation retry,
    worker auto-approval, short session aliases, conversation pruning, remote
    daemon management or public Internet deployment. Very fast responses may
    finish before the runner proves a preview preceded completion; report that as
    unverified. JSON ask remains final-only; work watch uses compact JSONL.
14. **Readiness.** Finish authorized real tool/delegation/restart acceptance and
    outstanding native checks before claiming the conversational foundation
    accepted. Use ordinary commits while evidence is incomplete; reserve the
    Phase 6 milestone commit for actual acceptance. Phase 7 requires separate
    scope and must not begin automatically.

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
