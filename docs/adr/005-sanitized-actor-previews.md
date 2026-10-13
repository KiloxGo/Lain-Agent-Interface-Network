# ADR 005: Ephemeral sanitized Actor previews

Date: 2026-10-13. Status: implemented; native macOS fixture checks verified,
real-provider and native Windows acceptance pending.

## Decision

Extend ADR 004's complete-record observation model with a separate ephemeral
preview channel. `laind` owns provider inference and parsing; the CLI only renders
typed public DTOs from the small `lain-wire` crate. Provider-private raw deltas,
sanitized previews, committed messages and authoritative turn snapshots are
distinct. Tool argument/reasoning deltas never become assistant prose or executor
input. Only the validated completed response drives tools and persistence.

Actor SSE starts with a committed `turn` snapshot. `preview` events carry turn,
preview and eventual message IDs, a monotonically increasing per-preview sequence
and a tagged `started`, `delta`, `finished` or `discarded` payload. Started may
contain a sanitized current checkpoint. Finished closes a preview segment and
does not claim that persistence or a worker operation completed. Durable messages
keep the assigned message ID. Existing committed snapshot revisions are retained.

Reconnect takes authoritative state and an optional current preview checkpoint;
it never replays inference. Clients suppress duplicate sequences and committed
message IDs. A gap falls back to final authoritative output. The broadcast ring
and per-client queue are bounded; lag requires resynchronization. Disconnect does
not cancel inference. Failure, interruption or mismatching final content marks
the preview unconfirmed rather than creating a false completed message.

Incremental redaction is bounded to 16 KiB per response. It withholds any suffix
that could extend into a known configured credential or a complete-record
sensitive pattern, including Unicode simple-case-fold equivalents. Sensitive or
oversized streams discard previews. Full-response validation remains mandatory.
Preview sanitization buffers use transient zeroizing storage. The private parser
also temporarily holds provider frames/structured records; none are written as
raw traffic to SQLite or normal logs. Only sanitized checkpoint text enters the
public channel.

The CLI has one input owner and response output coordinator. Raw terminal input
is scoped to an active line operation and restored by a guard. Empty ordinary
Enter retains its prompt, while empty confirmation denies. CR and LF are distinct
native events; a paired LF is consumed rather than printing another prompt or
answering confirmation. Provider text is escaped before terminal rendering.
Optional monotonic timings contain durations only and use stderr; JSON stdout
retains its one final-result contract.

## Limits

This is protection for known credential values and documented recognizable
patterns, not detection of every unknown, transformed or encoded secret. A future
unknown sensitive value can appear in preview. Ambiguous ordinary suffixes may
be delayed until completion; already rendered safe text cannot be erased from a
terminal's scrollback. Discard notices and final snapshots provide truth, not
rollback of what the viewer has seen. Preview resumption is current state, not
provider-token replay or continuation of an interrupted generation.

Native macOS PTYs prove gated incremental visibility, LF/CRLF, input ownership,
confirmation, cancellation, terminal restoration and reconnect with fixtures.
They do not prove native Windows console behavior or real provider compatibility.
No Web, worker intelligence or semantic memory subsystem is introduced.

Responses event names were checked on 2026-10-13 against the official
[streaming events reference](https://developers.openai.com/api/reference/resources/responses/streaming-events).
