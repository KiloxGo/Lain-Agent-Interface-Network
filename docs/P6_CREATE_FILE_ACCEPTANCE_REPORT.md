# P6 审批式文件创建 v0 验收

- 日期：2026-08-26
- 范围：workspace 内现有父目录中的单文件创建
- 工具：`create_file`

## 已验收行为

- schema 只接受 `path` 和 `content`；内容最大 1 MiB。
- prepare 只生成 new-file diff，不创建目标或临时文件。
- 统一 workspace policy 拒绝绝对路径、`..`、越界 symlink、Secret/构建目录；Windows 拒绝 ADS 与保留文件名。
- CLI 展示 `/dev/null → path` diff，并取得一次性 `y/N`。
- 执行前重新解析 parent；目标已存在、dangling symlink 或审批期间出现的竞态目标都会返回 `path_already_exists`。
- 同目录临时文件先完成 write/sync，再以 hard link 原子 no-clobber 发布；不覆盖任何已有内容。
- action hash、approval requested/decided 和 tool invocation 进入 SQLite；通用 executor 无法绕过审批。

## 自动检查

- `cargo test --all-targets --all-features`：84 passed。
- `node --test cli/lain.test.mjs`：2 passed。
- 覆盖审批前不落盘、批准后内容一致、并发目标保留和统一审批入口。

## 真实 DeepSeek 验收

模型按顺序完成：

1. `create_file` 创建 `frontend/create-file-acceptance.test.js`，内容是一条 Node test。
2. `run_check(node_test, frontend)` 执行 `node --test`。

CLI 分别展示 new-file diff 和 command/cwd，并取得两次独立授权。Node 自动发现新测试并返回 exit 0，模型最终回复 `CREATE_CHECK_OK`。

汇总 usage：9,506 input / 181 output / 7,808 cached input（82%），共 2 次 provider 请求。验收文件与 `.lain-create-*` 临时文件已删除，自有 kernel 已退出；授权预算累计约 21/50。

## 暂不包含

- 父目录创建、覆盖、删除、移动、批量 patch 或跨文件事务。
- 不支持 hard link 文件系统的非原子 fallback。

