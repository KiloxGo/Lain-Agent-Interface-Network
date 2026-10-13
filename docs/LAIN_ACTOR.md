# Lain Actor — first voice

The Kernel owns Lain's conversation, model lifecycle and typed work decisions.
The CLI is an HTTP/SSE text client. Lain is independent of Codex's worker model,
login, native threads, context management and coding tools. This phase adds no
semantic memory, World, scheduler, auto-routing or generic coding-agent loop.
The existing Web client remains frozen and has no Actor UI.

## Configure a real model explicitly

For normal use, keep an owner-managed JSON file outside the source checkout, in
the per-user directory described below. For checkout-based development, copy
[`config/actor.example.json`](../config/actor.example.json) to `config/actor.json`.
The example is tracked; `/config/actor.json` is explicitly ignored by Git. Neither
file contains an API key: `secret_ref` names an environment variable. Review
`model` and `endpoint` before use and set them to your provider's actual
Responses-compatible model and base URL.

The configuration has this shape:

```json
{
  "driver": "openai-responses",
  "model": "your-supported-model",
  "endpoint": "https://api.openai.com/v1",
  "secret_ref": "secret://env/actor",
  "reasoning_effort": "low",
  "timeout_seconds": 45,
  "max_output_tokens": 2048,
  "max_turn_tokens": 16384
}
```

The endpoint is a base URL; the private adapter appends `/responses`. Only HTTPS
or loopback HTTP is accepted, without user info/query/fragment. Reasoning effort
is optional, sent only when configured; actual support is provider/model-owned.
No default paid model, Codex credential reuse, automatic routing or fallback.
Use a provider/model that supports Responses function calling and SSE.

### Configure from a checkout

Run these commands from the repository root. macOS zsh:

```zsh
cp -n config/actor.example.json config/actor.json
chmod 600 config/actor.json
# Edit model and endpoint in config/actor.json before starting the daemon.
export LAIN_ACTOR_CONFIG="$PWD/config/actor.json"
read -rs 'LAIN_SECRET_ACTOR?Actor API key: '
printf '\n'
export LAIN_SECRET_ACTOR
lain up
```

Windows PowerShell (the API key is entered through a secure prompt):

```powershell
if (!(Test-Path 'config/actor.json')) {
    Copy-Item 'config/actor.example.json' 'config/actor.json'
}
# Edit model and endpoint in config/actor.json before starting the daemon.
$env:LAIN_ACTOR_CONFIG = (Resolve-Path 'config/actor.json').Path
$env:LAIN_SECRET_ACTOR = [System.Net.NetworkCredential]::new('', (Read-Host 'Actor API key' -AsSecureString)).Password
lain up
```

Stop an existing `lain up` with Ctrl-C before starting it in this configured
terminal; an already-running Kernel does not reload the caller's environment.
In another terminal, run `lain doctor`, then `lain chat`. The secure/hidden prompt
keeps the key out of command history; the key is inherited by the daemon only
through its environment. This entrypoint does not persist the key across new
shells. `.env` is ignored by Git but **is not automatically loaded**. Actor JSON
also is not discovered automatically: `LAIN_ACTOR_CONFIG` must be set when the
Kernel starts. The documented PowerShell commands have not been verified on a
native Windows host.

### Configure outside the checkout

PowerShell, in the daemon terminal (supply the credential privately):

```powershell
$env:LAIN_ACTOR_CONFIG = Join-Path $env:LOCALAPPDATA 'Lain\actor.json'
# Set LAIN_SECRET_ACTOR through your private deployment environment.
lain up
```

macOS zsh:

```zsh
export LAIN_ACTOR_CONFIG="$HOME/Library/Application Support/Lain/actor.json"
# Supply LAIN_SECRET_ACTOR through your private deployment environment.
lain up
```

`secret://env/actor` resolves only `LAIN_SECRET_ACTOR`, through the existing
namespaced SecretRef store. The credential goes only in the provider bearer
header. No raw credential is persisted in configuration, conversation, profiles,
worker history or public events. Restart the daemon to change environment-backed
values/configuration. Doctor validates local prerequisites without an inference
or account probe. Missing/invalid Actor setup leaves explicit work/profile APIs
usable; chat reports how to configure it. Keep credentials out of ordinary
terminal history and model text. Known values and recognizable secret patterns
are rejected/redacted before context/persistence/publication; arbitrary unknown or
encoded secrets cannot be detected reliably.

## Identity and context

`identity/persona.md` is the inspectable versioned baseline, compiled into laind.
`LAIN_PERSONA_PATH` optionally selects an owner-editable UTF-8 file at startup
(maximum 8 KiB). Model output cannot rewrite it. Rebuild for baseline changes or
restart for an override change. It supports natural Chinese/Japanese/English,
factual transparency and a calm resident voice without fabricated memory.

The deterministic context builder selects a recent window from the latest 20
turns, with a 48 KiB history budget. It retains the current request (at most 8 KiB),
recent user/assistant text and compact recorded tool provenance; older groups drop
as units. Persona and allowed tool definitions are explicit. Current worker state
is retrieved on demand; all work/history is never dumped into every prompt.
Tool/results and historical text are untrusted data, not authorization.

The direct adapter sends `store:false`, strict function schemas,
`parallel_tool_calls:false`, a bounded output budget and optional reasoning.
Provider tool names use underscores (`work_list`); the boundary maps to stable
Lain names (`work.list`). Encrypted reasoning continuation, when returned, stays
transient in that turn's adapter input, is bounded, and is never durable/public.
This small implementation follows the official
[Responses function-calling contract](https://developers.openai.com/api/docs/guides/function-calling)
verified on 2026-10-12; fixture acceptance verifies wire mapping, not a live account.

## Durable conversation domain

Migration 005 creates `lain_conversations(id, created_at_ms, updated_at_ms)` and
indexed `lain_actor_turns(sequence, id, conversation_id, request_id, snapshot)`.
Conversation, ActorTurn, Message and Action have distinct Lain UUID newtypes.
They never use a Codex thread ID. A turn snapshot holds messages (ID, conversation,
role, content, timestamp, status), typed actions/results, error, revision and
request/token counters. A row is bounded to 256 KiB. Retention is indefinite;
explicit pruning is deferred. Older work/profile/history schema migrates without
changing worker bindings or IDs.

Accepted user text and the request identity commit before inference is scheduled.
`(conversation_id, request_id)` is unique. Same ID/text returns the same turn;
different text conflicts. Only one Running/PendingConfirmation turn is permitted
per conversation. Other conversations can run independently. Completed text/tool
results commit before public notification. Stable message IDs and monotonic
revision make repeated snapshots safe to merge.

Turn states: Running, PendingConfirmation, Completed, Failed, Cancelled, Unknown.
Transport/inference uncertainty and interruption currently use Unknown; Failed is
reserved for conclusive future evidence. Startup marks Running turns/Executing
actions Unknown, preserving partial results; no inference or mutation is replayed.
Pending proposals survive restart and may be explicitly approved/denied later;
Supervisor revalidates current workspace/profile policy at execution. Completed
conversation records survive CLI exit and daemon restart. Operational conversation
continuity is separate from worker history and future semantic Lain Memory.

## Closed tool registry and confirmation

Read-only: `work.list`, `work.inspect`, `work.history`, `profile.list`,
`profile.inspect`. Actor profile views expose credential availability rather than
SecretRef handles; owner profile CLI/API views retain those handles. Mutating: `work.create`, `work.send`, `work.cancel`,
`work.resume`. Each has a strict argument shape, deny-unknown-fields parsing,
text/path/cursor validation, bounded output and a traceable Action record.
Lists return at most 50 entries; history at most 20 rows; tool results at most
16 KiB, with honest JSON truncation/redaction. Exact IDs/pagination remain available
through explicit CLI/API for larger histories.

There is no shell, filesystem, HTTP, SQL, worker-approval or profile-mutation tool.
The Actor uses Kernel reads and the existing Supervisor for every work mutation.
Workspace profile allowlists, frozen profile semantics, missing/disabled profiles,
backend availability and worker approvals retain their existing enforcement.

Every mutation first becomes a persisted Proposed action. Authenticated owner
confirmation atomically changes Proposed to Executing before dispatch. Duplicate
or stale decisions conflict. Denial records Rejected and performs no mutation.
The model cannot call the owner-decision endpoint; prose cannot grant privileges.
No blanket approval or prior-authorization policy is introduced.

`work.create` is one confirmed delegation containing exact absolute daemon-host
workspace, profile, sandbox and task. Supervisor creates real work; its ID is
persisted in the Action's partial result **before** submitting the task. Submission
returns the real worker Turn ID. The receipt states `worker_completion:not_observed`;
acceptance is not proof of execution completion. Later `work.inspect/history` use
actual Kernel state. A partial failure retains the created WorkSession ID. Unknown
outcomes are never automatically replayed. Even confirmed worker cancellation
still follows the existing acknowledgement-versus-terminal-evidence contract.

## Runtime limits and stream recovery

Per user turn: at most 4 model requests, 4 tool invocations, 2 tool errors, 120
seconds per active inference/execution phase (at most two phases, 240 seconds
cumulative, excluding time waiting for owner confirmation), configured cumulative reported token budget (default 16,384),
configured output per response (128–8192 tokens), 16 KiB assistant text per response,
128 KiB provider request/output continuation, 256 KiB stream frame and 2 MiB total
provider stream. Request timeouts are 1–90 seconds. Reaching a bound stops that
turn honestly; there is no automatic retry loop or automatic mutation retry.
Usage budgeting depends on usage fields supplied by the provider; request/output
counts, sizes and timeout bounds apply even if usage is absent.

Provider SSE is consumed incrementally, including fragmented UTF-8 and multiline
SSE data. The Kernel publishes bounded, ephemeral `preview` events for sanitized
assistant text, separate from durable `turn` snapshots. Preview identity includes
turn, preview and eventual message IDs, with a per-preview sequence. Possible
credential prefixes are withheld until unambiguous; a sensitive, oversized or
inconsistent preview is discarded. Tool argument deltas and reasoning stay private.
Only a validated complete response can commit messages or dispatch typed tools.

Streams start with an authoritative turn snapshot and may include a live preview
checkpoint. Reconnect restores current state rather than replaying provider tokens.
The current CLI consumes committed snapshots only: incremental terminal rendering
is still pending. Recorded monotonic timing fields cover queue and provider-request
milestones when observed; they do not yet measure CLI display latency. Inference lifecycle
is Kernel-owned: closing an HTTP/SSE connection does not cancel the turn or worker.
The client reconnects to the same turn and deduplicates ID/revision. Explicit
Actor cancel interrupts local inference; possibly dispatched work is Unknown and
requires inspection. Existing workers are not cancelled by Actor interruption.

## API

All routes use the existing owner bearer authentication and body limits.

| Endpoint | Behavior |
|---|---|
| GET `/v1/diagnostics` | Safe readiness/storage/backend/profile/Actor report; no model call |
| GET/POST `/v1/conversations` | Latest 100 metadata records / create conversation |
| GET `/v1/conversations/{id}?after=0&limit=20` | Forward indexed turn page, limit 1–100 |
| GET `/v1/conversations/{id}?recent=true` | Latest 20 turns, chronological, current/pending recovery |
| POST `/v1/conversations/{id}/turns` | `{request_id, text}`; 202 accepted durable turn |
| GET `/v1/actor-turns/{id}` | Authoritative turn snapshot |
| GET `/v1/actor-turns/{id}/events` | Snapshot first, then ephemeral previews and committed revisions; closes on non-Running state |
| POST `/v1/actor-turns/{id}/actions/{action}/decision` | `{approve: true/false}`; explicit owner decision |
| POST `/v1/actor-turns/{id}/cancel` | Explicit interruption/pending-proposal cancellation |

SSE lag emits `resync_required`; resnapshot the same turn. Event streams are not
provider replay cursors. HTTP 409 indicates duplicate decisions/changed idempotent
payload or unfinished-conversation conflict. Actor unavailable uses 503; explicit
work/profile commands still work. Extractor/operation errors never echo raw model
or rejected credential-bearing payloads.

## Acceptance and Phase 7 boundary

Ordinary Rust/native smoke checks use deterministic model injection, a real local
fixture Responses SSE server and the existing fake Codex process, consuming zero
model quota. They cover persistence/restart, request deduplication, structured
reads, confirmation/rejection/races, policy enforcement, partial delegation,
malformed calls, budgets, cancellation, context bounds, secrets and reconnect.
Native acceptance is in [CLI.md](CLI.md); actual platform evidence is in DEVLOG.md.

Paid smoke is explicit, never part of ordinary tests/CI:

```text
python scripts/actor_smoke.py --run-real --workspace <absolute-daemon-workspace>
```

It requires a real configured Actor and supported authenticated Codex. It checks
ordinary model conversation, typed work listing, an exact proposed delegation,
interactive owner confirmation, a real WorkSession receipt, later inspection and
conversation continuity between separate CLI processes. Owner denial leaves
acceptance incomplete. It does not stop a user's daemon or automatically approve
worker permission requests.

Then manually `lain chat --conversation <id>`, converse, `/exit`; observe the
worker independently with `lain work watch <id>`. Gracefully stop/restart `lain up`
with the same private database and resume chat/conversation. Verify the worker's
actual execution and retained history rather than treating a submission receipt
as completion. Record real-model, chat, independent-worker and restart acceptance
separately. No configured credential or no execution means unverified, never passed.

Deferred: token delta display, mid-inference resumption, conversation pruning,
short worker references, multiple Actor providers, server installation and native
Linux acceptance. Phase 7 memory candidates/summaries/retrieval/World are not begun.
