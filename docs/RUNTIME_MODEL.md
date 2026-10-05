# Runtime Model

**Status:** normative foundation  
**Last reviewed:** 2026-10-04

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

The implemented subset is configuration/logging, operational SQLite, worker
supervision/recovery, and a local HTTP/SSE server. No scheduler, world, or Lain Actor
is started. Startup fails before serving if configuration, signals, storage,
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
