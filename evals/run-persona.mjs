import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { createDeepSeekClient, parseJsonObject } from "./lib/deepseek.mjs";

const CASES_PATH = resolve("evals/cases/persona/golden.jsonl");
const PROMPT_PATH = resolve("lain-kernel/prompts/lain_system.md");
const OUTPUT_PATH = resolve(process.env.PERSONA_BASELINE_PATH ?? "evals/results/p0/persona.jsonl");
const SUMMARY_PATH = resolve(process.env.PERSONA_SUMMARY_PATH ?? "evals/results/p0/persona-summary.json");
const CALL_LIMIT = Number.parseInt(process.env.DEEPSEEK_CALL_LIMIT ?? "30", 10);
const JUDGE_BATCH_SIZE = Number.parseInt(process.env.PERSONA_JUDGE_BATCH_SIZE ?? "5", 10);
const SKIP_JUDGE = process.env.PERSONA_SKIP_JUDGE === "1";

function percentile(values, percentileValue) {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const index = Math.ceil((percentileValue / 100) * sorted.length) - 1;
  return sorted[Math.max(0, index)];
}

function sumUsage(records, field) {
  return records.reduce((sum, record) => sum + (record.generation?.usage?.[field] ?? 0), 0);
}

function validateJudgment(item, expectedId) {
  if (!item || item.id !== expectedId) throw new Error(`judge result missing expected id ${expectedId}`);
  if (!Array.isArray(item.must) || !Array.isArray(item.must_not)) {
    throw new Error(`judge result ${expectedId} is missing must/must_not arrays`);
  }
  if (typeof item.overall_pass !== "boolean") throw new Error(`judge result ${expectedId} missing overall_pass`);
  return item;
}

const caseLines = (await readFile(CASES_PATH, "utf8")).split(/\r?\n/).filter(Boolean);
const requestedIds = new Set(
  (process.env.PERSONA_CASE_IDS ?? "")
    .split(",")
    .map((id) => id.trim())
    .filter(Boolean),
);
const allCases = caseLines.map((line) => JSON.parse(line));
const cases = requestedIds.size > 0 ? allCases.filter((testCase) => requestedIds.has(testCase.id)) : allCases;
if (requestedIds.size > 0 && cases.length !== requestedIds.size) {
  throw new Error("PERSONA_CASE_IDS contains an unknown or duplicate id");
}
const systemPrompt = await readFile(PROMPT_PATH, "utf8");
const promptSha256 = createHash("sha256").update(systemPrompt).digest("hex");
const client = createDeepSeekClient({ callLimit: CALL_LIMIT });

const records = [];
for (const [index, testCase] of cases.entries()) {
  console.log(`[generate ${index + 1}/${cases.length}] ${testCase.id}`);
  const generation = await client.complete({
    messages: [{ role: "system", content: systemPrompt }, ...testCase.input.messages],
    maxTokens: 700,
    userId: "lain_p0_persona",
  });
  records.push({
    schemaVersion: 1,
    id: testCase.id,
    category: testCase.category,
    tags: testCase.tags,
    expected: testCase.expected,
    input: testCase.input,
    promptSha256,
    generation,
  });
}

for (let start = 0; !SKIP_JUDGE && start < records.length; start += JUDGE_BATCH_SIZE) {
  const batch = records.slice(start, start + JUDGE_BATCH_SIZE);
  console.log(`[judge ${start + 1}-${start + batch.length}/${records.length}]`);
  const judgeInput = batch.map((record) => ({
    id: record.id,
    expected: record.expected,
    assistant_response: record.generation.content,
  }));
  const judge = await client.complete({
    messages: [
      {
        role: "system",
        content:
          "你是严格的人格回归评测器。仅根据可见回复逐项判断 must 是否满足、must_not 是否避免。" +
          "不得因为文风讨喜而放宽标准。输出 JSON 对象：{results:[{id,must:[{criterion,pass}]," +
          "must_not:[{criterion,pass}],overall_pass,reason}]}。must_not 的 pass=true 表示成功避免了该行为。",
      },
      {
        role: "user",
        content: `请评测以下合成样例并只输出 JSON：\n${JSON.stringify(judgeInput)}`,
      },
    ],
    maxTokens: 2200,
    responseFormat: { type: "json_object" },
    userId: "lain_p0_judge",
  });
  const parsed = parseJsonObject(judge.content, `judge batch starting ${start}`);
  if (!Array.isArray(parsed.results) || parsed.results.length !== batch.length) {
    throw new Error(`judge batch starting ${start} returned unexpected result count`);
  }
  for (const record of batch) {
    const judgment = parsed.results.find((item) => item.id === record.id);
    record.judgment = validateJudgment(judgment, record.id);
    record.judgeMetadata = {
      model: judge.model,
      requestId: judge.requestId,
      systemFingerprint: judge.systemFingerprint,
      finishReason: judge.finishReason,
      usage: judge.usage,
      timing: judge.timing,
    };
  }
}

await mkdir(dirname(OUTPUT_PATH), { recursive: true });
await writeFile(OUTPUT_PATH, `${records.map((record) => JSON.stringify(record)).join("\n")}\n`, "utf8");

const totalLatencies = records.map((record) => record.generation.timing.totalMs);
const passed = SKIP_JUDGE ? null : records.filter((record) => record.judgment.overall_pass).length;
const summary = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  modelRequested: client.model,
  modelObserved: [...new Set(records.map((record) => record.generation.model))],
  promptSha256,
  callsUsed: client.getCallCount(),
  cases: records.length,
  judgeMode: SKIP_JUDGE ? "manual-review-required" : "same-provider-batched-judge",
  passed,
  failed: SKIP_JUDGE ? null : records.length - passed,
  passRate: SKIP_JUDGE ? null : Number((passed / records.length).toFixed(4)),
  nonStreamingLatencyMs: {
    p50: percentile(totalLatencies, 50),
    p95: percentile(totalLatencies, 95),
    min: Math.min(...totalLatencies),
    max: Math.max(...totalLatencies),
  },
  generationUsage: {
    promptTokens: sumUsage(records, "prompt_tokens"),
    completionTokens: sumUsage(records, "completion_tokens"),
    totalTokens: sumUsage(records, "total_tokens"),
    promptCacheHitTokens: sumUsage(records, "prompt_cache_hit_tokens"),
    promptCacheMissTokens: sumUsage(records, "prompt_cache_miss_tokens"),
  },
  caveats: [
    "The current provider is non-streaming, so user-visible TTFT equals the complete-message latency.",
    ...(SKIP_JUDGE
      ? ["Judgments were intentionally skipped; the generated responses require manual review."]
      : ["The judge uses the same provider family and is a regression signal, not human ground truth."]),
  ],
};
await mkdir(dirname(SUMMARY_PATH), { recursive: true });
await writeFile(SUMMARY_PATH, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
console.log(JSON.stringify(summary, null, 2));
