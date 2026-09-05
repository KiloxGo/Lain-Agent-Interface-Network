# 现代 Agent Harness 质量闭环调研

- 日期：2026-08-30
- 维护者：KiloxGo
- 范围：Claude Code、Codex、DeepSeek Harness 的文件改动、验证与恢复机制

## 成品实现的共同点

1. DeepSeek Harness 将文件“已观察状态”绑定到 session owner；默认组合要求 read-before-write/edit，并在执行时用版本 guard 拒绝陈旧修改。文件写入成功后才发布 `fs/observed`，错误带稳定 code 与恢复指引。
2. Claude Code 在 Write/Edit/NotebookEdit 前保存文件检查点，可恢复本 session 创建或修改的文件；shell 产生的文件副作用不在检查点范围内，因此 shell 不是首选编辑通道。
3. Codex 的 `apply_patch` 是受约束的文件 patch 协议，可在一个调用中描述新增、更新、移动和删除；App Server 把 `fileChange`、`TurnDiff`、命令 exit code 和 duration 作为结构化 item，而不是只依赖最终自然语言报告。
4. 三者都让宿主掌握事实：模型提出动作，宿主验证路径/版本、执行并返回可审计结果。质量闭环不能只依赖人格 prompt。
5. 编码能力评测必须检查 completion 后的仓库状态。SWE-bench 在隔离环境应用候选 patch、运行测试并由 grader 判定 resolved；OpenAI 的编码评估也使用 agent 完成后才运行的隐藏单元测试。模型自报的测试结果只能作为轨迹证据，不能作为最终质量标签。

## 对 Lain 的当前映射

- 已有：workspace canonical 边界、exact-once replacement、内容 hash 乐观并发、no-clobber 创建、一次性 diff/command 审批、工具/审批/assistant step 持久化、检查 exit code 回执。
- 本次增加：`create_file` 在审批成功后逐级创建缺失父目录，并拒绝 symlink、非目录、保留名与竞态目标；成功写入后由 host 注入可信复查指令，`run_check` 非零或工具失败后注入可信恢复指令。
- 后续增加：coding runner 可在 agent turn 完成后写入模型不可见的 Node test 文件并独立执行；hidden fixture 必须与可见 fixture 不重名，执行限时 30 秒、输出最多保留 16 KiB，并清除 API key/token/credential 等敏感环境变量，结果作为 `postCheck` 进入 baseline 和 CI 校验。grader 仍运行在宿主账户下且没有 OS sandbox，只用于受控本地合成 fixture。
- 已完成：最多 8 文件的 create/replace 批量 patch、合并预览、逐目标 precondition、父子目标冲突检查、结构化 `workspace.changed`；宿主确认精确写入后不再要求模型无价值地复读文件。
- 已完成：工具预算耗尽时将最终 handoff 标记为持久 run checkpoint；下一 run 可从 recent conversation 恢复，且不会越过当前用户意图。
- 仍延期：完整 TurnDiff、文件内容 checkpoint/undo，以及 delete/move 和极端中途 I/O 失败的 rollback journal。

## 验收含义

- 绿地任务不得为了创建目录而退化到平台 shell。
- 写入后至少复读结果或运行相关检查；检查失败不得直接宣称完成。
- 最终报告只能引用实际完成的检查和工具回执。
- 质量提升同时记录工具数、token 与真实任务结果，避免用更长轨迹掩盖失败率。

## 一手来源

- [DeepSeek Harness filesystem subsystem](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/subsystems/filesystem.md)
- [DeepSeek Harness filesystem tool package](https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/fs/tool-fs/README.md)
- [Claude Code checkpointing](https://code.claude.com/docs/en/checkpointing)
- [Claude Agent SDK file checkpointing](https://code.claude.com/docs/en/agent-sdk/file-checkpointing)
- [OpenAI: Unrolling the Codex agent loop](https://openai.com/index/unrolling-the-codex-agent-loop/)
- [Codex apply_patch instructions](https://github.com/openai/codex/blob/main/codex-rs/prompts/templates/apply_patch_tool_instructions.md)
- [Codex App Server protocol](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)
- [SWE-bench official evaluation harness](https://github.com/SWE-bench/SWE-bench/blob/main/swebench/harness/run_evaluation.py)
- [OpenAI coding evaluation with hidden unit tests](https://deploymentsafety.openai.com/gpt-5-3-codex/gpt-5-3-codex.pdf)
