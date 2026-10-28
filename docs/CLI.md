# CLI-first local development

Explicit normal private replies without a cumulative count/expiry: lain inference
private-daily <channel> --revision <n>. Other conversation/activity scopes remain
blocked; per-turn limits and accounting remain. Use inference stop to close it.
See [ADR 024](adr/024-explicit-private-daily-inference.md).

## D3c isolated replay — 2026-10-26

`scripts/daily_replay.py` drives a synthetic OneBot pair automatically, with a
persisted finite Kernel budget and owner-only output. Fixture mode is free;
real-model mode requires a new exact authorization record and environment
SecretRef. It never connects to personal QQ or approves work. Existing/interrupted
campaigns are inspect-only, with no automatic replay. See
[the five-step guide and evidence limits](DAILY_REPLAY_ACCEPTANCE.md).

## D3b inference management — 2026-10-26

`lain inference show/budgets/requests/enable/stop` uses authenticated Kernel APIs.
Enable requires explicit revision, immutable budget UUID, request cap and UTC
expiry. Stop retains usage; reopening an identical UUID cannot refill it. Request
records are content-free and partial latest-100 inspection does not limit actual
counting. See [D3b operation and release preparation](INFERENCE_BUDGET_ACCEPTANCE.md).

## D3a sharing receipt commands — 2026-10-26

`lain sharing recall <channel> --from-ms <UTC> --to-ms <UTC> [--text <topic>]`
searches current permitted projections; `sharing receipt <channel> <delivery>`
inspects one actual receipt; `sharing audit <channel>` shows content-free prepared
reads. These use the authenticated API and cannot send/retry or restore sources.
See [D3a controls and evidence](SHARING_RECALL_ACCEPTANCE.md).

## D2 sharing controls — 2026-10-26

`lain sharing show/enable/revoke/dnd/available/consider` manage independent
proactive contact over the authenticated Kernel API. Enable takes an explicit
complete ShareScope JSON file; available/DND uses a separate attention revision.
No field supplies a product default or live-operation permission. See
[D2 controls and upgrade boundary](SHARING_ACCEPTANCE.md).

## Reading processing and experiences (D1b, 2026-10-26)

Optional `reading enable --model-channel <id> --model-requests <cap>
--utc-offset-minutes <offset>` authorizes processing/disclosure separately from
acquisition-only configuration. `reading process/show/cancel/budget/audit` expose
actual jobs, conservative request reservations, reported usage and content-free
receipts. `experience search/show/history/correct/withdraw` control source-linked
projections. Withdraw also suppresses the exact URL; `reading suppressed` inspects
it, and `reading restore-source <url>` permits only new explicit acquisitions.
No command enables contact or restores old jobs/content/budget. Full five-step guide
and evidence distinction: [D1b acceptance](EXPERIENCE_ACCEPTANCE.md).

## Bounded text acquisition (D1a, 2026-10-26)

`lain reading grant/grants/enable/revoke` manage an independent default-off exact-URL
authorization. `reading start/list/show/cancel` inspect/control durable acquisition
activities. Start requires a caller-saved `--request-id` and grant `--revision`;
enable requires explicit URL, expiry, retention and request/body limits. No model
is called, and an acquired body is not a claim of reading/comprehension.
See the [five-step guide, API and rollback boundary](READING_ACCEPTANCE.md).

> **2026-10-23 scope:** this is the current local CLI operating guide, not the
> next product priority. QQ daily companionship now comes first; see
> [PRODUCT.md](PRODUCT.md) and [ROADMAP.md](../ROADMAP.md). Cloud pairing, a local
> background execution agent, Lain ASCII presentation and remote session control
> are planned, not available commands. Current `lain up` remains foreground.

Windows PowerShell and macOS zsh are equal development targets, sharing the same
Rust crates, commands and authenticated HTTP/SSE API. Linux remains the eventual
server target. `web/` is retained and frozen; npm, Vite and a browser are optional.

## Install the native binaries

Install stable Rust and a C compiler (MSVC Build Tools on Windows; Xcode Command
Line Tools on macOS). From the checkout, in either shell:

```text
cargo build --locked --workspace
cargo install --locked --path crates/lain-kernel --bin laind --force
cargo install --locked --path crates/lain-client --bin lain --force
lain setup --local
lain up
```

`lain up` stays in the foreground. In another terminal:

```text
lain doctor
lain status
lain profile list
lain chat
```

Cargo installs into its configured bin directory (normally `$HOME/.cargo/bin`
on macOS and `$env:USERPROFILE\.cargo\bin` on Windows). If PATH does not select
Rust's binary, use the explicit path. PowerShell development without installing:

```powershell
.\target\debug\lain.exe setup --local
.\target\debug\lain.exe up
# Another PowerShell:
.\target\debug\lain.exe doctor
.\target\debug\lain.exe chat
```

macOS zsh development without installing:

```zsh
./target/debug/lain setup --local
./target/debug/lain up
# Another zsh:
./target/debug/lain doctor
./target/debug/lain chat
```

Stop foreground `lain up` with Ctrl-C; Windows also supports Ctrl-Break. The
launcher and child share their native console/process group. It waits for laind's
existing graceful HTTP/worker/storage shutdown; it is not a service installer.
A second `up` checks the authenticated daemon's identity/version and readiness,
then returns `already_running`. It never kills an occupying process. Old daemon,
authentication mismatch and unrelated service occupancy fail with diagnostics.
Port-check/start races can still cause a bind failure, safely without killing
anything. No implicit local startup occurs from other CLI commands.

## Local configuration and credential boundary

| Host | Default private directory | Protection |
|---|---|---|
| Windows | `%LOCALAPPDATA%\Lain` | Current user's SID owns the directory/files; protected owner-only DACL with inheritable FullControl on the directory |
| macOS | `$HOME/Library/Application Support/Lain` | Owner-owned directory 0700; ordinary single-link files 0600 |
| Linux | `$XDG_CONFIG_HOME/lain`, otherwise `$HOME/.config/lain` | Same Unix owner/mode checks |

`LAIN_LOCAL_DIR` selects a different absolute directory outside a Git checkout.
Settings: `client.json` (version, local URL, absolute SQLite path and optional
explicit Actor JSON/env-file paths). Authentication:
`kernel-token` (32 bytes from the OS random generator encoded as 64 hexadecimal
characters). They are separate files; neither is source code or SQLite state.
`setup.lock` serializes setup and atomic settings replacements. All are private.
No encryption/keyring is invented.

Save Actor settings with `lain config actor set --file <actor.json>` and a protected
credential source with `lain config actor env-file --file <actor.env>`. Inspect with
`lain config actor`; use `lain config actor clear` to remove selections without
deleting their files. `lain up` reuses saved paths, or accepts `--env-file <path>`
for one launch. `lain doctor --env-file <path>` checks that same selection.
Only explicit trusted files are loaded; no workspace `.env` discovery/shell execution.
Precedence, supported keys, native permission preparation and safe explicit restart
are in [LAIN_ACTOR.md](LAIN_ACTOR.md#save-an-explicit-local-selection).

First setup restricts the directory before writing any credential and creates
files exclusively. Repeated setup validates and reuses the existing token/settings;
it never rotates/overwrites them. Interrupted setup can reuse an already-created
valid token. A malformed existing file, missing token in an existing setup, wrong
owner, permissive mode/ACL or linked file fails closed with files retained. Setup
does not silently repair permissive existing credentials. Restore their correct
permissions deliberately or restore a trusted backup. Tokens are absent from
normal output, command arguments, Git, model input, logs and operational history.

Windows ACL handling runs native Windows PowerShell's .NET security APIs with a
literal data path and a cleared/OS-allowlisted environment; no shell source interpolation or Lain credential in that process.
ACL inspection checks the current SID and rejects non-owner access. PowerShell
must be available; unsupported filesystems/ACLs fail explicitly. Elevated sessions
whose files are owned by a different SID may need an intentional owner correction.
Unix uses native filesystem modes and owner/link checks. Tests run only on their
real OS; neither platform is simulated on the other.

Existing `LAIN_API_TOKEN` overrides the saved token. With an explicit token,
`LAIN_URL` selects a remote endpoint; without it, an endpoint different from saved
local configuration is refused to avoid sending the local credential elsewhere.
With a token but no URL, the legacy `127.0.0.1:8787` default remains. `lain up`
rejects `LAIN_URL`, `LAIN_LISTEN` and `LAIN_DATABASE_PATH` overrides: it uses the
explicit local setup. Use `laind` directly for custom deployment configuration.
`LAIN_DAEMON_PATH` selects an explicit native binary; otherwise `laind` must sit
beside the running `lain`. The launcher's local database survives exits/restarts.
Existing project `data/lain.sqlite3` is not automatically imported; to retain
previous sessions, deliberately point local client.json's database to that trusted
absolute database while both daemons are stopped. Preserve WAL/SHM backup rules.

Actor configuration and `LAIN_SECRET_*` are daemon settings, inherited at launch;
other terminals need only saved local authentication. See [LAIN_ACTOR.md](LAIN_ACTOR.md).

## Diagnostics and PATH conflicts

`lain doctor` performs no inference/provider call. It reports CLI/native executable,
authenticated Kernel reachability/readiness, SQLite quick_check, backend executable,
exact supported Codex version, profile health/missing SecretRefs and Actor model
configuration. Local prerequisite health is not a claim about account/model quota.
`--json doctor` exposes the structured report. Unreachable/authentication/storage
failure exits nonzero. Backend/Actor/profile problems are reported separately so
worker commands can remain usable without Actor credentials.

An obsolete npm `lain` can shadow Rust on Windows. Inspect:

```powershell
Get-Command lain -All
where.exe lain
& (Join-Path $env:USERPROFILE '.cargo\bin\lain.exe') doctor
```

zsh: `whence -a lain`. Doctor lists matching PATH files and its own executable.
Check the installed Rust `lain --version` and use an explicit path until you
choose how to resolve the conflict. Lain never uninstalls npm packages or rewrites
PATH automatically. It cannot diagnose its own shadowing if the obsolete command
runs first; the explicit Rust invocation is the fallback.

## Commands

Existing work/profile/approval commands and the worker TUI remain available.
Worker workspace arguments are verbatim **daemon-host paths**, even in local
mode; there is no implicit current-directory or `--here` resolution.

```text
lain work list
lain work start --workspace <absolute-daemon-path> --profile codex/local
lain work send <session-id> "inspect tests"
lain work watch <session-id>
lain work turns <session-id>
lain work history <session-id>
lain profile list
lain approval list <session-id>
lain approval approve <session-id> <approval-id>
```

Watch only observes HTTP/SSE; Ctrl-C never cancels work. Existing attach/TUI
behavior is unchanged. UUIDs remain exact; shortened/current work references are
not implemented in this phase.

`lain --json work watch <id>` flushes one compact JSON record per line: the initial
session snapshot, then normalized events. Diagnostics stay on stderr. `work turns`
reads actual worker-turn states, including completion/failure, independently of
the Actor's submission receipt. Text-client signal handling shares the launcher's
narrow native boundary: SIGINT/SIGTERM on Unix, Ctrl-C/Ctrl-Break on Windows.
Native Windows terminal behavior still requires its own acceptance run.

Q1 native Windows regression also exercises local credential ACLs. Newly created
objects may initially belong to Administrators under an elevated token; setup
assigns the current user and owner-only protected ACLs before writing credentials.
The final directory is created exclusively so a concurrent existing directory is
never silently adopted. Existing paths remain check-only: setup does not take
ownership or repair their permissions. The ACL helper loads the modules shipped
with its Windows PowerShell runtime rather than inheriting PowerShell 7/user module
paths. This does not establish interactive ConPTY
acceptance; see [Q1_IMPLEMENTATION.md](Q1_IMPLEMENTATION.md) for current evidence.

```text
lain chat
lain chat --conversation <conversation-id>
lain chat --timing
lain ask --timing "你好"
lain --json ask "What work is running?"
lain --json ask "Continue" --conversation <conversation-id>
lain --json ask "Same text" --conversation <id> --request-id <same-request-uuid>
lain conversations
lain conversation <conversation-id> --after <cursor>
lain actor show <turn-id>
lain actor approve <turn-id> <action-id>
lain actor deny <turn-id> <action-id>
lain actor cancel <turn-id>
```

REPL: `/help /status /work /conversations /history /new /exit`. Confirmation shows
the exact persisted tool/arguments and defaults to denial; EOF retains a pending
proposal. Explicit `actor approve` is an owner decision, never automatic approval.
Noninteractive ask returns `pending_confirmation` with proposed actions and does
not execute them. JSON turn snapshots contain conversation/turn/request IDs,
messages, typed actions/results, errors, usage and final state. Errors are a JSON
envelope on stderr with nonzero exit; stdout contains the command's structured
result when one exists. Unknown/failed/cancelled ask or explicit action outcomes
exit nonzero. Watch is JSON-lines when requested.

On stream loss the client resnapshots the **same turn**, at most three attempts,
and deduplicates message IDs/revisions. It never resubmits text automatically.
If a submission response is lost, use the reported conversation/request ID and
identical text to retrieve the original accepted request. A changed payload under
that request ID conflicts. One unfinished turn per conversation is allowed.
Ctrl-C during generation explicitly asks the Kernel to interrupt that Actor turn;
possibly delivered mutations remain uncertain. Ctrl-C at the input prompt exits
chat; workers keep running. On resume the recent 20 turns include the latest
pending/running state; older records use conversation pagination.

Assistant previews are visible before provider completion and remain ephemeral
until a validated complete message commits. Final snapshots do not repeat already
displayed text. Failure/discard notices identify partial previews as unconfirmed.
Terminal controls are escaped, including Unicode bidi controls. Input is owned by
one scoped native terminal read; no separate stdin reader runs during generation.
Empty Enter at `You>` keeps the current prompt. LF and CRLF are handled explicitly;
the paired LF cannot answer a subsequent confirmation. Empty Enter at `[y/N]`
denies. Ctrl-C/EOF at confirmation exits and retains the proposal for review.

`--timing` emits safe measured durations to stderr, including in JSON mode:
CLI submission/first-visible/total milliseconds plus Kernel queue and per-request
first-body-byte, useful-text, preview, completion and commit milliseconds. Null
means no observation. CLI totals cover the active phase and exclude owner waiting;
provider timings are relative to that provider request. No conversation content
or credentials are included. This measures observed latency rather than guessing
why a provider is slow.

## Phase 6.6 delegated results in chat

After one explicit confirmation, delegated work runs independently while chat
returns to normal input. Its actual Completed/Failed/Cancelled/Unknown outcome
arrives as a separate `Worker>` notice in the same conversation, without another
owner message or model request. The excerpt contains available worker report,
command failures/errors, changed paths, truncation and a full-history command.
Completed is not a claim that every command/test passed. Missing final reports
are labelled explicitly rather than summarized by a model.

```text
Tool> work.create · awaiting confirmation
      Action: <action-id>
      Exact arguments: { ... full persisted proposal ... }
Confirm this exact action? [y/N] y
Tool> work.create · operation accepted
      WorkSession: <session-id>
      Task submitted; worker results are delivered separately.
You>
Worker> turn completed
        WorkSession: <session-id>
        Turn: <worker-turn-id>
        Worker report:
        <actual bounded report>
        Full history: lain work history <session-id>
You>
```

One input/output coordinator preserves partially typed text, including wrapped
Unicode input, around notices. Background observers/acknowledgers do not print.
Human receipts are compact; exact proposed arguments remain visible before
confirmation and complete receipts remain in `/history`/Actor inspection. Existing
`--json ask`, worker history and machine snapshots retain their schema.

Known slash commands match explicitly. Multi-component Unix paths, drive/UNC
paths and paths containing spaces are ordinary conversational input; there is no
client-side filesystem resolution. A bare single slash word such as `/statu`
remains reserved for unknown-command feedback because it is ambiguous with a
root directory. For a one-component directory use `/name/` or `path: /name`.
Unknown commands never execute a mutation.

Inspect without model inference on PowerShell or zsh:

```text
lain notices <conversation-id> --after 0
lain delegations <conversation-id>
lain work turns <session-id>
lain work history <session-id>
```

Notice pages retain all entries, including acknowledged ones; `next_cursor` and
`has_more` support pagination. Chat acknowledges display after flushing output,
and subsequent opens skip acknowledged notices. Within one process a cursor
suppresses ordinary reconnect duplicates. Delivery is at least once: crash,
network failure or an unqueued acknowledgement can repeat a notice on reopening.
Owner clients share display acknowledgements; these never approve worker
permissions or accept result correctness. Workers survive chat exit. See
[ADR 007](adr/007-durable-delegation-result-delivery.md) and the
[Phase 6.6 acceptance procedure](PHASE_6_6_ACCEPTANCE.md). Real acceptance is opt-in.

## Native acceptance

```powershell
powershell -NoProfile -File scripts/check-native.ps1
```

```zsh
zsh scripts/check-native.zsh
```

Both run native build, fmt, all-target Clippy, deterministic tests and CLI smoke.
The smoke refuses a mismatching `--platform`; it uses native processes, permissions
and shutdown signals, a local deterministic Responses server, and the existing
fake Codex App Server. macOS additionally exercises actual controlling PTYs:
LF/CRLF, empty/slash/consecutive inputs, gated Unicode streaming before completion,
no-preview/failure, JSON/timing separation, denial, approval, Ctrl-C during input
and generation, EOF, restoration, reconnect checkpoints and detached inference.
Windows ConPTY input acceptance remains pending. Linux can run the same Cargo checks and
`python3 scripts/native_smoke.py --platform linux` on a native Linux host.
These are manual entrypoints, with no paid-model or oversized CI matrix.

Acceptance status for this change is recorded in DEVLOG.md. Windows and Linux
are unverified until these checks actually run on those hosts. Real provider
acceptance is separately opt-in, never inferred from fixture success.


## QQ diagnostics and private memory (Q1 repair / Q2-A)

All commands use the configured Kernel's existing administrator bearer credential
and invoke no model. Kernel Ready, saved binding and Online are separate states.

```text
lain channel list
lain channel show <channel-id>
lain channel memory-grant <channel-id>
lain channel memory-enable <channel-id> --revision 0
lain channel memory-grant <channel-id>
lain channel memory-revoke <channel-id> --revision 1
lain channel memory-grants <channel-id>
lain channel memory-history <channel-id> --after 0
lain memory list --channel <channel-id>
lain memory search "柠檬茶" --channel <channel-id>
lain memory show <memory-id>
```

Read the current revision before each grant change; example revisions are not
reusable defaults. Optional `--expires-at-ms <Unix-milliseconds>` requires a future
deadline. `memory-grants` returns the latest 100 decisions, newest first;
`memory-history` pages content-free effects (50/page, `next_cursor`/`has_more`).
`--channel` conflicts with other scope flags. Grant changes and reads do not
start a disconnected daemon or modify `napcat.json`.

The memory grant defaults off and only governs this binding's selection/recall/
correction/forgetting. It cannot approve Work or persona changes. Natural-language
correction requires a clear single target; inspect the resulting MemoryId and
revision, not merely the assistant's claim. A normal `memory.propose` still waits
for review. See [MEMORY.md](MEMORY.md) for provenance/deletion bounds and
[Q2_A_ACCEPTANCE.md](Q2_A_ACCEPTANCE.md) for safe upgrade and staged manual checks.

## Independent memory and factual summaries (Phase 7.1)

Commands use only the authenticated Kernel API and need no configured LLM:

```text
lain memory add "Both independent continuity and agent supervision matter" --kind preference --global --request-id <uuid>
lain memory add "Keep worker intelligence in worker harnesses" --kind project-knowledge --workspace <absolute-server-directory>
lain memory list --workspace <absolute-server-directory>
lain memory search "continuity" --global
lain memory show <memory-id>
lain memory disable <memory-id> --revision 1
lain memory enable <memory-id> --revision 2
lain memory supersede <old-id> --with <active-new-id> --revision 3
lain memory delete <memory-id> --revision 4
lain memory candidates --after 0
lain memory candidate <candidate-id>
lain summary create <work-session-id> --turn <worker-turn-id>
lain summary show <summary-id>
lain summary list <work-session-id>
lain memory from-summary <summary-id> "Selected observed fact" --workspace <absolute-server-directory> --request-id <uuid>
lain memory accept <candidate-id>
lain memory reject <candidate-id>
lain --json memory list
```

Memory kinds are `episodic`, `project-knowledge`, `preference`, `reflection`.
Scope defaults to Global; `--global`, `--internal`, `--workspace` are mutually
exclusive. Project knowledge requires Workspace; reflection requires Internal.
Daemon-host paths are sent verbatim by the client, then canonicalized by Kernel.
PowerShell users should quote paths/text with spaces as usual; native Windows
verification remains outstanding. No client SQLite access or browser is involved.

List/search defaults to Active; `--status active|disabled|superseded`, `--kind`,
`--limit` (1–50), `--after` select a bounded deterministic page. Read commands do
not mutate or invoke a model. JSON uses typed IDs and stable snake_case enums;
`show` exposes content, provenance, confirmation timestamp, status and revision.
A returned `next_cursor` continues only when `has_more` is true. Pages may contain
fewer records than `--limit` to honor their byte bound. Summaries list the latest
50 for one session; older summaries remain inspectable by ID.

A direct add is the owner's explicit exact statement and immediately commits.
Supply and retain `--request-id` when retrying creates/summary selections; default
UUID generation treats each deliberate invocation as a new request. Changed text
with an existing ID conflicts; retrying an already-deleted create cannot restore
it. Status changes/delete require the inspected revision to avoid stale edits.
Delete removes semantic content/index and scrubs matching candidate copies while
retaining independent conversation/worker/summary audit; it is not secure erasure.

Actor proposals instead wait for their existing exact-action confirmation. Inspect
with `memory candidate <action-id>` and approve/deny through `lain actor` or chat;
generic `memory accept/reject` cannot bypass that action. Actor proposals remain
Inferred after acceptance. `from-summary` creates ProjectKnowledge candidates only
in that summary's workspace; owner acceptance remains UnverifiedSelection.
Neither summary creation nor a Completed worker turn proves all commands passed,
and neither automatically creates memory. See [MEMORY.md](MEMORY.md).

These operations inherit the existing protected local directory/owner-only ACL
setup and trusted-transport remote boundary. Keep manual `laind` data in a dedicated
owner-only directory; never use a shared project folder for personal memory.

## Private history source lookup (Q2-B1)

These authenticated commands are administrative. The Actor cannot create grants.
Use actual ChannelIds from `lain channel list` and Unix **UTC receive** milliseconds;
the interval is half-open, at most 31 days and must end no later than enabling.

```text
lain channel source-grant <channel-id>
lain channel source-enable <channel-id> --revision <current> --from-ms <inclusive> --to-ms <exclusive> [--expires-at-ms <future-utc-ms>]
lain channel source-revoke <channel-id> --revision <current>
lain channel source-grants <channel-id>
lain channel source-search <channel-id> [--text <literal-keywords>] [--from-ms <inclusive> --to-ms <exclusive>] [--limit 3] [--after <exact-next_cursor-JSON>]
lain channel source-inspect <channel-id> <incoming-id> [--from-ms <inclusive> --to-ms <exclusive>]
lain channel source-reads <channel-id> [--after <sequence>]
```

Enable/replacement is CAS, not an upsert with a guessed revision. Default is off;
reply/memory grants remain independent. Search limits are 1–10 (Actor 1–3), encoded
records 8 KiB and scan 512 rows/page. A partial no-match is not proof an event never
happened. Copy the returned JSON cursor unchanged with identical text/window;
changed grant/filter requires a fresh query. Paired time options narrow the grant.

API: `GET/POST /v1/channels/{id}/source-grant`,
`GET .../source-grant/history`, `POST .../sources/search`,
`POST .../sources/inspect`, `GET .../source-reads?after=0`.
Search body is `{text:null,window:null,after:null,limit:3}`; inspect body is
`{incoming_id:<actual UUID>,window:null}` (valid JSON uses quoted field names).
Grant body has `enabled`, `expected_revision`, `window` and `expires_at_ms`;
disabling requires both window and expiry null. Enabling requires window with
`from_received_at_ms` and `to_received_at_ms`. Reads/decisions share owner bearer
middleware and no-store. No raw keywords/source body is stored in read audits.

Schema 010 requires a stopped, consistent pre-upgrade backup for rollback; old
binaries cannot read it. See [Q2-B1 acceptance](Q2_B1_ACCEPTANCE.md) for the isolated
repeatable fixture command and at most five separately authorized live steps.

### C1 private chat inspection and upgrade boundary

Current development uses schema 011. New private Actor snapshots add
`private_chat` with decision, wait/reconsideration counters, admitted input
sequence, IDs/revisions of memory and bounded source selections/fingerprints
supplied in the latest request. Existing
`lain actor show <turn-id>` exposes these fields; `lain channel show <id>` exposes
original input/batch associations, delivery purpose and `superseded` unsent drafts.
These are operational records, not authorization or human-read receipts.

`doctor` retains identity `persona_revision` and configured daemon diagnostics add
`private_chat_revision` for the assembled identity/policy/behavior/expression
bundle. No new chat-management command, approval shortcut or npm step is required.
The separate [C1 guide](C1_ACCEPTANCE.md) provides an offline process command,
multi-turn experience pack and five manual acceptance steps. Schema 011 rollback
also requires a complete consistent stopped pre-upgrade backup and the old binary;
never run an old binary against the upgraded personal database.
