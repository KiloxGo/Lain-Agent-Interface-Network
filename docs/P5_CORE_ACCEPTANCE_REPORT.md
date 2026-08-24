# P5 Memory v1 Core 验收报告

- 日期：2026-08-20
- 分支：`feat/p5-memory-v1`
- 维护者：KiloxGo
- 结论：Core checkpoint 通过；P5 全阶段尚未完成

## 已验收范围

- forward-only `0003_memory_v1.sql`：candidates/items/links、active key 唯一约束和 FTS5 trigram projection；
- 回复完成事务原子入队 `memory_extract`；
- 单 consumer lease、lease 排他、失败退避、重试和 dead letter；
- 独立、版本化 JSON extractor prompt；
- server-bound source message/event；evidence quote 原文验证；
- instruction-like 拒绝、敏感/推断/低置信度 pending、显式低敏感 auto-commit；
- correction 生成 item version 与 supersedes/source links；
- active/validity/session/never-recall 过滤，FTS reciprocal rank 和 bounded boosts；
- Context Compiler 以 untrusted evidence 注入 item/version/source event；
- pending confirm/reject、pin、never-recall、dispute/retract 的本地 API 和前端面板。

## 自动化结果

| 门禁 | 结果 |
| --- | --- |
| Rust tests | 78 passed，4 suites |
| Clippy | `--all-targets --all-features -- -D warnings` 通过 |
| rustfmt | `cargo fmt --all -- --check` 通过 |
| Frontend lint | 通过 |
| Frontend production build | 通过，主 JS 200.83 kB / gzip 63.36 kB |
| Eval fixture validation | persona 50 + memory 20，合计 70 cases 通过 schema/unique 校验 |
| Dependency policy | npm production audit 0 漏洞；Rust bans/licenses/sources 通过 |
| Memory retrieval latency | 250 items、100 queries，本机 p95 2.3875 ms；门槛 < 50 ms |
| False-personalization smoke | 无关 lexical query 返回空；superseded/expired/never-recall 不注入 |

## 真实 DeepSeek smoke

使用合成输入，不包含真实会话或用户资料。累计项目调用预算使用量由 4 增至 6，剩余 44/50。

第一次调用得到合法 JSON，但缺少 `subject`，因此验收失败。根因是 extractor prompt 给出了枚举值域，却没有规定每个 candidate 的完整对象形状。随后将全字段 JSON shape、不得漏字段/增字段写入版本化 prompt。

第二次调用通过：

- model：`deepseek-v4-flash`
- candidates：1
- 输出：grounded `dark mode` preference；未保存注入语句或 credentials 文本
- prompt tokens：449
- completion tokens：76
- total tokens：525
- total latency：1429.32 ms
- 记录：`evals/results/p5/memory-extractor-smoke.json`

该失败—修正—复验记录证明真实 provider 契约不是由 mock 推断出来的。worker 仍会在 Rust 侧再次执行 deny-unknown-fields、原文证据、分数、日期、敏感和注入校验。

## 故障与降级验收

- worker 未启用：主对话正常，job 只保留在队列；
- provider/JSON 失败：主回复已提交，job 独立重试；
- FTS 查询失败：记录 optional context degraded，使用空 memory evidence 继续 run；
- pending/secret：确认前没有 canonical item，不能进入检索；
- correction：旧 active item 和旧 FTS row 在同一事务内 supersede/remove；
- retract/dispute/never-recall：立即退出检索，历史和 provenance 保留。

## 尚未完成，不作为本报告通过项

- temporal query expansion；
- alias/topic canonicalization、consolidation/reflection；
- `VectorIndex`/EmbeddingProvider 和 sqlite-vec feature gate 评估；
- 旧 profile/session/topic Markdown 到结构化 memory 的专门导入；
- memory value 编辑（当前支持确认、状态和召回控制）；
- LongMemEval 小规模适配、跨语言语义检索和至少 100 case 的最终 P9 memory suite；
- 备份加密、完整删除和 restore 语义（P8）。

因此当前可以合并/部署为默认关闭的 Memory v1 core，但不能宣称 P5 全阶段完成。

首次远端 CI 的 Windows/Linux Rust、frontend/eval 和 bans/licenses/sources job 均通过；advisory job 因运行期间新收录的 `RUSTSEC-2026-0258` 失败。其影响 `h2 0.4.13`，已将 lockfile 精确升级至修复版 `0.4.16` 并触发复验。
