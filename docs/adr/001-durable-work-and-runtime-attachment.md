# ADR 001: Durable work, detachable runtimes and operational history

Date: 2026-10-04. Status: accepted; implemented 2026-10-05 in Phase 3.

## Context

Phase 2 treats every owned worker process exit as a backend disconnect and
publishes useful completed output only live. Historical idle sessions therefore
require processes at boot and a reconnecting terminal cannot recover their work.

## Decision

WorkSession is durable logical work. Runtime attachment is a separate dimension
with Detached, Starting, Attached, Stopping and Lost observations; process IDs are
private infrastructure. Planned idle detach retains Idle and provider identity.
Unexpected active loss or unconfirmed shutdown makes unfinished work Unknown.
Known idle sessions boot detached without spawning a worker. Prompt/resume attaches
on demand and reconciles the existing provider identity before acting.

Use a configurable idle timeout, default five minutes. A timer is eligible only
with no active turn, unresolved approval or current operation and a resumable
provider identity. Codex persistence of a new, unused thread must be verified;
never replace its binding silently or submit synthetic model work to make it durable.

Store normalized meaningful events in indexed SQLite rows, separately from
session snapshots and semantic memory. State and associated history commit in one
transaction before publication. Raw fragments remain live-only. Bounded sanitized
completed messages and command output make the history useful without storing raw
provider frames, hidden reasoning or credentials. Retain these rows indefinitely
initially; explicit pruning is future work.

The terminal executable uses HTTP and authenticated SSE exclusively. Reconnect
merges paginated history, the stream's authoritative snapshot and live events by
stable EventId and server-assigned history order. It does not own backend processes
or inspect daemon storage. Terminal control sequences from worker output are
treated as text and terminal state is restored on exit.

## Consequences

Idle historical work does not imply resident processes. Startup cost depends on
uncertain active work rather than all idle history. Codex-native persistence still
determines whether reattachment succeeds; failures remain explicit. History is an
operational timeline, not worker context, a SessionSummary or Lain memory.
