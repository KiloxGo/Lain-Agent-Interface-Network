# ADR 035: Bounded startup and live invalidation

Date: 2026-11-04. Status: selected design, implementation in progress.

2026-11-05: schema032 implements the persistent instance/revision/deadline index,
guarded change signalling, bounded bootstrap and invalidation route. Terminal
startup/manual refresh consume bootstrap and rebuild checked history, including
same-revision withholding and live-versus-history race fencing. The runtime
consumer now fences/cancels old reads, enforces a monotonic local lease, preserves
in-flight effect receipts and blocks automatic retry after authentication failure.
Private bounded metadata caching stores no source bodies or credentials. Native
Windows and Linux checks pass; actual HTTPS Worker acceptance remains separate
evidence.
The concrete contract and limits are in [CLIENT_SYNC](../CLIENT_SYNC.md).

## Problem

Independent history/device reads are not a consistent startup contract. A change
between snapshot and subscription can be missed, source expiry need not update
an Actor revision, and appending fresh history does not remove old terminal text.
The schema031 publication checks alone do not invalidate a client's old view.

## Decision

The Kernel owns a persistent instance UUID and transactionally increasing sync
revision. Clients negotiate the implemented protocol and receive a bounded owner
snapshot for an explicit conversation, a resource/capability manifest, project
and device metadata, mapping versions, current permission/work pointers and
notification recovery cursors. No semantic-memory database, raw archive, Worker
transcript, provider credential or internal prompt is synchronized wholesale.
Only actual implemented capabilities are advertised. Persona/policy/configuration
revisions identify validated non-secret startup resources; they do not transfer
the internal prompt or authorize rewriting tasks.

Coalesce persisted changes into content-free invalidation, rather than adding
another general event archive. SQLite triggers advance the revision in the same
transaction as relevant changes. A guard around the Kernel's existing storage
lock signals committed changes on release. Rollbacks do not advance revision;
reads do not signal themselves. This reuses the single writer and native watch
primitives, without a new broker, polling loop or inference.

The owner subscribes with its snapshot cursor and lease deadline. Subscribe
before comparing committed state; a changed, future, foreign or expired cursor
requires a new snapshot. Notifications contain no source content. Source/grant
deadlines are indexed when their persisted snapshot changes. The snapshot lease
ends at the earliest pending deadline or a finite maximum. Timed expiry forces
invalidation even without a write; a same Actor revision is not proof that its
body is still disclosable. Queue wakeups and re-read committed state at transport
consumption. Retain only a finite cursor window; older positions require a
snapshot and never imply replaying an effect.

Clients distinguish request generation from server revision. Invalidation,
authentication failure, lost subscription, context/resource change and lease
expiry remove current content and permission previews before resynchronization.
A late response from the earlier generation cannot revive it. A fresh snapshot
rebuilds the rendered conversation from checked projections; an appended delta
cannot stand in for deleting a revoked body. Keep drafts, original uncertain
request IDs and exact decision receipts, but never resend or answer permissions
merely because synchronization succeeded. Work keeps its reviewed configuration.

Private local caching is disposable, bounded, expires and is atomically replaced.
Only resource/connection metadata needs disk caching in this delivery; semantic
and private transcript bodies do not. Corrupt or stale metadata is discarded and
re-fetched, not merged into cloud authority. Native owner-only permissions apply.
An offline view is explicitly stale and cannot authorize actions.

## Boundaries and acceptance

Owner bootstrap routes do not accept device credentials. Device-specific control
retains the existing own-device protocol and exact permission checks. Source
origin, semantic visibility and Worker execution authority remain distinct.
Reading a snapshot performs no inference, task submission or approval.

Deterministic acceptance must cover startup/subscription races, rollback and
restart identity, deadline-only invalidation, stale/foreign/future cursors,
queued transport reads, out-of-order/duplicate/late responses, same-revision
withholding, draft preservation, private cache permissions/corruption and no
uncertain mutation replay. Native Windows and Linux and real HTTPS two-machine
Worker acceptance remain separate evidence. This document is not a completion
or deployment claim.
