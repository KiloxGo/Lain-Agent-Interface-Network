# D1b reading processing and recent experiences

Date: 2026-10-26, native Windows/MSVC. Contract: [ADR 014](adr/014-source-linked-reading-experiences.md).
Implementation continues [D1a](READING_ACCEPTANCE.md), not completion of the
[two-stage goal](DAILY_COMPANION_IMPLEMENTATION.md).

## Evidence status

Final revision gates passed on Rust/Cargo 1.98.1, `x86_64-pc-windows-msvc`:
format, strict workspace/all-target Clippy, workspace build and full workspace
tests (**180 passed, zero failed, three manual ignored**). Reading suites contain
22 tests (12 new experience tests plus 10 retained acquisition tests).
Final native cross-process regression: **24 checks, 68 fake model requests, zero
paid calls, cleanup true**. It includes corrected and withdrawn subsequent recall;
operation-only evidence retained the committed generation fact without old content.
Final schema **11 → 13** binary-only upgrade/full rollback: five checks, three fake
requests, zero paid, no personal state, cleanup true. The final native executable
hashes match both process reports after the last build:
CLI `75828a20f429de019fe671f6bc0d80eed027c292c85daaac9fc9f310451045ce`,
daemon `4547157cb1d322532febf522ba17744d4f3944333d028fe603b624636bd6625b`.

Local evidence remains ignored under `target/d1b/`: `evidence.json`,
`workspace-release.log`, `clippy-release.log`, `build-release.log`,
`process-release.log`, `upgrade-release.log` and earlier failed/intermediate logs.
These final checks ran sequentially against the same output directory. No Cargo
build ran while the process fixtures used its executables. As in D1a, compilation
used process-scoped no-incremental/no-debug-symbol settings to bound disk use.

Targeted deterministic experience tests cover actual injected Actor execution
with bounded input/no tools, durable cumulative budget across restart, dedup,
cancel/revoke during model execution with returned usage retained, malicious typed
Worker call rejection, private fresh recall, cross-binding exclusion, owner correction,
withdraw/restore without resurrection, stale Ready/in-flight reply rejection,
expiry preserving operation truth, atomic terminal/experience rollback and explicit
UTC-offset midnight boundaries, schema-12 body/grant preservation and content-free
post-withdrawal operation truth with revocation during generation. These fake outputs establish mechanics, not model
truthfulness, naturalness or real article understanding.

An initial test failed by slicing a Chinese string at byte 100; the test now uses
character boundaries. Initial Clippy found a test's lock lifetime across await;
it now uses a lexical block. Neither failure was recorded as a pass.
Before the operation-truth extension, full workspace validation passed 179 tests
with three manual ignored, and 24 cross-process checks used 66 fake requests. A
later final run below supersedes those counts; these earlier results are retained
without claiming they covered the extension.

The operation-truth extension specifically follows C1.1's known failure: an empty
active lookup must not become “I never saved/processed it”. Actual content-free
committed receipts are supplied in the current scope/window; instructions alone
are not treated as proof that a real model will use them correctly. Real-model
replay of this new capability still needs a new authorized budget.

The first schema-11 upgrade attempt reached the new-binary stage but the fixture
incorrectly treated its existing C1 private decision as newly invented. The repair
compares original versus migrated private fields and asserts no reading origin,
permission or requests were invented. The corrected five-check schema 11 → 13
backup/upgrade/refusal/full-rollback rehearsal passed with three fake calls, no
personal state and complete cleanup. Only the installed release's two executable
files were copied into synthetic state; its personal DB/settings were not used.

The process fixture explicitly seeds a **synthetic acquired body** into its own
stopped DB, then uses actual native binaries, HTTP Responses streaming, independent
Kernel storage/restart and local OneBot for processing/recall/correction. It does
not pretend that this seeding is a production HTTP fetch. Real local socket fetch
validation is separately in the D1a tests; public HTTPS/TLS/source compatibility
still needs authorization and live evidence. No fixture configuration bypasses
production public-address filtering.

No new paid call, personal DB/credential access, QQ contact, real source, deployment
or device connection occurs in these tests. Temporary model/transport credentials
and generated state are synthetic. The installed personal C1.1 release remains on
schema 11; source/memory/live-call authority is not inferred from this milestone.

## Five-step operator flow

1. Inspect `lain reading grant`, `budget`, `suppressed` and `audit`. Deliberately
   select exact URLs, expiry/retention, acquisition limits, one private channel,
   an aggregate processing-request cap and a fixed UTC offset. Processing defaults off.
2. Use the existing `reading enable` arguments plus `--model-channel <id>
   --model-requests <cap> --utc-offset-minutes <offset>`. Start an acquisition with
   its saved request ID. On successful acquisition the optional processing event
   runs; `reading process <id>` can explicitly inspect/start a currently eligible
   job. Repeats return the same job even when Unknown, without new inference.
3. Inspect `reading show <id>` and the linked `actor show <id>` for actual processing
   state, input extent, request/usage receipts and cancellation request. Use
   `experience search --from-ms <utc> --to-ms <utc> [--text <topic>]`, `show <id>`
   and `history <id>`; model summaries retain uncertainty and provenance.
4. Correct with `experience correct <id> <replacement> --revision <current>`;
   withdraw with `experience withdraw <id> --revision <current>`. Withdrawal also
   suppresses that exact URL and removes all its stored bodies/derived summaries.
   `reading restore-source <url>` permits new explicit acquisitions only; old
   withdrawals, old jobs and spent requests do not reset. This is not an archive,
   canonical Actor audit or backup erasure operation.
5. Cancel pending acquisition/processing with `reading cancel <id>`; inspect the
   processing cancellation request and Actor's actual uncertain outcome. Revoke the
   grant with its current revision to stop future processing/disclosure. No contact
   is sent by processing; private replies still require a new user input and their
   separate reply grant. Proactive contact is D2, not implied here.

API additions: POST `/v1/reading/activities/{id}/process`; GET `/v1/reading/budget`,
`/v1/reading/audit`, `/v1/reading/suppressed`; POST `/v1/reading/restore-source`;
POST `/v1/experiences/search`; GET `/v1/experiences/{id}` and `/history`;
POST `/correct` and `/withdraw`. All use existing administrator authentication and
no-store responses. Correction/withdraw require expected_revision. Audits are
bounded recent metadata and explicitly report partial coverage.

Schema 13 upgrade/rollback follows the stopped full-instance/ACL-preserving method
in D1a and C1.1. Older binaries reject it; rollback restores the matching complete
stopped backup, retaining later state separately. No personal upgrade is performed.
Native macOS/Linux, real model/source/QQ quality, delayed multi-day continuity and
active sharing remain unverified. Fixed offsets do not imply DST-aware scheduling.
