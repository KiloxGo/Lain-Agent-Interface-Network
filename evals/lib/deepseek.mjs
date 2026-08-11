import { performance } from "node:perf_hooks";

const DEFAULT_TIMEOUT_MS = 180_000;

function cleanBaseUrl(value) {
  return value.replace(/\/+$/, "");
}

function parseJsonBody(text, context) {
  const normalized = text.trim();
  try {
    return JSON.parse(normalized);
  } catch (error) {
    throw new Error(`${context}: invalid JSON response: ${error.message}`);
  }
}

export function createDeepSeekClient({
  apiKey = process.env.DEEPSEEK_API_KEY,
  baseUrl = process.env.DEEPSEEK_BASE_URL ?? "https://api.deepseek.com",
  model = process.env.DEEPSEEK_MODEL ?? "deepseek-v4-flash",
  callLimit = 50,
  timeoutMs = DEFAULT_TIMEOUT_MS,
} = {}) {
  if (!apiKey) throw new Error("DEEPSEEK_API_KEY is required");
  if (!Number.isInteger(callLimit) || callLimit < 1) throw new Error("callLimit must be a positive integer");

  let callCount = 0;

  async function complete({
    messages,
    maxTokens = 700,
    responseFormat,
    userId = "lain_p0_eval",
  }) {
    if (callCount >= callLimit) throw new Error(`DeepSeek call limit ${callLimit} reached`);
    callCount += 1;

    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), timeoutMs);
    const startedAt = performance.now();

    let response;
    try {
      response = await fetch(`${cleanBaseUrl(baseUrl)}/chat/completions`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${apiKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          model,
          messages,
          stream: false,
          thinking: { type: "disabled" },
          max_tokens: maxTokens,
          user_id: userId,
          ...(responseFormat ? { response_format: responseFormat } : {}),
        }),
        signal: controller.signal,
      });
    } catch (error) {
      const reason = error.name === "AbortError" ? `timeout after ${timeoutMs} ms` : error.message;
      throw new Error(`DeepSeek request ${callCount} failed: ${reason}`);
    } finally {
      clearTimeout(timeout);
    }

    const headersAt = performance.now();
    const responseText = await response.text();
    const completedAt = performance.now();
    const requestId = response.headers.get("x-request-id") ?? response.headers.get("cf-ray") ?? null;

    if (!response.ok) {
      const safeBody = responseText.replaceAll(apiKey, "[REDACTED]").trim().slice(0, 800);
      throw new Error(`DeepSeek request ${callCount} returned HTTP ${response.status}: ${safeBody}`);
    }

    const body = parseJsonBody(responseText, `DeepSeek request ${callCount}`);
    const choice = body.choices?.[0];
    if (!choice?.message || typeof choice.message.content !== "string") {
      throw new Error(`DeepSeek request ${callCount}: missing choices[0].message.content`);
    }

    return {
      callIndex: callCount,
      content: choice.message.content,
      finishReason: choice.finish_reason ?? null,
      model: body.model ?? model,
      requestId,
      systemFingerprint: body.system_fingerprint ?? null,
      usage: body.usage ?? null,
      timing: {
        headersMs: Number((headersAt - startedAt).toFixed(2)),
        totalMs: Number((completedAt - startedAt).toFixed(2)),
      },
    };
  }

  return {
    complete,
    getCallCount: () => callCount,
    model,
  };
}

export function parseJsonObject(text, context = "model JSON") {
  const trimmed = text.trim();
  const withoutFence = trimmed
    .replace(/^```(?:json)?\s*/i, "")
    .replace(/\s*```$/, "")
    .trim();
  return parseJsonBody(withoutFence, context);
}

