# ADR 0006：一次性审批的精确 workspace 编辑

- 状态：Accepted
- 日期：2026-08-26
- 维护者：KiloxGo

## 背景

P6 只读工具已经能帮助模型理解项目，但不能完成最小代码修改。写入工具必须同时满足：用户看得见将要发生什么、授权只对应这一次具体动作、文件在等待确认期间发生变化时不覆盖、Windows workspace 不能因路径或后端差异越界。

参考现代 harness 的稳定原则，而不复制其框架：

- DeepSeek Harness 的 approval subsystem 将单次许可建模为 `allowed-once`，缺少审批器、断线或取消时 fail closed，并分别记录 asked/decided。
- DeepSeek Harness 的 editor 采用唯一字面量替换，并要求编辑前已读取文件。
- Gemini CLI 的 `replace` 以 path/old/new 描述精确修改，并在写入前要求人工确认。
- Codex `apply_patch` 只接受结构化文件操作与相对路径。
- DeepSeek Harness 的 Windows workspace escape 讨论说明：UI 预览边界和实际文件系统执行边界必须重复校验，不能只信任上层策略结果。

## 决策

1. v0 只增加 `replace_text(path, old_text, new_text)`；只编辑 workspace 内现有 UTF-8 文件，`old_text` 必须非空且恰好出现一次。
2. 沿用只读工具的 canonical workspace、绝对路径/`..`/越界 symlink/Secret 排除和 1 MiB 文件上限。
3. prepare 阶段不写盘，只生成最多 32 KiB 的 diff、原文件 SHA-256 与绑定 tool/path/arguments/original hash 的 action hash。
4. SQLite 持久化 `tool.approval_requested` 与 `tool.approval_decided`；审计记录 action hash 和决定，不持久化文件正文或 diff。
5. CLI 展示 path/diff 后只接受一次 `y`/`yes`。默认答案、非 TTY、审批回调异常、连接断开、取消或 5 分钟超时均拒绝写入。
6. 执行前再次解析 workspace 路径并比较原文件 SHA-256；不一致返回 `file_changed`。
7. 写入同目录临时文件、flush/sync、保留权限后原子替换；Windows 使用 `MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)`。
8. 通用工具执行入口仍拒绝 `replace_text`，避免任何调用方绕过审批流程。

## 暂不采用

- 通用 patch、create/delete、shell、MCP 和“本次会话始终允许”。
- 持久化 diff/文件正文或增加独立 policy framework。
- 并行副作用工具。

这些能力只在精确替换不能覆盖真实任务，且已有相应沙箱、回滚和评测证据时增加。

后续变更：单文件创建已由 [ADR 0008](0008-approved-no-clobber-file-creation.md) 以独立 no-clobber 语义接受；覆盖、删除、通用 patch 与 shell 仍不在本决策范围内。

## 官方参考

- [DeepSeek Harness approval subsystem](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/subsystems/approval.md)
- [DeepSeek Harness tool catalog](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/tool-catalog.md)
- [Gemini CLI tools reference](https://github.com/google-gemini/gemini-cli/blob/main/docs/reference/tools.md)
- [Codex apply_patch instructions](https://github.com/openai/codex/blob/main/codex-rs/core/prompt_with_apply_patch_instructions.md)
- [DeepSeek Harness Windows workspace escape discussion](https://github.com/deepseek-ai/deepseek-harness/discussions/523)
