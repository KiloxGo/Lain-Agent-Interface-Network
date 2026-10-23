# QQ private conversation channel

Implemented; final revision verification deferred by the user (2026-10-23).
See [validation evidence and limits](Q1_IMPLEMENTATION.md). Real QQ, real model and
cloud deployment remain unverified. This guide describes the selected Q1 slice;
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
backup/migration decision. Schema 008 is additive, but an older binary cannot open
the newer schema; rollback needs the consistent pre-upgrade backup.

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

## Inspect and revoke

Use the existing authenticated Kernel HTTP API. All requests below require the
administrator bearer token; the NapCat token cannot use this API.

| Method and path | Result |
| --- | --- |
| GET `/v1/channels` | Binding IDs, stable conversations and current reply grants |
| GET `/v1/channels/{id}` | Binding plus latest 100 incoming records, batches and deliveries per category |
| POST `/v1/channels/{id}/reply-grant` | `{ "enabled": false, "expected_revision": 1 }` revokes the current grant |

Read the current revision before changing permission. Stale revisions conflict.
Re-enabling uses `enabled: true` and the current revision explicitly. Configuring
`allow_reply: true` again never overrides a persisted revocation; false can revoke
on restart. Re-granting does not replay older queued work. The inspection window
is bounded, not a full archive export or source-coverage guarantee.

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
- A fixed 750 ms collection window from the oldest eligible input; at most 16
  messages and 8 KiB combined input per Actor turn. Later inputs cannot extend the
  window forever. Inputs during inference are persisted for the next batch.
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

Q1 remains pending real acceptance. Phase 6.6/7.1 real-provider gaps, including
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
