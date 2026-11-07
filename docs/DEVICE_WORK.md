# W4 conversation-bound device work

Implementation increment: 2026-11-03. [ADR 032](adr/032-conversation-bound-device-work.md).
Local deterministic Rust checks and eleven native cross-chain scenarios pass.
This is not complete W4 or real-provider/cross-device deployment acceptance.

## Direct task entry and exact-request receipt — 2026-11-05

Work → N now builds the existing QueueDeviceWork directly, with exact original
text and no Actor/tool action. The full device revision, workspace, profile,
sandbox and finite deadline are reviewed before explicit submission. Work/Chat
drafts remain independent; setup/diagnostics belong to Devices. See [Work desk](WORK_DESK.md).

Owner-only `GET /v1/conversations/{conversation}/device-work/{device}/requests/{request}`
returns the exact conversation-linked DeviceWorkRecord or null. It reads existing
command/link tables, introduces no schema, queues no command and invokes no Worker.
Device credentials cannot use this owner route. The client checks route support
before first dispatch, then checks the fresh target; old servers stop before POST.
An uncertain POST retains its request and only queries the same scoped receipt.
Receipt matching verifies the exact payload/origin; a truncated recent-work list
is not evidence that the original command is absent. No automatic retry/replay is
introduced, and process loss does not currently preserve a pending UI draft.

## Permission stages and explicit task cancellation

The selected task shows immutable cloud decisions separately from cumulative
device receipt/application reports. Exact matching and conservative native
resolution are specified in [execution evidence](DEVICE_EXECUTION.md). Native
allow/deny and missing-resolution cases pass; a completed turn without a matching
permission-resolution event retains Received, never Applied.

Terminal F8 reviews the actual device command, original task, workspace/profile/
sandbox, revision and execution/session/turn before Ctrl-K requests cancellation.
The existing scoped `POST .../cancel` is used after a fresh exact-revision GET;
this adds no new command protocol or automatic approval. Esc/navigation/exit do
not cancel. Queue cancellation is cloud state; reported native Cancelled is a
separate observation; a recorded request is not termination or rollback.

Ctrl-R recovers current facts after uncertainty. A cancellation flag proves a
current request exists, not which client submitted it. An unchanged uncancelled
revision leaves the outcome uncertain. A newer uncancelled revision prevents the
old reviewed request from later applying; a new stop needs new review. Explicit
`/cancel-retry` retains the original revision and preflight, never silently
adopts newer scope or replays a native RPC. Native work11/terminal8 passed for this
latest UI increment, including close-review/no-cancel, preserved Lain draft and
actual reported cancellation after Kernel restart. Seven client regressions also
pass; [whole W4 gates](W4_ACCEPTANCE.md) remain open.

## Same-session follow-up

W4c implementation, 2026-11-03; eleven native isolated cross-chain cases passed.

Schema028 adds explicit continuation families, not new Worker intelligence. The
original QueueDeviceWork stays unchanged. A follow-up is a new command/turn with
immutable DeviceContinuation: root command, parent command/reviewed revision,
original device-local session and previous turn. Workspace/profile/sandbox are
inherited; a continuation cannot select broader scope. See [ADR033](adr/033-same-device-session-continuation.md).

The parent must be the latest uncontinued mapped completed/failed turn without
cancellation/interruption/pending approval. Running steering, cancelled/unknown/
unmapped/rejected parents are not supported by this path. The parent gets a
continued_by pointer and revision update in the same transaction as child admission,
lineage, origin link and any confirmed Actor receipt. One child per parent forms
a linear chain; subsequent instructions target that returned child after it is
known idle. An unknown or rejected child does not authorize silently falling back
to an ancestor with another nonce or creating a replacement session.

### Owner API and CLI

Under `/v1/conversations/{conversation}/device-work/{device}/{parent}/followups`:

| Method | Contract |
| --- | --- |
| POST | FollowupDeviceWork; returns the actual child command, never a fabricated WorkSession |
| GET | Current single recorded child or null; read-only, no Worker RPC |

The request contains request_id, expected_device_revision, expected_parent_revision,
exact text (1..8192 UTF-8 bytes) and a fresh finite expires_at_ms. Existing current
device authority/deadline checks apply. Wrong conversation/device, stale parent,
busy/uncertain parent, payload change or another existing child fails closed.
An exact same-origin request recovers its original child after later turns/restart;
it does not requeue or replay the backend. Device credentials cannot use owner APIs.

```text
lain device command continue <device> <parent-command> --conversation <conversation> --file <exact-followup.json>
lain device command continuation <device> <parent-command> --conversation <conversation>
```

Actor `device_send` uses the same explicit per-action confirmation as dispatch;
the actual ActionId supplies RequestId, original user records remain untouched,
and Changed/Unchanged inspection shows the actual outbound text. Confirmed action
receipt, child admission and lineage commit atomically. No automatic enrichment.

### Unified terminal

In the selected device task, F6 reads the actual conversation-bound parent and
device, then opens a separate remote Worker composer. Alternatively Chat accepts
`/continue <device-uuid> <parent-command-uuid>`. It never sends on opening.
Work shows one current head per remote family with the original session identity;
Devices retains separate per-command history. Enter on an already continued
parent opens its recorded child. Full IDs remain inspectable in task details.

Enter previews the complete target/session/workspace/profile/sandbox and exact
request/text with Unchanged for direct owner input. Contiguous full reading is
required before Ctrl-Y sends. The client rereads current parent/device before the
first POST; Kernel independently enforces revision/authority. Esc edits/closes
without sending; Tab returns to Lain. Ctrl-R before submission rereads the target,
retains both drafts and clears the old review. Per-target Worker drafts stay separate from
the Lain chat draft and survive in-process navigation, not process loss.

While a submission is pending, its Worker input is frozen; Lain chat remains
available. Ctrl-R reads the recorded child. Null does not prove an earlier POST
failed, so the exact RequestId/text remain pending. `/followup-retry` explicitly
retries that identical cloud request only after a read; there is no automatic
POST or native replay. Another client's recorded child is identified as another
decision, not acceptance of this client's task. A confirmed queued child is not
proof of execution or goal completion. Native full-terminal entry remains absent
unless a backend actually advertises it.

### Native binding and uncertainty

The host requires the original protected parent intent/submission fence with the
same manifest/policy, workspace/profile/sandbox, family, session and previous turn.
It also requires the actual operational provider binding/latest known idle turn
before acceptance/launch. Missing original scope rejects fresh unaccepted work
without creating a replacement session or stopping unrelated service work.

The child has distinct deterministic execution/turn IDs and its own durable
launch/submission/outbox/permission fences. Only a separately verified idle parent
allows a session binding to predate the child's launch fence. An unexplained root
binding still stays Unknown. A submitted unmapped child still stays Unknown and
cannot be replayed. The host resumes/reuses that original Supervisor session;
ordinary input does not become a generic planner or tool loop.

Native expanded acceptance passed all eleven cases. Initial real-process rehearsal found
the old root recovery rule incorrectly classified the verified existing parent
binding as Unknown. A minimal red regression reproduced this; the narrowed idle
parent branch passed the original no-replay rules and the full native chain.
The final client also prevents a late historical queue receipt from overwriting a
newer matching completed snapshot. This slice is not complete W4 acceptance.

## Identity and exact dispatch

The cloud Kernel retains the conversation and its dispatch origin. A device
command retains the original `QueueDeviceWork`; its execution/session/turn belong
to the separate native device Supervisor. No cloud-local WorkSession is invented.

Schema027 adds `device_work_links`, indexed by originating conversation and
creation time. Command creation, metadata change, link and any confirmed Actor
partial receipt commit together. A failed link/receipt leaves nothing for a
device to execute. An existing unrelated command cannot be adopted or moved to
another conversation. No personal database is copied or upgraded by development.

The typed `device_queue` proposal carries actual device/workspace IDs, expected
device revision, `codex/local`, sandbox, exact task and finite UTC deadline.
The model cannot choose its RequestId: Kernel uses that particular ActionId's
UUID. The original user request stays in Actor history; the complete proposed
tool arguments are shown for existing explicit owner action review before queueing.
No compiler, extra Worker intelligence or permission expansion is introduced.

The terminal dispatch review also displays `Changed` or `Unchanged` by exact text
comparison with the current turn's original user records. A single byte-identical
record is Unchanged; multiple records stay separately attributed and are Changed
rather than inventing one original. JSON string representations preserve all
whitespace/control characters, and a Changed review shows the complete replacement
diff plus the actual full tool payload. This is text inspection, not a semantic
fidelity claim or an enabled prompt compiler. The dispatch RequestId is visible.
Missing/mismatched original records cannot be approved. The same complete review
is recomputed before a decision; changed originals or arguments invalidate it.
The same text inspection applies to cloud-local work.create/work.send actions.

Direct owner dispatch can link to an existing conversation:

```text
lain device command queue <device-id> --file <exact-work.json> --conversation <conversation-id>
```

Without `--conversation`, the existing independent command API retains its old
semantics. Retries keep the same complete input and RequestId. A queued receipt
does not prove a host is running or that a Worker accepted/completed the work.
Pairing, proposing or queueing installs/starts no device service.

## Authenticated owner API

| Route under `/v1/conversations/{conversation}/device-work` | Behavior |
| --- | --- |
| `POST /{device}` | Exact QueueDeviceWork, atomically linked on first creation |
| `GET /` (base route, no trailing slash required) | Latest 32 linked command projections, `truncated` if more |
| `GET /{device}/{command}` | One actually linked command; wrong conversation returns NotFound |
| `GET /events` | `device_work_snapshot` SSE, fresh bounded current projection |

All require the administrator principal and no-store responses. Device credentials
cannot use these routes. The ordinary device-principal ledger/event/decision
protocol is unchanged. The stream subscribes to existing device notifications
before its first read; on changes, lag or the finite expiry timer it reads durable
current records. It starts no work or model and invents no replayable history.
Slow consumers are bounded; shutdown closes observation. Reconnecting repeats a
safe snapshot read, never dispatch or a Worker RPC.

## Lain context and terminal observation

Ordinary owner-triggered inference receives at most three recent linked work
projections, tagged as Kernel-attributed untrusted device execution evidence.
`device_work_list` explicitly returns up to eight; `device_work_inspect` reads one
linked exact record. These remain within existing Actor request/tool/context
budgets. `device_list` exposes declarations/revisions/workspaces/expiry and a
last-report timestamp, omitting credential fingerprints. It explicitly does not
claim that registration/inventory proves an execution service online.

Device output, command failures and reported relative paths remain attributed
reports. Completed is a Worker-turn fact, not goal acceptance. Paths are not
uploaded, opened or independently verified artifacts. Reports are not memory or
instructions. Reading/sharing background activities do not acquire these tools
or trigger work summaries. No unsolicited QQ delivery is enabled.

The pure Rust terminal observes snapshots concurrently with Actor conversation.
Work lists cloud-local Workers and this conversation's device commands, with
different routes: Enter on remote work opens its device details, never a fake
local attach. Meaningful state/permission/result changes add bounded factual
chat updates. Identical/lower-revision snapshots do not repeat or regress them.
On recovery, newer tasks appear after older ones in chat while Work stays newest
first. Each full command ID has its own line. Drafts and read positions remain
frontend state. `desk --conversation` restores the explicit same conversation.

## Remote permission review

In a device task, F4 opens the actual pending Worker permission; in Chat, F4 keeps
the separate Lain action review. The remote view shows device/command/revision,
execution/session/turn, workspace/profile/sandbox and the complete actual
permission description/fingerprint/choices. Every displayed page must be reached
contiguously; jumping over unshown scope cannot enable a decision.

Only offered choices appear: Ctrl-Y allow once, Ctrl-D deny, Ctrl-K cancel.
Truncated/redacted/incomplete scope never offers allow. No default choice. Existing
cloud decision receipts are fetched with task details, and an already decided
approval cannot be offered again. Closing the view has no permission effect.

Before a new decision POST, the client rereads the current exact command and
checks scope/identity/revision/expiry/cancellation. Kernel independently enforces
current authority and fingerprint; the native host separately checks policy and
persists its response fence before its existing Supervisor call. A stale local
view or a model statement cannot authorize an approval.

An interrupted POST retains its original decision RequestId/fingerprint/choice.
Ctrl-R reads receipts; `/decision-retry` is an explicit retry of the identical
cloud receipt request. No automatic POST replay and no new choice/identity.
Receipt recovery does not replay an uncertain backend mutation. If preflight
fails before any POST, the client says no decision was submitted. A receipt from
another concurrent explicit decision is displayed as another decision, not as
acceptance of this client's choice.

The interface distinguishes the recorded cloud choice from device receipt and
Worker application: a cloud receipt proves only the former. Subsequent native
execution evidence is separate. Do not label a database write as backend success.

## Verification and remaining work

Latest increment, 2026-11-03: full workspace 350 passed, 0 failed, 3 existing
manual/live ignored; workspace format, strict all-target Clippy and native build
passed. Expanded `w4_work_smoke.py` passed seven cases, adding prerequisite-only
diagnostics (before service start and at an actual running quiescent boundary) and
unreachable-cloud reporting to the original five; 3 fake requests, 0 paid/QQ,
cleaned state. It now exercises Changed/original/outbound/full-scope page reading.
The same source then passed all 14 existing host and eight desk native regressions.
Cargo test relinked regular binaries between those runs: separate actual hashes
are preserved, not claimed identical. Latest report/images:
`<local-evidence>/lain-w4b-2026-11-03/fidelity-diagnostics-attempt5/`
and sibling `fidelity-diagnostics-terminal/`. Three new tests cover exact text,
multiple/escaped originals, missing origin and stale-original confirmation.

The following earlier dispatch increment remains historical evidence:

Native Windows GNU verification: full workspace 346 tests passed/3 existing
manual/live ignored, then the final client restoration fix and extra regression
passed 19 targeted desk tests with strict client Clippy/build. That covers 347
distinct Rust tests; it is not another full workspace rerun after the final edit.
Strict workspace Clippy/build passed before that client-only restoration edit.

`w4_work_smoke.py` passed all five scenarios: confirmed natural Actor dispatch,
exact remote deny/factual conversation result, explicit allow verified in native
Supervisor state, parallel chat/frontend close/restore/exact retry, and Kernel
restart with the same remote session plus actual cancellation evidence.
There were 3 fake Responses requests, 0 paid calls and 0 QQ sends; owned temporary
state was cleaned. Same-machine real processes do not establish remote TLS or
paid-Codex acceptance. Local-only report/images:
`<local-evidence>/lain-w4b-2026-11-03/work-attempt-3/`.

The final native binaries also passed all 14 existing device-host cases (including
uncertain submit/event receipts, crash reconciliation, network cutoff and revoke)
and all eight existing desk/portrait cases. Those used respectively one and four
fake model requests, no paid/real QQ calls and cleaned state. Schema27 in the host
report is read from actual temporary storage rather than a hardcoded version.

Rust tests cover atomic rollback, original origin/RequestId, non-adoption and
conversation isolation, restart without a new Actor or cloud-local Worker,
owner-only API, snapshot changes, frontend independence, stale/unread/truncated
permission suppression, contiguous review, unknown receipt identity, and readable
restoration order. The existing migration fixtures reconstruct schema1/8–25
correctly by dropping the new link table too; no IF NOT EXISTS workaround weakens
production schema checks.

```text
cargo test --workspace --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo build --workspace --locked --offline
python scripts/w4_work_smoke.py --bin-dir <native-binaries> --artifacts <outside-checkout> --portrait-file <explicit-local-art>
```

The native test uses separate real Kernel/device processes, structured fake Codex,
fake Responses and ConPTY with temporary state. Its images render actual VT output
with synthetic content, not Windows Terminal GUI captures. The ConPTY F4 encoding
is SS3S; this machine drops legacy CSI14 before the application. Test reports and
failures must retain their actual verification scope.

Same-session remote follow-up and the full
W4 completion audit remain. The current protocol cannot silently reuse a session
across new commands to pretend follow-up support. GUI font fallback, other native
platforms, TLS, real paid providers and multi-day use remain separately unverified.
An explicit read-only [native prerequisite check](DEVICE_HOST.md#read-only-native-prerequisite-check)
now complements service operation; it deliberately makes no service presence claim.
