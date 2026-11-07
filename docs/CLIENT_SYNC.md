# Client startup and live invalidation

## Source setup draft isolation — 2026-11-06, local only

The N/E/S editor retains only same-Kind/source-key unsent field strings, focus and
cursor across synchronization. Fresh source and reader metadata replace old state;
Preview/read approval is always cleared. Typing/paste during reloading stays out
of Chat. Opening another source control retires its local view generation. A
SourceSetupOpened completion must still match generation, surface and loading
intent; Esc cancels it. System invalidation preserves pending-result focus while
explicit close/navigation cancels that focus. No old mutation is replayed.


## Source-access editor intent — 2026-11-06, local only

G on /sources is handled before the stale-input branch. During synchronization,
only its metadata-opening intent is retained; it is never inserted into Chat.
Invalidation withdraws the old source editor, selection and complete-read gate.
A fresh bootstrap may resume GETs for the explicitly requested source editor;
navigation/Esc cancels this intent. No old draft/approval or mutation is resumed.
Uncertain grant writes retain their exact pending identity and recover only by
owner-authenticated receipt GET. Historical receipt success does not claim current
authority. Native Windows isolated acceptance is in DAILY_BRIEFING; not deployed.


Status: protocol, startup and terminal runtime consumer implemented, 2026-11-05.
Native verification is recorded separately below. Subsequent actual authenticated
HTTPS Worker12 and memory7 cases, the deployed same-schema maintenance release
and final operational inventory are recorded in [cloud acceptance](CLOUD_ACCEPTANCE.md).
Decision: [ADR035](adr/035-client-startup-and-live-invalidation.md).

## Implemented owner contract

`POST /v1/client-sync/bootstrap` accepts
`{ "protocol": 1, "conversation_id": null | "UUID" }`. Unsupported versions
return 426. The existing owner bearer boundary applies; a device credential
cannot use it. No Actor inference, Worker submission or permission decision is
started by this read. A null conversation negotiates resources without creating
a conversation; selecting one restores that existing conversation only.

The snapshot contains:

- A persistent Kernel instance UUID, a per-open boot UUID and committed revision.
  A boot UUID is operational generation, not another Lain identity.
- Issued time and a finite validity deadline, at most five minutes and bounded
  earlier by the next indexed source/grant deadline.
- Protocol, actual capability names and hashes of validated non-secret Actor
  configuration, persona and policy resources. Internal prompts and provider
  credentials are not transferred.
- At most 128 device headers, 128 projects and 1024 explicit mappings. Headers
  include admitted workspace IDs and last report sequence, not filesystem scans,
  complete reports, local paths or device credential verifiers.
- Selected conversation context and at most 20 checked Actor projections. No
  semantic-memory table, source archive or Worker transcript is downloaded.
- At most 32 associated work/permission pointers and 100 notification pointers,
  with explicit truncation and recovery cursor metadata. Pointers do not approve
  anything; detail/review continues through its existing exact-scope API.

The complete serialized snapshot is at most 1 MiB. Whole historical turns may be
omitted to meet that bound; truncated body text is not presented as a complete
proposal. Context and source checks are the same as ordinary checked publication.
The frontend validates the selected conversation, supported capabilities, byte
bound and lease duration, accounting conservatively for the request's elapsed
time rather than requiring matching Windows/server wall clocks.

`GET /v1/client-sync/events` takes `instance_id`, `boot_id`, `after` and
`lease_until_ms` from the snapshot. It subscribes before comparing committed state
and queues no source body. A changed revision emits a content-free `invalidate`
event; foreign boot/instance, future revision or a position over 256 revisions
behind emits `resnapshot`. Expiry emits `expired` even when no database write
occurred. Each event ends that stream and requires a fresh snapshot. A fresh
snapshot is a read, not an instruction to replay an uncertain effect.

The cursor window coalesces invalidation; it is not a new unbounded event log or
a promise to replay historical content. Wakeups are rechecked when the HTTP
stream consumes them. Connection loss must be treated as loss of freshness.

## Persistence and signalling

Schema032 adds `client_sync_state` and an indexed per-row set of deadlines. It
does not rewrite existing memory, Actor, source or Worker bodies. The first
migration creates the persistent operational instance ID; reopen retains it.
Existing Kernel tables receive INSERT/UPDATE/DELETE triggers. Their revision
updates and deadline replacements commit or roll back with the original write.
The index retains distinct future deadlines even alongside already expired ones;
deleting a source row removes its deadline entries.

The existing Kernel storage lock has a guard that compares committed revision
before/after the operation and wakes native watch subscribers on change. It
neither wakes itself on reads nor adds a polling service, broker or model call.
Out-of-band manual writes that bypass the Kernel are not a supported live-update
interface. Startup/recovery may update operational state before serving, and
clients distinguish a new boot from the persistent identity.

## Terminal runtime and disposable cache

`lain`/`lain desk` negotiate protocol/resources before creating or restoring a
conversation. Startup and Chat refresh use the unified snapshot. Protocol failure
does not fall back to an unversioned private-history read. Lower snapshot
revisions cannot replace a newer applied one; another Kernel instance is refused
within the existing connection rather than inheriting uncertain requests.

A checked refresh rebuilds rendered history, retaining the input draft and
uncertain request identity. A same-revision SourceUnavailable view clears old
rendering and requires fresh history; late available text cannot restore it.
Accepting a newer live Actor observation retires any older in-flight Chat history
generation, so a delayed HTTP response cannot restore a running/proposed state
over the newer completion. These reads never auto-submit a draft or approval.

The terminal subscribes after each accepted bootstrap. A content-free invalidation,
subscription failure or local monotonic lease expiry retires the current view
before reading again. The monotonic deadline starts before the bootstrap request,
so queueing/network time cannot extend validity. Actor, Work, notices, diagnostics
and detail reads belong to that connection generation. Retirement aborts old
readers and drops late queued reads. An observed invalidation establishes a
revision floor even before the replacement snapshot arrives. Lower snapshots
and foreign instances cannot be installed. The event stream has a 16 KiB total
byte budget and carries no source body.

Retirement clears rendered history, source-derived previews, task/device lists
and permission review panels. Chat/Worker drafts and exact unresolved operation
identities remain. In-flight effect requests are not cancelled with read tasks:
their exact receipts may settle pending state, but stale receipt bodies are not
retained in the display. Reconnection never submits a draft, decision, follow-up
or cancellation. Local Worker lifetime and reviewed resource scope are unchanged.
The existing native Worker subview remains independent; root results/deadlines
are drained before returning to root rendering.

Transient bootstrap/stream failures use read-only backoff from 2 to 30 seconds.
HTTP 401/403/426 from bootstrap, ordinary reads, notices or current-generation
effect/recovery responses clears the cache and blocks automatic reconnect.
Ctrl-R explicitly retries after configuration is corrected. Late effect receipts
cannot restart a blocked connection. Successful synchronization rechecks the
bounded read-only chat diagnostic without sending retained input. Effects whose
outcomes are unknown continue to require the existing explicit recovery/retry.

Optional `client-sync-cache.json` is stored in the existing native private local
settings directory, outside Git. It is at most 16 KiB and contains only format,
owner role, endpoint, Kernel instance, resource hashes/capabilities, timestamps
and an untrusted conversation routing hint. It contains no credentials, messages,
memory, work bodies or permission scopes. Resource hints expire in at most five
minutes and are expired on connection retirement; the routing hint lasts at most
seven days and is usable only after authenticating that endpoint/instance and
fetching checked conversation state. An explicit conversation overrides it. A
cache record never confers live freshness or permission. There is no offline
semantic/transcript cache; cold startup still requires a reachable Kernel.

Existing unsafe ACL/mode/link state is refused without repair. Replacement uses
an exclusively created private same-directory stage, file sync and atomic rename
(plus directory sync on Unix). Corrupt or oversized optional metadata can be
replaced by a fresh bounded snapshot without modifying server state. Cache I/O
failure does not prevent fresh authenticated network use. On runtime disconnection,
input remains editable while sending/reviewing is blocked until synchronization.

## Acceptance boundaries

Deterministic tests exercise the actual authenticated bootstrap/events routes,
snapshot/subscription gaps, identity recovery, transactional/idempotent revision,
multiple deadlines, expiry-only events, stale/foreign/future cursors, terminal
withholding and late live/history races. Separate native evidence records actual
passes and ignored manual tests. Real HTTPS cross-machine Worker execution,
schema upgrade/database rollback, paid model/QQ and multi-day experience remain
separate; this contract does not authorize them automatically.

Earlier 2026-11-05 protocol increment: the same 225-input uncommitted source passes Kernel279/wire6 plus
actor10/actor-reliability7/foundation11/memory9/profiles5 on native Windows and
Linux (327 each, two manual ignored). Windows additionally passes 70 terminal
tests (one manual ignored), workspace/all-target strict Clippy and format. Eight
new focused tests cover this increment. Two isolated-copy clock/generation guard
removals fail their expected regressions; temporary mutation outputs were removed.
Independent static review found and rechecked the fixed late-history race.
Linux Clippy was not rerun; the bootstrap's earlier Rust1.89 style exception is
recorded separately. Runtime still uses schema28; these results are not upgrade
or automatic live-client acceptance.

Runtime consumer increment: native Windows passes 84 terminal unit tests, one
CLI HTTP integration and four shared-local tests (89 passed, one existing manual
test ignored), plus strict workspace/all-target Clippy and formatting. Eleven
HTTP/SSE lifecycle tests cover runtime changes, transport loss, local/queued
expiry, observed revision floors, stale reads, effect receipt retention, read
task cancellation, diagnostic recovery and fatal authentication paths. Native
Windows ACL cases also reject inherited, foreign, null/empty and hardlinked
replacement targets without altering their contents. Independent scoped static
review found the diagnostic/receipt/notice gaps, then rechecked their fixes.
The same 228-input snapshot passes the complete native Linux workspace: 429
tests, three existing manual tests ignored, including 83 terminal unit tests,
the CLI integration and two Unix local-file tests. Linux Clippy was not rerun;
the earlier bootstrap's Rust1.89 style exception remains separate. Native Windows
ConPTY passes ten real interaction cases with the selected portrait, Kernel
restart, parallel Worker inspection, durable rejection and configuration repair;
all five model calls are deterministic fixtures. Two independent-copy mutations
removing ordinary-read auth retirement and private target validation fail their
intended assertions; copies/targets were removed after hashing the restored
source. These native checks alone imply no deployment or real HTTPS Worker acceptance.

Subsequent deployment evidence: the cloud was explicitly backed up and upgraded
from schema28 to32. Native Windows verified the actual public HTTPS certificate,
401 without credentials, authenticated readiness/bootstrap, 426 for an unsupported
protocol and an actual content-free SSE resnapshot event. After an actual Kernel
restart, instance UUID stayed the same, boot UUID changed and revision advanced.
This transport acceptance performs no inference and does not by itself prove a
two-machine Worker execution loop. Subsequent Worker12 and memory7 cases use the
actual Linux service binary and native Windows clients over verified HTTPS; they
add fault recovery, exact permission application, changed-context/source
withdrawal across all rendered pages/reopen, and idle expiry. Paid inference,
real QQ, native macOS and multi-day experience remain unverified.
