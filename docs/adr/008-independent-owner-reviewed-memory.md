# ADR 008: Independent, scoped and owner-reviewed memory

Date: 2026-10-19. Status: implemented; deterministic acceptance is recorded in
DEVLOG. Native Windows/Linux and real-provider Phase 7.1 acceptance remain pending.

## Decision

Extend the existing Kernel services, SQLite Store and authenticated owner API.
Use typed MemoryRecords and independent MemoryCandidates/SessionSummaries, not
conversation or worker-transcript copies. Preserve Lain's own continuity and her
worker-supervisor role with equal standing. Persona stays separately controlled.

Origin is assigned at the authenticated service boundary: direct exact owner
statements are Explicit; Actor suggestions remain Inferred after confirmation;
owner-selected summary facts remain UnverifiedSelection. Invalid kind/scope and
source/authority combinations fail. No worker completion status verifies its
claims or proves all commands passed.

Migration 007 uses structured SQLite queries, bounded deterministic sequence
pages and FTS5 trigram literal search, including CJK substring matching. No external
embedding service, vector database, new persistence owner or generic RAG subsystem.
Each query selects exactly one scope. No entire-store prompt injection or automatic
cross-project/global union. Retrieved content is untrusted evidence, not authority.

Reuse persisted Actor mutation confirmation as the candidate's sole review gate.
Memory acceptance and candidate state commit atomically; an uncertain Actor receipt
is inspected rather than automatically replayed. Direct owner operations need no
LLM. Stable request IDs deduplicate creates, revisions prevent stale mutations,
and delete tombstones prevent request retries from restoring removed memories.

Reuse the Phase 6.6 bounded normalized evidence projection for explicit factual
SessionSummary creation. Summaries stay independent of result notices and memory;
promotion requires selected text and owner review in the same workspace. Do not
start background summarization, extraction, reflection or prompt compilation.

## Consequences

The owner can inspect scope, origin, source identity and confirmation separately
from technical verification. Deleting semantic content leaves independent audit
history/summaries intact; it is not secure disk/backup erasure. Trigram search has
literal semantics, simple deterministic ordering and scoped-scan cost for short
queries. Linguistic ranking, embeddings and automatic deduplication are deferred.
Workspace selection currently requires an existing daemon-host directory; ID
inspection remains available if it moves. Windows/macOS share Rust logic, and
actual native evidence must be recorded separately. See [MEMORY.md](../MEMORY.md)
for the normative schema, lifecycle, retrieval and retention contract.
