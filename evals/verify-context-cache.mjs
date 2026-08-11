import { performance } from "node:perf_hooks";

const wsUrl = process.env.LAIN_V1_WS_URL ?? "ws://127.0.0.1:3000/v1/ws";
const timeoutMs = Number.parseInt(process.env.LAIN_CACHE_TIMEOUT_MS ?? "180000", 10);
const socket = new WebSocket(wsUrl);
const queue = [];
let waiter;

socket.addEventListener("message", (event) => {
  const value = JSON.parse(String(event.data));
  if (waiter) {
    const resolve = waiter;
    waiter = undefined;
    resolve(value);
  } else {
    queue.push(value);
  }
});

await new Promise((resolve, reject) => {
  const timer = setTimeout(() => reject(new Error(`WebSocket open timeout: ${wsUrl}`)), 10_000);
  socket.addEventListener("open", () => {
    clearTimeout(timer);
    resolve();
  });
  socket.addEventListener("error", () => {
    clearTimeout(timer);
    reject(new Error(`WebSocket connection failed: ${wsUrl}`));
  });
});

function nextEvent() {
  if (queue.length > 0) return Promise.resolve(queue.shift());
  return new Promise((resolve) => {
    waiter = resolve;
  });
}

async function waitFor(predicate) {
  const deadline = performance.now() + timeoutMs;
  while (performance.now() < deadline) {
    const remaining = Math.max(1, deadline - performance.now());
    const event = await Promise.race([
      nextEvent(),
      new Promise((_, reject) => setTimeout(() => reject(new Error("event timeout")), remaining)),
    ]);
    if (event.type === "error") throw new Error(`server error: ${event.code}`);
    if (predicate(event)) return event;
  }
  throw new Error("event timeout");
}

socket.send(JSON.stringify({ type: "client.hello", versions: [1] }));
const hello = await waitFor((event) => event.type === "server.hello");
const sessionId = hello.scope.session_id;
socket.send(JSON.stringify({ type: "session.subscribe", session_id: sessionId, after_seq: 0 }));
await waitFor((event) => event.type === "session.snapshot");

async function runTurn(index, text) {
  const clientMessageId = `p4-cache-${Date.now()}-${index}`;
  socket.send(JSON.stringify({
    type: "message.create",
    session_id: sessionId,
    client_message_id: clientMessageId,
    text,
  }));
  let runId;
  let context;
  while (true) {
    const event = await waitFor((candidate) => {
      if (candidate.type === "run.queued" && candidate.payload?.trigger_message_id) return true;
      return runId !== undefined && candidate.run_id === runId;
    });
    if (event.type === "run.queued") runId = event.run_id;
    if (event.type === "context.compiled") context = event.payload;
    if (event.type === "run.failed") throw new Error(`run failed: ${event.payload?.code}`);
    if (event.type === "run.completed") {
      return {
        index,
        runId,
        context,
        usage: event.payload?.usage,
      };
    }
  }
}

const first = await runTurn(1, "缓存验收第一轮：只回复 P4_CACHE_ONE。不要添加其他内容。");
const second = await runTurn(2, "缓存验收第二轮：只回复 P4_CACHE_TWO。不要添加其他内容。");
socket.close(1000, "cache verification complete");

const result = {
  schemaVersion: 1,
  callsUsed: 2,
  identityVersion: first.context?.identity_version,
  compilerEstimated: [first.context?.estimated, second.context?.estimated],
  estimatedInputTokens: [first.context?.estimated_tokens, second.context?.estimated_tokens],
  providerInputTokens: [first.usage?.input_tokens, second.usage?.input_tokens],
  cachedInputTokens: [first.usage?.cached_input_tokens, second.usage?.cached_input_tokens],
  cacheMissInputTokens: [first.usage?.cache_miss_input_tokens, second.usage?.cache_miss_input_tokens],
  secondTurnCacheHit: Number(second.usage?.cached_input_tokens ?? 0) > 0,
  planIdsPresent: Boolean(first.context?.plan_id && second.context?.plan_id),
  promptHashesDiffer: first.context?.prompt_hash !== second.context?.prompt_hash,
};
console.log(JSON.stringify(result, null, 2));
