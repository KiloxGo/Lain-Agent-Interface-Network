# ADR 041: Bounded phrase lookup within an existing briefing

Date: 2026-11-06
Status: Accepted; implemented and deployed on the original database at schema040

## Context

Actual-provider synthetic-source follow-ups sometimes spend their finite request
allowance paging through reports before opening an original. The existing report
contains at most four items per page and original inspection requires its stable
item number. More allowance does not repair this missing topic-locating operation.

## Decision

Extend the existing Actor `briefing_read` arguments with nullable `query`.
Missing/null retains ordinary reads. A nonempty literal phrase of at most128 UTF-8
bytes, without control characters, matches the Unicode-lowercased title, summary,
quoted time phrase or checked citation quotations of the same manifest's already
processed items. Trim surrounding whitespace. No token splitting, regex, semantic
search, archive scan, new model request or new source authority is introduced.

Filter before paging. Return at most four items and12KiB as before. `after` and
`next_after` remain original item numbers within the unchanged query. Preserve
ordinary total-generated/group/display counts; `total_matching_items` separately
describes this filter. Matching folded duplicates are independently locatable by
their original number, with their actual existing relation/group metadata.
`matching_citation_indices` identifies checked quotations containing the phrase;
an empty array means the match occurred in another item field. The Actor can then
inspect an actual citation with `briefing_inspect`, copying returned IDs/numbers.

A zero match is not evidence that a notice is absent from original archives or
even from differently worded extracted items. Results remain untrusted generated
reports; reading a summary is not original inspection. Related correction/conflict
statements must still be read before stating the current arrangement.

## Authority and compatibility

Use the same whole-manifest current-source checks, secret checks and durable
disclosure capture as ordinary reads, including empty results. Default latest
first-page freshness applies equally to filtered reads. No implicit processing,
grant change or replay occurs. Explicit snapshots keep historical semantics.

Schema040 is a reader-compatibility gate only: persisted Actor actions may now
contain a query that old deny-unknown-fields readers cannot decode. It creates no
table and rewrites no old action, source, item, manifest, memory or grant. New
readers accept legacy missing-query actions and omit null queries when saving.
Upgrade requires the normal backup/maintenance path; do not point an old program
at a40 database or replace the live39 database with a test copy.

## Limits

This improves bounded topic location, not general factual accuracy, semantic
matching or malformed-provider-argument recovery. It does not guarantee a fixed
number of calls or complete source history. Actual-provider and real-source
acceptance must be recorded separately from deterministic fixture tests.
