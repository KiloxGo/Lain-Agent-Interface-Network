# ADR 0008：审批式 no-clobber 文件创建

- 状态：Accepted
- 日期：2026-08-26
- 维护者：KiloxGo

## 背景

`replace_text` 能安全修改现有文件，但不能新增测试、模块或配置模板。通用 write/patch 同时包含创建与覆盖语义，审批卡片与执行器容易对“目标是否已存在”产生分歧，因此 v0 需要一个语义更窄的创建动作。

## 决策

1. 增加 `create_file(path, content)`；只创建一个 UTF-8 文件，内容最大 1 MiB，父目录必须已经存在。
2. path 必须是 workspace-relative；拒绝绝对路径、`..`、越界 symlink、Secret/构建目录和已有的任何目录项，包括 dangling symlink。
3. Windows 额外拒绝 alternate data stream `:`、尾随点/空格以及 CON/PRN/AUX/NUL/COM1-9/LPT1-9 等保留文件名。
4. prepare 阶段不落盘，只生成最多 32 KiB 的 new-file diff，并将规范化 path 与 content SHA-256 绑定到 action hash。
5. CLI 沿用单次 `y/N`；断线、超时、取消和非 TTY 均 fail closed，asked/decided 与 invocation 进入 SQLite。
6. 执行前重新解析 canonical parent 并确认目标不存在；随后写入同目录临时文件、flush/sync，再用原子 hard link 发布目标。hard link 在目标已存在时失败，因此审批后的并发创建不会被覆盖。
7. 发布后删除临时 link。通用 executor 继续拒绝直接执行 `create_file`。

## 暂不采用

- 自动创建父目录、覆盖已有文件、delete/move、通用 patch 或批量文件事务。
- 用“目标不存在”的预检查代替原子 no-clobber 发布。
- 为了兼容不支持 hard link 的文件系统而降级到可见的分段写入。

若真实任务需要多文件原子操作，应设计带完整 diff 集合、逐目标 precondition 和 rollback journal 的独立协议，而不是扩大本工具参数。

## 官方参考

- [DeepSeek Harness tool catalog](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/tool-catalog.md)
- [DeepSeek Harness adding a tool](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/cookbook/adding-a-tool.md)
- [Codex apply_patch instructions](https://github.com/openai/codex/blob/main/codex-rs/core/prompt_with_apply_patch_instructions.md)
- [Gemini CLI tools reference](https://github.com/google-gemini/gemini-cli/blob/main/docs/reference/tools.md)

