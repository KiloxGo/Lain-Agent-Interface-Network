# P6 受限检查闭环 v0 验收

- 日期：2026-08-26
- 范围：审批后运行预定义 Cargo/Node/npm 项目检查
- 工具：`run_check`

## 已验收行为

- schema 不接受 executable、自由参数或 shell，只接受八个固定检查和 workspace-relative cwd。
- Cargo/npm manifest 必须存在；npm script 必须在 `package.json` 中声明。
- CLI 在执行前显示解析后的 command、cwd 和未启用 OS sandbox 的警告，并单次 `y/N` 审批。
- action hash、approval requested/decided 和 tool invocation 进入 SQLite；命令输出不进入 approval 审计事件。
- stdin 关闭；常见 key/token/secret/password/credential 环境变量被移除；Git 不允许弹出凭据提示。
- stdout/stderr 持续排空但最终输出上限为 64 KiB；结果包含稳定的 exit-code marker。
- 300 秒超时和 run cancellation fail closed；cwd 在批准后重新解析。
- 通用 executor 仍拒绝 `run_check`，不能绕过审批。

## 自动检查

- `cargo test --all-targets --all-features`：83 passed。
- `node --test cli/lain.test.mjs`：2 passed。
- 覆盖预定义命令映射、审批前拒绝、真实 bounded child process、取消终止和敏感环境变量识别。

## 真实 DeepSeek 验收

合成任务要求严格依次执行：

1. `replace_text` 将 `check-loop-test.txt` 的 `check-before` 改为 `check-after`。
2. `run_check` 在 `lain-kernel` 运行 `cargo fmt --all -- --check`。

CLI 分别展示 diff 与 command/cwd，并取得两次独立 `y`。编辑成功，检查返回 exit 0，模型最终仅回复 `CHECK_LOOP_OK`。

汇总 usage：8,654 input / 153 output / 7,168 cached input（83%），共 2 次 provider 请求。合成文件已删除，未遗留 lain-kernel/cargo/rustc 进程；授权预算累计约 19/50。

## 暂不包含

- 任意 shell/命令参数、网络或文件系统 sandbox、后台任务、PTY、会话级永久授权。
- 跨平台进程树强制回收；这是开放通用命令前的硬前置条件。

