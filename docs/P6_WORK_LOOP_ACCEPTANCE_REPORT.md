# P6 外部项目工作闭环 v1 验收报告

- 日期：2026-08-29
- 分支：`feat/cli-mvp`
- 维护者：KiloxGo
- 结论：根级项目规则、受限多轮工具循环、验证与收尾报告已形成最小可运行闭环。

## 已验收范围

- 只采用标准根级 `AGENTS.md`；不存在时继续工作，存在时每个 run 重新读取。
- canonical workspace 边界、UTF-8、32 KiB 文件上限和 8192-token ContextBudget 均由 host 强制。
- ContextPlan 记录 `project_instructions` 来源、内容 hash、trust level 和 token 数，不记录本地路径。
- Harness Contract 要求 inspect → edit → check → report；项目规则不能替代任何一次性工具审批。
- 工具循环扩至最多 8 轮/16 次调用；chat 可使用工具，complex-task 等 reasoning replay 完成后再接入。

## 自动化结果

- Rust tests：98 passed，5 suites。
- Clippy：`--all-targets --all-features --locked -- -D warnings` 通过。
- rustfmt：`cargo fmt --all -- --check` 通过。
- CLI：3 passed。
- eval：persona 50 + memory 20 + coding 11，共 81 cases；所有 11 个在线工具基线通过。

## 真实 DeepSeek 验收

`coding-work-loop-001` 在一次性临时 Node workspace 中提供根级 `AGENTS.md`、错误的 `math.js`、测试与 npm script。Lain 的轨迹为：

1. `list_files`；
2. 同一只读批次并行执行 4 次 `read_file`；
3. 一次性审批后 `replace_text(math.js)`；
4. 一次性审批后 `run_check(npm_test)`，exit 0；
5. 最终报告包含 `AGENTS_OK`、`math.js` 和 `npm_test`。

最终 usage：14597 input / 404 output / 11520 cached input tokens。结果保存在 `evals/results/p6/work-loop-smoke.json`；fixture 和结果均不含真实项目数据或密钥。

## 明确延期

- 父目录/子目录 `AGENTS.md` 继承与覆盖。
- 结构化 plan/todo 状态、跨 run 未完成任务恢复。
- 多文件 patch 的合并预览与批量审批。
- complex-task 的 reasoning replay 与工具循环。
- 自动 git diff/status 汇总；当前最终报告来自工具回执。
