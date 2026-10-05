# Codex worker adapter

Status: Phase 2 implemented and real Windows acceptance passed.
Verified: 2026-10-04. Installed CLI: `codex-cli 0.160.0` (Windows x64).

## Evidence and transport choice

Sources checked:

- [Official App Server documentation](https://learn.chatgpt.com/docs/app-server)
- [Current Codex source](https://github.com/openai/codex/tree/main/codex-rs/app-server)
- [Current ACP adapter](https://github.com/agentclientprotocol/codex-acp)
- The installed binary's `app-server --help` and generated JSON schemas
  (`codex app-server generate-json-schema --out target/codex-schema`).
- A real local, non-inference handshake and two `thread/start` requests.

Select direct App Server. The current ACP adapter itself launches App Server and
translates its operations/events. Lain already owns its normalized public domain;
an additional ACP translation is unnecessary for this first backend.

Launch `codex app-server --listen stdio://` as an owned subprocess. Messages are
newline-delimited JSON, bidirectional JSON-RPC requests/responses/notifications,
without a `jsonrpc` field. Send `initialize` with Lain client metadata, wait for
its response, then send `initialized`. Do not enable experimental APIs implicitly.
The generated 0.160.0 schema, rather than a newer online example, is the contract.

## Findings before implementation

- One server accepted two distinct non-ephemeral threads. Both returned `idle`.
- `thread/start` succeeds before a rollout necessarily exists. Immediately
  resuming an unused thread returned `-32600`, `no rollout found`. A saved provider
  ID alone does not guarantee resumability. Never substitute a new thread silently.
- `turn/start` returns a turn identity/status; completion is a later
  `turn/completed` notification. Completed/interrupted/failed are separate outcomes.
- `turn/interrupt` takes thread and turn identities; an empty success result is
  an acknowledgement, not evidence that execution has stopped.
- Command/file approvals are server requests. Reply to their original request
  ID; `serverRequest/resolved` means answered or cleared, not tool success.
- Permission approvals use a granted subset and turn/session scope. They are not
  command approval strings. Unknown request types must fail closed.
- `thread/resume` returns history that can reconcile previously uncertain turns.
  A process exit does not prove a turn failed, completed, or was cancelled.

## Runtime contract

`backend::Supervisor` owns one App Server process and one serialized command task
per WorkSession. A single server can host several threads, but per-session
processes provide direct cancellation/crash isolation without a multiplexing
registry. Frontends never own these processes. No worker intelligence is
implemented in Lain.

Discovery prefers an explicit native `LAIN_CODEX_PATH`, otherwise PATH (including
the Windows npm package's native executable). The probe requires exactly
`codex-cli 0.160.0`. A different/missing version produces `available: false` and
no advertised capabilities; read APIs remain usable. The reported capabilities
are the verified adapter contract, not server-side method discovery. A version
match does not prove authentication, quota or successful initialization.

Initialize/start/resume operations have 30-second write/response bounds. Requests
have numeric client IDs; server request IDs can be strings or integers and stay
private. A dedicated reader validates JSONL frames up to 8 MiB and routes responses
independently of storage. Its notification queue holds 512 entries; overflow,
malformed framing or process exit ends this runtime rather than discarding
important evidence. Stderr is drained without retention or logging. Raw protocol
and provider error messages are never logged; rejections retain method and code.
Protocol receive is cancellation-safe; decoding/replies finish outside the
supervisor's command-vs-notification selection so a command cannot truncate a reply.

Thread creation sets cwd, selected optional model, sandbox (`read-only` by default,
or `workspace-write`), `approvalPolicy: on-request`, `approvalsReviewer: user`, and
`ephemeral: false`. Phase 3 then materializes an unused durable binding before
exposing successful creation; see below. Resume reapplies the selected options and verifies the returned
thread identity. No global config is rewritten. The allowlisted environment uses
Codex's existing local authentication; see [configuration](../CONFIGURATION.md).

EOF is the normal shutdown request. The process gets two seconds to exit, then
Lain terminates its owned process tree; the adapter waits at most five seconds
for closure. Windows uses a kill-on-close Job Object. Unix uses a separate process
group on managed shutdown; service-manager cgroup cleanup is needed for daemon
SIGKILL. Native Windows shutdown has been exercised; native Linux remains unverified.

## Session, turn and event mapping

| Codex evidence | Lain meaning |
|---|---|
| New thread, idle | WorkSession Ready |
| `turn/start` response, inProgress | Bind opaque native turn ID to persisted Lain TurnId; Running |
| `turn/started` | Correlated TurnStarted; duplicate state observations are ignored |
| `turn/completed`, completed/interrupted/failed | Turn Completed/Cancelled/Failed; reusable WorkSession Idle |
| Agent text / command output delta | Live OutputProgress character counts |
| Completed agent message | Bounded sanitized AssistantMessageCompleted |
| Started/completed command | CommandStarted/CommandCompleted, safe output and optional exit code |
| Completed successful file change | FileChanged paths; failed/declined changes are not presented as success |
| MCP tool started/completed | ToolActivity (completion is not a success claim) |
| `error` | Safe ErrorObserved; retry information without raw diagnostic content |
| Transport loss / closed thread | BackendDisconnected; unfinished turns Unknown |

Public IDs are always Lain IDs. Private migration-002 metadata preserves provider
thread/turn IDs and selected options. Raw prompts/transcripts are not stored by
Lain. File diffs, reasoning fragments, token usage and unsupported item types are
not mirrored into the domain. Only progress counts are live-only. Meaningful
completed output, lifecycle, tools and approvals commit to migration-003 history
before publication; state and associated history are atomic.

## Approvals and cancellation

Command and file-change approval requests map to ApprovalKind::Command/FileChange.
Owner decisions offered are the supported subset of accept/decline/cancel,
normalized as approve_once/deny/cancel. Session-wide acceptance and policy changes
are not offered. Permission requests map separately: approve_once grants only
the requested permissions with `scope: turn`; deny grants an empty permission set.
The original request ID and permission object remain in the connection-local adapter.

Descriptions are sanitized and bounded. The Kernel persists Pending before
publishing ApprovalRequested, then Responding plus the chosen decision before
sending it. `serverRequest/resolved` produces Resolved (or Expired if cleared
without a decision). This is not command/file execution success. Unknown request
methods receive JSON-RPC -32601; nothing is automatically approved. Duplicate or
foreign-session approval decisions are rejected.

Cancellation persists Requested, sends `turn/interrupt`, then persists
Acknowledged on its success response. Only interrupted terminal evidence confirms
Cancelled. A racing completed turn remains Completed. Rejected interrupts become
Rejected and can be retried; transport loss remains uncertain. One real smoke run
observed an interrupt operation fail during immediate startup; the acceptance
script waits for live progress before testing active-work interruption. It does
not hide the possibility of runtime rejection from API clients.

## Recovery and limitations

Restart first atomically marks active sessions/turns uncertain and expires old
approval connections. Resume reuses the same WorkSession and provider thread ID,
reconciles known turns from provider history, and returns Idle/Running only with
backend evidence. A returned historical inProgress turn in an idle thread remains
Unknown. Never silently create a replacement thread when resume fails.

A bare unused thread/start can lack a rollout. Phase 3 handles new creation as
described below. A submission lost before its native turn ID
was bound cannot be safely matched to a later history entry by order or text;
that original Lain turn remains Unknown. Codex owns its rollout storage, and
resumability depends on that storage and account/config remaining available.
Completed turns do not imply WorkSession Completed; archive/close semantics remain
future work. Failed session creation leaves a queryable record for diagnosis.

Pending approval IDs cannot be reused after process restart. Unanswered approvals
expire; possibly delivered decisions become Unknown. An approval resolution or
interrupt acknowledgement cannot prove an external side effect did/did not occur.

Known credentials and recognizable key/token patterns are blocked/redacted, and
raw deltas are not forwarded. Unknown/encoded secrets cannot be detected reliably;
do not expose credential files or request their contents. Completed output is
bounded by UTF-8 byte limits and fetched through indexed history on reconnect,
not SSE replay. See the [history contract](../DEVELOPMENT.md#11-operational-history).

## Validation

Deterministic ordinary tests use a Rust fake subprocess and no model/login. They
cover handshake, early turn notifications, numeric/string approvals, file and
permission approvals, unknown request rejection, cancellation rejection/retry and
completion races, two isolated sessions, secret fragments, malformed framing,
queue overflow, crash and restart. API tests cover authentication, commands,
snapshot-first SSE, lag/resync and observer detachment. Storage tests cover schema
1-to-2 migration and atomic rollback on corrupt worker records.

The opt-in `scripts/codex_smoke.py --run-real` runs the actual daemon over HTTP/SSE
using temporary workspaces and the local authenticated Codex runtime. On
2026-10-04 it passed two real threads, initial/follow-up turns, scoped cancellation,
client detach/reconnect, explicit owner approval, owned-process crash/resume, and
daemon restart followed by a resumed turn with both provider IDs unchanged.
The local model configuration was `gpt-6-astra` with high reasoning and ChatGPT
authentication. Final checks and milestone details are in `DEVLOG.md`.

## Phase 3 unused-thread persistence (verified 2026-10-04/05, Codex 0.160.0)

A bare `thread/start` followed immediately by `thread/fork` failed with no rollout
in the installed native build. A metadata-only read did not solve it. In this
verified version, a full `thread/read` with `includeTurns: true` initializes the
default history storage, even when it returns -32601 (`list_turns` unsupported).
A subsequent non-ephemeral `thread/fork` materializes a persistent empty rollout.
This agrees with the upstream persistent-fork materialization implementation in
the [thread processor](https://github.com/openai/codex/blob/main/codex-rs/app-server/src/request_processors/thread_processor.rs).
The read side effect is a version-specific empirical compatibility behavior,
not a generic upstream guarantee. The adapter accepts only that read's -32601
limitation; other failures abort creation. The existing exact version gate remains.

Creation binds Lain only to the successfully materialized fork ID, before reporting
success. It never rebinds an existing session, injects invalid items or submits a
synthetic prompt. The unused seed is Codex-owned storage; its retention is Codex's
responsibility. A native no-inference ignored test creates, closes and resumes
the same unused binding through a fresh process. The focused real smoke additionally
proved first and follow-up model turns on that binding after idle detach/restart.
Do not extend version support without repeating these checks.

Idle timeout and normal idle shutdown leave Idle + Detached. Unexpected idle loss
leaves Idle + Lost; active unconfirmed loss/shutdown leaves Unknown. Known idle
sessions do not spawn Codex at boot; uncertain provider-backed work reconciles
selectively and releases its runtime if idle. Historical connection-local approvals
are never replayed. Provider rollout removal, account changes, workspace removal
and unbound uncertain turn IDs can still prevent successful recovery.
