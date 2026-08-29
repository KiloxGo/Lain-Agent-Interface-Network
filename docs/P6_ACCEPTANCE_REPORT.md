# P6 CLI/MCP v0 验收报告

> 这是引入宿主 shell 前的 checkpoint；后续扩展与当前边界见 [`P6_SHELL_ACCEPTANCE_REPORT.md`](P6_SHELL_ACCEPTANCE_REPORT.md)。

- 日期：2026-08-29
- 分支：`feat/cli-mvp`
- 维护者：KiloxGo
- 结论：CLI/MCP v0 checkpoint 通过；P6 通用框架与前端项目仍明确延期，不宣称整个 P6 完成。

## 已验收范围

- 模型工具循环最多 4 轮、8 次调用；安全只读批次可并行，混合或有副作用批次串行。
- 内置记忆、时间、实例状态、workspace 读取/列举/搜索工具，以及精确编辑、no-clobber 创建和预定义项目检查。
- 所有文件写入、检查和 MCP 调用使用一次性审批；审批绑定精确 action hash，超时、断线、拒绝和 hash 变化均 fail closed。
- MCP stdio 支持当前 discovery 和旧 initialize fallback；工具名带 server namespace，只允许调用已发现工具。
- MCP metadata/output 一律视为不可信，有目录、描述和输出上限；MCP 不可用不会阻断普通聊天。
- SQLite 保存 tool invocation、approval 和风险分类；`ToolRisk` 已在 Rust 域模型中类型化，wire/数据库值仍稳定为 `read_only`、`workspace_write`、`external_side_effect`。
- balanced tool eval 共 8 类：失败恢复、创建、精确编辑、间接注入、记忆、运行时信息、no-tool 负向选择和 MCP echo；4 类需要审批，4 类不需要审批。

## 安全边界

- 不开放任意 shell、自由 executable 或任意参数执行。
- workspace 路径、secret 环境变量、输出大小和 no-clobber 写入均在 host 侧约束。
- 模型不能根据 MCP annotations 降低风险；全部 MCP 调用按 `external_side_effect` 审批。
- 审批事件不保存文件正文或 diff；工具输出带不可信来源标记并接受 canary 泄漏检查。

## 验收门禁

最终本地收尾运行以下门禁：

- `cargo test --all-targets --all-features --locked`：94 passed，4 suites。
- `cargo clippy --all-targets --all-features --locked -- -D warnings`：通过。
- `cargo fmt --all -- --check`：通过。
- `node --test cli/lain.test.mjs`：3 passed。
- `node evals/validate.mjs`：persona 50 + memory 20 + coding 8；tool suite 8/8，其中 4 个需审批、4 个无需审批。
- `node --check evals/run-coding-task.mjs`：通过。

P6-020 的 8 个 balanced case 均有在线基线；最后补充的创建、编辑、no-tool 和 MCP echo 合成任务均通过。provider usage 保存在对应结果文件中。本次仅收紧 Rust 风险类型且不改变 provider 协议，因此没有额外消耗 DeepSeek 调用。

## 明确延期，不计入本次通过

- 单实现 `Tool` trait：出现第二种本地执行器或插件实现时再抽象。
- 通用 `ToolPolicyDecision`：当前一次性审批就是唯一策略，不建立第二套策略框架。
- `ToolRegistry` 和版本冲突检测：出现第二 MCP server、插件或动态重载需求时再实现。
- MCP 通用 JSON Schema 校验、可重试远程副作用工具的 idempotency key、通用内容 redaction。
- 前端 approval UI：按 terminal-first 决策延期。

因此当前可以把终端工具链作为最小可运行、可审计的 agent harness checkpoint 继续使用；下一阶段应优先推进真实任务能力或 P7 人格状态，而不是提前建设尚无消费者的通用工具框架。
