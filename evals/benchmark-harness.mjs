import { appendFile, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { performance } from "node:perf_hooks";

const WS_URL = process.env.LAIN_WS_URL ?? "ws://127.0.0.1:3000/ws";
const OUTPUT_PATH = resolve(process.env.HARNESS_BENCHMARK_PATH ?? "evals/results/p0/harness.json");
const REQUEST_TIMEOUT_MS = Number.parseInt(process.env.HARNESS_TIMEOUT_MS ?? "180000", 10);
const prompts = [
  "用三句话介绍你自己。",
  "给我一个今天可以完成的低负担计划。",
  "解释 Rust 所有权，但不要超过一百五十字。",
  "如果你不确定一个事实，应该怎样回答？",
  "总结个性化 Agent 最重要的三个工程边界。",
];

function percentile(values, percentileValue) {
  const sorted = [...values].sort((a, b) => a - b);
  const index = Math.ceil((percentileValue / 100) * sorted.length) - 1;
  return sorted[Math.max(0, index)];
}

async function roundTrip(prompt) {
  const socket = new WebSocket(WS_URL);
  await new Promise((resolveOpen, rejectOpen) => {
    const timeout = setTimeout(() => rejectOpen(new Error(`WebSocket open timeout: ${WS_URL}`)), 10_000);
    socket.addEventListener("open", () => {
      clearTimeout(timeout);
      resolveOpen();
    });
    socket.addEventListener("error", () => {
      clearTimeout(timeout);
      rejectOpen(new Error(`WebSocket connection failed: ${WS_URL}`));
    });
  });

  const startedAt = performance.now();
  const response = await new Promise((resolveMessage, rejectMessage) => {
    const timeout = setTimeout(() => rejectMessage(new Error("WebSocket response timeout")), REQUEST_TIMEOUT_MS);
    socket.addEventListener("message", (event) => {
      clearTimeout(timeout);
      resolveMessage(String(event.data));
    });
    socket.addEventListener("error", () => {
      clearTimeout(timeout);
      rejectMessage(new Error("WebSocket request failed"));
    });
    socket.send(prompt);
  });
  const completedAt = performance.now();
  socket.close();
  return {
    prompt,
    response,
    responseChars: response.length,
    userVisibleTtftMs: Number((completedAt - startedAt).toFixed(2)),
    totalMs: Number((completedAt - startedAt).toFixed(2)),
  };
}

async function benchmarkMarkdownAppend() {
  const benchDir = await mkdtemp(join(tmpdir(), "lain-p0-storage-"));
  const file = join(benchDir, "session.md");
  const sample = `---\n**User:** synthetic benchmark\n\n**Lain:** ${"x".repeat(512)}\n\n`;
  const timings = [];
  try {
    for (let index = 0; index < 200; index += 1) {
      const startedAt = performance.now();
      await appendFile(file, sample, "utf8");
      timings.push(Number((performance.now() - startedAt).toFixed(4)));
    }
  } finally {
    await rm(benchDir, { recursive: true, force: true });
  }
  return {
    operation: "append synthetic Markdown exchange without fsync",
    iterations: timings.length,
    p50Ms: percentile(timings, 50),
    p95Ms: percentile(timings, 95),
    minMs: Math.min(...timings),
    maxMs: Math.max(...timings),
  };
}

const runs = [];
for (const [index, prompt] of prompts.entries()) {
  console.log(`[harness ${index + 1}/${prompts.length}]`);
  runs.push(await roundTrip(prompt));
}
const latency = runs.map((run) => run.totalMs);
const result = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  wsUrl: WS_URL,
  providerMode: "non-streaming single complete WebSocket message",
  callsUsed: runs.length,
  runs,
  userVisibleLatencyMs: {
    p50: percentile(latency, 50),
    p95: percentile(latency, 95),
    min: Math.min(...latency),
    max: Math.max(...latency),
  },
  storage: await benchmarkMarkdownAppend(),
};
await mkdir(dirname(OUTPUT_PATH), { recursive: true });
await writeFile(OUTPUT_PATH, `${JSON.stringify(result, null, 2)}\n`, "utf8");
console.log(JSON.stringify(result, null, 2));

