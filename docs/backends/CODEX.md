# Codex worker adapter

## Command-output burst recovery — 2026-11-05

The first selected real Windows work turn emitted its opening message and began a
read command, then lost its protocol connection before a final summary. Native
logs show a burst of command outputDelta notifications. A real-subprocess fixture
with 12,000 such notifications reproduces failure of the old 512-entry transport
queue. The original uncertain command is retained; this fix does not resend it,
invent completion or authorize replay of a backend mutation.

The reader now absorbs only well-formed, request-ID-free
`item/commandExecution/outputDelta` notifications before the lifecycle queue.
These previously supplied cosmetic live character counts; completed command text
still arrives through `item/completed.aggregatedOutput`, under existing bounds
and redaction. No raw command fragments are logged or persisted. Assistant
messages, approvals, command completion, turn completion and errors keep their
existing route. Request-like or malformed messages are not filtered. Frame limits
and fail-closed overflow handling for other events remain unchanged.

Transport closure diagnostics now retain a fixed reason (read/EOF/frame limit/
invalid JSON/queue full/consumer closed/process exit), without original frame,
stderr or provider error text. Seven native Windows Worker integration tests pass,
including exact final command/assistant output, same-session later approval and
the existing genuine overflow/bad-frame/cancellation/recovery cases. One classifier
test and 17 native device-service scenarios also pass, including cloud reporting
of the final summary after the burst. Strict Kernel all-target Clippy and workspace
build pass. The checked Windows host is side-installed behind the stable
`Start-Codex-Host.cmd` entry; existing processes require an explicit normal restart.
Real-provider success after installing the fix remains separately unverified.

## Current managed allowlist — 2026-11-05

Exact managed-control versions: 0.160.0 and the native Windows installation
0.166.0-alpha.5.1. The latter has consumed-schema field/enum checks and actual
isolated Rust adapter profile-injection/original-empty-session probes, without
inference or personal login. Adjacent versions remain rejected. See
[current verification boundary](../AGENT_BACKENDS.md#explicit-installed-codex-alpha-contract--2026-11-05).
Archive-reader compatibility below remains independent; this does not claim
0.166 archive support or real-provider work acceptance.

## Visible archive reader: native 0.160.1 — verified 2026-10-27

The installed CLI now reports 0.160.1. Managed Worker control below remains
verified only for 0.160.0 and stays unavailable on 0.160.1; reading has a separate
gate/report. The reader uses initialize/initialized, metadata-only thread/read,
thread/turns/list (one turn, itemsView notLoaded) and bounded thread/items/list for
a terminal turn. It never resumes/forks/starts/steers/interrupts a source thread
or answers its approvals.

Generated 0.160.1 schemas and the official
[App Server reference](https://learn.chatgpt.com/docs/app-server) define paginated
history. An unused thread initially rejected some history calls; those method
rejections remain failures rather than fake empty histories. An isolated real
native turn using one local fake Responses request produced user/assistant items;
after restart, native read APIs and the actual Lain CLI archived them without
resume or another model request. Visible native source rows stayed unchanged.
This is native read evidence, not real-provider/Worker-execution/personal-history
acceptance. Contract: [ADR 020](../adr/020-explicit-visible-worker-archive-reads.md).

Status: Phase 2 implemented and real Windows acceptance passed.
Phase 4 (2026-10-05) explicitly advertises `model_selection` in the provider-neutral
capability report for the existing validated session `options.model` support.
This adds no new provider protocol or version support.
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
one of the exact versions in the current managed allowlist above. A different/missing version produces `available: false` and
no advertised capabilities; read APIs remain usable. The reported capabilities
are the verified adapter contract, not server-side method discovery. A version
match does not prove authentication, quota or successful initialization.

Initialize/start/resume operations have 30-second write/response bounds. Requests
have numeric client IDs; server request IDs can be strings or integers and stay
private. A dedicated reader validates JSONL frames up to 8 MiB and routes responses
independently of storage. Well-formed command output progress is absorbed as
described above; remaining notifications use a 512-entry queue. Its overflow,
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
| Agent text delta | Live OutputProgress character counts |
| Well-formed command output delta notification | Absorbed before the lifecycle queue; final text comes from completed command |
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

## Phase 5 launch-scoped profiles (verified 2026-10-05, Codex 0.160.0)

The installed native version still reports `codex-cli 0.160.0`; the exact gate
has not been loosened. Its app-server help documents repeatable `-c key=value`
TOML overrides. Schemas regenerated with `app-server generate-json-schema`
confirm thread start/resume/fork model, modelProvider, sandbox, approvalPolicy,
approvalsReviewer and config fields, response model/reasoningEffort, and structured
CodexErrorInfo plus willRetry. Current official references are
[App Server](https://learn.chatgpt.com/docs/app-server),
[configuration](https://learn.chatgpt.com/docs/config-file/config-reference) and
[Responses provider setup](https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server).
Installed help/schema and a native zero-inference probe establish this version's
behavior; documentation alone does not expand compatibility.

The adapter-owned CodexLaunchSpec maps:

| Lain resolved field | Codex boundary |
|---|---|
| model | process `-c model`, plus explicit thread model |
| reasoning_effort | process `model_reasoning_effort` and thread config override |
| provider label + SecretRef | private `lain_runtime` Responses provider, thread modelProvider |
| base_url | `model_providers.lain_runtime.base_url`; absent uses official API URL |
| resolved credential | child-only `LAIN_PROFILE_CREDENTIAL` provider env_key |
| managed provider auth | requires_openai_auth=false; HTTP transport (supports_websockets=false) |
| sandbox | thread read-only/workspace-write |
| approval_policy | thread on-request/untrusted; approvalsReviewer=user |
| existing local auth | no endpoint/provider override or injected credential |

No raw value in argv. The child retains env_clear and its OS/path/proxy/local-home
allowlist; only its required profile credential is added. Shell environment policy
excludes that dedicated variable from ordinary tool subprocesses. There is no
global config/auth mutation, arbitrary env/argv surface or global active provider.
Managed profiles use Responses only, not Chat Completions, dynamic auth commands,
arbitrary HTTP headers or account switching. Model and effort access remain
provider-dependent. Profile health never performs inference or an external probe.

Start, durable fork and resume all receive selected thread options. The initial
structured response fills unspecified local model/effort into the persisted
snapshot. Subsequent attachment uses that snapshot and re-resolves SecretRef;
registry edits do not alter it. Codex local tool/home settings and local endpoint
defaults remain externally owned, not copied into Lain. Legacy revision-0 metadata
keeps its old options and captures unknown safe defaults on successful attachment.

Native `probe_profile_launch_configuration` passed with two simultaneous real
App Servers, dummy credentials and separate loopback endpoint/model/effort
configurations read through config/read, without any inference request. This
verifies parsing/isolation of launch overrides; it does not verify two real
provider accounts or model access. No second real gateway credential was supplied.
Deterministic subprocess/API/browser acceptance is the profile-isolation proof;
paid two-provider acceptance remains optional and unclaimed.

## Phase 5 safe errors and credential redaction

Error normalization uses only recognized structured discriminants/status fields:
unauthorized/401 => authentication_failed, rate/usage limit/429 => rate_limited,
sandbox/denial/policy/403 => permission_denied, overload/flex/server/5xx or known
response connection failures => provider_unavailable, recognized request/context
execution conditions => execution, otherwise unknown. Raw message, additional
details and misalignment prose are discarded. HTTP 404 is not evidence of a missing
Lain session. ErrorObserved carries optional retryable from willRetry and never
changes turn state. Final failed turn evidence uses retryable=false; old persisted
records decode with None. RPC rejection codes remain safe numeric codes.

Each runtime redactor includes the daemon API token, namespaced daemon credentials
and its resolved credential. Redaction happens on complete records before output
bounds/history/SSE, including commands, approvals and file/tool observations.
Buffers clear on drop. Unknown/encoded secrets remain outside reliable detection;
do not expose credential files or retrieve environment dumps. Transport uncertainty
still differs from proven execution failure, and intermediate retries cannot
manufacture a terminal turn.
