import { createInterface } from "node:readline";

const mode = process.argv[2] ?? "modern";
const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });

for await (const line of lines) {
  const message = JSON.parse(line);
  if (
    mode === "modern" &&
    message.method !== "initialize" &&
    message.params?._meta?.["io.modelcontextprotocol/protocolVersion"] !== "2026-07-28"
  ) {
    process.stdout.write(`${JSON.stringify({
      jsonrpc: "2.0",
      id: message.id,
      error: { code: -32602, message: "missing protocol metadata" },
    })}\n`);
    continue;
  }
  if (message.method === "server/discover") {
    const response = mode === "modern"
      ? {
          jsonrpc: "2.0",
          id: message.id,
          result: {
            resultType: "complete",
            supportedVersions: ["2026-07-28"],
            capabilities: { tools: {} },
            ttlMs: 0,
            cacheScope: "private",
            _meta: {
              "io.modelcontextprotocol/serverInfo": {
                name: "fixture-server",
                version: "1.0.0",
              },
            },
          },
        }
      : {
          jsonrpc: "2.0",
          id: message.id,
          error: { code: -32601, message: "unknown method" },
        };
    process.stdout.write(`${JSON.stringify(response)}\n`);
  } else if (message.method === "initialize") {
    process.stdout.write(`${JSON.stringify({
      jsonrpc: "2.0",
      id: message.id,
      result: {
        protocolVersion: "2025-11-25",
        capabilities: { tools: {} },
        serverInfo: { name: "fixture-server", version: "0.9.0" },
      },
    })}\n`);
  } else if (message.method === "tools/list") {
    const secondPage = message.params?.cursor === "page-2";
    const tool = secondPage
      ? {
          name: "server.status",
          description: "Return fixture status.",
          inputSchema: { type: "object", additionalProperties: false },
        }
      : {
          name: "echo",
          description: "Echo one message.",
          inputSchema: {
            type: "object",
            properties: { message: { type: "string" } },
            required: ["message"],
            additionalProperties: false,
          },
        };
    process.stdout.write(`${JSON.stringify({
      jsonrpc: "2.0",
      id: message.id,
      result: {
        ...(mode === "modern" ? { resultType: "complete" } : {}),
        tools: [tool],
        ...(secondPage ? {} : { nextCursor: "page-2" }),
      },
    })}\n`);
  } else if (message.method === "tools/call") {
    const text = message.params?.name === "echo"
      ? `echo: ${message.params.arguments?.message ?? ""}`
      : "fixture status: ready";
    process.stdout.write(`${JSON.stringify({
      jsonrpc: "2.0",
      id: message.id,
      result: {
        ...(mode === "modern" ? { resultType: "complete" } : {}),
        content: [{ type: "text", text }],
        structuredContent: { text },
        isError: false,
      },
    })}\n`);
  }
}
