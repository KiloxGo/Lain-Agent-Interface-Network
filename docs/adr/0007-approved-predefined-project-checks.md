# ADR 0007：审批后运行预定义项目检查

- 状态：Accepted
- 日期：2026-08-26
- 维护者：KiloxGo

## 背景

精确编辑完成后，agent 必须取得真实的编译、测试或 lint 反馈，才能形成“修改—验证—修正”闭环。当前 kernel 尚无 OS 级进程沙箱；直接暴露 `bash -c`、PowerShell 或任意 executable/arguments 会把本机完整权限交给模型，不适合作为最小版本。

现代 harness 的共同边界是：命令、cwd 和权限必须在执行前可见；授权只绑定本次动作；超时、取消、输出上限和环境变量属于 executor 而不是 prompt 约定；没有 sandbox 时应缩小工具能力，而不是把审批等同于隔离。

## 决策

1. 增加 `run_check(check, path?)`，只接受八个闭集预设：`cargo_check`、`cargo_test`、`cargo_clippy`、`cargo_fmt_check`、`node_test`、`npm_test`、`npm_lint`、`npm_build`。
2. 模型不能提供 executable、自由参数或 shell 字符串。`path` 必须解析为 workspace 内现有目录；Cargo/npm 预设要求对应 manifest，npm 预设还要求 script 已声明。
3. 每次执行前，CLI 展示解析后的 command、cwd 和“当前无 OS sandbox”提示，只接受一次性 `y`/`yes`；审计沿用 asked/decided 与 action hash。
4. 子进程 stdin 关闭、stdout/stderr 分别持续排空，合并结果最多保留 64 KiB；结果总是带 `[exit code: N]`，非零退出作为可供模型修正的正常工具结果。
5. 默认超时 300 秒；run 取消会终止当前直接子进程。执行前再次解析 cwd，防止审批期间路径重定向。
6. 子进程继承正常构建环境，但移除常见 API key、token、secret、password 和 credential 环境变量，并关闭 Git 交互提示及 npm audit/fund/progress。
7. 不提供网络权限切换、后台任务、持久 shell 状态、自由 workdir 或永久授权。

## 已知边界

项目自己的 Cargo build script、npm script 或 Node test 本质上仍是本地代码，可能访问 workspace 外文件或网络；因此每次都审批并明确提示未沙箱化。v0 超时只保证终止直接子进程，不承诺跨平台进程树完全回收。

在开放任意命令、后台任务或自动批准前，必须先实现并验证 Windows Job Object 与 Unix process group、workspace-write 文件系统沙箱、网络默认拒绝和进程树测试。

## 官方参考

- [DeepSeek Harness bash tool](https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/shell/tool-bash/README.md)
- [DeepSeek Harness shell subsystem](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/subsystems/shell.md)
- [DeepSeek Harness approval subsystem](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/subsystems/approval.md)
- [Codex command approval policy](https://github.com/openai/codex/blob/main/codex-rs/prompts/templates/permissions/approval_policy/on_request_rule_request_permission.md)
- [Codex app-server approvals](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md#approvals)

