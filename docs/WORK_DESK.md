# Work-first local Agent delivery

## Real small-webpage workflow — 2026-11-05

A separate, short-lived writable project passes three actual Codex turns in one
native session through direct Work creation, F6 continuation and naturally routed
Lain Chat proposal plus explicit owner review. Parallel real chat, frontend
close/reopen during execution, all three instruction/result pairs and independent
browser interactions pass. The original device's read-only policy is unchanged;
the temporary device is stopped/revoked. See [scope, evidence and remaining setup
limitations](WORK_WEB_ACCEPTANCE.md). Earlier preparation-only checkpoints below
remain historical; this selected real workflow has now been exercised.

## Computer details and a preselected Work destination — 2026-11-05

Devices now opens a named computer detail page before its advanced task inventory.
It shows OS, registration expiry, historical capability/version reports and full
registered project paths. Up/Down selects a project; N opens Work with that exact
device/workspace preselected after a fresh GET. Drafts survive navigation. A
missing/revoked/expired destination leaves no fallback selected; explicit task
review, admission and local permission checks are unchanged.

C checks the cloud connection and rereads inventory; it cannot prove a running
host or provider login. No heartbeat or policy-report protocol is added, so host
presence and local write access stay unconfirmed. I provides platform-specific
startup/recovery instructions without executing them. F7 exposes non-secret
diagnostics; T opens the device-wide task inventory (including other chats),
separate from current-conversation Work. Esc returns through computer details.
Details support wheel/PageUp/PageDown and compact narrow-screen hints.

The scoped native Windows regression passes 143 client unit tests plus one CLI
integration case, strict client Clippy, formatting and build. The isolated actual
ConPTY smoke covers computer/details/startup/diagnostics, narrow layout and wheel,
exact Work destination and preserved Chat draft, with zero model calls, task
submissions or Worker starts. Linux/macOS UI behavior is unverified in this slice.
See [terminal design](TERMINAL_DESIGN.md#devices-电脑详情与项目入口--2026-11-05).

## Readable tasks and retained turn history — 2026-11-05

Work now lists original-request summaries, latest receipt time with local timezone,
state/known round count and the registered device directory. Selection exposes the
original/latest instruction and creation time. Missing directory/root metadata is
labelled, never guessed from the CLI working directory. Native Kernel workers
remain a separate source with only the facts their API actually supplies.

The default detail is an ordered instruction/result timeline for the selected
continuation family, rather than only its newest output. Missing recent-snapshot
ancestors use the existing scoped GET, with device/family/session/workspace checks.
At most 64 rounds are loaded; an incomplete chain is explicit. Pages contain at
most eight rounds and also enforce a conservative display budget. F5/Shift-F5
select older/newer pages without retargeting follow-up, cancellation or permission
controls. Each response remains a bounded device-reported result, not a full
terminal transcript or independent artifact verification. See TERMINAL_DESIGN.md.

## Current user acceptance and compact activity update — 2026-11-05

After the host repair, the user confirms real read-only Codex work returned project
summaries. The supplied screens include succeeded/completed reports, command-error
counts and truncation flags; this establishes that selected real task delivery
works, not independent result correctness or general Worker acceptance. Same-session
continuation and parallel live chat retain their separate checks.

Chat now presents stable compact task cards; old baseline work stays in Work.
F7 opens the activity menu in Chat and toggles result/diagnostics in a device task.
Work result text uses Markdown and independent source-based copying, while retaining
the current bounded/excerpt nature of device output. Tool details are folded and
all control/approval boundaries remain. See [terminal behavior](TERMINAL_DESIGN.md).

Selected 2026-11-05: the user prioritizes real local Codex work before further
companionship expansion, and subsequently explicitly deferred Claude Code.
Retain the accepted portrait/sidebar. Claude research below is future reference,
not an in-scope implementation requirement.

## Product flow

Work is the direct task entry: choose a declared execution device/workspace,
choose an actually supported Agent, enter the task, inspect the exact outbound
text and scope, then submit. Starting work requires no conversational model call.
Chat remains Lain conversation and optional natural-language dispatch. Devices
is connection/setup/diagnostics; it should explain missing prerequisites instead
of presenting an unexplained empty task list. No tab starts a runtime implicitly.

The cloud owns task intent, conversation association and durable observations.
The PC owns native execution, provider login and local execution policy. Never
send a Windows path to the cloud-local Worker endpoint as though it were a server
directory. Reuse the existing conversation-bound device-work API and Supervisor.

Direct task input is immutable and Unchanged; no prompt compiler is introduced.
The UI defaults to read-only. Explicit workspace-write selection remains subject
to the PC policy and actual Worker permission decisions. Queue acceptance is not
proof of a started process, completed work or correct result.

## Implementation sequence

1. **Direct Work entry.** New-task form, declared target selection, independent
   task/chat drafts, exact review, explicit submission, receipt recovery and
   actionable empty states. Add one read-only exact-request receipt route over
   existing stored commands/links; no new schema or execution API. First supported
   backend remains codex/local; do not offer a fictitious Claude execution choice.
2. **Usable local Codex connection.** Discover/select the native executable;
   verify its actual protocol, not only its name/version. Provide understandable
   pairing/workspace/host checks and explicit lifecycle operations. Run work while
   chatting, inspect output/permissions, cancel and continue the original session.
3. **Claude adapter.** Verify the installed CLI's structured contract, then add
   only justified shared supervision boundaries. Preserve provider-owned sessions,
   exact permission requests, cancellation and partial-failure semantics. Expose
   capabilities truthfully; no terminal scraping or fake Codex event translation.
4. **Daily workflow acceptance.** Native process fixtures first, then a separately
   selected finite real-provider task in an explicit workspace. Verify return to
   Lain, continuation, client exit without killing work and reconnect recovery.

## Local discovery and constraints — 2026-11-05

The PC exposes Codex CLI 0.166.0-alpha.5.1. At initial discovery, managed execution
accepted only 0.160.0. The exact alpha was added only after the contract/native
checks recorded below; installation alone does not establish provider readiness.
Unknown adjacent versions remain unavailable.

Claude Code 2.1.318 is present inside the official native VS Code extension,
outside PATH. Read-only --version/--help confirms stream-json input/output,
explicit session IDs/resume, safe-mode and permission options. Check the selected
executable at use time; extension auto-update paths are not stable public config.
Neither discovery command establishes login or provider access.

Protocol references checked 2026-11-05:
- [Codex App Server](https://developers.openai.com/codex/app-server)
- [Claude programmatic CLI](https://code.claude.com/docs/en/headless)
- [Claude CLI options](https://code.claude.com/docs/en/cli-reference)

Use per-session overrides and existing local authentication. Do not rewrite global
configuration, extract/copy provider tokens to the cloud, import external sessions
as controlled work or enable bypass-permissions. Claude's --bare skips subscription
authentication and is not a drop-in choice for the user's existing login.

## First-increment acceptance

- Work exposes a visible new-task action and explains its relation to Chat and
  Devices. Opening/navigation/cancelling the form submits nothing.
- Eligible device/workspace selection is based on an authenticated snapshot;
  registration and a historical report are never described as current presence.
- Task text is separate from the Lain draft, bounded, unchanged and inspectable.
- Review binds exact device revision/workspace/profile/sandbox/deadline/request.
  Recheck the selected target before POST; server authority remains decisive.
- Lost submission response retains the original request and recovers read-only
  from conversation-linked work. Do not create another request or replay a Worker.
- Disconnection retires target/review state while preserving task text and any
  unresolved receipt. A stale read cannot reopen the form or authorize submission.
- Returning to Chat does not cancel work; idle-only follow-up limitations remain
  explicit until a separately implemented running-steering capability exists.

The initial interface checks alone do not establish provider access, an installed
PC service or cloud rollout. The operational follow-up below records the actual
selected installation separately. No Claude adapter is introduced.

### Implemented first increment — 2026-11-05

Work now accepts N/n to open a direct new-task form. Select an active declared
Codex device/workspace, enter the unchanged task, choose read-only/workspace-write
with F5 and 15/60/480 minutes with F7.
Enter previews; full reading then Ctrl-Y submits. Esc backs up/closes; Tab returns
to Lain. Device registration/historical reports still do not prove presence.

The client verifies the selected device and the new owner-only GET
`/v1/conversations/{conversation}/device-work/{device}/requests/{request}` before
the dispatch POST. An older server without this route stops before any task is
queued. An uncertain response freezes the original request; Ctrl-R only reads its
receipt. The receipt is scoped to the same conversation/device/request and does
not depend on a truncated recent-work page. No new schema is introduced.

Target/review state retires on synchronization loss; the independent input and
unresolved request remain in this client process. Task drafts/pending requests
are not promised across process loss. Accepted tasks remain discoverable from
Kernel state. Late responses do not pull the user away from Chat or overwrite a
newer task snapshot. No frontend exit or navigation cancels the native Worker.

The installed Codex 0.166.0-alpha.5.1 also passes an isolated native initialization,
empty thread start/fork and same-ID resume probe with no personal config/login or
inference. Its initial empty-thread read returns the already handled -32601.
Subsequent selected-schema checks and both existing native Rust adapter probes
also pass. The explicit gate now accepts this exact alpha alongside 0.160.0 and
rejects unknown adjacent versions. Real provider execution/permissions remain
separate. Daily launcher/cloud service are not replaced by source tests.

New device initialization can explicitly select `--codex-work`; default setup
still declares no capability. This is an intended-work declaration, never a claim
of login, presence, host startup or permission approval. The host still verifies
the native version and local policy independently before any execution.

### Paste and native terminal evidence

F9 explicitly reads text from the local clipboard into the current editable
Chat/task/follow-up draft. It preserves multiline text and never submits it.
The read is bound to the original view, conversation, target and unchanged draft;
late results are discarded after navigation or editing. Whole-input byte limits
apply before insertion, without partial truncation. Clipboard contents are not
logged, persisted, sent to the Kernel or included in prompts by this action.

Ordinary bracketed Paste events are also routed to editable drafts, but native
Windows ConPTY testing found LF loss in crossterm's Windows key-event path.
Ordinary Windows terminal paste is therefore not promised to preserve multiline
input; use F9 for that path. Do not change global terminal bindings to compensate.

The native Windows ConPTY acceptance passes exact multiline F9 insertion, explicit
review/submission of exactly one task, correct Work-tab highlighting, preserved
Chat draft and persisted queued work after client exit. It uses an isolated
private window station with synthetic clipboard text, never the interactive
user clipboard. Zero model calls, Worker starts or paid requests occur. The latest
desk suite passes 91 tests (two existing native/manual checks ignored); the
separate native host fixture passes 16 scenarios. These are deterministic fixture
evidence, not real cloud/device/provider execution acceptance.

### Selected installation follow-up — 2026-11-05

The selected cloud Kernel now serves the exact-request receipt API. A seven-file
maintenance snapshot passes 14 selected native Linux tests and the scoped Linux
Clippy check with the documented existing-code nonminimal_bool exception. Native
Windows client tests pass 122 unit cases plus one CLI integration case (three
existing checks ignored), with strict client/Kernel all-target Clippy, format and
workspace build passing. The original schema32 database/identity, configuration,
QQ binding, memory grant and budget decision remain unchanged. Public HTTPS checks
confirm owner GET/null, denial of device/unauthenticated credentials and healthy
readiness/SSE. Publication retains the immediate previous release; final retention
leaves two releases and seven backups. No operator model/task submissions occur.

The Windows daily cloud launcher selects the new native client. Its real ConPTY
navigation reaches Work → N, displays the paired PC/project and opens the editor,
then exits without submission or changing task IDs. No interactive clipboard,
chat transcript or screenshot is retained by this production navigation check.
The selected portrait and existing conversation/connection stay in place.

The PC is explicitly paired for this repository with a seven-day finite device
registration and a read-only local Codex policy. Its native `--check` confirms the
exact executable/version, canonical workspace and cloud authority. This means
ready for explicit service start; it does not establish provider access or a
running host. At this preparation checkpoint the host had not been started.
The user subsequently started it and submitted the first read-only task. It
received a native turn/opening message, then became Unknown during command
output; it produced no final summary. The transport correction and its separate
verification are recorded in [Codex output recovery](backends/CODEX.md#command-output-burst-recovery--2026-11-05).

For this selected Windows installation, keep the following foreground process in
a separate terminal while executing work:

```powershell
& "$env:LOCALAPPDATA\Lain-Cloud\Start-Codex-Host.cmd"
```

Ctrl-C in that terminal requests host shutdown; wait for exit. In another terminal
open the existing Lain cloud launcher, use Work → N, select the project, enter a
read-only task, review and confirm. Closing the Lain UI does not stop this host.
Without the host, accepted tasks stay queued. Workspace-write remains unavailable
under this local policy even if selected in the form; widening it needs explicit
local configuration after stopping the host. Installed-service lifecycle controls,
first real-provider task and continuation/parallel-chat acceptance remain follow-up.

The stable host entry above selects the separately installed output-burst repair;
the old explicit executable command still starts the previous binary. Stop that
old process before using the new entry. The repair passes 17 native Windows host
scenarios, seven Worker integration tests and one classifier test without paid
calls. Installation does not replay the old Unknown task or establish a successful
real-provider run. See [host repair/start status](DEVICE_HOST.md#selected-command-output-repair--2026-11-05).

Private operation/test evidence remains outside Git in the session artifact
directory `lain-work-entry-2026-11-05`; no connection credential, host address or
personal transcript is added to the repository.
