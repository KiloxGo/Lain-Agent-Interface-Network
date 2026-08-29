# P6 宿主 Shell 与全局 CLI 验收报告

- 日期：2026-08-29
- 分支：`feat/cli-mvp`
- 维护者：KiloxGo
- 结论：一次性审批的宿主 shell 与全局 `lain` 命令通过本地及真实 provider 验收。

## 已验收范围

- `shell(command, path?)` 接受最多 4096 bytes 的非空单行命令；cwd 必须是 workspace 内现有目录。
- shell 固定为 `external_side_effect`，不能并行；执行前展示精确 command、cwd 和无 OS sandbox 警告，审批绑定两者的 action hash。
- 复用现有 stdin 关闭、敏感环境清理、64 KiB 输出上限、300 秒超时、取消和 untrusted output 标记。
- Windows 使用 Job Object，Unix 使用独立 process group；shell 主进程退出、超时或取消后均回收仍存活的后代，不提供后台任务或持久 shell。
- 根目录 `package.json` 使用标准 npm `bin` 暴露 `lain`；`npm link` 后已从仓库外目录成功执行 `lain --help`。CLI 通过 realpath 判断入口，兼容 Windows npm junction。
- 未显式设置 `LAIN_WS_URL` 时，CLI 启动自己拥有的 workspace kernel；避免误复用另一个项目的默认端口进程。
- 自动启动的 kernel 以规范化 workspace 路径的 SHA-256 标题选择稳定 session；不同项目不共享消息上下文，但继续共享 instance/user 人格与记忆，数据库不保存原始路径。

## 自动化结果

- Rust tests：98 passed，5 suites。
- Clippy：`--all-targets --all-features --locked -- -D warnings` 通过。
- rustfmt：`cargo fmt --all -- --check` 通过。
- CLI：3 passed；根包 `npm test` 通过。
- eval：persona 50 + memory 20 + coding 11，共 81 cases；tool suite 11/11，6 个需审批、5 个无需审批。
- `node --check evals/run-coding-task.mjs` 通过。

## 真实 DeepSeek 验收

- `coding-shell-command-001`：在终态 SelfState 更新接入后复跑，只调用 `shell`；审批精确绑定 `echo SHELL_OK` 与 `.`；exit 0；最终回复 `SHELL_OK`。usage：4919 input / 57 output / 3328 cached input tokens。
- `coding-list-files-001`：只调用 `list_files`，无审批；最终回复 `LIST_OK`。usage：4688 input / 50 output / 4352 cached input tokens。
- `coding-work-loop-001`：自动采用根级 `AGENTS.md`，读取陌生 Node 项目、只修改 `math.js`、审批后运行 `npm test` 并通过，最终报告包含规则要求的 `AGENTS_OK`、变更文件与检查名。usage：14597 input / 404 output / 11520 cached input tokens。

结果保存在 `evals/results/p6/`，均只使用合成数据。

## 仍然明确不包含

- OS 文件系统或网络 sandbox。获批命令拥有宿主账户权限，因此用户必须拒绝不理解的下载、凭据、删除或系统级命令。
- PTY、交互程序、后台任务、持久 cwd/env、会话级永久授权和命令 allowlist。
- 数据目录级完全隔离；需要独立人格和记忆库时仍应显式设置 `LAIN_DATA_DIR`。
