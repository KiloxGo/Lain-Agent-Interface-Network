# ADR 013: Separately authorized bounded public text acquisition

Date: 2026-10-26 (Asia/Shanghai). Status: accepted for D1a implementation.
This is the first slice of the combined [delivery plan](../DAILY_COMPANION_IMPLEMENTATION.md),
not completion of experiences, autonomous reading or proactive contact.

## Decision

Kernel owns one current ReadingGrant and its immutable revision history. Default
revision zero is disabled. Administrator CAS changes specify exact canonical HTTPS
URLs (at most eight), an expiry no more than 31 days away, a positive retention
duration no more than 31 days, a total attempt cap 1–100 and a body limit 1–65536
bytes. These are implementation ceilings, not user-confirmed default settings.
There are no inherited grants, wildcard hosts, query strings, URL credentials or
redirects. Replacing a grant invalidates old work/content conservatively, even if
the replacement contains the same URLs. Re-enabling is a new explicit decision.

Migration 012 adds separate grant/activity/result tables. It does not reclassify
existing messages, SourceGrants, memories or Worker state. One request ID binds
the exact URL and grant revision durably. Admission consumes one attempt, including
subsequently cancelled, failed and uncertain work. At most one unfinished activity
is admitted. Idempotent repeats return the recorded activity without restarting;
changed payload conflicts. Counters come from persisted admissions, never from a
resettable process counter. This slice makes zero model requests.

Activities transition Pending → Running → Completed/Failed; cancellation and
revocation terminate unfinished activities. Restart makes unfinished work Unknown
(or Expired if no longer eligible), without automatic fetching. A completion and
its body commit together. Cancelled/revoked/expired work cannot later commit a body.
Completed remains evidence of actual past acquisition after the body is unavailable.
It is not a reading/summary/verification or human-read receipt.

Network acquisition is an asynchronous bounded GET, off the storage lock. Resolve
the host once, reject private/special addresses, and pin the validated addresses
to that client's connection to avoid a second DNS decision. TLS verification stays
enabled. Disable environment proxies, redirects, cookies and ambient credentials;
the reader has no model or Kernel API bearer. Accept only status 200, unencoded
UTF-8 `text/plain`, with a streaming body bound and an overall 20-second deadline
including DNS (five-second connect bound). Do not retain partial/invalid/sensitive
bodies or log upstream errors/URLs. Broad conservative special-address exclusion
is intentional; this is not an arbitrary intranet reader.

Known runtime credentials/patterns are checked on the entire completed text before
storage and again on disclosure. Stored text is untrusted source data and currently
available only through the administrator inspection endpoint. No content is parsed
as a tool call or passed to an Actor in D1a. This separation must be preserved when
typed Actor reads are added in D1b.

Cancellation/replacement notifications drop in-flight acquisition futures; shutdown
also stops them. A request already started cannot be recalled from the server.
Recheck active scope and deadline under the Kernel lock before committing content.
Use one finite timer for the next persisted retention deadline, rebuilt on startup
and activity changes; no idle polling/model loop. Expiry removes the result body,
retaining operation metadata. Inspection also prunes and denies expired content.
Database pages, WAL, external backups and previously disclosed copies are not
claimed securely erased. Metadata URL/time/audit retention is distinct from body
retention. Source replacement removes all old bodies rather than silently carrying
their permission into a new revision.

## Limits and validation

No HTML extraction, attachments, arbitrary filesystem access, continuous scheduler,
summary generation, experience projection, Actor source tool or contact grant.
Those remain explicit subsequent milestones. Native fixtures and remaining live
checks: [READING_ACCEPTANCE.md](../READING_ACCEPTANCE.md).

Address exclusions were checked on 2026-10-26 against the official
[IANA IPv4 special-purpose registry](https://www.iana.org/assignments/iana-ipv4-special-registry)
and [IPv6 registry](https://www.iana.org/assignments/iana-ipv6-special-registry).
TLS/public routing and real source compatibility still require separate authorized
acceptance; a loopback HTTP test seam does not claim those properties were exercised.
