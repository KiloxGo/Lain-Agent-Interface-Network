# Lain Actor — first voice

## Q2-B1 private source lookup — 2026-10-25

Q2-B1 adds the independent `source.search`/`source.inspect` read capabilities (provider function names `source_search`/`source_inspect`) only when an admitted private turn has a currently valid source grant. They accept bounded keywords, a narrower optional receipt-time window, and actual IncomingIds/cursors. Kernel supplies fresh, attributed `untrusted_source_evidence`; receipts contain IDs and pagination metadata, not source text. Every prior selection is revalidated before further inference and after responses. Historical instructions cannot approve Work, Worker permissions or memory writes. Existing four-request/four-tool, token and wall-clock budgets still apply. See [MEMORY.md](MEMORY.md) and [Q2-B1 acceptance](Q2_B1_ACCEPTANCE.md).

## Target relationship and current runtime

2026-10-23: [PRODUCT.md](PRODUCT.md), [CHAT_DESIGN.md](CHAT_DESIGN.md) and
[ADR 009](adr/009-cloud-resident-equal-companion.md) define equal companionship,
QQ-first delivery, natural recall and bounded autonomous activities. Future
standing grants replace universal per-action review as a product target, not by
removing current checks. Source observation does not itself authorize action.

The rest of this file documents the existing Phase 6–7.1 Actor, now also used by
the Q1 channel service and Q2-A private-memory capabilities. The Actor itself has no QQ protocol, general standing grants
or autonomous daily activities. Its request/tool/time
bounds remain actual current limits, not a requirement to use the identical turn
budget for every future background activity. New budgets require explicit design.

The compiled `identity/persona.md` v3 uses equal interaction, calm/offbeat language
and truthful experiences while preserving ordinary per-action review. Q1 reply
authorization never confirms tool proposals: QQ receives a local-review notice,
and chat text such as “同意” cannot approve an action. The separate Q2-A memory
grant is the only newly implemented exception; see [MEMORY.md](MEMORY.md),
[ADR 010](adr/010-scoped-private-conversation-memory.md) and [QQ_CHANNEL.md](QQ_CHANNEL.md).

C1 assembles identity, always-appended `identity/actor-policy.md`, and private-only
`identity/private-behavior.md` / `identity/private-expression.md` separately. A
custom persona cannot remove the Kernel tool/permission constraints. Configured
diagnostics keep `persona_revision` for identity content and add
`private_chat_revision` over the identity/policy/behavior/expression bundle.
New private turns expose typed `chat_wait` and `chat_finish` with Kernel-enforced
finite decisions; terminal turns do not receive these tool definitions or private
style instructions. Original requests, actual effects, unsent drafts and transport
receipts stay distinct. See [ADR 012](adr/012-bounded-private-conversation.md).

### Q2-A capabilities

For a channel turn with a currently valid explicit memory grant, the Kernel adds
`memory.recall`, `memory.remember`, `memory.correct` and `memory.forget` (wire names
use underscores). Fresh source/record evidence is added as attributed user-role
data at each request, never instructions or authorization. The existing four model
requests/four tools, time and token budgets apply. There is no separate extraction
request or background model task. Effect tools proceed as Executing under the
narrow grant; ordinary memory proposals and Work mutations still require review.
Current grants, sources, single inspected targets and revisions are checked in
storage before effects. Revocation/expiry removes the next request's private
tools/evidence and rejects pending effects. Historical private tool arguments and
record bodies are omitted from model observations; correlated committed effect
metadata remains truthful without replaying forgotten content. Fresh authorized
memory evidence also supplies the binding's latest three IDs-only committed
effects, separately marked as Kernel operation metadata. Empty active recall does
not negate a prior write or deletion; semantic records are always refreshed. Dialogue
and audit remain separately retained. Generic tools cannot read/write Channel
scope. The Actor cannot establish a grant or approve Worker permissions.

## Existing Phase 6–7.1 implementation

The Kernel owns Lain's conversation, model lifecycle and typed work decisions.
The CLI is an HTTP/SSE text client. Lain is independent of Codex's worker model,
login, native threads, context management and coding tools. Phase 7.1 adds explicit, scoped semantic memory tools and owner-reviewed proposals.
World, scheduler, auto-routing and generic coding-agent loops remain deferred.
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

### Save an explicit local selection

Run `lain setup --local`, then record an existing non-secret file:

```text
lain config actor set --file <path-to-actor.json>
lain config actor
```

The path is resolved explicitly and saved in private per-user `client.json`.
The selected file is never copied, overwritten or silently migrated. Your ignored
`config/actor.json` can remain in place; a normal installation can select a file
outside the checkout. `config actor show` is equivalent to `config actor` and
shows selection/source/revisions/credential availability, never credential values.
`config actor clear` removes saved selections while retaining the files; explicit
process environment still takes precedence.

### Reuse credentials without repeated shell exports

Use a dedicated owner-only env file outside every Git checkout, in a private
owner-only directory. The existing per-user Lain directory is suitable.
[`config/actor.env.example`](../config/actor.env.example) is a credential-file
template containing a placeholder only. Copy its contents to the protected file
below and replace the placeholder in your editor; never put a real key in the
tracked example or select an env file inside the source checkout.
macOS zsh, after setup:

```zsh
local_dir="$HOME/Library/Application Support/Lain"
secret_file="$local_dir/actor.env"
if [[ ! -e "$secret_file" ]]; then
  (umask 077; set -C; : > "$secret_file")
fi
chmod 600 "$secret_file"
```

Windows PowerShell, after setup (native Windows verification is still pending):

```powershell
$secretFile = Join-Path $env:LOCALAPPDATA 'Lain\actor.env'
if (!(Test-Path -LiteralPath $secretFile)) {
    ([System.IO.File]::Open($secretFile, [System.IO.FileMode]::CreateNew,
        [System.IO.FileAccess]::Write, [System.IO.FileShare]::None)).Dispose()
}
$sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
$acl = New-Object System.Security.AccessControl.FileSecurity
$acl.SetOwner($sid)
$acl.SetAccessRuleProtection($true, $false)
$rule = New-Object System.Security.AccessControl.FileSystemAccessRule($sid, 'FullControl', 'Allow')
$acl.AddAccessRule($rule)
Set-Acl -LiteralPath $secretFile -AclObject $acl
```

In your editor, put the following line in that protected file, replacing the
placeholder with the actual API key. Do not enter the key in command arguments:

```dotenv
LAIN_SECRET_ACTOR=replace-in-editor
```

Then register the file once. zsh:

```zsh
lain config actor env-file --file "$secret_file"
lain up
```

PowerShell:

```powershell
lain config actor env-file --file $secretFile
lain up
```

Future `lain up` launches load both saved selections automatically. No repeated
exports are required. For a single launch, use `lain up --env-file <path>`;
`lain doctor --env-file <same-path>` compares that selection without saving it.
File contents are never evaluated as shell code. Parsing accepts UTF-8 LF/CRLF,
blank lines, whole-line `#` comments and unquoted `KEY=VALUE` entries. Values are
literal, including `$`, `=`, spaces and `#`; no interpolation, quote removal,
inline comments, `export`, commands or arbitrary environment keys are supported.
Duplicate keys, empty/control-bearing/oversized values and malformed entries fail.

Allowed keys are `LAIN_ACTOR_CONFIG`, `LAIN_PERSONA_PATH` and namespaced
`LAIN_SECRET_*` variables. `LAIN_API_TOKEN`, PATH, listener/database overrides and
executable selection are not accepted from this file. Relative config/persona
paths in an env file resolve against that explicitly chosen file's directory.
Explicit process paths resolve before the launcher changes its working directory.
The file is at most 64 KiB/128 entries; each value at most 8192 bytes.

Precedence: explicit process environment > explicitly selected/saved env file >
saved non-secret Actor file selection > built-in defaults. `--env-file` selects
which file is used instead of the saved env-file path. Empty explicit process
values do not fall back silently. No workspace/current-directory `.env` discovery
occurs. Unsafe owners, links, permissions/ACLs or checkout locations fail closed;
Unix files/directories require 0600/0700, Windows requires protected owner-only ACLs.

### Apply changes safely

Configuration/persona/environment is resolved at daemon startup. `doctor` compares
SHA-256 revisions of validated non-secret Actor JSON bytes and persona content
with the authenticated daemon. Changed JSON formatting can conservatively request
a restart. Older daemons without revision metadata are `not_observable`; remote
mode does not compare local files. This is not a provider/auth probe. Credentials
are never hashed or compared, so rotating a key still requires explicit restart.

Inspect `lain work list` before stopping active work. In the terminal running your
owned foreground `lain up`, use Ctrl-C (Windows also supports Ctrl-Break), wait
for graceful shutdown, then run `lain up` again with the same local setup/database.
`up` reports a configuration mismatch and open worker count rather than stopping
an existing daemon. No guessed PID, unknown service or remote daemon is killed.
Conversations and public IDs remain intact; worker recovery follows its existing
uncertainty rules and never automatically replays mutations. There is no hot reload.

Direct/manual `laind` deployments may still supply `LAIN_ACTOR_CONFIG`,
`LAIN_PERSONA_PATH` and namespaced credentials through their deployment environment.
The env file is a launcher input; it is not implicitly read by direct `laind`.
`secret://env/actor` resolves only `LAIN_SECRET_ACTOR`. An optional protected env
file is an explicit plaintext credential source; no values are stored in
`client.json`, Actor JSON, SQLite conversations/profiles/history, ordinary logs or
public events. The credential goes only in the provider bearer header. Codex login
remains separate. Missing/invalid Actor setup leaves explicit work/profile APIs
usable. Known values/patterns are rejected or redacted; unknown or encoded secrets
cannot be detected reliably.

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

Message roles are typed; durable message status is a separate `MessageStatus`
dimension with only `completed`. A complete user, assistant or tool body remains
complete even if enclosing inference or execution becomes Unknown. Private stored
JSON records now carry `_message_format: 2`, absent from public snapshots/domain
entities. Versionless Phase 6 records normalize their legacy enclosing-turn status
on message bodies without changing IDs, text, actions or turn uncertainty. New
records reject contradictory statuses instead of silently normalizing them.

Accepted user text and the request identity commit before inference is scheduled.
The submission and confirmation handoff are Kernel-owned before their first
storage await; losing the HTTP caller cannot strand a committed Running turn or
Executing action without its execution task. Repeated requests/decisions still
deduplicate or conflict through durable state, never launch a second operation.
`(conversation_id, request_id)` is unique. Same ID/text returns the same turn;
different text conflicts. Only one Running/PendingConfirmation turn is permitted
per conversation. Other conversations can run independently. Completed text/tool
results commit before public notification. Stable message IDs and monotonic
revision make repeated snapshots safe to merge.
Final body persistence precedes its measured commit-time metadata; the turn stays
Running until that final metadata/terminal-state write succeeds. A reconnect
between those writes cannot observe premature Completed state. If the last write
fails, the complete body may remain durable while the turn becomes Unknown.

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
workspace, profile, sandbox and task. The real WorkSession, worker metadata,
creation event and Action partial receipt commit in one transaction **before**
backend startup or task submission. The Kernel verifies that receipt belongs to
the exact confirmed action. Failure to commit the receipt rolls back creation;
startup failure retains its inspectable ID. Submission
returns the real worker Turn ID. The receipt states `worker_completion:not_observed`;
acceptance is not proof of execution completion. Later `work.inspect/history` use
actual Kernel state. A partial failure retains the created WorkSession ID. Unknown
outcomes are never automatically replayed. Even confirmed worker cancellation
still follows the existing acknowledgement-versus-terminal-evidence contract.

Bounded `work.history` results retain a cursor only through returned events and
set `has_more` when additional records were omitted. An event that cannot fit is
reported as a bounded-result fallback; it cannot silently move the cursor past
unseen history. Complete explicit CLI pagination remains available.

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
The CLI renders sanitized previews while inference is running, then reconciles
the completed message by stable ID and content prefix without printing it twice.
Interrupted/discarded previews are visibly marked as unconfirmed. Sequence gaps
fall back to authoritative completed output; checkpoints suppress overlap.
`lain chat --timing` and `lain ask --timing` report measured submission, first
visible text, provider milestones, commit and wall-clock durations on stderr.
Missing observations remain null; JSON ask stdout remains one final snapshot.
Inference lifecycle
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

## Phase 6.6 automatic delegated results

Confirmed work.create/work.send now records the source conversation/action and
specific worker Turn before external prompt submission. The exact proposed task
is submitted verbatim and remains inspectable with its workspace/profile/receipt
through `lain delegations <conversation-id>`. No prompt compiler is active.
Actor mutation confirmation and worker permission approval remain separate.

The Kernel persists a separate ConversationNotice when normalized evidence shows
Completed, Failed, Cancelled or Unknown. A technically Completed Turn can include
failed commands; notices report these separately. The factual excerpt includes
available final assistant output, safe errors, changed paths and truncation,
with worker identities and a command to inspect full operational history. It
does not infer task correctness. Completion does not reopen an ActorTurn, create
a synthetic message, consume model quota or wait for another owner message.
Optional model interpretation remains deferred.

| Authenticated API | Contract |
|---|---|
| GET `/v1/conversations/{id}/delegations` | Exact non-secret dispatch links/receipts |
| GET `/v1/conversations/{id}/notices?after=0&limit=50` | Ordered notice page, next_cursor, has_more; limit 1–100 |
| GET `/v1/conversations/{id}/notices/events?after=0` | Durable notice-page SSE; stays open beyond ActorTurn completion |
| POST `/v1/conversations/{id}/notices/{notice}/ack` | Idempotent owner display acknowledgement; no worker mutation |

Notices and establishing worker/history writes commit together. Stable notice
IDs and cursor reads recover after detach, restart, broadcast lag and loss before
publication. Concurrent tasks and later Turns in a session retain their own
correlation. A pre-prompt startup failure remains SubmissionUnknown with the real
created session ID; no task is regenerated. Existing Phase 6.5 records remain
readable but are not retrospectively guessed into dispatch links.

Delivery is at least once. Chat suppresses already-rendered cursors while
reconnecting and skips durable acknowledged notices on a later open. Output is
flushed before queuing acknowledgement; crash or failed/unqueued acknowledgement
can repeat a notice on reopening. Multiple connected owner clients may each see
it. `lain notices <conversation-id>` retains all notices for inspection, including
acknowledged ones. [ADR 007](adr/007-durable-delegation-result-delivery.md) defines
the evidence and queue bounds. The CLI preserves input while results arrive;
human receipts are compact but exact proposed arguments remain visible before
confirmation. JSON Actor snapshots and existing history are unchanged.

## Historical Phase 6 acceptance and Phase 7 boundary

Ordinary Rust/native smoke checks use deterministic model injection, a real local
fixture Responses SSE server and the existing fake Codex process, consuming zero
model quota. They cover persistence/restart, request deduplication, structured
reads, confirmation/rejection/races, policy enforcement, partial delegation,
malformed calls, budgets, cancellation, context bounds, secrets and reconnect.
Native acceptance is in [CLI.md](CLI.md); actual platform evidence is in DEVLOG.md.

Paid smoke is explicit, never part of ordinary tests/CI:

```text
python scripts/actor_smoke.py --run-real --workspace <absolute-daemon-workspace> --evidence <new-summary.json>
```

Explicit owner authorization must cover the configured provider and exact
workspace before invoking `--run-real`. The maximum planned scope is nine Actor
turns (up to 36 provider requests under Kernel limits) and one read-only Codex
turn. No compatibility retry or replacement worker is automatic. It requires an
interactive terminal, a configured Actor and supported authenticated Codex.
`--max-model-requests N` (4–36) caps a run, reserving up to four requests before
each Actor turn. Subtract earlier attempts from the owner's authorized allowance;
the limit does not grant permission for additional calls. Separately inspected
supplemental read evidence is also counted and verified during recovery.

The runner checks Chinese prose, meaningful preview while the turn is Running,
all five typed read tools with Kernel receipts, an owner-denied proposal with no
new work, then a separately reviewed/approved exact proposal. It verifies the real
WorkSession/worker-turn receipt, rejects duplicate approval, observes actual worker
termination through SSE plus turn snapshots, and checks separate CLI histories.
macOS also exercises actual PTY chat exit/reopen; other native terminal acceptance
is explicitly pending. Fast provider completion before the read-only preview probe
leaves incremental evidence unverified. Worker failure, approval waits and timeout
remain distinct from successful completion; no worker permission is auto-approved.

The runner writes a new, explicitly selected summary containing only IDs, hashes,
states and timing. It never stores provider traffic, replies or credentials. On
uncertain approval it reads the existing turn/partial receipt and stops without
replaying. Inspect open work, gracefully stop/restart your own foreground `lain up`
with the same database, then run the read-only recovery stage:

```text
python scripts/actor_smoke.py --check-recovery --evidence <same-summary.json>
```

The owner attests the separate restart; the runner verifies retained Actor IDs,
complete snapshot hashes and terminal worker IDs/results without inference. It
does not stop or replace the daemon. Missing evidence stays incomplete. Native
smoke exercises this same runner with explicitly marked local fixture evidence;
that never counts as real provider acceptance. See
[the acceptance record and conditional handoff](PHASE_6_5_ACCEPTANCE.md).

On 2026-10-14, the owner's MiMo Actor and Codex 0.160.0 passed native macOS real
evidence checks across controlled attempts: Chinese preview, all five reads,
denial, separate exact approval, terminal worker completion, CLI reopen and
graceful same-database restart. This was not one uninterrupted successful run.
MiMo sometimes described a tool operation without calling it; `work.history`
passed in an isolated conversation after missing/uncertain attempts in the main
conversation. Tests require real receipts, retain failures and never accept prose
as execution. Provider latency varied substantially; see the measured report.

Deferred: mid-inference provider resumption, conversation pruning,
short worker references, multiple Actor providers, server installation and native
Linux acceptance. Phase 7.1 now implements memory candidates/summaries/retrieval
below; World remains deferred.


## Phase 7.1 memory tools

The closed registry also exposes `memory.search`, `memory.list`, `memory.inspect`
and `memory.propose` (Responses names use underscores). Reads require an explicit
scope and return only Active memory, at most ten records and a bounded encoded
page with provenance/cursor. No memory is inserted into every system prompt.
`memory.inspect` checks both ID and scope. A workspace must be an existing
absolute daemon-host directory; the model must not guess projects or broaden scope.

`memory.propose` accepts only a bounded draft (content/kind/scope). The Kernel
assigns LainGenerated/Inferred provenance with the actual conversation/turn/action.
The durable candidate and Proposed action commit together; one ordinary owner
confirmation accepts it, denial rejects it. It cannot choose explicit-owner
origin, rewrite persona, commit memory on invocation, or accept its own proposal.
`lain actor approve/deny` and interactive chat use the same decision boundary.
Accepted candidate receipts expose MemoryId; a crash can leave an Unknown action
with a separately Accepted candidate, which the owner can inspect without replay.

Retrieved memory remains untrusted attributed evidence. Current owner instructions,
persona, permissions and confirmation gates retain their authority regardless of
stored instructions. The provider receives only explicitly retrieved bounded
records, never the entire store or private World. The Kernel does not attempt to
prove a model's interpretation correct; it enforces the closed registry and
explicit mutation decisions. [MEMORY.md](MEMORY.md) documents authority and
retention. Fixture conversations test retrieval from reopened SQLite through both
the Actor API and native Responses adapter; real-provider Phase 7.1 acceptance is
still pending separate authorization. No extra model request is used to commit
accepted memory or inspect candidates/summaries through CLI.
