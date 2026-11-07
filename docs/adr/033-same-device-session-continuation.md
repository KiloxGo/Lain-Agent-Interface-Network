# ADR 033: Explicit continuation of one native Worker session

Date: 2026-11-03. Status: implemented; eleven native isolated cross-chain cases passed.
Continues [ADR032](032-conversation-bound-device-work.md). Not complete W4.

## Decision

One cloud DeviceCommand still denotes one immutable task/turn, not a whole Worker
conversation. An explicit follow-up creates a new command and new execution/turn,
linked to the original cloud conversation and the same actual device-local Worker
session/provider thread. It never creates a cloud-local WorkSession.

`FollowupDeviceWork` carries a stable RequestId, current device/parent revisions,
exact text and fresh finite deadline. Device/workspace/profile/sandbox cannot be
changed through continuation: the original immutable scope is inherited. The
parent must be the current uncontinued known completed/failed mapped turn, with
no cancellation/interruption/pending permission. Running input/steering is not
advertised by the selected adapter; busy/unknown/cancelled/unmapped inputs fail
explicitly, never wait in a hidden queue or become replacement sessions.

Schema028 retains execution/turn uniqueness and replaces the blanket session
uniqueness with explicit continuation-family validation. The immutable lineage
binds root command, parent, original session, parent turn and reviewed revision.
An unrelated command cannot claim that session. One child per parent provides a
linear chain; further instructions target the returned child after it is known
idle. A rejected/unsubmitted/unknown child is not a verified completed turn and
does not silently authorize falling back to an ancestor with a new nonce.

Child command/link/lineage, parent's continued_by revision/change and any confirmed
Actor partial receipt commit together. RequestId reuse must preserve exact origin,
parent and payload, even after later children. An Actor uses ActionId as RequestId;
direct owner dispatch uses an explicit stable request. No retry repeats Worker RPC.

## Native effects and recovery

Before writing a child intent, the native host must find its original protected
parent intent and submission fence. Device, manifest/policy, workspace/profile/
sandbox, session, parent turn and family root must match. The child reuses only
that local session and derives new execution/turn IDs from its command.

Before first acceptance/launch, the operational store must retain the provider
binding and latest known completed/failed parent turn, without unresolved local
permissions. Missing state is rejection/Unknown, not preparation of a replacement
session. Existing metadata preparation may validate the original scope only after
that proof; the host resumes/reuses the existing Supervisor session.

Each child retains its own launch/submission/permission/outbox fences. Restart
reconciles only the original mapped turn. A claimed unmapped submit stays Unknown;
cloud receipt recovery never repeats native work. Original requested text is not
rewritten, and authorization is freshly checked before each effect.

## Interface and acceptance

Owner conversation API: POST device-work/{device}/{parent}/followups. Actor
device_send retains explicit full-scope review and exact Changed/Unchanged text
inspection. Terminal Worker drafts stay separate from Lain drafts; a continuation
must name the actual target and permit exact receipt inspection/retry on uncertainty.
Work should show a family as one remote Worker rather than inventing extra sessions.

Required deterministic/native evidence: same actual session and provider thread,
distinct turns, explicit idle capability, original payload/RequestId, stale/foreign/
unrelated-scope denial, rollback, duplicate recovery, frontend/host restart and
unknown submit without replay. TLS, paid providers, other native platforms and
native GUI font fallback remain separate evidence; no deployment is authorized.
