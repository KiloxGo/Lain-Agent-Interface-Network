# ADR-0013：一次性审批的宿主 shell

- 状态：Accepted
- 日期：2026-08-29
- 决策者：KiloxGo

## 背景

八种预定义项目检查能验证常见 Cargo、Node 和 npm 项目，但不足以完成陌生仓库中的真实任务。开放 shell 会让模型构造可访问网络和 workspace 外文件的宿主命令，因此不能沿用只读或普通 workspace-write 风险，也不能只终止直接子进程。

## 决策

1. 增加 `shell(command, path?)`。`command` 必须是非空单行、最多 4096 bytes 且不含控制字符；`path` 必须解析为 workspace 内现有目录。
2. 每次执行均标记为 `external_side_effect`。CLI 展示精确 command、cwd 和“宿主权限、网络可用、无 OS sandbox”警告；一次性审批绑定 command 与 cwd 的 action hash。
3. Windows 优先使用 PATH 中的 PowerShell 7 `pwsh.exe`，否则回退系统 `powershell.exe`；Unix 使用 `/bin/sh -lc`。Windows 子进程显式使用 UTF-8 stdout，工具 schema 会把实际 PowerShell 语义告知模型。stdin 关闭，常见 provider、云、GitHub、token、password 和 credential 环境变量被移除。
4. stdout/stderr 持续排空，返回内容最多 64 KiB；超时保持 300 秒，并响应 run cancellation。
5. Windows 子进程进入带 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 的 Job Object；Unix 子进程进入独立 process group。超时、取消以及 shell 主进程正常退出时都会回收仍存活的后代，当前不支持后台任务或持久 shell。
6. shell 输出继续作为 untrusted tool output 返回模型；拒绝、断线、超时和 action hash 不匹配均 fail closed 并进入现有 SQLite 审计链。

## 备选方案

- 继续只提供预定义检查：边界最小，但无法覆盖 Python、Go、Git 和项目自定义命令。
- 等待完整 OS sandbox：更强，但会阻断当前终端 harness 的真实任务验证；本决策明确显示宿主权限且逐次审批，不伪装为 sandbox。
- 只接受 executable/argv：少一层 shell 解析，但不能覆盖管道、条件执行和项目文档中的原生命令。

## 影响

- Lain 可以在明确授权后运行真实项目命令，并将失败输出送回模型形成修复闭环。
- 每条命令都需要人工确认；暂不提供 allowlist、会话级永久授权、PTY、后台任务或持久 cwd/env。
- `shell` 比文件写入风险更高，不能并行，也不能根据模型或工具 metadata 降级风险。

## 安全与隐私

审批不是 OS sandbox。用户批准的命令仍可访问宿主账户可访问的文件与网络；CLI 必须完整显示命令，用户应拒绝不理解的下载、凭据、删除或系统级操作。provider 密钥不会进入子进程环境，但命令仍可能主动读取磁盘上的其他数据。

## 迁移与回滚

没有数据库或 wire schema 迁移。回滚时删除 `shell` definition、prepare 分支和 transport 分支即可；现有审计记录仍可读取。

## 验证方式

- 单元测试证明直接执行被拒绝、审批预览稳定、控制字符被拒绝，并通过真实 shell `echo` 与 Windows 中文输出验证有界执行和 UTF-8 解码。
- Rust 全量测试、Clippy、rustfmt、CLI 测试和 eval fixture validation 必须通过。
- Windows 手工验证全局 `lain` 命令可从仓库外目录启动。
