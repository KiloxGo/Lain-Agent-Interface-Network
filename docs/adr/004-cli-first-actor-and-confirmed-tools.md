# ADR 004: CLI-first Actor, durable conversations and owner-confirmed tools

Date: 2026-10-12. Status: implemented; cross-platform/real-provider acceptance pending.

Phase 6.5 extensions: [ADR 005](005-sanitized-actor-previews.md) defines sanitized
ephemeral previews; [ADR 006](006-explicit-local-actor-files.md) defines explicit
saved launcher configuration and credential-source files.

## Decision

Pause Web development and prioritize shared Rust CLI/Kernel capabilities. Retain
legacy Web routes/assets and compatibility checks. Windows PowerShell and macOS
zsh are first-class development platforms; Linux remains the server target.

The CLI may explicitly configure/launch a local laind, using private per-user
settings and a separate OS-random API token. It never embeds a Kernel or launches
workers directly. Environment-only remote access remains supported. Unix private
modes/owners and native Windows SID/DACL checks live behind the local filesystem
boundary. Foreground launch shares native console/process-group shutdown signals.
Never adopt/kill a process by guessed PID or silently forward a local token to a
remote override. Native build/test/smoke evidence is required separately per OS;
cross-compilation or fixtures cannot substitute for native acceptance.

Lain Actor lives in laind, with explicit model configuration separate from worker
profiles and Codex login. Use one small direct Responses adapter with SecretRef,
strict function arguments, bounded requests/tools/time/bytes/tokens and no generic
coding tools. Persona is versioned and optionally owner-overridden at startup.
The model cannot mutate persona or configuration.

Persist Lain conversations and indexed Actor turns in migration 005, separately
from WorkSession, operational worker history and future semantic memory. Accepted
request IDs and messages commit before inference. A request ID identifies exactly
one text payload in a conversation. Inference is detached from HTTP observers;
restart/interruption marks unfinished execution Unknown, with no automatic replay.

Expose a closed typed registry of reads and work mutations backed by existing
Kernel/Supervisor services. Every mutation is a durable proposal, requires an
explicit authenticated owner decision, and atomically enters Executing before
external dispatch. No model tool can confirm actions or worker permission
requests. A create+task delegation retains its real WorkSession ID before prompt
submission. Partial/uncertain outcomes preserve provenance and block replay of
that action. Accepted submission is never advertised as worker completion.

Consume provider SSE, but expose complete redacted, committed message/action/turn
snapshots. Raw token deltas could split credentials across frames; initial safety
and recovery take precedence over token-by-token display. Reconnect resnapshots
the same turn with message IDs/revisions; true mid-inference resumption is deferred.

## Consequences

No credentials in conversation/model context/normal logs/history. Namespaced env
secrets require daemon restart for rotation. Owner persona/configuration remains
inspectable. Workers keep their own coding intelligence, provider bindings,
approvals and lifecycle. Existing explicit work/profile commands function without
Actor model configuration. Phase 7 semantic memory/World/summarization stays
unimplemented. Fixture and native acceptance, and paid provider acceptance, are
reported separately. See CLI.md, LAIN_ACTOR.md and DEVLOG.md for evidence/limits.

## Phase 6.5 reliability strengthening (2026-10-13)

Submission/confirmation transfer to a Kernel-owned task before their first
storage await. A disconnected HTTP caller cannot leave committed Running or
Executing state without a task responsible for its terminal/uncertain outcome.
Duplicate durable requests/decisions still never launch another operation.

Confirmed work creation atomically commits the WorkSession, worker metadata,
creation event and Actor partial receipt under the existing Kernel storage lock.
The receipt is validated against the exact Executing work.create action and is
committed before backend startup. Receipt failure rolls back work creation;
startup failure/cancellation retains an inspectable ID without replay. This
strengthens the existing partial-receipt requirement rather than adding another
worker orchestrator. Native deterministic regressions include cancelled callers,
startup failure and injected receipt-commit failure.
