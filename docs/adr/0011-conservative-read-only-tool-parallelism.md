# ADR-0011：保守的只读工具并行调度

- 状态：Accepted
- 日期：2026-08-27
- 决策者：KiloxGo

## 背景

一个 assistant step 可以返回多个互相独立的工具调用。逐个执行只读文件、记忆与实例查询会叠加延迟，但并行写入、命令或审批会破坏顺序、授权和审计假设。

## 决策

1. 只有 `time_now`、`instance_status`、`memory_search`、`memory_get`、`list_files`、`read_file` 和 `search_text` 显式允许并行；未知工具默认串行。
2. 只有包含至少两个调用且全部安全的批次才使用现有 `join_all` 并行执行。任何写入、检查或未知调用都会让整个批次保持模型顺序串行执行。
3. 所有 `tool.started` 在执行前按模型顺序持久化；并行结果等待全部完成后，仍按模型顺序持久化 `tool.completed` 并送回 provider。
4. 现有单 run 最多 8 次调用的上限同时充当并行上限，不增加配置或调度依赖。
5. 暂不实现混合批次的连续并行组和 exclusive barrier；真实 trace 显示该限制造成显著延迟时再扩展。

该设计参考 DeepSeek Harness 的显式并发安全声明、exclusive barrier 和结果有序提交原则，但采用适合当前固定工具集的最小批次策略。

## 备选方案

- 所有调用并行：会让副作用和审批竞态，拒绝。
- 按名称以外的“看起来只读”启发式分类：新增工具可能被意外放行，拒绝。
- 立即实现通用 registry、滚动池和动态 classifier：当前最多 8 次固定工具调用，不需要。

## 安全、迁移与回滚

没有 schema 或数据库迁移。并行执行不改变权限、输出标记和调用上限；回滚为删除并行分支，所有调用恢复串行。

## 验证

- 单元测试覆盖单调用、全安全批次、混合读写批次和未知工具 fail closed。
- 在线烟测记录 `tool.started`/`tool.completed` 顺序，确认两个只读内置工具同时进入执行批次且结果按原顺序提交。

## 官方参考

- [DeepSeek Harness parallel tool-call execution](https://github.com/deepseek-ai/deepseek-harness/blob/master/.agents/notes/implemented/feature/2026-07-10-parallel-tool-call-execution.md)
