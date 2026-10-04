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

Clients should not depend on an unbroken WebSocket history from daemon startup.

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
- waiting for approval;
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
