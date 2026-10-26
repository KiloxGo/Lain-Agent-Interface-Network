# QQ private conversation channel

## D2 ordinary proactive contact — 2026-10-26

Optional sharing uses a separate default-off grant and explicit attention, never
the reply grant alone. It retains the same authenticated account/peer, literal
text frame, echo/receipt handling and no-retry transport. The service and writer
recheck source/input/grant/attention immediately before sending. Configuration,
CLI and recovery: [ADR 015](adr/015-bounded-proactive-sharing.md) and
[D2 acceptance](SHARING_ACCEPTANCE.md). No real proactive QQ contact is accepted.

Implemented; final-revision deterministic gates passed on native Windows (2026-10-24).
See [validation evidence and limits](Q1_IMPLEMENTATION.md). Basic real private QQ/model
conversation is user-confirmed; complete live reliability and cloud deployment
remain unverified. This guide describes the selected Q1 slice;
it does not grant permission to perform live tests.

## Boundaries

One configured NapCat **Lain account**, one explicitly bound user, friend-private
plain text only. No user-account observer, groups, temporary group chats, media
downloads, quoted/forwarded messages, Telegram, proactive contact or Bridge.
Mixed-media inputs are ignored in full rather than pretending extracted text is
the complete message. Configure array message reporting; CQ strings are not parsed.
Output is a single explicit text segment, so model-generated CQ-like text cannot
become a platform action.

`channels::domain` and `ChannelService` own generic identity, queueing and delivery.
`channels::napcat` alone knows OneBot actions, QQ fields, account login checks,
authentication and transport. `ConversationChannel` has connection notification
and bounded text sending. A fixture adapter implements the same contract. Turning
QQ off leaves the existing terminal Actor and Worker interfaces available.

The Utakera observation Bridge is separate. Its draft at `8e30fd2` is an architectural
reference, not a dependency or approved shared protocol. Neither source messages nor
archival rights imply command authority; this channel authenticates a conversation
with the configured person. Databases, persona and semantic memory remain Lain-owned.

## Configuration and startup

Use an isolated instance and database for initial live acceptance. Do not point
an experimental binary at existing personal state without a separately reviewed
backup/migration decision. The development schema is 013 (008 channel, 009 private memory,
010 source lookup, 011 bounded private chat, 012 independent text acquisition,
013 explicitly enabled reading processing/experiences).
The installed C1.1 personal release remains on schema 011; a development build
does not upgrade it. Acquisition-only grants do not enable model processing; the
optional processing scope is separately specified and never grants proactive QQ contact.
an older binary cannot open
the newer schema; rollback needs the consistent pre-upgrade backup.

D1b private `experience.search`/`experience.inspect` can read valid source-linked
reading summaries only under that optional scope's selected binding. Reply,
MemoryGrant and historical SourceGrant do not enable it. Correction/withdrawal,
expiry and source changes are checked before committing/sending affected replies.
Actual prior Confirmed/Unknown outcomes are retained. See
[D1b acceptance and operations](EXPERIENCE_ACCEPTANCE.md).

Build `laind` and `lain` with the native Rust toolchain. Ordinary builds do not call
models or log into QQ. Configure the Actor separately as described in
[LAIN_ACTOR.md](LAIN_ACTOR.md); QQ never reuses Codex credentials.

Copy [the example](../config/napcat.example.json) to a private location outside the
repository, replace the two example account numbers and choose a stable binding
name. The endpoint must be the forward WebSocket's combined `/` interface. A bare
`ws://` endpoint is allowed only for a literal loopback IP; use validated `wss://`
for a remote connection. URL credentials, query parameters and fragments are rejected.

Set `LAIN_NAPCAT_CONFIG` to that JSON path. `secret://env/napcat` resolves the
`LAIN_SECRET_NAPCAT` environment variable; supply it from your private credential
workflow. It must differ from the Kernel API token; startup rejects identical
values. Never paste it into JSON, URLs, screenshots, Git, shell history or logs.
`allow_reply: true` is an explicit initial grant for bound-user replies, including
Actor processing of accepted messages. The example is deliberately disabled.

Configure NapCat through its own UI:

1. Use the Lain account, not the user's observation account.
2. Enable a forward WebSocket server with `messagePostFormat: array` and a nonempty
   token matching the secret above. Disable raw/debug logging and self reports.
3. Restrict the listener to loopback for the co-located topology below.
4. Establish the friend relationship manually; Lain cannot accept friend requests.

Start the existing `lain up` local flow with these explicit environment settings,
or run `laind` with `LAIN_DATABASE_PATH`, `LAIN_LISTEN` and `LAIN_API_TOKEN` in a
dedicated service environment. `LAIN_LISTEN` remains loopback-only. Do not run both
against one database. Launching with configured real credentials is live operation,
not an offline test. Without `LAIN_NAPCAT_CONFIG`, no QQ connection task is created.

The daemon verifies `get_login_info` before ingesting messages. Reconnection repeats
the check. A nickname is never identity evidence. Reusing a binding name with a
different account, peer or adapter is rejected. Keep the same name and database
across restarts to continue the same conversation.

Saving a file is not proof a running service loaded it. Lain reads its selected
Actor/NapCat files and credential environment at process startup; edit then perform
an intentional stopped-instance restart to apply them. NapCat's own saved network
settings may likewise need its explicit Apply/reload/restart. Inspect the active
listener and Lain connection status rather than treating a saved toggle as live.

For the user's existing 2026-10-24 Windows instance the external directory is
`F:\Lain-Q1-Acceptance`, and its already prepared launcher is
`& 'F:\Lain-Q1-Acceptance\start-qq-test.ps1'`. This script is outside Git and is
not installed by the repository. `%LOCALAPPDATA%\Lain-Q1-Acceptance` was an earlier
example and is not this instance's launcher location. Desktop app private/virtualized
AppData and a normal PowerShell can resolve differently. Select the same explicit
external directory in both windows; do not recreate personal state because a
launcher path is absent. Development has not replaced that running binary.

## Inspect and revoke

Use the existing authenticated Kernel HTTP API. All requests below require the
administrator bearer token; the NapCat token cannot use this API.

| Method and path | Result |
| --- | --- |
| GET `/v1/channels` | Binding IDs, stable conversations and current reply grants |
| GET `/v1/channels/status` | Bindings with ephemeral connection phase, safe failure category and change time |
| GET `/v1/channels/{id}` | Binding, connection plus latest 100 incoming records, batches and deliveries per category |
| POST `/v1/channels/{id}/reply-grant` | `{ "enabled": false, "expected_revision": 1 }` revokes the current grant |

Read the current revision before changing permission. Stale revisions conflict.
Re-enabling uses `enabled: true` and the current revision explicitly. Configuring
`allow_reply: true` again never overrides a persisted revocation; false can revoke
on restart. Re-granting does not replay older queued work. The inspection window
is bounded, not a full archive export or source-coverage guarantee.

Use `lain channel list` and `lain channel show <id>` for the same authenticated
inspection. Connection phases are `not_started`, `connecting`, `authenticating`,
`online`, `reconnecting`, `stopped`. WebSocket handshake alone is Authenticating;
only matching account identity permits Online. A binding with no adapter in this
process is NotStarted, including after restart, even if it was previously Online.
Shutdown or task exit makes its observation Stopped. `changed_at_ms` is a local
Unix-millisecond observation timestamp (0 before the first observation), not uptime
or platform last-seen time. The last failure remains visible while reconnecting
until successful identity validation clears it. Identical repeated failures do not
emit repeated warnings; a changed failure or fresh online-to-offline transition does.

| Failure | Check |
| --- | --- |
| `connection_unavailable` | Forward-WS address/listener, reachability and handshake availability |
| `authentication_failed` | OneBot token and endpoint auth (HTTP 401/403), distinct from WebUI login |
| `account_mismatch` | Actual logged-in Lain account versus configured `self_id` |
| `identity_timeout` | Server answers `get_login_info` with the matching echo within ten seconds |
| `protocol_error` | WS/OneBot frame/identity format; use installed-version settings without raw logs |
| `disconnected` | Connection closed or interrupted; inspect receipts before retrying anything |

These are safe categories, not raw upstream errors. No token, auth header, original
frame or conversation text enters this observation/log. Online describes the
validated transport, not successful inference, a reply grant or gap-free coverage.
The separate default-off private memory grant and its routes are described in
[CLI.md](CLI.md), [MEMORY.md](MEMORY.md) and [Q2_A_ACCEPTANCE.md](Q2_A_ACCEPTANCE.md).

Revocation invalidates queued inputs, prepared/running batches and unsent replies.
The send claim is the durable dispatch boundary; an already initiated send cannot
be undone by later revocation. The adapter checks the current grant again before
writing. In-flight inference may finish, but its response cannot acquire a new
grant by itself. None of these permissions approve Worker actions or memory writes.

When the Actor proposes a mutation, QQ receives a factual notice with conversation
and turn IDs. Review the exact action using the existing terminal/API confirmation
flow. Typing “同意” in QQ is ordinary queued text, not an approval. The conversation
waits while the proposal is pending; other conversations and Workers continue.
Resolve or explicitly cancel the proposal through the existing Actor interface.
`lain chat --conversation <conversation-id>` opens the same conversation in the
terminal; see [CLI.md](CLI.md) for action inspection/confirmation commands.
This slice sends the pending notice once; subsequent approval/execution details
remain inspectable in the terminal, without unsolicited QQ result notifications.

## Queueing and recovery

Initial engineering bounds, subject to real experience review:

- 8 KiB input text, at most 128 queued inputs per binding.
- A trailing 750 ms quiet window, capped at three seconds from the oldest eligible
  input; at most 16 messages and 8 KiB combined input per Actor turn. An expired
  window does not reopen. During inference, eligible supplements may join the
  same turn through at most two reconsiderations, retaining original IDs/text.
  Excess supplements stay queued for the next turn; cumulative budgets do not reset.
- A private Actor can explicitly wait once for 1–5 seconds or finish a short
  social exchange without sending. Kernel guards refuse silence for protected
  requests and non-chat tool activity. New input wakes a wait or a fresh turn.
  See [ADR 012](adr/012-bounded-private-conversation.md) and [C1 acceptance](C1_ACCEPTANCE.md).
- Ten-minute lifetime from the oldest input for processing/reply. Restart does not
  refresh it. Expired backlog is not sent as a catch-up flood.
- One outgoing request at a time, at least one second between send starts;
  three-second socket write bounds, a 15-second receipt deadline and a 20-second outer
  bound. Connection/identity checks have ten-second limits; reconnect backs off
  from one to thirty seconds, with ping/liveness checks.

These limits do not replace existing Actor tool/token/time bounds. There is no
idle inference loop, automatic model retry or self-generated conversation.
Accepted text remains in operational history; it is not automatically semantic
memory. Input IDs deduplicate by durable binding and platform message ID. A reused
ID with changed content conflicts instead of changing the recorded instruction.
Queue/expiry bounds limit pending work; they do not delete historical records.
There is no automatic archive-retention/deletion feature in this slice. Monitor
private database size and use a fresh test instance for initial acceptance.

| Delivery state | Meaning |
| --- | --- |
| `ready` | Generated and persisted, no dispatch claim yet |
| `sending` | Claimed before transport submission; completion not yet recorded |
| `confirmed` | Matching successful platform response with a message ID; not human read confirmation |
| `not_sent` | Transport/policy prevented submission; not automatically retried |
| `rejected` | Explicit matched platform failure; not automatically retried |
| `unknown` | Outcome cannot be proved, including interrupted claimed sends |
| `revoked` / `expired` | Unsent work invalidated by grant or lifetime |
| `superseded` | Unsent ordinary draft invalidated by newer input or changed memory/historical source evidence; no send claim |

The OneBot `echo` correlates responses; it is **not** a server idempotency key.
If a send may have reached QQ, reconnect never sends that intent again. A crash
after platform success but before database commit becomes Unknown. Inspect QQ
manually and make a new explicit request if needed; there is no retry button that
silently replays the old side effect.

Already accepted inputs and batch IDs survive restart. Actor request deduplication
recovers the exact committed turn after commit-before-link interruption. Unfinished
Actor work becomes Unknown under the existing policy and is not regenerated.
Reconnection restores live reception only: the implementation does not backfill
messages missed while disconnected, nor claim complete history.

## Deployment design

First topology: one private server/service account hosting Kernel and NapCat.
NapCat's OneBot listener and the Kernel API bind to loopback. The QQ connection
is outbound from the adapter; the Kernel adds no public listener. Protect NapCat's
WebUI separately: do not expose its default binding publicly. Remote administration
uses an authenticated SSH tunnel, not a public bearer-only API.

For a separately hosted NapCat, use an explicitly authenticated private tunnel to
a local endpoint, or a TLS-protected reverse proxy restricted to the channel
endpoint. Keep certificate verification enabled. The adapter never follows an
HTTP redirect to forward a credential. Do not expose Worker control endpoints as
part of this arrangement. Real host provisioning is outside this implementation.

Use a dedicated private state directory: Unix 0700 directories/0600 files, or
owner-only Windows ACLs. `lain setup --local` supplies the existing private local
instance layout (including current-user ownership for newly created objects when
run elevated). Existing non-private directories are rejected rather than adopted.
Manual service deployment must establish equivalent permissions
before opening the database, including its WAL/SHM/lock files. Keep credentials in
the service credential environment, not unit files committed to the repository.
Do not copy desktop Worker configuration or a personal database to the cloud as
an implicit migration. Service installation and Linux acceptance remain separate.

## Troubleshooting and live acceptance

### Windows hands-on acceptance

Run Kernel and NapCat on the same Windows machine for this first check. This is
manual live operation and consumes the configured Actor provider's quota. Start
with a few ordinary messages and inspect failures before sending more. Record
NapCat/QQ versions and the selected provider/model; never record credentials.
The following uses a fresh private instance and port 18787, independently of the
normal instance. Keep its directory, binding name and account IDs unchanged when
testing restart continuity.

Use a QQ build supported by the selected NapCat release. For an old desktop QQ,
the official [Windows OneKey package](https://napneko.github.io/guide/boot/Shell)
can prepare a separate runtime: extract it into a fresh directory, run
`NapCatInstaller.exe`, then start `napcat.bat` inside its generated
`NapCat.XXXX.Shell` directory. This is distinct from `launcher.bat` in the normal
Shell archive. Merely replacing NapCat's files does not update the installed QQ.

1. In NapCat, log in as Lain and manually add your own QQ as a friend. In WebUI's
   network configuration create and enable a **WebSocket server / forward WS**:
   host `127.0.0.1`, port `3001`, message format `array`, a nonempty OneBot token,
   self-message reporting off and debug/raw reporting off. Lain connects to
   `ws://127.0.0.1:3001/`; this is not the WebUI port or a reverse-WS client.
   Protect the WebUI separately. Its login token and the OneBot token have different
   roles. Configuration fields were checked against the
   [official guide](https://napneko.github.io/config/basic) on 2026-10-24; the
   installed version still needs live verification.

2. Open a fresh PowerShell in the repository after the native workspace build.
   Use the built executable explicitly. The first command clears manual launcher
   overrides in this PowerShell process only, so the test uses its own saved
   endpoint, credentials and Actor selection.

   ```powershell
   Remove-Item Env:LAIN_URL, Env:LAIN_API_TOKEN, Env:LAIN_LISTEN, Env:LAIN_DATABASE_PATH,
       Env:LAIN_ACTOR_CONFIG, Env:LAIN_PERSONA_PATH, Env:LAIN_DAEMON_PATH -ErrorAction SilentlyContinue
   $lain = (Resolve-Path .\target\debug\lain.exe).Path
   $env:LAIN_LOCAL_DIR = 'F:\Lain-Q1-Acceptance'
   if (Test-Path -LiteralPath $env:LAIN_LOCAL_DIR) {
       throw 'Choose a new test directory for first setup; reuse the existing one only for an intentional restart.'
   }
   & $lain setup --local
   if ($LASTEXITCODE -ne 0) { throw 'Local setup failed' }

   $settingsFile = Join-Path $env:LAIN_LOCAL_DIR 'client.json'
   $settings = Get-Content -Raw -LiteralPath $settingsFile | ConvertFrom-Json
   $settings.url = 'http://127.0.0.1:18787'
   [IO.File]::WriteAllText($settingsFile, ($settings | ConvertTo-Json),
       [Text.UTF8Encoding]::new($false))
   Copy-Item -LiteralPath .\config\actor.example.json -Destination (Join-Path $env:LAIN_LOCAL_DIR 'actor.json')
   Copy-Item -LiteralPath .\config\napcat.example.json -Destination (Join-Path $env:LAIN_LOCAL_DIR 'napcat.json')
   ```

   `setup` creates the private owner-only directory and token. Edit the two copied
   JSON files in your editor and save UTF-8 without BOM. In `actor.json`, select
   your actual Responses-compatible provider base URL and model, with
   `secret_ref: "secret://env/actor"`. The base URL excludes `/responses`; retain
   the existing request/output/token bounds. The example's model and endpoint are
   examples, not proof of current provider compatibility.

   In `napcat.json`, set `self_id` to **Lain's QQ**, `user_id` to **your QQ**, keep
   both as JSON strings, use the endpoint above and set `allow_reply` to `true`.
   Keep `secret_ref: "secret://env/napcat"`. Only the outgoing OneBot wire field
   becomes a number; configuration IDs remain strings. Never put a token in JSON.

3. In that same PowerShell, select the Actor file and enter the two credentials
   through hidden prompts. They remain process environment values and are not
   saved to a credential file by these commands.

   ```powershell
   & $lain config actor set --file (Join-Path $env:LAIN_LOCAL_DIR 'actor.json')
   if ($LASTEXITCODE -ne 0) { throw 'Actor configuration failed' }
   $env:LAIN_NAPCAT_CONFIG = Join-Path $env:LAIN_LOCAL_DIR 'napcat.json'
   $actorCredential = Read-Host 'Actor API key' -AsSecureString
   $env:LAIN_SECRET_ACTOR = [Net.NetworkCredential]::new('', $actorCredential).Password
   $napcatCredential = Read-Host 'NapCat OneBot token' -AsSecureString
   $env:LAIN_SECRET_NAPCAT = [Net.NetworkCredential]::new('', $napcatCredential).Password
   & $lain up
   ```

   Keep this window open. Startup connects to the real NapCat, but ordinary
   startup/doctor do not invoke the model. An accepted bound-user message does.
   Stop with Ctrl-C. To restart in this same window, run `& $lain up` again;
   do not rerun fresh setup or change the binding. In a new window reselect the
   same directory/configuration and re-enter the credentials first.

4. In a second PowerShell in the repository, inspect the instance without a model
   call. Do not print the token or the headers variable.

   ```powershell
   Remove-Item Env:LAIN_URL, Env:LAIN_API_TOKEN, Env:LAIN_ACTOR_CONFIG,
       Env:LAIN_PERSONA_PATH -ErrorAction SilentlyContinue
   $lain = (Resolve-Path .\target\debug\lain.exe).Path
   $env:LAIN_LOCAL_DIR = 'F:\Lain-Q1-Acceptance'
   & $lain doctor
   $base = 'http://127.0.0.1:18787'
   $headers = @{ Authorization = 'Bearer ' + ([IO.File]::ReadAllText(
       (Join-Path $env:LAIN_LOCAL_DIR 'kernel-token')).Trim()) }
   $bindings = @(Invoke-RestMethod "$base/v1/channels" -Headers $headers)
   $binding = $bindings | Where-Object { $_.spec.name -eq 'qq-personal' }
   if (@($binding).Count -ne 1) { throw 'Expected one qq-personal binding; inspect configuration' }
   $channelId = $binding.id
   $binding | Select-Object id, conversation_id, reply_enabled, grant_revision
   $snapshot = Invoke-RestMethod "$base/v1/channels/$channelId" -Headers $headers
   $snapshot.deliveries | Select-Object id, state, receipt
   ```

   If you chose another binding name, change the filter. A binding/ready Kernel
   alone does not prove NapCat connected; a real accepted input and matched send
   receipt establish the live path. `doctor` in this second window may report the
   Actor credential unavailable locally because it is held only in the first
   window; inspect the daemon's Actor diagnostics as well. Refresh `$snapshot`
   after each test to inspect `incoming`, `batches` and `deliveries`.

5. Run these small checks in order:

   | Action | Expected evidence |
   | --- | --- |
   | Send Lain a plain Chinese private message | One accepted input/batch and a visible reply; delivery `confirmed` with a receipt |
   | Send two or three short messages quickly | A trailing 750 ms quiet window, capped at 3 seconds, groups inputs; in-flight supplements may join the same bounded turn, with original IDs/text retained |
   | Mention a harmless detail, wait for completion, Ctrl-C/restart Kernel, ask about it | Same binding/conversation ID and recent conversational context; this is not proof of semantic memory |
   | After a completed reply, disable/re-enable NapCat's WS endpoint, then send a new message once reconnected | New message can complete, old confirmed replies are not resent; messages during disconnection are not guaranteed to be recovered |
   | Revoke, send a message, explicitly re-grant, send a new message | No new reply under revoked permission; re-grant does not replay the revoked backlog |

   Revoke/re-grant from the second window using the **current** revision each time:

   ```powershell
   $snapshot = Invoke-RestMethod "$base/v1/channels/$channelId" -Headers $headers
   $body = @{ enabled = $false; expected_revision = $snapshot.binding.grant_revision } | ConvertTo-Json
   Invoke-RestMethod "$base/v1/channels/$channelId/reply-grant" -Method Post `
       -Headers $headers -ContentType 'application/json' -Body $body | Out-Null
   # Test revoked behavior, then explicitly grant again with its new revision.
   $snapshot = Invoke-RestMethod "$base/v1/channels/$channelId" -Headers $headers
   $body = @{ enabled = $true; expected_revision = $snapshot.binding.grant_revision } | ConvertTo-Json
   Invoke-RestMethod "$base/v1/channels/$channelId/reply-grant" -Method Post `
       -Headers $headers -ContentType 'application/json' -Body $body | Out-Null
   ```

   If a turn waits for an action decision, use `& $lain chat --conversation
   $binding.conversation_id` in the second window to inspect and explicitly deny
   or approve the exact proposal. QQ “同意” cannot approve it. If sending is
   `unknown`, inspect the real chat before any new request; never retry the old
   intent automatically. Record the step, timestamp, safe IDs/state, visible reply
   and latency. A `confirmed` delivery means platform acknowledgement, not human
   read confirmation. Stop the test daemon with Ctrl-C when finished.

- No replies: inspect the grant, configured Actor, private-friend/text-array format,
  account binding and pending confirmation. Check safe channel warnings by ID.
- Connection rejected: verify endpoint type, token and authenticated login account;
  do not log raw handshake headers or frames to diagnose it.
- Input not accepted: check identity/format, 8 KiB limit, sensitive text and backlog.
  Protocol reception does not prove the Kernel accepted or processed the message.
- Unknown send: inspect the recorded intent and the actual chat; never infer failure
  solely from disconnection. Duplicate source events cannot trigger a second reply.
- Changed account: create an explicitly reviewed new binding; do not edit the old
  database mapping or silently merge conversation history.

Separate real acceptance requires: installed NapCat/QQ versions, the two bound
account IDs and friendship, private endpoint/auth setup, a fresh instance location,
and an explicitly authorized Actor provider/model/request budget. Record all of
these without recording secret values. Test normal Chinese chat, split messages,
restart continuity, network reconnect, revoke/re-grant and observable receipts.
Assess persona and timing using real responses; deterministic fixtures cannot
establish naturalness. Do not deliberately create uncertain real sends repeatedly
just to reproduce an already covered fake-server failure.

Basic ordinary private conversation is manually confirmed; the remaining Q1 live
reliability/experience checks remain pending. Phase 6.6/7.1 real-provider gaps, including
failed memory-retrieval evidence, remain unchanged. No native Linux/macOS result
is implied by a Windows build.

## Protocol references

Reviewed 2026-10-23 against the documented NapCat V4 / OneBot 11 interface, not a
running NapCat release. Exact installed-version compatibility is a live gate.

- [NapCat network configuration](https://napneko.github.io/config/basic)
- [NapCat event fields](https://napneko.github.io/onebot/event)
- [NapCat API support](https://napneko.github.io/develop/api)
- [OneBot forward WebSocket contract](https://github.com/botuniverse/onebot-11/blob/master/communication/ws.md)
- [OneBot bearer authentication](https://github.com/botuniverse/onebot-11/blob/master/communication/authorization.md)
- [OneBot public API](https://github.com/botuniverse/onebot-11/blob/master/api/public.md)

## Bounded private-history lookup (Q2-B1)

Source reads require a separate default-off administrative grant selecting a fixed
received-UTC window (at most 31 days, ending in the past). Reply and memory grants
do not enable it. Search/inspect operate on existing records only, exclude Q2-A
suppressed sources and report partial coverage. Unknown sending outcomes remain
uncertain. The actual personal instance is not upgraded or enabled by development.
See [CLI source commands](CLI.md#private-history-source-lookup-q2-b1),
[the source contract](MEMORY.md#implemented-q2-b1-private-source-lookup),
and [Q2-B1 acceptance and manual steps](Q2_B1_ACCEPTANCE.md).
