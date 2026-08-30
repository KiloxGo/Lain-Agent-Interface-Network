# P6 工具上限与 Windows 恢复验收

- 日期：2026-08-30
- 分支：`feat/cli-mvp`
- 维护者：KiloxGo
- 结论：用户复现的 Windows shell 误判、空目录不可见、工具上限硬失败、下一轮上下文丢失和 CLI 乱码均已建立回归保护。

## 参考约束

- Claude Code 官方会话契约持续保存 message、tool use 和 result，并支持在达到 max turns 后恢复同一 session：<https://code.claude.com/docs/en/agent-sdk/sessions>
- Claude Code 在 Windows 原生选择 PowerShell 7，并回退 Windows PowerShell 5.1：<https://code.claude.com/docs/en/tools-reference>
- DeepSeek Harness 以 append-only session events 作为交互历史唯一真相源，逐 step 记录 assistant message、tool call 和 tool result：<https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/subsystems/session.md>
- OpenCode v2 在最后 step 不再物化工具并设置 `toolChoice: none`：<https://github.com/anomalyco/opencode/issues/45328>

## 修复范围

- 工具调用 step 在继续执行前写入 SQLite，并发布 `assistant.step_completed`；失败 run 的自然语言进度会进入下一轮 Context Compiler。
- 第 9 次 provider 请求清空工具 schema、设置 `ToolChoice::None`，要求模型只输出可继续的 handoff。
- `list_files` 返回目录项并以 `/` 结尾，空目录不再被报告为“工作区为空”。
- Windows shell 改为 PowerShell，优先 `pwsh.exe`、回退 `powershell.exe`，stdout 固定 UTF-8；`instance_status` 返回 platform 与 shell。
- CLI 修复 approval 标题乱码；终态 state 仍更新本地状态，但不再写入下一条 readline 输入行。

## 验证结果

- Rust：100 passed（5 suites）；Clippy `-D warnings` 与 rustfmt 通过。
- CLI：3 passed；前端 TypeScript/Vite production build 通过。
- 离线评测：persona 50 + memory 20 + coding 11，共 81 cases；历史工具基线 11/11 有效。
- 真实 DeepSeek `coding-shell-command-001`：只调用一次 PowerShell shell，审批精确匹配，exit 0，最终 `SHELL_OK`。
- 真实 DeepSeek `coding-work-loop-001`：`list_files`、并行读取、精确修改、`npm test` exit 0 与最终完成报告全部通过。

测试 workspace 均由 runner 在系统临时目录创建并回收；结果只写入系统临时目录，不覆盖仓库基线。
