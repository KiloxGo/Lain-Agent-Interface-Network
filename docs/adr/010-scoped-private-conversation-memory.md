# ADR 010: Scoped private-conversation memory and runtime channel diagnostics

Date: 2026-10-24 (Asia/Shanghai).

Status: accepted; implemented in the selected Q1 repair / Q2-A slice. Validation and
remaining live gates are recorded in [Q2_A_ACCEPTANCE.md](../Q2_A_ACCEPTANCE.md).

## Decision

Q1 exposes Kernel-owned ephemeral connection observations independently of durable
bindings and reply grants. Only verified account identity permits Online. Restart
does not restore an old Online flag. Safe closed failure categories replace opaque
or raw protocol errors; repeated identical failure warnings are suppressed.

Q2-A adds an explicit administrator-controlled memory grant for one existing
private binding. Its default is disabled; revisions, revocation and optional
expiry are persisted independently of the reply grant. No model or chat text can
create a grant. Revocation/expiry is checked again under the storage lock at each
memory effect, not merely at inference admission.

Every grant revision captures the committed incoming sequence watermark. New
autonomous selections must use a source after it and in the current actual batch;
queued pre-grant inputs and archived history do not become new eligible sources.

New autonomous memory is isolated to the binding, uses the existing Memory/FTS
tables, and retains actual input/batch/Actor/action IDs, a bounded source quotation,
time and the authorization revision. It remains model interpretation. The existing
owner-confirmation timestamp becomes optional for these new sources; legacy
records retain their exact timestamps and meanings. No historical text is extracted
or reclassified during migration.

Migration 009 appends four small Kernel-owned tables: grant revisions, effect
receipts, suppressed input IDs and inspected target IDs/revisions. No second
semantic store, observer or migration of the running personal instance is added.

The existing Actor request/tool/token/time bounds apply. Remember/correct/forget
are typed, source-bound capabilities; they cannot select another binding or scope.
Corrections require an inspected target and revision; at most one correction/forget
effect may commit per turn, including across separate calls. Forgetting removes the
semantic projection and suppresses replay from its old source; it does not erase
the independently retained conversation/audit or a backup. Effects and tombstones
commit transactionally, with inspectable idempotent receipts. Uncertain work is
never blindly replayed.

Suppression is conservative at whole incoming-message granularity. Current-source
excerpts and recalled records each have a 4,096-byte JSON budget; recall returns
at most three Active records using at most eight literal keywords/bigrams. Large
autonomous records that cannot fit later recall are rejected transactionally.
The existing four-request/four-tool Actor bound is unchanged. Dialogue, audit,
superseded versions and provider requests retain their separate lifetimes.

Recall is bounded, relevance-filtered and refreshed from active records rather
than a persistent model-context cache. Evidence remains untrusted attributed data.
Without a valid grant, existing candidate review remains enforced. The grant does
not authorize work, worker approvals, persona changes, source expansion or contact.

## Supersession and limits

This narrowly replaces ADR 004/008's universal per-item memory confirmation target
only for the explicitly enabled private binding. Existing reviewed memory and all
other mutation/approval boundaries remain unchanged. No observer, group collection,
external Worker archive, scheduler, autonomous contact, cloud/device runtime or
vector service is included. The running personal instance is not migrated or
enabled by development or by this ADR. Native/real acceptance is recorded separately.
