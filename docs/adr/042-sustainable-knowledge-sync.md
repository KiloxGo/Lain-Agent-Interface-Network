# ADR 042: Knowledge sync that can run for a busy group

Date: 2026-11-07 (Asia/Shanghai). Status: implemented. Amends the freshness and
backlog rules of ADR 022 and ADR 023.

## Problem

Real collection of one 63-member group produced 84 messages in ten minutes. The
existing rules could not serve daily briefings from that:

- A caught-up read gave a 30-second lease, and briefings refuse a stale source. Staying
  readable took a read every 20 seconds, which reaches the 10,000-read limit in about
  2.3 days.
- Every read cleared the lease when admitted, so a briefing in progress could fail
  during any routine read.
- A backlog waited the full interval between pages, at 32 changes per page.
- The record limit counted all of a source's history, including earlier grants, so a
  renewed grant could not recover. The grant maximum of 10,000 lasts about three days
  at 3,000 messages a day.

## Decision

- A scheduled read's lease is its interval plus 60 seconds, at most one hour. An
  unscheduled read keeps 30 seconds and still clears the lease when admitted.
- A scheduled read does not clear the lease when admitted. A failed read stops the
  schedule, so the lease simply runs out.
- While a backlog remains, the next page is read after at most 5 seconds. A page that
  is not caught up keeps the existing lease and does not extend it. A source that has
  never caught up has no lease.
- The record limit counts the current grant only, and expiry still does not refund it.
  The grant maximum rises to 200,000.

## Consequences

An upstream withdrawal can take up to an hour to reach Kernel instead of 30 seconds,
and only as long as the chosen interval requires. Revoking in Kernel remains immediate.
This trades tighter upstream revocation for a source that stays usable, which suits a
personal server where the same person controls both sides.

The 10,000-read receipt limit is unchanged. A 15-minute interval with 3,000 messages a
day uses about 190 reads a day, or roughly 52 days in total, after which sync stops
with a visible reason. Pruning old receipts is left for later, once real volume is
known. Sync and archive grants still end within 31 days and must be renewed.
