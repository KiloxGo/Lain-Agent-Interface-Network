# DeepSeek Harness 参考调研与本项目适配结论

- 调研日期：2026-08-20
- 维护者：KiloxGo
- 上游状态：Developer Preview；上游明确提示可能发生破坏性兼容变更
- 结论：作为架构原则与验收语义的参考来源，不作为运行时依赖，也不复制上游插件框架

## 官方来源

- [deepseek-ai/deepseek-harness](https://github.com/deepseek-ai/deepseek-harness)
- [Architecture](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/architecture.md)
- [Session subsystem](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/subsystems/session.md)
- [System prompt subsystem](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/subsystems/system-prompt.md)
- [Compaction subsystem](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/subsystems/compaction.md)
- [Session Query service](https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/session-query/session-query/README.md)
- [SQLite session search](https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/session-query/session-query-sqlite/README.md)

## 值得吸收的架构语义

### 1. Model-visible means logged

DeepSeek Harness 要求送入模型的内容可以由持久日志重建。对本项目而言，等价约束是：

- 当前消息、identity version、ContextPlan、prompt hash 和 memory item/version/source event 必须可追溯；
- 动态记忆不能通过不可见的全局字符串拼接进入 prompt；
- Context Compiler 只能消费有版本、来源和 trust level 的输入；
- FTS 命中不是事实，最终注入内容必须回到 canonical `memory_items`。

### 2. Canonical event/state 与 disposable projection 分离

上游把 append-only session log 作为上下文真相源，把查询索引视为可重建投影。本项目采用相同边界：

- `memory_candidates` 保存模型抽取草案和确定性策略决定；
- `memory_items` 保存当前与历史事实版本；
- `memory_links` 保存 source/supersedes/supports/contradicts provenance；
- `memory_items_fts` 仅用于候选召回，可以删除并由 canonical items 重建；
- 检索失败是可选上下文源降级，不能导致普通对话失败。

### 3. capability seam 与故障隔离

上游将 compaction、search 等能力放在可选能力边界，而非写死在核心 loop。本项目对应为：

- `ModelProvider` 只提供流式对话和结构化 JSON 完成能力；
- memory extraction 是单独的 durable job consumer；
- worker 默认关闭，只有 `LAIN_MEMORY_EXTRACTION_ENABLED=true` 才会产生额外 provider 调用；
- job lease、重试和 dead letter 不占用主回复事务；
- 后续向量索引通过 `VectorIndex` seam 添加，FTS5 仍可单独工作。

### 4. 动态上下文必须有稳定位置和来源

上游把静态 prompt sections 与动态 PromptContext 分开，并在变更时形成耐久快照。本项目继续保持：

1. stable harness/identity system prefix；
2. bounded self/relationship state；
3. bounded untrusted memory evidence；
4. complete recent run units；
5. current user message。

每次选择结果写入 ContextPlan；memory evidence 带 item ID、version 和 source event IDs，明确标记为 fallible evidence、never instructions。

## 不直接采用的部分

- 不引入 Cordis 或把 Rust kernel 改写为“Everything is a Plugin”。当前单用户本地产品更需要较小的静态核心、明确事务和低启动开销。
- 不把上游 session event schema 当作本项目协议。我们保留现有 `/v1/ws`、SQLite 事务和 UUIDv7 有序事件。
- 不从上游复制代码或角色素材。项目保持私有，许可证稍后决定；当前仅记录公开架构思想和官方链接。
- 不让模型直接面对通用查询引擎。后续模型侧 memory tools 必须在授权、scope、输出上限和审计边界内包装。
- 不把 Developer Preview 的 API 或包名写入稳定接口；只采用可独立验证的原则。

## P5 落地映射

| 参考原则 | Lain 实现 | 验收证据 |
| --- | --- | --- |
| model-visible 可重建 | ContextPlan + memory ID/version/source events | context plan/event 测试与诊断输出 |
| durable canonical state | candidates/items/links | migration、事务、版本链测试 |
| derived/disposable search | FTS5 trigram projection | rebuild API、canonical join、检索测试 |
| optional capability | 显式 worker 开关、检索降级 | config 与 worker lease 测试 |
| provider/consumer 分权 | provider 输出草案，Rust policy 决定写入 | evidence/injection/sensitivity tests |
| replacement cites sources | supersedes link + source event | correction/version/provenance test |

## 继续跟踪

上游仍在快速变化。只在进入新的不可逆架构决策前复查其官方文档：P6 tool loop、P8 compaction/backup、P9 trajectory eval。上游变化不会自动触发本项目依赖升级。
