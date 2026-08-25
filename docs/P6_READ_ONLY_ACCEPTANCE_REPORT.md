# P6 只读 Tool Loop v0 验收

- 日期：2026-08-25
- 范围：CLI workspace 的结构化只读工具，不含写文件、shell、审批或 MCP。
- 工具：`list_files`、`read_file`、`search_text`。

## 安全边界

- workspace 由 `LAIN_WORKSPACE` 固定；CLI 自动启动时默认为调用目录。
- 拒绝绝对路径、`..`、越界 symlink 和被排除路径。
- 默认排除 `.git`、`.lain`、`target`、`node_modules`、`.env`、常见 credential/private-key 文件。
- UTF-8 文本文件最大 1 MiB；单次输出最大 64 KiB；遍历与匹配数量有硬上限。
- 工具结果作为 untrusted data 回送模型；事件不包含参数或文件正文。
- tool loop 最多 4 轮、8 次调用；v0 顺序执行，不启用副作用工具。

## 自动验证

- Rust 全量测试：81 passed。
- CLI 协议测试：1 passed。
- 覆盖 workspace 越界、Secret 文件拒绝、列表/搜索结果、DeepSeek tool call ID 回送与 SQLite 审计脱敏。

## 真实 DeepSeek 验收

第一次输入要求模型读取仓库 `README.md` 并仅返回标题中的项目名。模型在一轮中请求 `list_files` 与 `read_file`，两个调用均成功，第二轮返回 `Lain Agent Interface Network`。

- 汇总 usage：9,404 input / 105 output / 5,248 cached input（56%）。
- 第一次验收产生 2 次 provider 请求。

第二次输入要求模型查找 `MAINTAINERS.md` 中的维护者，连续完成 `list_files`、`search_text`、`read_file`，最终回复 `KiloxGo`；汇总 usage 为 12,634 input / 179 output / 10,880 cached input（86%）。该次暴露的大小写搜索浪费已通过默认 case-insensitive literal match 修正。两次验收后累计授权预算约 15/50。

## 尚未完成

- 写文件、补丁、命令执行及其 approval pause/resume。
- tool timeout/cancel、幂等 key、并行安全调度和完整 tool eval suite。
- MCP client/registry；核心只读闭环尚不需要该抽象。

迁移 `0005_tool_invocations.sql` 只新增审计表和索引；代码回滚时保留该表即可，当前 SQLite migration 不执行破坏性 down migration。
