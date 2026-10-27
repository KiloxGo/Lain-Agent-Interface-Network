# ADR 020: Explicit, bounded visible Worker archive reads

Date: 2026-10-27 (Asia/Shanghai). Status: implemented W1b contract, acceptance
recorded separately. Extends the W1a source store; does not enable autonomous
collection, control an external session or grant semantic-memory/model disclosure.

## Selection and authority

An authenticated administrator explicitly submits one `ReadWorkerArchive` command
for one already registered CodexSession source, with the current active archive
grant revision, an immutable RequestId, a limit of 1–8 source items and optionally
a preceding Lain read-receipt ID. This command is the authorization for **that
read operation**. Existing import/disclosure grants alone never start a collector.
Registration, old owner imports or an LLM tool cannot trigger a backend read.
There is no schedule, polling collector, discovery scan or new Actor tool.

Only `installation=codex/local` is currently implemented. It denotes the daemon's
selected local native Codex installation/CODEX_HOME, not a paired/remote device or
authenticated cloud identity. The actual returned thread ID and canonical local
workspace must match the registered source before and after item retrieval.
A missing/mismatched workspace, foreign installation or unsupported history mode
fails explicitly. No alternate thread/workspace is silently selected.

The read adapter is independently verified against **native Codex 0.160.1** and
paginated history. The existing **managed Worker control** gate remains 0.160.0;
read compatibility does not imply start/resume/cancel/approval compatibility.
Backend diagnostics expose `archive_read_version` separately. Unknown read versions
fail closed. No global Codex configuration is rewritten or model key injected.

## Protocol and normalized content

Reuse owned App Server JSONL/process framing, environment isolation, timeouts,
stderr draining and process-tree cleanup. The reader calls only initialize,
initialized, thread/read (metadata), thread/turns/list (one turn, no item bodies)
and thread/items/list (the selected terminal turn). It never starts, resumes,
forks, subscribes to, steers, interrupts or approves an external thread/turn.
No inference is initiated by a read. A local version probe precedes each read;
RPC work has a 35-second outer deadline and owned shutdown is awaited.

Completed/failed/interrupted turns are read as **reported source state**, not
independent success verification. An in-progress turn returns a waiting receipt
with the same read position; it is revisited only by another explicit command.
No active item's final state is skipped by blindly advancing past it.

Retain visible user text, assistant text, terminal command-result reports and
successful file-change **path reports**. Non-text user attachments, oversize/
empty/invalid-control text and unsupported kinds are omitted and counted. Never
inspect/archive reasoning/raw response content, hook prompts, tool arguments,
file diffs or unknown item bodies. A bounded slice is not complete transcript,
image interpretation or evidence that Lain participated in the work.

Native source item identity is scoped to its source and hashed turn/item pair;
the source position preserves the native pair when within the bounded metadata
limit. Source-reported item time can be absent; Kernel receipt time is separate.
Source data and injection attempts remain data. Every normalized batch passes
existing credential/text/kind/size filters before publication. A too-large aggregate
fails atomically so the caller can request a smaller page. No silent truncation,
scope bypass or permission inferred from transcript content.

Opaque native cursors stay inside adapter/persistence. Clients continue using a
Lain RequestId. Its committed cursor is tied to source, grant epoch and reader
version; callers cannot submit a raw cursor for another source. Each item page
revalidates the selected terminal turn. Source changes/conflicts surface rather
than overwriting source history, reviving a forgotten item or renewing retention.

## Durable lifecycle and import

Migration **018** appends one read-receipt table/index, creates no old reads and
preserves prior grants/records/Worker state. RequestId binds immutable source and
request fields; exact retries return the existing receipt without another read.
There is one unfinished read in storage and one owned read transport slot.
Prepared/Reading transitions commit before external work. Cancel/revoke/shutdown
invalidate pending publication; cancellation closes and awaits the owned reader.
Boot recovers unfinished receipts to Unknown and never replays them automatically.

Normalized record insertion/dedup plus Completed receipt/cursor share one SQLite
transaction. Storage failure cannot publish half a page or advance its cursor.
Records use `adapter_observation` origin, independent of prior `owner_import` and
subsequent `owner_correction`. Read receipts contain IDs, counts, time, reader
version, coverage and a safe typed failure/code; no transcript copy. The archive
remains subject to existing source grant/retention/correction/forget projection.

Reads do not create WorkSessions, ActorTurns, memory, experiences, Worker authority,
model reservations or chat messages. Authenticated inspection remains possible
after source revoke without restoring its body. Receipt storage is capped at
10000 operations; no background resource growth or hidden retry is introduced.

## API and operation

- `POST /v1/archives/sources/{id}/read`: explicit request; returns durable receipt.
- `GET /v1/archives/reads/{request_id}`: inspect state/counts/next/failure.
- `POST /v1/archives/reads/{request_id}/cancel`: cancel; terminal receipts stay true.
- CLI: `lain archive read <source> --request-id <uuid> --revision <n>
  [--after <preceding-request-id>] [--limit 1..8]`, `read-job`, `cancel-read`.

Before first read, register the exact source key and explicitly enable finite
archive retention/capacity. This is one operation per command, not a promise of
continuous synchronization. Source-derived Actor continuity, invalidation across
knowledge/semantic stores, actual authorized personal Worker collection and paired
devices remain separate work in the combined goal. No personal-instance upgrade
or broader paid/Worker permission follows from this document.

References checked 2026-10-27:
[official App Server](https://learn.chatgpt.com/docs/app-server), native 0.160.1
help/generated schemas, and isolated native read probes. The native history test
uses an isolated CODEX_HOME and local synthetic Responses server: one fake request,
zero paid requests, no personal thread access. It proves actual persisted native
user/assistant item reads after process restart, without read-side resume/inference.
