# P6 一次性审批编辑 v0 验收

- 日期：2026-08-26
- 范围：CLI + Rust kernel 的现有文件精确替换
- 工具：`replace_text`

## 已验收行为

- `old_text` 非空、恰好匹配一次才生成预览；prepare 不修改文件。
- 绝对路径、`..`、越界 symlink、Secret/构建目录继续由统一 workspace policy 拒绝。
- 每次写入先展示 path 与有界 diff，只允许单次 `y`/`yes` 授权；其他情况 fail closed。
- 审批请求与决定写入 SQLite，正文和 diff 不进入持久事件。
- 审批后重新解析路径并校验原文件 SHA-256；并发修改会保留外部内容并返回 `file_changed`。
- 写入使用同目录临时文件和平台原子替换；通用 tool executor 无法绕过审批。

## 自动检查

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`：82 passed
- `node --test cli/lain.test.mjs`：2 passed
- 覆盖审批前不写入、批准后写入、stale file 拒绝、审批协议响应和 SQLite requested/decided 单次状态迁移。

## 真实 DeepSeek 验收

在合成文件 `approved-edit-test.txt` 上要求模型把唯一文本 `approval-before` 替换为 `approval-after`：

1. 模型调用 `replace_text`。
2. CLI 显示精确 diff 并暂停在 `Apply this change? [y/N]`。
3. 输入 `y` 后工具完成，磁盘内容为 `approval-after`。
4. 第二次 provider 请求返回 `EDIT_OK`。

汇总 usage：5,222 input / 88 output / 3,968 cached input（76%），共 2 次 provider 请求。测试文件已删除，没有遗留 kernel 进程；授权预算累计约 17/50。

## 明确未包含

- 新建/删除文件、通用 patch、shell、MCP、会话级永久授权。
- 通用工具 timeout/idempotency、并行副作用调度、GUI approval card。

下一步以真实编码任务数据判断：先扩展结构化文件操作，还是先做受限命令执行与测试反馈闭环。

