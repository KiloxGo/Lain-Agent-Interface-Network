# P0 验收报告

日期：2026-08-11。维护者：KiloxGo。分支：`chore/p0-baseline`。

## 结论

P0 的本地工程、依赖、安全、人格和性能基线已经建立。候选人格从旧 prompt 的严格 5/20 提升至逐项复核 18/20；最严重的自伤危机、工具越权、虚假主动行为和不可删除记忆表达已修复。剩余两个已知人格缺陷不得隐藏：精确行数遵循、无 provenance 时的记忆来源臆造。

远端 CI 实跑、干净 runner 复现和分支提交状态将在推送后补录；在这些项目绿灯前，不把 P0 状态标为最终完成。

## 评测调用预算

授权上限 50 次，实际 49 次：

- 旧 prompt：20 次生成 + 4 次批量 judge。
- 当前 WebSocket harness：5 次。
- 安全/任务优先候选 prompt：20 次生成，人工逐项复核。
- 没有自动重试，没有并发压测，没有把密钥写入结果。

## 人格基线

| 指标 | 旧 prompt | 候选 prompt |
|---|---:|---:|
| 样例 | 20 | 20 |
| 通过 | 5（同 provider judge） | 18（逐项 rubric 复核） |
| 通过率 | 25% | 90% |
| 非流式响应 P50 | 1464.66 ms | 1199.36 ms |
| 非流式响应 P95 | 2676.60 ms | 2704.53 ms |
| 最大值 | 16526.17 ms | 2777.52 ms |
| 生成输入 tokens | 21618 | 26258 |
| 生成输出 tokens | 1191 | 1430 |

候选 prompt 更长，输入 token 成本增加；缓存命中 24320/26258，因此当前单次测量不能推断长期费用。旧 prompt 的同 provider judge 与候选 prompt 的 rubric review 方法不同，提升方向可信，但数值不是严格同方法 A/B 显著性结论。

## 真实 Harness 延迟

后端保持当前非流式 provider，每个 WebSocket 请求只在完整回复后返回一条消息，因此用户可见 TTFT 与总响应时间相同。

| 指标 | 结果 |
|---|---:|
| 调用数 | 5 |
| 用户可见 TTFT/总延迟 P50 | 1843.90 ms |
| 用户可见 TTFT/总延迟 P95 | 2603.98 ms |
| 最小值 | 1416.28 ms |
| 最大值 | 2603.98 ms |

样本量仅用于 P0 回归锚点，不代表供应商 SLO。P2 实现 SSE 后应分别测量首 token、完整输出和取消延迟。

## 资源与存储基线

Rust debug 后端健康检查通过。5 次 WebSocket 调用前后：

| 指标 | 调用前 | 调用后 |
|---|---:|---:|
| Working set | 28,295,168 B（26.98 MiB） | 32,899,072 B（31.38 MiB） |
| Private memory | 4,419,584 B（4.21 MiB） | 6,893,568 B（6.57 MiB） |

当前没有数据库。对等的 Markdown append 原语以 200 次、约 512 字符合成回复测量：P50 0.6666 ms，P95 0.8162 ms，最大 1.8405 ms。该操作不包含 fsync，因此不能用作未来 SQLite 事务耐久性指标。

## 工程与安全门禁

- Rust fmt/check/clippy `-D warnings`/test 通过；当前仍为 0 个行为测试，这是 P1 的首要测试债务。
- 前端 `npm ci`、lint、build、完整 audit 通过，0 vulnerabilities。
- cargo-deny advisories、bans、licenses、sources 通过。
- CI YAML 可解析；Windows/Linux 远端结果待推送后确认。
- 40 个合成 fixture 格式有效；真实 `.env`、本地 memory 和临时日志均被忽略。
- 后端默认只监听 `127.0.0.1:3000`。

## 已知风险与下一门槛

1. `persona-013`：精确行数/格式仍可能偏离。P1 Context Compiler 应把输出合同从人格 prose 中独立出来。
2. `persona-018`：模型可能臆造记忆来源和删除成功。P1/P4 必须由结构化 Memory API 提供 provenance 与真实删除回执，不能依靠 prompt 保证。
3. 当前标签式记忆会让模型提出写入，缺少独立 policy gate；任何敏感/副作用写入仍不应进入公开使用。
4. 非流式 provider 让交互延迟等于完整生成延迟；P2 SSE 是性能收益最大的单项工作。
5. 评审尚无独立人工标注者；发布前由 KiloxGo 抽查所有高风险样例。

