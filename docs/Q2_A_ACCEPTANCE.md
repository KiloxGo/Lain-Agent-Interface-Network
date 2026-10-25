# Q1 diagnostics repair and Q2-A private-memory acceptance

Date: 2026-10-25 (Asia/Shanghai), following the 2026-10-24 scope decision. Selected implementation only; full Q2/Q3 is not
included. Decision: [ADR 010](adr/010-scoped-private-conversation-memory.md).

Q2-B1 后续版本使用 schema 010，并扩展了同一验收脚本。下面 schema-9 / 126 项 /
12 场景等数字保留为 Q2-A 当时的实测证据；当前升级与回查步骤见
[Q2-B1 验收](Q2_B1_ACCEPTANCE.md)。

## Evidence boundary

The user manually confirmed real multi-turn Chinese private QQ send/receive on
2026-10-24. That screenshot is ordinary-chat evidence only. It does not establish
memory, reconnect/restart/revocation, multi-day naturalness or cloud acceptance.
Historical Phase 6.6/7.1 real-provider failures remain recorded and unresolved.

This change uses temporary SQLite databases, fake models and loopback WebSocket/
HTTP servers. No paid model call, real QQ send/login, private credential read,
personal database migration, service restart or real memory-grant enablement is
part of development validation. Builds use `target/q2a`, independently of the
running `target/debug` binaries. No new CI or routine remote model evaluation.

## Requirement-to-test map

The Rust test names below are searchable in `crates/lain-kernel/src/channels/`,
`crates/lain-kernel/src/persistence/private_memory.rs` and the existing integration
tests. CLI contract coverage is in `crates/lain-client/src/channel.rs`.

| Requirement | Deterministic evidence |
| --- | --- |
| Saved binding is not current Online; shutdown and restart truth | `durable_binding_never_restores_online_and_shutdown_closes_observations` |
| Safe distinct connect/auth/account/protocol failures; task abort stops observation | `transport_failures_have_safe_distinct_categories_and_abort_stops_projection` |
| Handshake is not Online; validated identity and subsequent loss | `online_requires_identity_and_connection_loss_is_observable` |
| Identity deadline never marks Online | `identity_timeout_is_distinct_and_does_not_mark_online` |
| Same repeated failure does not flood warnings | `repeated_authentication_failures_emit_one_safe_warning` |
| Authenticated diagnostics/grant/effect routes, default off, expiry validation, stale CAS and reply-grant independence | `management_api_authenticates_diagnostics_and_revisioned_memory_grants` |
| CLI parser, paths, authorization, enable/revoke payload, history cursor and scope conflicts | `parsed_channel_commands_use_authenticated_management_contracts` |
| Default-off, queued pre-grant source excluded even with later admission, revision persistence/revoke/re-grant, restart Unknown and action-idempotent effect | `grants_default_off_are_revisioned_persistent_and_do_not_backfill_old_sources` |
| Exact real source, isolated binding, inspected targets/revisions, at most one correction/forget per turn, supersession/deletion, source tombstones | `correct_forget_scope_evidence_and_tombstones_change_active_retrieval` |
| Memory/FTS/suppression/effect transaction rollback | `effect_failures_roll_back_memory_fts_suppression_and_receipts` |
| Expired grant, reply revocation, invented quote, sensitive or over-budget record rejected | `expiry_reply_revocation_invalid_evidence_and_secret_content_fail_closed` |
| Bounded relevant records, no unrelated match, immediate exclusion after disable | `recall_is_bounded_relevant_and_does_not_cache_retired_records` |
| Schema 8 → 9 preserves exact old memory snapshots and real confirmation times, no invented grants/extraction | `schema_eight_migration_preserves_reviewed_records_and_defaults_grants_off` |
| Actual MemoryId/record reaches model context after restart and eviction of original dialogue; correct/forget removes stale semantic context | `actor_independent_memory_recalls_actual_id_then_corrects_and_forgets_without_stale_context` |
| Revoke while inference is gated, effect fails and next private tools disappear; old candidate review and Work review remain | `revocation_during_generation_and_legacy_review_cannot_be_bypassed` |
| Existing receive/dedup/batching/reconnect/receipt/Unknown semantics | Existing channel suite, including `napcat_wire_identity_reconnect_receive_during_model_and_lost_receipt`, `napcat_failed_malformed_and_missing_receipts_never_retry`, restart/revocation fixtures |
| Legacy candidate approval, summaries, lifecycle/credentials/worker boundaries | Existing memory, Actor, profiles, foundation, worker and native CLI smoke suites |

The independent-memory Actor fixture creates 22 completed filler turns directly
in its temporary Kernel without model calls, asserts the original preference is
absent from recent dialogue, reopens storage, and verifies the selected MemoryId
in the actual fake-model request. A fluent answer alone is not a memory test.

## Local verification

Native host: Windows x86_64/MSVC, Rust/Cargo 1.98.1. Verification completed on
2026-10-25 using only deterministic fixtures. macOS and Linux have not been run
for this change; native Windows does not establish either platform's acceptance.

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --offline --locked --workspace --all-targets --target-dir target/q2a -j 2 -- -D warnings` | PASS |
| `cargo test --offline --locked --workspace --no-fail-fast --target-dir target/q2a -j 2` | PASS: 126 passed, zero failed, three manual tests ignored; 15 suites, final sequential run |
| `cargo build --offline --locked --workspace --target-dir target/q2a -j 2` | PASS; MSVC linker informational stdout only |
| Native Kernel test executable, parallel harness | 45 passed, zero failed, two manual probes ignored |
| `LAIN_TEST_BIN_DIR=target/q2a/debug` / `python -X utf8 scripts/native_smoke.py --platform windows` | PASS, exit 0; temporary-data cleanup completed, zero paid quota |
| Windows PowerShell parser against operating-guide blocks | PASS, nine blocks; parsed without executing live commands |
| Changed documentation relative links / whitespace / API-key patterns | PASS; no non-fixture key pattern found |

The native CLI fixture exercises setup/ACLs, shutdown/restart, doctor, streamed
Actor calls, confirmation, fake Worker work and existing Phase 7.1 scoped memory/
summary/retrieval persistence. Its embedded acceptance runner still reports native
interactive/ConPTY and full real acceptance incomplete; this run does not close
those obligations. New Q2-A contracts are covered by the Rust fixture tests above,
not by a real provider or QQ memory run.

Development encountered insufficient space for MSVC PDB files in the isolated
target. Only this task's `target/q2a` cache was cleaned. Rebuilds disable local
incremental output and dev/test debug symbols; no repository build profile or
global toolchain setting is changed. This does not change protocol/policy tests.
Run Cargo build/test steps before native smoke. Windows holds an executing exe
open; rebuilding the same test output while smoke is running can fail with access
denied. Development observed and corrected that validation-order mistake without
touching the personal instance. The repeated-warning fixture also uses a
process-wide capture filtered by its unique ChannelId to isolate parallel tests.

## Process-level private-memory fixture — 2026-10-25

From the repository root, after the isolated native workspace build:

```text
python -X utf8 scripts/qq_memory_smoke.py --bin-dir target/q2a/debug
```

This focused script uses actual `laind`, `lain`, the existing Codex test process,
SQLite/FTS, OneBot WebSocket and the actual Responses HTTP/SSE adapter. The
WebSocket and model endpoints are local fake servers bound to ephemeral loopback
ports. There is no real-endpoint or credential option. It generates its own test
credentials, creates an owner-private temporary instance with `lain setup`,
scrubs inherited Lain/provider configuration and runs real administrator CLI/API
commands only against that instance. No personal state is imported or upgraded.

The script requires Python and the `websockets.sync` server API. The local native
run uses **Python 3.13.5 / websockets 15.0.1**, already installed on this host; no
package was downloaded or added to product/Cargo dependencies. Existing
`native_smoke.py` remains independent of this optional test dependency. Other
Python/library versions and native macOS/Linux have not been verified here.

| Check emitted by the runner | Evidence |
| --- | --- |
| `identity_gates_online` | Actual daemon API stays Authenticating until the fake account identity response is released |
| `connection_loss_and_recovery` | Closing the wire produces Disconnected and then validated Online with a cleared failure |
| `default_off_review_and_management_auth` | No private tools/context or memory before enable; actual proposal remains Candidate/PendingConfirmation; unauthorized management reads and enable return 401 |
| `sensitive_input_rejected` | A message containing the generated model credential is rejected before incoming persistence and model context |
| `remember_actual_source_and_memory_id` | Actual private input creates memory through a typed call; CLI record matches real input/batch/conversation/turn/action/grant IDs with Inferred authority and no individual confirmation |
| `independent_recall_after_eviction_and_restart` | 21 ordinary fake QQ exchanges evict the source, read-only stopped-DB inspection confirms it is absent from recent 20 turns, restart preserves the record, real model HTTP input contains that exact record and recall receipt contains its MemoryId |
| `correct_forget_and_source_replay` | Actual correction relation, semantic deletion and Active query exclusion; refreshed model evidence excludes superseded/deleted records, later recall is empty, and replaying the original OneBot ID plus a processed barrier creates no duplicate input/effect |
| `revoke_during_provider_generation` | Real provider request is gated, administrator revokes, stale tool effect fails, and the next model request loses private tools/context |
| `work_review_unchanged` | With memory enabled, Work creation remains a proposal and creates no Worker until administrator review |
| `worker_permission_unchanged` | A separately administrator-started fake Worker has a real pending permission request; an Actor attempt cannot change it; the test administrator explicitly denies it for cleanup |
| `expiry_during_provider_generation` | A future grant deadline passes while the actual provider request is gated; its returned effect is rejected |
| `schema_receipts_tombstones_and_secret_redaction` | Read-only schema-9, effect ledger, suppression, semantic/FTS deletion and credential absence in DB, CLI/API and daemon logs |

The fake provider derives source quotations, target IDs and revisions from actual
request data. It does not pre-seed memory or use canned MemoryIds. The fixture's
ordinary exchanges use the same receive/Actor/reply path, not direct database
insertion. Database inspection is read-only and connections close explicitly.
The 42 model requests in this deterministic scenario all terminate at the local
fake provider, with zero paid calls. This does not establish real model memory
quality, naturalness or installed NapCat compatibility.

The final JSON report includes checks, host/runtime versions, actual MemoryIds,
request count, tested binary SHA-256 hashes and `cleanup: true`. Exit 0 means all
checks and cleanup passed. The default work deadline is 300 seconds, configurable
with `--timeout-seconds 60..900`; per-operation and shutdown bounds remain finite.
Failure reports identify the current stage and safe assertion/location, without
dumping provider bodies, credentials or the private temporary log.

Two explicit failure paths exercise cleanup, with **expected exit code 1**:

```text
python -X utf8 scripts/qq_memory_smoke.py --bin-dir target/q2a/debug --inject-failure startup
python -X utf8 scripts/qq_memory_smoke.py --bin-dir target/q2a/debug --inject-failure model
```

They fail after daemon startup or on a real provider request, then release gates,
stop the test daemon/fake Worker, close and join fake services/handlers, and remove
the temporary instance. Successful and injected-failure cleanup also checks for
leftover background threads. Startup/provider failure does not enable or touch
the running personal QQ instance.

Closeout verification on native Windows, 2026-10-25: all 12 normal process checks
passed (exit 0), both injected failures returned the expected exit 1 with successful
cleanup, and the current workspace rerun passed **126 tests, zero failed, three
manual tests ignored** (15 suites, 46.98 seconds). Format, strict workspace
all-target Clippy, workspace build and Python compilation also passed. MSVC build
output contained only its existing linker informational messages. The machine-
readable local report is `target/q2a/qq-memory-closeout.json` (ignored build output,
not personal state); it records the tested binary hashes and normal/failure checks.
The earlier native CLI smoke above is retained baseline evidence and was not
unnecessarily repeated for this Python/doc-only addition.

### Scope review

The review followed `channels::napcat`, `ChannelService`, Kernel channel/Actor
admission, private-memory persistence, Actor context/tools, memory lifecycle and
authenticated management API/CLI. In that scope no additional product defect was
reproduced requiring a Rust behavior change. Identity and exit guards, revision/
deadline/source checks, transactional/idempotent effects, scoped refreshed recall,
suppression and unchanged approval boundaries agree with the tests. Existing
Rust tests remain the evidence for transaction rollback, cross-binding isolation,
old-schema migration, queued pre-grant sources and interrupted/Unknown recovery;
the new script does not pretend to repeat every fault at process level.

The new entry point and this evidence record are additions to the existing
uncommitted Q1/Q2-A tree. No Rust product code, storage schema, milestone scope,
real credentials, service or deployment was changed by this closeout task.

## Administration contracts

All routes use the existing administrator bearer authentication and `no-store`.
NapCat/WebUI credentials cannot administer Kernel grants.

| Method/path | Meaning |
| --- | --- |
| GET `/v1/channels/status` | Bindings plus ephemeral connection observations |
| GET `/v1/channels/{id}` | Existing bounded channel snapshot plus connection |
| GET `/v1/channels/{id}/memory-grant` | Current decision; absent means disabled revision 0 |
| POST `/v1/channels/{id}/memory-grant` | `{enabled, expected_revision, expires_at_ms}`; optional future deadline, null on revoke |
| GET `/v1/channels/{id}/memory-grant/history` | Latest 100 persisted decisions, newest first |
| GET `/v1/channels/{id}/memory-effects?after=0` | 50 content-free effect receipts/page with cursor and `has_more` |
| POST `/v1/memory/query` | Existing explicit-scope management query; Channel scope uses `{kind:"channel",channel_id:...}` |

`memory-grants` is a bounded history window, not a full audit export. Revision
conflict means re-inspect the current decision; it does not mean retry with a
guessed revision. Expiry does not delete existing memory or create a new revision.
Grant revocation is independent of reply permission. It stops private recall/new
effects; ordinary conversation can continue. Changing a grant cannot recall an
already submitted provider request or undo an already committed memory effect.

## Staged manual acceptance (Chinese operating guide)

以下步骤供用户另行决定升级、授权真实模型/QQ 实测后手动执行，本次没有执行。
普通聊天截图不算 Q2-A 验收。每条真实消息可能在现有预算内触发最多四次模型请求。

### 1. 停机备份、升级并检查实例

已有个人实例目录是 **`F:\Lain-Q1-Acceptance`**，已有启动脚本是
`F:\Lain-Q1-Acceptance\start-qq-test.ps1`。不要再用之前不存在的
AppData 启动脚本，也不要在现有目录上重新运行首次 setup。

目前正在运行的旧二进制没有本次新接口。新 CLI 访问旧服务可能收到 404；这表示
服务尚未升级，不要据此反复改 token、重新绑定或新建数据库。

如要升级：先正常停止原实例，确认数据库释放；保留一份**一致的停机备份**与旧
二进制，备份同样只放在受保护的仓库外位置。新版本会将所选数据库前向迁移到
schema 10（Q2-A 原版为 9）；旧程序不能打开新 schema，回退需要恢复原来一致的备份，不能只换回 exe。
只测试新能力也可以选择全新的独立测试目录、数据库和端口；不得两套服务争用同一
数据库或让两套 QQ 消费者同时回复。当前没有自动部署/升级脚本。

用同一次独立构建的 `lain.exe` 与 `laind.exe`。手动升级现有启动脚本时检查其最终
启动的 exe 路径；若它固定引用 `target/debug`，仅改调用端 CLI 不会升级服务。
使用 `lain up` 时可显式设 `LAIN_DAEMON_PATH` 为同目录的新 `laind.exe`，并沿用
已有私有 Actor/NapCat 配置和凭据加载流程。不要把凭据复制进仓库或输出到终端。

只读检查连接（不调用模型）：

升级并启动正确实例后，在第二个 PowerShell 选择同一外部实例目录：

```powershell
$lain = 'F:\Lain-Agent-Interface-Network\target\q2a\debug\lain.exe'
$env:LAIN_LOCAL_DIR = 'F:\Lain-Q1-Acceptance'
# 如本窗口有旧的 LAIN_URL/LAIN_API_TOKEN 手工覆盖，先移除覆盖再使用保存的实例配置。
& $lain --json channel list
```

从列表复制目标绑定的 id（不是 QQ 号码）：

```powershell
$channelId = '<从列表复制的 ChannelId>'
& $lain --json channel show $channelId
& $lain --json channel memory-grant $channelId
```

预期连接 Online，身份为配置中的 Lain 账号；首次迁移的记忆授权 disabled、revision 0。
只看到 Kernel Ready 或数据库里有 binding 不代表 QQ 在线。状态失败分类与排查方法见
[QQ_CHANNEL.md](QQ_CHANNEL.md)。在没有待发回复时，手动停用/恢复 NapCat WS，观察
Reconnecting → Authenticating → Online；不要为了测试制造真实 Unknown 发送。
如本轮只验诊断，到这里即可，保持记忆授权关闭。

### 2. 显式启用一个短期记忆授权

这一步是管理员授权，不通过 QQ 文本授权。可用未来 30 分钟的测试期限：

```powershell
$grant = & $lain --json channel memory-grant $channelId | ConvertFrom-Json
$deadline = [DateTimeOffset]::UtcNow.AddMinutes(30).ToUnixTimeMilliseconds()
& $lain --json channel memory-enable $channelId --revision $grant.revision --expires-at-ms $deadline
& $lain --json channel memory-grants $channelId
```

启用之前的消息不会被回填。之后在 QQ 发一条无敏感信息的明确偏好，例如
“我平时喜欢喝柠檬茶，这个可以记下来”。等这一轮真正结束后查：

```powershell
& $lain --json memory search '柠檬茶' --channel $channelId
& $lain --json channel memory-history $channelId
```

必须出现实际 Active MemoryId，来源 ChannelSelection、正确的消息/会话/轮次/action
与 grant revision，authority 为 inferred，`owner_confirmed_at_ms` 为 null。
仅回复“记住了”没有记录/回执时不能判成功。模型可以不选择无价值信息；若没有保存，
检查 Actor action/错误/预算和授权，不要把每句话全量写入作为修复办法。

### 3. 回忆与重启

记录上一步 MemoryId，正常停止/重启同一实例，再执行 `memory show <id>` 确认记录
持久存在。通过 QQ 问相关偏好，在 `channel show` 的 batch 找 ActorTurnId，再用
`lain --json actor show <turn-id>` 检查 `memory.recall` 的真实 MemoryId 回执。
立即追问仍可能只利用近期对话，不能仅凭答对来宣称独立记忆验收成功。

更严格的真实回忆验证可以等正常聊天推进，确认原话退出最近 20 个 turn/上下文字节
窗口后再问；不必自动刷 22 条付费消息。确定性测试已用零模型的 filler turn 验证
这一边界。真实模型是否主动检索、选取合适关键词和自然表达仍需实际观察。

### 4. 明确纠正与忘记

先发“关于喝的偏好我说错了：以前的柠檬茶改成白水，修改那一条”。检查旧 MemoryId
为 Superseded，`superseded_by` 指向新 MemoryId，新记录内容和来源正确；Active 检索
不再返回旧认识。若目标含糊，应该先澄清，不能批量改写。

再发“忘记刚才那条白水偏好”。检查 forget effect 与目标 MemoryId，Active 检索
不再返回该记录，直接 show 原目标返回不存在。再问时不能把旧记录当有效语义记忆。
已有对话仍可能提过这个词，旧 superseded 版本和审计也可能保留文本；这是语义删除，
不代表删除聊天、审计、备份、模型已收到的请求或安全擦除磁盘。

### 5. 撤销与过期

```powershell
$grant = & $lain --json channel memory-grant $channelId | ConvertFrom-Json
& $lain --json channel memory-revoke $channelId --revision $grant.revision
& $lain --json channel memory-grant $channelId
& $lain --json channel memory-grants $channelId
```

撤销后普通私聊回复可继续，新的自动记忆 effect 不应出现。普通记忆提议仍会等待原有
逐项审核；QQ“同意”不能批准它。未决 proposal 可在终端查看并显式拒绝/取消，避免
误以为聊天故障。若选择验过期，用更短的未来期限重新显式授权，过期后效果与撤销
相同；当前 enabled 标记可能仍为 true，需同时检查 expires_at_ms 与当前时间。

每步只记录安全 ID、状态、时间与结果，不将凭据或个人聊天拷进 Git。到 Q2-A 实测
结束就停止，不据此开启观察主号、群聊、Worker 档案、主动联系或 Q3。

## Known limits

Literal substring/keyword recall has no embedding model or linguistic guarantee;
poor keywords may miss a record. Source quotes prove provenance, not semantic truth
or user intent. Ambiguity handling is instructed model behavior with typed
single-target/revision/scope enforcement, not a new natural-language proof system.
There is no bulk tool, source-archive purge, observer, derived-profile subsystem,
cross-contact disclosure, scheduler, background extraction or deployment here.
Real memory/provider behavior and native macOS/Linux remain unverified.
