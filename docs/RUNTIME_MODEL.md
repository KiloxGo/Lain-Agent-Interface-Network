# Runtime Model

## W2a explicit device proof/report runtime — 2026-10-28

Administrator enrollment, device proof, reporting and revocation serialize through
Kernel storage. Every device operation checks verifier/state/expiry in the same
critical section as its receipt; authorization cannot survive a prior revoke.
Report sequence and lifecycle revision are separate. Restart preserves identities,
receipts and revocation without reenrollment, key rotation or command replay.
The native CLI probes only its selected workspace paths on an explicit report;
there is no background heartbeat, model request, remote Worker or public listener.
See [ADR 026](adr/026-explicit-device-enrollment.md).

## W1f bounded source-memory selection — 2026-10-28

The existing private Actor selects only after actual archive disclosure and a
captured independent write epoch. One transaction commits semantic memory,
allowance and action receipt. Existing freshness gates cover flattened lineage;
the archive retention timer also removes expired derived rows. User correction/
forget clears transient source dependencies and rebuilds subsequent model input
from current user statements and committed effect metadata. There is no extractor,
new model scheduler or automatic repair/replay. See
[ADR 025](adr/025-selected-source-memory.md).

## W1e finite synchronization runtime — 2026-10-27

One nearest-deadline timer wakes only for explicitly enabled source policies or
archive/shutdown notifications. Owned-slot acquisition precedes atomic request
reservation/Prepared receipt. Successful completion schedules the next interval;
failure/Unknown stops, and restart preserves allowance/cursor. Revocation/expiry
reject late results. See [ADR 023](adr/023-finite-knowledge-synchronization.md).


## W1d explicit source bridge runtime — 2026-10-27

One authenticated command admits a bounded, owned read-only HTTP task. Admission
withholds a prior freshness lease; completion atomically commits source state,
records/tombstones and its cursor. Cancel/shutdown discard late results and await
the owned task; interrupted receipts recover Unknown without replay. A caught-up
snapshot may be recalled for 30 seconds from commit, with fresh expiry/suppression
checks through delivery. No idle network collector or model polling is started.
See [ADR 022](adr/022-scoped-knowledge-archive-pulls.md).

## W1c archive conversation recall — 2026-10-27

Explicit private tools select up to three stored reports; lookup/audit and Actor
versions commit together. Further requests project fresh bounded evidence, with
an archive freshness check at adapter entry and existing response/commit/prewrite
guards. New-input reconsideration clears selections without resetting budgets;
later history cannot replay archive-derived bodies. No new idle/background model,
collection or source-control path. See [ADR 021](adr/021-scoped-archive-conversation-recall.md).

## W1b read-only archive operations — 2026-10-27

Explicit authenticated admission commits an immutable source/revision receipt
before a bounded native read. One unfinished receipt and one owned reader slot
bound concurrency. No model/Worker turn is started. Cancel/revoke/shutdown discard
late results and close the reader; Supervisor shutdown awaits it. Normalized
import and Completed cursor commit together. Boot marks unfinished reads Unknown
without replay. No periodic collector/model loop. See
[ADR 020](adr/020-explicit-visible-worker-archive-reads.md).

## W1a archive runtime — 2026-10-26

Explicit authenticated imports commit bounded batches and source-scoped identities
transactionally without Actor/Worker wake-up. Startup pruning, nearest-retention
timers and fresh reads remove expired bodies. Replay cannot renew retention/refund
caps; revoke/regrant cannot redisclose old epochs. No source poller or external
session resume is added. See [ADR 018](adr/018-selected-visible-source-archives.md).

## D3b inference admission — 2026-10-26

Every Lain Actor model request reserves against the same current finite ceiling
in the Actor transaction, including tool continuations and reconsiderations.
Budget/epoch/expiry is rechecked before adapter entry. Returned normalized usage
is recorded even if later source checks reject speech. Terminal/restart recovery
marks prepared work NotAttempted and attempted work Unknown, with no refund/retry.
Own model/text HTTP clients explicitly disable built-in retries. There is no new
idle model poll. See [ADR 017](adr/017-kernel-wide-inference-budget.md).

## D3a fresh receipt reads — 2026-10-26

Private model requests can use a recent content-free receipt pointer or typed
sharing reads. Fresh projection/audit/version commits precede disclosure; response,
completion, Ready and prewrite checks reject changed transport/source versions.
New-input reconsideration clears selected receipts within existing finite caps.
There is no new model polling, retry or send path.
See [ADR 016](adr/016-sharing-receipt-continuity.md).

## D2 sharing runtime — 2026-10-26

New eligible reading completions wake channel consumers through Kernel events,
including publication by a different Actor instance. Current replies/batches take
priority. A bounded no-tool share Actor runs independently of chat; a service-owned
send task keeps receipt waiting off chat inference. Persisted admission and fresh
prewrite gates enforce attention, source/input revisions, quotas and no-response
control. Restart expires unsent candidates, marks in-flight sends Unknown and
advances the boot watermark without replay. See [ADR 015](adr/015-bounded-proactive-sharing.md).

## D1b event-driven reading processing — 2026-10-26

An optional explicitly granted processing event follows acquisition completion;
otherwise there is no model job. At most one reading job is active, independently
of private chat/Workers. The existing Actor bounds, no-tool mode, durable pre-request
reservation and same-transaction completion/experience publication prevent budget
reset and false results. Cancel/revoke/expiry/source changes invalidate late results;
already returned usage is retained. Recovery remains Unknown with no auto-processing
or inference retry. The retention timer also removes derived summary content.
Private recall rechecks current projections through response/commit/prewrite;
ordinary reply and Work delivery semantics remain separate. Scope and limits:
[ADR 014](adr/014-source-linked-reading-experiences.md).

## D1a finite acquisition runtime — 2026-10-26

Authenticated explicit admission persists an attempt before spawning a finite
asynchronous acquisition. Only one unfinished activity is admitted; retries of the
same request return its recorded state. A 20-second outer bound includes DNS;
scope/retention are checked again under the Kernel lock before result commit.
Cancellation, grant changes and shutdown notify the task; there is no fetch replay
after restart. Unfinished eligible work becomes Unknown, never fabricated success.
One next-deadline timer deletes expired bodies and is rebuilt from durable records
at startup; no pending deadline means event-wait only. It makes no model calls.
See [ADR 013](adr/013-bounded-text-acquisition.md) for exact limits and the distinction
between completed acquisition and read/summary/verification.

**Status:** resident-runtime target plus current local runtime contracts

**Last reviewed:** 2026-10-24

## Q2-B1 private source lookup — 2026-10-25

Q2-B1 source lookup rechecks its independent grant during lookup and before preparing every new model request. All earlier selections in the current turn are freshly projected; changed scope, suppression or reply visibility stops continuation. A second check after inference discards stale generated text/calls and accounts for tokens already used. Source-influenced previews remain held until this check. Subsequent turn context keeps original user requests but omits assistant/action observations from source-reading turns. Preparation under the Kernel storage lock is the authorization linearization point, not proof of HTTP receipt. Revocation after dispatch cannot recall that request. See [ADR 011](adr/011-bounded-private-source-lookup.md).

## Current target runtime — 2026-10-23

Cloud residency now prioritizes daily QQ interaction. The cloud keeps one identity
and continuous records; paired PCs own execution and report evidence. Detailed
proposal: [RESIDENT_DESIGN.md](RESIDENT_DESIGN.md). Chat behavior:
[CHAT_DESIGN.md](CHAT_DESIGN.md). Distributed execution is not implemented. Q1 now
has an optional QQ private-text adapter. Basic real private chat was user-confirmed
on 2026-10-24; full reliability, memory and cloud acceptance remain open.

The target distinguishes receiving, archiving, reading, selecting memory, deciding
to share and actually sending. Raw message arrival does not require a model call.
Explicit user messages, useful accumulated sources, relevant work transitions and
bounded timers can trigger activities under scoped authorization and resource limits.
No endless self-prompting is required. Concrete intervals/budgets remain undecided.

### Scheduling and interruption

- User conversation remains available while reading and Worker execution proceed.
- New messages can update a not-yet-sent response; interruption does not undo an
  external action or automatically cancel an independent Worker.
- Combine split messages with bounded waiting; avoid unbounded inference restarts.
- Course/focus/do-not-disturb changes re-evaluate pending private shares.
- Activity completion can produce an experience or a share candidate without
  forcing a message. Old nonessential candidates may expire.

### Failure and recovery boundaries

| Event | Target behavior |
| --- | --- |
| Terminal closes | Owned background execution continues where deployed |
| Device cannot reach cloud | Retain local evidence; cloud reports last known state |
| Device loses internet | Model requests may fail; local child processes may still run |
| Device sleeps/shuts down | No claim of continued local execution |
| Cloud restarts | Recover committed records and valid pending activities; no notification flood |
| External send acknowledgement is lost | Reconcile if supported, otherwise retain uncertainty; do not blindly resend |
| Source access/grant revoked | Stop new work and check queued actions before dispatch |

Recovery replays evidence or safe queries, never uncertain mutations. Outgoing QQ
delivery is separate from existing notice display acknowledgements; a database
commit does not make an external send exactly once. Device process survival across
terminal closure needs a genuine native background deployment, not merely a promise.

The existing foreground `lain up` and local supervised-process behavior below
remain current behavior. Future daemon/device installation and shutdown semantics
require explicit implementation and native acceptance.

### Implemented Q1 event flow

The optional transport verifies its account, filters and normalizes inputs, and
persists them before waking the channel application. A trailing 750 ms quiet window
capped at three seconds merges up to 16 messages/8 KiB; the pending queue holds at
most 128 messages. Inference
does not block transport ingestion or other Kernel/Worker operations. One channel
conversation waits behind its current turn or explicit pending confirmation.

Application wake-ups use incoming/grant/Actor/connection events and finite merge or
expiry timers. There is no idle model polling. Inputs and unsent responses have a
10-minute lifetime; offline drafts expire rather than indefinitely accumulating
catch-up work. The adapter has one in-flight send, a minimum one-second spacing,
three-second write bounds, a 15-second receipt deadline and bounded reconnect backoff.

An interrupted Sending intent becomes Unknown at restart and never auto-replays.
Prepared batches recover their committed Actor request; interrupted Actor inference
uses the existing Unknown policy. Revocation blocks queued processing and new sends,
but cannot undo an already initiated external write. No history backfill or gap-free
NapCat delivery is assumed. See [QQ_CHANNEL.md](QQ_CHANNEL.md).

### C1 private turn timing and draft truth

Private-chat contexts add explicit Reply/Waiting/NoReply/Superseded/Notice/
Interrupted decisions, distinct from transport delivery state. Waiting uses one
1–5-second timer, not idle inference. New input during generation may be adopted
transactionally into the same batch without changing original request text. It
invalidates opaque continuation and held drafts, with at most two reconsiderations
inside existing four-request/four-tool/token/120-second bounds. Returned usage is
recorded even for a discarded response. Over-cap input remains queued and an
honest capacity notice replaces the obsolete answer. Work and approvals retain
their independent lifecycle.

Final text and new actions check current input under the Kernel lock. Selected
memory IDs/revisions and re-projected source fingerprints are checked on final
commits and at send checks, in addition to response checks. Revoke/delete/correction,
historical source suppression and scope/expiry changes block stale recollection
even after a draft is Ready. Ready ordinary drafts
can become Superseded. A claimed but prewrite-invalidated request is NotSent;
Confirmed/Unknown sends retain their actual truth. Approval/failure notices are
preserved. Later history uses confirmed outgoing text and withholds changed
memory/source-derived replies. See [ADR 012](adr/012-bounded-private-conversation.md).

### Q2-A request-time memory

No extractor, idle model polling or additional per-message inference job is added.
An existing channel ActorTurn can select memory within its existing four-request,
four-tool, token/time limits. Before each model request the Kernel supplies fresh
bounded relevant Active memory for that binding. It stores only inspected IDs and
revisions for target checks, not a reusable content cache. Recall receipts carry
IDs; current records are re-read for the next request. Grant revocation/expiry
removes subsequent capabilities/context and rejects pending effects under the
same storage lock used by administrative grant changes. A request already sent
to a provider cannot be recalled. Recent dialogue and audit have independent
lifetimes and are not erased by forgetting semantic memory.

Memory writes, supersession/deletion, old-source tombstones and effect receipts
are transactional. Restart never re-executes an uncertain effect; inspect its
receipt. New authorization takes a source sequence watermark, excluding queued
pre-grant inputs as well as older archives. See [Q2_A_ACCEPTANCE.md](Q2_A_ACCEPTANCE.md).

## Historical foundation and current implementation

## 1. Goal

Lain is intended to run 24 hours a day on a personal server.

The key rule is:

> The Kernel is continuously resident. Lain's expensive cognition is not.

Persistence, events, schedules, memory, and availability create continuity. An endless LLM inference loop does not.

## 2. Runtime layers

~~~text
always resident
--------------------------------
laind
public API
persistent storage
event processing
worker supervision
scheduler/timers
health state

event-driven / on demand
--------------------------------
Lain Actor inference
session summarization
memory extraction
reflection
semantic consolidation
worker creation/steering
~~~

## 3. Kernel lifecycle

~~~text
Booting
   |
   v
Recovering
   |
   v
Ready
   |
   +--> Degraded
   |
   v
ShuttingDown
~~~

### Booting

- load static configuration;
- open database/filesystem;
- initialize secret backend;
- initialize logs/metrics;
- construct internal services.

### Recovering

- load persisted WorkSessions;
- restore schedules;
- mark uncertain previously-active sessions conservatively;
- probe configured worker backends;
- resume/reconnect where supported;
- rebuild in-memory projections.

### Ready

- serve clients;
- accept chat;
- accept work requests;
- process backend events;
- execute schedules.

### Degraded

One unavailable backend must not make Lain unavailable as a whole.

## 4. Resident versus awake

"Awake" means a wake cycle involving LainActor cognition.

The Kernel can process operational events without waking Lain.

Normally no wake is needed for:

- command stdout;
- tool deltas;
- file edit notifications;
- heartbeats;
- most progress events;
- frontend connect/disconnect.

A wake may be justified by:

- owner chat;
- a significant worker completion;
- a complex approval that needs explanation;
- a scheduled reflection;
- an explicit automation.

## 5. Wake cycle

~~~text
WakeTrigger
    |
    v
WakePolicy
    |
    +-- reject/defer
    |
    v
ContextBuilder
    |
    v
Lain inference
    |
    +-- user response
    +-- work-control action
    +-- memory proposal
    +-- world update
    `-- no-op
~~~

Every wake should be attributable to a trigger.

The system should eventually be able to answer: "Why did Lain wake?"

## 6. Cognitive budget

Background cognition must be bounded.

A future budget may constrain:

- maximum background wakes per day;
- maximum background token/cost budget;
- maximum parallel background jobs;
- allowed reflection depth.

Do not schedule frequent self-prompts just to make the system appear alive.

## 7. Scheduled routines

### Session completion summary

~~~text
WorkSession -> Completed
       |
       v
collect normalized facts
       |
       v
SessionSummary
       |
       v
MemoryCandidates
       |
       v
policy/review
       |
       v
optional memory/world updates
~~~

### Daily reflection

A daily reflection may consider:

- conversations with the owner;
- completed/failed sessions;
- unresolved work;
- new memory candidates;
- important project changes.

It should not replay all raw logs.

### Memory maintenance

Possible work:

- merge obvious duplicates;
- mark stale or contradicted knowledge;
- improve provenance links;
- archive low-value episodic records;
- refresh retrieval indexes.

## 8. Scheduler boundary

The Scheduler owns:

- persistent schedule definitions;
- next-run calculation;
- trigger emission;
- scheduling-level retry behavior.

It does not own LLM prompts, memory semantics, or worker semantics.

~~~text
Schedule
  |
  v
ScheduledTrigger
  |
  v
Job Dispatcher
  |
  +-- reflection job
  +-- memory maintenance
  `-- explicit owner automation
~~~

## 9. Worker concurrency

Lain should support several simultaneous WorkSessions.

Limits may exist for:

- total worker processes;
- provider rate limits;
- workspace mutation conflicts;
- owner-configured caps;
- background cognition.

A reasonable first policy is multiple sessions in different workspaces and conservative handling of simultaneous mutation in the same workspace.

Do not permanently hard-code that policy.

## 10. Workspace isolation

Two coding agents modifying the same checkout are not automatically safe.

Later versions should consider worktree/sandbox isolation.

~~~text
same workspace?
   |
   +-- read-only work -> maybe allow
   |
   `-- mutating work -> serialize or isolate
~~~

## 11. Frontend lifecycle

Frontend lifecycle is independent of worker lifecycle.

~~~text
Web/TUI connects
      |
      v
snapshot + subscribe
      |
      v
disconnect
      X   worker continues
      |
later reconnect
      |
      v
authoritative snapshot + live events
~~~

Clients should not depend on an unbroken event stream from daemon startup.

## 12. Worker runtime lifecycle

Infrastructure runtime states may look like:

~~~text
NotStarted
Starting
Connected
Degraded
Exited
Restarting
Stopped
~~~

These are not WorkSession states.

One WorkerRuntime may serve multiple WorkSessions.

## 13. Recovery

### Kernel restart

After restart:

- in-memory backend connections are gone;
- persisted session records remain;
- previously Running sessions become uncertain until verified;
- adapters probe/resume where supported;
- otherwise state becomes Disconnected, Unknown, or Failed according to evidence.

Never silently claim work is still running when the backend cannot confirm it.

### Worker crash

The adapter reports failure. The Supervisor decides whether the session is resumable, retryable, failed, or requires owner action.

### Frontend crash

No worker state transition should occur.

## 14. Durable background jobs

Durable jobs are appropriate when loss matters, including:

- owner-created scheduled tasks;
- pending summary after session completion;
- important memory maintenance.

Ephemeral jobs are acceptable for:

- UI projections;
- caches;
- best-effort index refresh.

Do not build a distributed job platform for v1.

## 15. Time and timezone

Time-based behavior must use an explicit owner timezone.

Persist timestamps unambiguously, preferably UTC internally.

A schedule like "02:30 daily" must carry timezone semantics.

## 16. Idleness

A worker can be alive but idle.

Idle can mean:

- a turn completed and the session is resumable;
- waiting for another prompt;
- connected with no active turn.

Silence is not completion.

## 17. Graceful shutdown

A graceful daemon shutdown should:

1. stop accepting new work;
2. persist pending state;
3. stop creating new scheduled jobs;
4. safely detach or stop managed backends;
5. terminate only owned processes according to policy;
6. close database/files cleanly.

## 18. Observability

The owner should eventually be able to inspect:

- daemon uptime;
- backend health;
- active runtimes;
- session counts/states;
- scheduled work;
- pending approvals;
- recent failures;
- recent Lain wakes and reasons;
- usage/cost when exposed by providers.

The dashboard should use structured domain state rather than parse logs.

## 19. Initial implementation boundary

V1 runtime should stay small:

- one laind process;
- SQLite;
- local filesystem world;
- one Codex backend;
- multiple WorkSessions;
- live event streaming;
- basic scheduler;
- bounded completion summaries;
- no distributed deployment;
- no high availability cluster.

## 20. Implemented lifecycle and recovery

The implemented subset is profiles/SecretRefs, configuration/logging, operational
SQLite, worker supervision/recovery, a bounded on-demand Lain Actor, and a local
HTTP/SSE server. No scheduler or world is started. Startup fails before serving if configuration, signals, storage,
migration, persisted records, or the listener cannot be initialized.

`Booting` opens storage. `Recovering` loads and validates session and worker snapshots
in one transaction. Known idle (including unused Ready) provider-backed sessions
with confirmed terminal turns recover as Idle + Detached without spawning a worker.
Starting, Running, WaitingForApproval, Disconnected and other uncertain sessions
become Unknown because no live backend can confirm their state.
Created, Unknown, Completed, Cancelled, Failed, and Archived remain unchanged.
Unfinished turns become Unknown; pending approvals expire, and decisions with
uncertain delivery become Unknown. Recovery is persisted before serving and does
not infer worker failure or completion. Invalid data aborts the whole recovery
transaction rather than being skipped. Associated recovery history commits in the
same transaction. The supervisor attempts resume only for uncertain sessions with
persisted provider identities and detaches again if idle. Unresumable sessions stay Unknown;
they are never silently replaced by a new provider session.

Worker validation also rejects Confirmed cancellation on a non-Cancelled turn,
resolved/responding approvals without an offered owner decision, and pending
approvals that already carry a decision. Expired/Unknown delivery remains
independent of whether an attempted decision is recorded; recovery does not
manufacture a successful outcome.

`Ready` is entered after recovery commits, the listener binds and resume attempts finish.
`GET /ready` returns 200 only in this state, otherwise 503 while a listener is
available. During backend recovery a socket is bound but requests are not served
yet. `/health` reports only HTTP liveness. Both endpoints require authentication.
Missing/unsupported Codex leaves the query API ready; inspect `/v1/backends` for
worker availability. Readiness does not promise that every backend is connected.

SIGINT/SIGTERM on Unix and Ctrl-C/Ctrl-Break on Windows enter `ShuttingDown`.
New control commands are rejected, the HTTP listener stops accepting, SSE ends, and
in-flight requests may drain for at most five seconds. Exceeding that bound
returns a failure exit status. Owned workers are then stopped: idle work becomes
Idle + Detached; interrupted, unconfirmed active work and turns become Unknown.
Pending approvals expire and possibly delivered decisions remain Unknown. A normal
shutdown is a planned stop, without a synthetic BackendDisconnected failure.
Each in-flight RPC has a bounded write/response timeout, followed
by EOF shutdown and forced process-tree termination if necessary. Shutdown does
not manufacture successful cancellation. Dropping Kernel ownership closes SQLite
and releases its OS file lock. The `.lock` sidecar is retained on disk; the OS
releases the held lock on process exit, including a crash. Do not delete it while
the daemon is running. One Kernel owns a database at a time. Aliased hard links
and network-filesystem locking are outside the supported local-file deployment.

## 21. Worker and client lifetimes

At most one supervised task/process per attached WorkSession isolates failures and cancellation.
A bounded command channel serializes prompt, cancel and approval operations for
that session. After a command is queued, dropping the HTTP response or SSE client
does not cancel it. The daemon owns the task and process independently.

The stdout reader never awaits SQLite. It dispatches responses immediately and
queues notifications for the session task. Durable state changes run on blocking
tasks; progress fragments only publish live counts. A malformed/oversized frame,
queue overflow, timeout or process loss stops the affected runtime and triggers
conservative recovery. Pending approvals cannot be transferred to a new process.

An event-driven idle timer defaults to 300 seconds (`LAIN_WORKER_IDLE_TIMEOUT`,
1–86400 integer seconds). It runs only with no active turn, unresolved
connection-local approval or current serialized operation. It gracefully closes
the process, removes supervisor attachment, retains the provider ID and leaves
Idle + Detached. Observation/attach clients do not reset it or start processes.
Prompt and explicit resume lazily attach and reconcile the existing binding;
concurrent requests share the attachment/startup result. A closing runtime finishes
teardown before its replacement starts. Invalid prompts cannot trigger model work.
Unexpected active loss uses Unknown plus Lost; idle loss keeps Idle plus Lost.

Windows workers belong to a kill-on-close Job Object. Unix workers belong to an
owned process group, killed on managed shutdown. For Unix daemon SIGKILL/crashes,
deploy under a service manager with control-group cleanup; Rust destructors cannot
run after SIGKILL. Native Linux lifecycle validation remains outstanding.

## 22. Browser observation (Phase 4)

The Web client opens authenticated fetch SSE, takes its fresh atomic snapshot and
merges indexed history through the snapshot watermark with live events. Reconnect,
resync, online and visibility-return events reconstruct the disposable projection.
Network loss changes browser connectivity only, not WorkSession/Turn truth.
History pagination never starts or resets a worker idle timer; only normal prompt
or explicit resume APIs request attachment. Closing/reloading/two browser tabs
have no worker-lifecycle effect. Refresh drops credentials and requires re-entry;
the selected public ID can remain in the hash. Optional static asset serving adds
no resident cognition, backend privileges or second runtime. See [WEB.md](WEB.md).

## Phase 6 bounded on-demand Actor runtime

Only explicit owner conversation submissions invoke the Actor model. Doctor,
profiles, work observation and daemon startup perform no paid inference. Each
conversation admits one unfinished turn; its task belongs to Kernel ownership,
independent of HTTP/SSE observers. Explicit interruption or shutdown can leave
inference/mutation uncertain; restart reconciles Running/Executing to Unknown
without retries. Worker lifecycle and cancellation evidence remain independent.

A turn is limited to 4 model requests, 4 typed tool invocations, 2 tool errors,
120 seconds per active phase (at most 240 seconds per turn, excluding owner wait), configured per-request timeout/output and cumulative reported tokens,
plus deterministic context/result/frame bounds. Mutation first waits for explicit
owner confirmation, then calls Supervisor and records partial/accepted receipts.
Streaming publishes committed complete-record snapshots and separately typed,
bounded sanitized previews. Previews are ephemeral and cannot authorize tools or
substitute for durable completion. CLI previews are reconciled with completed
messages by identity/prefix; snapshot reconstruction never repeats inference.
No provider token replay or mid-generation provider resumption is claimed. Persona is stable at startup. No scheduler,
reflection, semantic memory or background autonomy was added.

Foreground local `lain up` launches native laind in the same console/process group
and waits for graceful native signals/storage shutdown. It resolves saved explicit
Actor/env-file selections before spawning, with process environment taking
precedence. Config/persona differences are observable through non-secret startup
revisions; credentials are not hashed. Changes apply on the next explicit owned
foreground stop/start, preserving database and IDs. No reload or guessed-PID
process termination is introduced. Windows and macOS native
acceptance are separate obligations. See [CLI.md](CLI.md) and [LAIN_ACTOR.md](LAIN_ACTOR.md).


## Phase 7.1 on-demand memory

Kernel startup migrates independent memory alongside existing operational state,
without model calls or summary extraction. Direct owner memory/summary APIs are
quota-free and use the existing storage lock. Actor retrieval is explicit and
bounded within an ordinary owner-triggered turn. Model proposals reuse persisted
owner confirmation; restart never accepts candidates or retries uncertain memory
mutations. Pending proposals and committed memory survive independently of a
provider connection. No scheduler, reflection, background summarizer or autonomous
extraction is started. See [MEMORY.md](MEMORY.md).
