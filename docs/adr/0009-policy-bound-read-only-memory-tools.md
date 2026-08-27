# ADR-0009：受检索策略约束的只读记忆工具

- 状态：Accepted
- 日期：2026-08-27
- 决策者：KiloxGo

## 背景

Context Compiler 会自动注入与当前消息相关的记忆，但任务型对话还需要让模型主动检索和核验长期记忆。直接暴露记忆表或复用面向用户的管理 API，会绕过 session scope、状态、时间和 `never_recall` 策略。

## 决策

1. 在配置 workspace 的 chat mode 提供只读 `memory_search(query, limit?)` 和 `memory_get(id)`，不提供模型侧记忆写入工具。
2. `memory_search` 复用现有 `retrieve_memory_evidence`，继承 user/session scope、FTS、时间版本、状态和 `never_recall` 过滤；limit 最大为 10。
3. `memory_get` 不信任模型给出的 ID，每次重新校验 user、session scope、状态和 `never_recall`。
4. 两个工具沿用统一 invocation 审计；客户端事件不包含参数或正文。返回模型的内容统一标为 untrusted evidence。
5. 第一版继续使用现有 SQLite FTS，不新增向量索引、ToolRegistry 或依赖；需要 headless 主动记忆工具时再拆分独立 mode 开关。

## 备选方案

- 向模型暴露完整记忆列表或管理 API：权限面过宽，拒绝。
- 让模型生成 SQL：无法稳定约束 scope 和隐私策略，拒绝。
- 等待向量检索后再提供工具：FTS 已足以支撑最小可用版本，拒绝等待。

## 安全与隐私

- `never_recall`、retracted、disputed 和跨 session 记录不可由 ID 绕过。
- 工具无副作用、无需审批；结果仍可能包含指令式文本，因此不得提升信任级别。
- 数据库错误只返回固定的 backend unavailable，不向模型泄露路径或 SQL。

## 迁移与回滚

无 schema 迁移。回滚时移除两个 tool definition 与 dispatch；现有记忆和索引不受影响。

## 验证

- Storage 测试覆盖可召回记录与 `never_recall` 拒绝。
- Tool 测试覆盖空检索、未知 ID fail closed 和 schema 名称。
- DeepSeek 烟测覆盖真实 `memory_search` tool call、空结果和最终回答。
