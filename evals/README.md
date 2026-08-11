# Evals

这里保存人格与记忆的版本化验收样例。P0 数据只定义可机械检查的输入与期望约束，不把“看起来像”当作评分标准，也不调用付费模型。

## 数据格式

每行一个 JSON 对象，UTF-8：

- `id`：集合内唯一、稳定，不复用。
- `category`：覆盖维度。
- `input`：合成输入；人格为 `messages`，记忆为候选记录与查询。
- `expected`：可由规则评分器或后续 judge 评分器检查的结果。
- `tags`：用于切片分析。

`golden.jsonl` 是最小回归集。修改期望必须在 PR 中解释行为改变，不能为了让新实现过关而无记录地降低标准。

## 校验

```powershell
node evals/validate.mjs
```

校验器检查 JSONL 语法、ID 唯一性、最小样例数、必需字段、禁止真实 secret 的占位模式，以及人格/记忆集合的核心结构。后续 runner 将追加确定性规则分数、模型 judge、延迟/token 成本和基线对比。

## 受控在线基线

在线评测只在维护者明确授权后执行，密钥由 Node 的 `--env-file` 从忽略的本地文件加载：

```powershell
node --env-file=lain-kernel/.env evals/run-persona.mjs
node evals/benchmark-harness.mjs
```

`run-persona.mjs` 默认最多使用 30 次调用：20 次生成、4 次批量 judge，并保留余量用于异常重跑。它不会保存系统提示词或密钥，只保存提示词 SHA-256、合成输入、输出、usage、时延和 judge 结果。`benchmark-harness.mjs` 通过实际 WebSocket 使用 5 次调用，并测量当前 Markdown append 原语；运行前需要启动后端。
