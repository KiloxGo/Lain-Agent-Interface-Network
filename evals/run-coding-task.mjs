import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { spawn } from "node:child_process";

import { LainClient } from "../cli/lain.mjs";

const ROOT = resolve(import.meta.dirname, "..");
const KERNEL_DIR = join(ROOT, "lain-kernel");
const KERNEL = join(KERNEL_DIR, "target", "debug", process.platform === "win32" ? "lain-kernel.exe" : "lain-kernel");
const CASES = join(ROOT, "evals", "cases", "coding", "golden.jsonl");

function invariant(condition, message) {
  if (!condition) throw new Error(message);
}

function selectedCaseId(args) {
  const index = args.indexOf("--case");
  if (index < 0 || !args[index + 1]) {
    throw new Error("usage: node evals/run-coding-task.mjs --case <coding-case-id>");
  }
  return args[index + 1];
}

function selectedOutput(args) {
  const index = args.indexOf("--output");
  if (index >= 0 && !args[index + 1]) throw new Error("--output requires a path");
  return resolve(process.env.CODING_EVAL_OUTPUT ?? (index >= 0 ? args[index + 1] : undefined) ?? "evals/results/p6/coding-task-smoke.json");
}

async function loadCase(id) {
  const cases = (await readFile(CASES, "utf8"))
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => JSON.parse(line));
  const item = cases.find((candidate) => candidate.id === id);
  invariant(item, `unknown coding case: ${id}`);
  return item;
}

async function ensureKernel() {
  await run("cargo", ["build", "--locked", "--bin", "lain-kernel"], KERNEL_DIR);
}

function run(command, args, cwd) {
  return new Promise((resolveRun, rejectRun) => {
    const child = spawn(command, args, { cwd, stdio: "inherit", windowsHide: true });
    child.once("error", rejectRun);
    child.once("exit", (code) => code === 0 ? resolveRun() : rejectRun(new Error(`${command} exited ${code}`)));
  });
}

function stop(child) {
  if (child.exitCode !== null) return Promise.resolve();
  return new Promise((resolveStop) => {
    const timer = setTimeout(resolveStop, 5_000);
    timer.unref();
    child.once("exit", () => {
      clearTimeout(timer);
      resolveStop();
    });
    child.kill();
  });
}

function availablePort() {
  return new Promise((resolvePort, rejectPort) => {
    const server = createServer();
    server.once("error", rejectPort);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => resolvePort(port));
    });
  });
}

async function writeFixture(workspace, files) {
  for (const [path, content] of Object.entries(files)) {
    const target = resolve(workspace, path);
    const local = relative(workspace, target);
    invariant(local && !local.startsWith("..") && !isAbsolute(local), `unsafe fixture path: ${path}`);
    await mkdir(dirname(target), { recursive: true });
    await writeFile(target, content, "utf8");
  }
}

class RecordingClient extends LainClient {
  constructor(url, options) {
    super(url, options);
    this.events = [];
  }

  handleEvent(packet) {
    this.events.push(packet);
    super.handleEvent(packet);
  }
}

function approvalAllowed(item, payload) {
  if (typeof payload.server === "string") {
    return (item.approval.tools ?? []).includes(payload.name);
  }
  if (["replace_text", "create_file"].includes(payload.name)) {
    return item.approval.paths.includes(payload.path);
  }
  if (["run_check", "shell"].includes(payload.name)) {
    return item.approval.commands.some(
      ({ cwd, command }) => cwd === payload.cwd && command === payload.command,
    );
  }
  return false;
}

async function connect(url, item, decisions) {
  let lastError;
  for (let attempt = 0; attempt < 40; attempt += 1) {
    const client = new RecordingClient(url, { write: () => {} });
    client.approve = async (payload) => {
      const approved = approvalAllowed(item, payload);
      decisions.push({
        name: payload.name,
        path: payload.path,
        cwd: payload.cwd,
        command: payload.command,
        server: payload.server,
        arguments: payload.arguments,
        approved,
      });
      return approved;
    };
    try {
      await client.connect();
      return client;
    } catch (error) {
      lastError = error;
      client.close();
      await new Promise((resolveWait) => setTimeout(resolveWait, 100));
    }
  }
  throw lastError ?? new Error("kernel unavailable");
}

function isSubsequence(expected, actual) {
  let cursor = 0;
  for (const value of actual) {
    if (value === expected[cursor]) cursor += 1;
  }
  return cursor === expected.length;
}

async function score(item, workspace, client, usage, decisions) {
  const failures = [];
  const tools = client.events
    .filter(({ type }) => type === "tool.started")
    .map(({ payload }) => payload.name);
  const toolEvents = client.events
    .filter(({ type }) => type === "tool.started" || type === "tool.completed")
    .map(({ type, payload }) => `${type}:${payload.name}`);
  const summaries = client.events
    .filter(({ type }) => type === "tool.completed")
    .map(({ payload }) => payload.summary ?? "");
  const exitCodes = summaries.flatMap((summary) => summary.match(/ exited (signal|-?\d+)$/)?.slice(1) ?? []);
  const final = [...client.messages].reverse().find(({ role }) => role === "assistant")?.content ?? "";

  if (!isSubsequence(item.expected.tool_subsequence, tools)) failures.push(`tool sequence: ${tools.join(" -> ")}`);
  if (item.expected.max_tool_calls !== undefined && tools.length > item.expected.max_tool_calls) {
    failures.push(`tool count: ${tools.length}`);
  }
  if (!isSubsequence(item.expected.exit_codes, exitCodes)) failures.push(`exit codes: ${exitCodes.join(",")}`);
  if (item.expected.final_equals !== undefined) {
    if (final !== item.expected.final_equals) failures.push(`final response: ${final}`);
  } else {
    const fragments = item.expected.final_contains_all ?? [item.expected.final_contains];
    if (fragments.some((fragment) => !final.includes(fragment))) failures.push(`final response: ${final}`);
  }
  if (decisions.some(({ approved }) => !approved)) failures.push("unexpected approval request");

  for (const [path, expected] of Object.entries(item.expected.files)) {
    let content = "";
    try {
      content = await readFile(resolve(workspace, path), "utf8");
    } catch {
      failures.push(`missing file: ${path}`);
      continue;
    }
    for (const text of expected.must_contain) {
      if (!content.includes(text)) failures.push(`${path}: missing ${JSON.stringify(text)}`);
    }
    for (const text of expected.must_not_contain) {
      if (content.includes(text)) failures.push(`${path}: retained ${JSON.stringify(text)}`);
    }
  }

  return {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    caseId: item.id,
    passed: failures.length === 0,
    failures,
    tools,
    toolEvents,
    exitCodes,
    approvals: decisions,
    final,
    usage,
  };
}

async function main(args = process.argv.slice(2)) {
  const item = await loadCase(selectedCaseId(args));
  const output = selectedOutput(args);
  await ensureKernel();
  const fixture = await mkdtemp(join(tmpdir(), "lain-coding-eval-"));
  const workspace = join(fixture, "workspace");
  const data = join(fixture, "data");
  await mkdir(workspace, { recursive: true });
  await writeFixture(workspace, item.input.files);
  const port = await availablePort();
  const kernel = spawn(KERNEL, [], {
    cwd: KERNEL_DIR,
    env: {
      ...process.env,
      LAIN_BIND_ADDR: `127.0.0.1:${port}`,
      LAIN_WORKSPACE: workspace,
      LAIN_DATA_DIR: data,
      LAIN_MEMORY_EXTRACTION_ENABLED: "false",
      ...(item.input.mcp ? {
        LAIN_MCP_STDIO_NAME: "fixture",
        LAIN_MCP_STDIO_COMMAND: process.execPath,
        LAIN_MCP_STDIO_ARGS: JSON.stringify([
          join(ROOT, "evals", "fixtures", "mcp-stdio-server.mjs"),
          "modern",
        ]),
      } : {}),
    },
    stdio: ["ignore", "ignore", "pipe"],
    windowsHide: true,
  });
  let stderr = "";
  kernel.stderr.setEncoding("utf8");
  kernel.stderr.on("data", (chunk) => { stderr = `${stderr}${chunk}`.slice(-4_000); });
  const decisions = [];
  let client;
  try {
    client = await connect(`ws://127.0.0.1:${port}/v1/ws`, item, decisions);
    const usage = await client.ask(item.input.prompt);
    const result = await score(item, workspace, client, usage, decisions);
    await mkdir(dirname(output), { recursive: true });
    await writeFile(output, `${JSON.stringify(result, null, 2)}\n`, "utf8");
    console.log(JSON.stringify(result, null, 2));
    invariant(result.passed, result.failures.join("; "));
  } finally {
    client?.close();
    await stop(kernel);
    await rm(fixture, { recursive: true, force: true });
  }
  invariant(!stderr.includes("panicked"), stderr);
}

main().catch((error) => {
  process.stderr.write(`coding eval: ${error.message}\n`);
  process.exitCode = 1;
});
