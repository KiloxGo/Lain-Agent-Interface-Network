# Phase 7.1 acceptance: independent memory and project knowledge

Date: 2026-10-19. Baseline: fetched `origin/main` `17a676d`.
Implementation and native macOS deterministic validation are available. An
owner-authorized real-provider attempt and an additional authorized attempt are
recorded below; neither passed. No
actual Codex work was run. Real-provider Phase 7.1 retrieval,
Phase 6.6 real-provider delivery and native Windows/Linux acceptance remain pending.
Historical Phase 6.5 real evidence is unchanged.

The implementation reuses Kernel services, the existing locked SQLite Store,
authenticated API, persisted Actor confirmation, wire types, CLI and Phase 6.6
result-evidence projection. It adds no parallel persistence, worker intelligence,
Web UI, scheduler or model-based routine CI.

| Requirement | Deterministic evidence |
| --- | --- |
| Exact explicit owner text, stable IDs, origin/time/scope, retries/conflicts and restart | `owner_memory_is_idempotent_scoped_searchable_and_restart_durable`; native CLI add/show before and after actual fixture-daemon restart |
| Actor proposals persist without active memory; denial/acceptance use one owner decision | `actor_candidates_require_one_persisted_confirmation_and_keep_inferred_origin`; native pending/deny/approve and refusal of generic candidate acceptance |
| Pending review survives restart; approval performs no inference | `pending_memory_candidate_survives_restart_without_inference_or_automatic_acceptance` |
| Scope isolation, Global vs Workspace, kind filter and multilingual literal search | Owner persistence test, native workspace-list isolation and `bounded_pages_advance_only_over_retained_records` |
| Bounded pages/cursor correctness | Bounded-page test with seven large CJK records, no missing or repeated IDs |
| Disable/re-enable, supersession, stale-write conflict and deletion/retry tombstones | `disable_enable_conflicts_supersession_and_deletion_affect_only_semantic_retrieval`; native disable/search/enable/delete |
| Typed authority/lifecycle contradictions and malformed scope are rejected | `invalid_authority_scope_size_and_secrets_never_enter_memory`; wire candidate lifecycle test and Store corrupted-index test |
| Known credentials/API token/patterns and terminal controls do not enter memory | Invalid-input test, `authenticated_memory_api_reads_are_quota_free_and_reject_credentials`; native SQL/log inspection retains existing configured-credential checks |
| Deletion remains possible when existing content now matches sensitive patterns | Store `deletion_is_available_when_existing_content_is_now_sensitive` |
| Retrieved evidence cannot authorize shell or bypass mutation confirmation | `actor_retrieves_persisted_memory_as_evidence_and_injection_cannot_authorize_tools`; actual reopened SQLite record ID in the model fixture's next request and final response |
| Real native Responses adapter/CLI retrieves saved memory | Native local HTTP fixture receives `memory_search`, then answers with the actual persisted record content/ID after daemon restart; this is not real-provider acceptance |
| Correct WorkSession/Turn evidence, failures under Completed, no implicit promotion | `summary_selects_exact_turn_evidence_without_promoting_to_memory`: twelve nonzero commands under Completed, eight retained excerpts, distinct follow-up evidence, source IDs/watermark, separately accepted same-workspace candidate |
| Atomic record/FTS/request writes and candidate acceptance | Store `memory_and_candidate_review_are_atomic_and_legacy_schema_migrates` injects transaction failures |
| Old migration/Actor JSON compatibility | Exact migration-006 upgrade test plus existing Phase 1/profile/Actor/delegation migration tests; typed Actor snapshot round-trip with memory action |
| Existing Phase 6.6 result notices preserved | Entire existing delegation suite and native PTY automatic delivery, reconnect/display ack, wrapped Unicode input, worker independence and restart checks |
| Reads/diagnostics/summaries don't call models | Empty fixture model requests in API test, native provider-call baselines around CLI reads/summaries, summary creation without Actor |
| Unix private storage and native lifecycle | Native 0600 database/lock and 0700 local-directory checks, actual foreground launcher signals, CLI processes and PTYs |

Local acceptance entrypoints:

```text
zsh scripts/check-native.zsh
powershell -NoProfile -File scripts/check-native.ps1
```

The macOS entrypoint runs locked workspace build, fmt, warnings-denied Clippy,
workspace tests and native CLI/PTY smoke. Windows must run on a native Windows
host; Linux can use `python3 scripts/native_smoke.py --platform linux` following
native Cargo checks. No emulation/cross-compilation result counts as acceptance.
Exact executed counts/results are in [DEVLOG](../DEVLOG.md).

## Remaining acceptance and deferrals

A real-provider conversation retrieving an earlier persisted memory has **not**
been validated. That definition-of-done item remains open; the authorized failed
attempt below does not close it. A future opt-in check
must separately authorize the Actor provider/model/budget, exact selected non-secret
memory and scope, and any changes to the owner's installed daemon/database.
A sufficient bounded probe would explicitly store one owner-provided non-secret
memory, restart only a task-owned daemon if needed, ask it to retrieve that scope,
and inspect the real tool receipt/MemoryId/provenance. Model prose without a real
retrieval receipt does not count. Do not create real Codex work for this check.

Native Windows PowerShell/ACL/ConPTY and Linux are unavailable here and remain
unverified. Existing Phase 6.6 real-provider result delivery is still separately
pending; historical Phase 6.5 real runs are not reused as evidence for new paths.
Implementation completion and deterministic acceptance do not close those gaps.

World/journals, personality evolution, automatic extraction/acceptance, background
summarization/reflection/consolidation, embeddings/vector services, new backends,
agent-to-agent memory sharing and the Delegation Compiler remain deferred.
Suggested Phase 7.2 is a small inspectable World/notes foundation with explicit
memory/summary links, requiring separate owner scope. Do not begin Phase 7.2 or
Phase 8 automatically. See [MEMORY.md](MEMORY.md) and [ADR 008](adr/008-independent-owner-reviewed-memory.md).


## Authorized real-provider attempt — 2026-10-19

The owner separately authorized one Actor turn, at most four model requests,
using the current provider with a task-owned temporary database/daemon. The exact
non-secret Global preference was “Lain 的独立记忆与 Agent 管理能力同等重要”. The
selected model was `mimo-v2.6-flash` with the existing Responses driver. Actual
owner daemon/config/database and Codex work were outside the authorized scope
and remained unchanged.

Memory creation and exact record equality after a native test-daemon restart
passed. The subsequent `lain --json ask` returned a nonzero status, so retrieval
was **not accepted**. The temporary acceptance wrapper incorrectly discarded the
CLI's failed-turn snapshot before preserving its state/counters/tool receipts.
Only a generic wrapper RuntimeError was retained; actual model-request count and
retrieval evidence are therefore **unverified**, rather than fabricated from the
maximum budget. The test process stopped, and the temporary database was removed.
No automatic second real attempt was made. Provider-specific cause is unknown.

Private safe evidence: `acceptance/phase-7-1-memory-2026-10-19-7748512f.json` under
the per-user Lain directory. MemoryId:
`8029678f-677f-43eb-a3c2-6f4506a16ca6`. Evidence contains safe metadata and content
digest, not raw provider frames, replies or credentials. The wrapper was corrected
to retain valid failed Actor snapshots and their request counters, with a quota-free
deterministic check for nonzero-CLI/valid-snapshot, missing-snapshot and normal JSON
cases. Because unused budget cannot be established, another real attempt requires
an additional explicit allowance rather than assuming the first allowance remains.


## Additional authorized attempt — 2026-10-19

After the wrapper correction, the owner separately authorized one additional
Actor turn, again capped at four model requests under the same temporary isolated
scope. This attempt also **did not pass**. The real Actor made **2 model requests**
and reported **4,838 usage tokens**, ending **Unknown** with **no executed memory
tool receipt** and no citation of the actual saved MemoryId. These observations
are not a successful retrieval. The underlying provider/call incompatibility is
not established by the safe evidence; no raw rejected call was retained, and no
product change is justified solely by guessing its contents.

Exact persistence after the native test-daemon restart passed. No WorkSession was
created, inspection left the Actor snapshot unchanged, and the task-owned process
stopped. No owner instance/database was changed and no actual Codex ran. Private
safe evidence is `acceptance/phase-7-1-memory-2026-10-19-aa23914a.json` under the
per-user Lain directory:

- MemoryId: `ca698129-487f-4bc5-8620-b44f7d7a3e12`
- ConversationId: `cc04e3c9-3293-42b1-a406-e82952ad7379`
- ActorTurnId: `df2e27eb-3c97-4f10-9c76-0001214c3a37`

The first attempt's request count remains unknown (bounded by its authorized
maximum of four); it is not retroactively assigned the second attempt's count.
The two attempts together therefore have an upper bound of six requests, with
only the additional attempt's two requests/usage directly recorded. No third real
run was made. Deterministic implementation remains available, while the real
conversation definition-of-done item and final Phase 7.1 acceptance remain open.
Further investigation should capture safe typed rejected-call diagnostics or use
a separately authorized compatible provider; do not replay uncertain mutations,
spend new quota automatically or claim that either of these runs passed.
