# P0 online baseline

生成时间：2026-08-11。全部输入为仓库内合成数据；没有保存 API Key 或完整 system prompt。

- `persona.jsonl` / `persona-summary.json`：旧 prompt 的 20 次生成与 4 次同 provider 批量 judge，严格通过 5/20。
- `persona-candidate.jsonl` / `persona-candidate-summary.json`：安全/任务优先 prompt 的 20 次生成，留给独立评审。
- `persona-candidate-review.json`：对候选输出的逐项 rubric 复核，18/20；不是独立人类 ground truth。
- `harness.json`：5 次真实 WebSocket 往返和 200 次合成 Markdown append 测量。

本轮共执行 49 次 DeepSeek 调用，没有失败重试。旧基线保留用于证明行为变化，不能用候选结果覆盖历史失败。

