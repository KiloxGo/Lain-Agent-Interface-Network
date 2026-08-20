# ADR-0005：有来源、可确认、可重建检索的 Memory v1

- 状态：Accepted
- 日期：2026-08-20
- 维护者：KiloxGo

## 背景

旧链路允许模型在主回复中生成 `<memory>`/`<topic>` 标签，再直接修改 Markdown profile。该方式无法可靠区分用户事实与模型猜测，缺少来源、版本、敏感度和撤回语义，也使人格提示词逐渐膨胀。P4 已提供带 trust/budget/provenance 接口的 Context Compiler，P5 需要把个性化从自由文本副作用迁移为独立、可审计的能力。

DeepSeek Harness 的官方架构将持久事件视为上下文真相源，要求模型可见内容可由日志重建，并将 SQLite 搜索视为可丢弃投影。本 ADR 吸收这些语义，但不引入其插件运行时。详细对照见 `docs/research/DEEPSEEK_HARNESS_REFERENCE.md`。

## 决策

### Canonical data

- `memory_candidates` 保存 extractor 的原始结构化草案、extractor version、来源 run/message、证据原文、策略结果和原因。
- `memory_items` 保存已确认事实。item 有 user/session/relationship scope、type、sensitivity、validity、status、version、supersedes、never_recall 和 pinned。
- `memory_links` 保存 source event/message 与 memory-to-memory 关系。
- `memory_items_fts` 是 FTS5 trigram 投影，不是事实源；检索必须 join 回 active canonical item。
- 同一 user/scope/session/canonical key 同时最多一个 active item。修正先将旧 item 标为 superseded，再原子写入新版本和 provenance。

### Extraction boundary

主回复完成事务只入队 `memory_extract`，不等待第二次模型调用。单并发 worker 使用 lease 消费；lease 过期可回收，失败指数退避，超过最大次数进入 dead letter。worker 默认关闭，显式设置 `LAIN_MEMORY_EXTRACTION_ENABLED=true` 后才会产生额外调用。

Extractor 使用 maintenance/JSON output 路由和版本化的独立 prompt。模型只提出 candidates，不拥有写权限。服务器端从 run 绑定 source message/event，忽略模型自报来源，并限制每条消息的候选数量。

### Deterministic policy

Rust policy 在事务前执行：

- evidence quote 必须是 user message 的连续原文子串；
- required fields 非空、字段长度与分数范围受限；
- `valid_from`/`valid_to` 必须是 RFC3339 且范围有效；
- instruction-like、system prompt 或越权指令内容直接 reject；
- sensitive/secret、inferred、低置信度、relationship candidate 进入 pending confirmation；
- 仅 explicit、grounded、低敏感、置信度至少 0.85 的 user/session candidate 自动 commit。

任何模型输出都不能直接修改 profile、identity、权限或关系状态。

### Retrieval boundary

检索先做 user/session scope、active、validity 和 never_recall 过滤，再用 FTS5 trigram 召回，按 reciprocal rank、有限 importance/pinned/session boost 排序。敏感候选只有经过用户确认才会成为 item；确认后的 item 仍可随时设置 never_recall 或撤回。没有有效查询词时返回空；不因“最近写入”而无条件注入。FTS 或可选状态读取失败时记录 degraded source 并继续普通对话。

Context Compiler 对 evidence 去重、排序并按 memory token budget 截断。渲染项携带 item ID、version、source event IDs，并置于 `Untrusted Memory Evidence`，明确声明它不是指令。最终选择和 drop reason 持久化到 ContextPlan。

## 安全与隐私

- extractor 只接收当前 user message，不发送完整 profile 或历史数据库；
- secret 不会自动进入 canonical items；sensitive candidate 在用户确认前不参与检索；
- 日志只记录 job/candidate/item ID、状态和错误码，不记录 API key；
- memory 管理 UI 未完成前，pending candidate 不会被主对话读取；
- FTS 数据库当前位于同一 SQLite 文件，后续加密/OS secret 和完整删除语义由 P8 完成。

## 备选方案

- 保留 hidden tags：成本低，但模型拥有隐式写权限且无法保证证据与版本，拒绝。
- 每轮同步抽取：结果即时，但增加首 token/完成延迟并让可选能力破坏主链路，拒绝。
- 只存 embedding：语义召回强，但索引不可解释且不能替代 canonical provenance，拒绝。
- 直接采用通用 agent memory framework：会隐藏 scope、事务、版本和 prompt 注入顺序，拒绝。

## 迁移与回滚

`0003_memory_v1.sql` 是 forward-only migration。回滚 binary 时保留新表；旧 binary 不读取它们。可以关闭 `LAIN_MEMORY_EXTRACTION_ENABLED` 立即停止额外 provider 调用，已排队 job 保留。FTS 投影可通过 repository API 重建，不删除 canonical items。旧 Markdown 记忆在专门导入器完成前不自动写入新表。

## 验证

- migration、foreign key、active unique key 与 FTS5 可用性；
- lease 排他、失败重试、过期回收、dead letter；
- JSON schema、候选上限与 source server-binding；
- fabricated evidence、instruction injection、敏感、推断、低置信度、时间范围；
- 重复 job 幂等、correction version、supersedes/source provenance；
- scope/expiry/never_recall、无关查询空结果、检索时延；
- ContextPlan 中的 item/version/source event 和 memory token cap；
- 启用开关后少量真实 DeepSeek JSON 调用，不超过已授权的 50 次总预算。
