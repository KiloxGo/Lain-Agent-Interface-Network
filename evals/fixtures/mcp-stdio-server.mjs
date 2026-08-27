import { createInterface } from "node:readline";

const mode = process.argv[2] ?? "modern";
const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });

for await (const line of lines) {
  const message = JSON.parse(line);
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
  }
}
