# ADR 032: Conversation-bound device work

Date: 2026-11-03. Status: implemented; five native isolated dispatch/permission/
result scenarios passed. Remaining W4 scope below is not declared complete.

## Decision

Keep device commands and device-local execution/session/turn IDs separate from
cloud-local Supervisor WorkSessions. Existing ConversationNotice requires a local
WorkSession; remote commands must not invent one to fit that contract.

Schema027 adds a narrow device_work_links index from an actual originating
conversation to its device command. A command can be associated only when first
created, never adopted from unrelated owner work or moved to another conversation.
Origin is either direct owner dispatch or a specific confirmed Actor/action.
Queue insertion, change publication, link and Actor partial receipt share one
transaction. The Actor action UUID supplies the stable RequestId; the model cannot
choose it. Original task/profile/workspace/sandbox/deadline remain immutable.

Conversation-bound owner APIs and typed Actor tools read bounded fresh command
projections and explicit device evidence. A separate snapshot stream reuses device
ledger notifications, allowing clients to recover current truth after disconnect
without model wakes or another operational log. These snapshots are last-known
device reports, not proof of host liveness, verified artifacts or user-goal success.
They are not semantic memory, shared social archives or proactive QQ delivery.

Remote queuing keeps the existing explicit Actor action review; Worker permissions
keep separate exact-scope owner decisions and device-side application fences.
Reading tools cannot decide permissions or start services. No model polling,
automatic Worker approval, rewriting compiler, public deployment or personal DB
migration is introduced. Native Worker terminal entry stays capability-dependent.

## Remaining scope

This decision establishes dispatch/result association, not native session
continuation. W3's unique command/session binding cannot be bypassed to call a new
session a follow-up. That path needs its own deliberate protocol extension and
tests. Actual UI, native execution and cross-chain acceptance must be evidenced
separately; documentation does not mark W4 complete.
