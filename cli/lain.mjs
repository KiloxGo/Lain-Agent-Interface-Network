#!/usr/bin/env node

import { randomUUID } from "node:crypto";
import { spawn } from "node:child_process";
import { realpathSync } from "node:fs";
import { createServer } from "node:net";
import { dirname, join } from "node:path";
import { createInterface } from "node:readline/promises";
import { fileURLToPath } from "node:url";

const DEFAULT_URL = "ws://127.0.0.1:3000/v1/ws";
const REPOSITORY_ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const KERNEL_DIRECTORY = join(REPOSITORY_ROOT, "lain-kernel");
const KERNEL_BINARY = join(
  KERNEL_DIRECTORY,
  "target",
  "debug",
  process.platform === "win32" ? "lain-kernel.exe" : "lain-kernel",
);
const pink = (text) => color("\x1b[38;5;205m", text);
const dim = (text) => color("\x1b[2m", text);

function color(code, text) {
  return process.stdout.isTTY && !process.env.NO_COLOR ? `${code}${text}\x1b[0m` : text;
}

export function banner() {
  return pink(String.raw`
          .-------.
      .---| LAIN  |---.
      |   '---+---'   |
      o-------+-------o
          W I R E D
`) + dim("local harness / protocol v1\n");
}

export function approvalPrompt(approval) {
  if (typeof approval.command === "string") {
    return {
      detail: `${dim(approval.cwd ?? ".")}\n$ ${approval.command}`,
      question: "Run this check? [y/N] ",
    };
  }
  if (typeof approval.server === "string") {
    return {
      detail: `${dim(`mcp:${approval.server}`)}\n${approval.tool ?? approval.name ?? "tool"}\n${JSON.stringify(approval.arguments ?? {}, null, 2)}`,
      question: "Call this external tool? [y/N] ",
    };
  }
  return {
    detail: `${dim(approval.path ?? "")}\n${approval.diff ?? ""}`,
    question: "Apply this change? [y/N] ",
  };
}

export class LainClient {
  constructor(
    url = DEFAULT_URL,
    {
      socketFactory,
      write = (text) => process.stdout.write(text),
      approve = async () => false,
    } = {},
  ) {
    this.url = url;
    this.socketFactory = socketFactory ?? ((address) => new WebSocket(address));
    this.write = write;
    this.approve = approve;
    this.socket = null;
    this.sessionId = null;
    this.activeRunId = null;
    this.messages = [];
    this.pending = null;
    this.startedReply = false;
  }

  connect() {
    return new Promise((resolve, reject) => {
      const socket = this.socketFactory(this.url);
      this.socket = socket;
      const timer = setTimeout(() => reject(new Error("kernel handshake timed out")), 5_000);
      socket.addEventListener("open", () => this.send({ type: "client.hello", versions: [1] }));
      socket.addEventListener("message", (event) => {
        try {
          const packet = JSON.parse(event.data);
          if (!packet || packet.v !== 1 || typeof packet.type !== "string") {
            throw new Error("invalid kernel protocol packet");
          }
          if (packet.type === "server.hello") {
            if (typeof packet.scope?.session_id !== "string") {
              throw new Error("invalid kernel handshake");
            }
            this.sessionId = packet.scope.session_id;
            this.send({ type: "session.subscribe", session_id: this.sessionId, after_seq: 0 });
          } else if (packet.type === "session.snapshot") {
            this.messages = Array.isArray(packet.snapshot?.messages) ? packet.snapshot.messages : [];
            clearTimeout(timer);
            resolve();
          } else if (packet.type === "error" && !this.pending) {
            throw new Error(packet.code ?? "kernel protocol error");
          } else {
            this.handleEvent(packet);
          }
        } catch (error) {
          clearTimeout(timer);
          reject(error);
        }
      });
      socket.addEventListener("error", () => {
        clearTimeout(timer);
        reject(new Error(`cannot connect to ${this.url}`));
      });
      socket.addEventListener("close", () => {
        this.pending?.reject(new Error("kernel connection closed"));
        this.pending = null;
      });
    });
  }

  ask(text) {
    if (!this.sessionId || this.pending) throw new Error("client is not ready");
    this.startedReply = false;
    return new Promise((resolve, reject) => {
      this.pending = { resolve, reject };
      this.send({
        type: "message.create",
        session_id: this.sessionId,
        client_message_id: randomUUID(),
        text,
      });
    });
  }

  cancel() {
    if (this.activeRunId) this.send({ type: "run.cancel", run_id: this.activeRunId });
  }

  close() {
    this.socket?.close();
  }

  status() {
    return `${this.url} / session ${this.sessionId ?? "connecting"}`;
  }

  send(command) {
    this.socket.send(JSON.stringify(command));
  }

  handleEvent(packet) {
    const payload = packet.payload ?? {};
    if (packet.type === "run.queued" || packet.type === "run.started") {
      this.activeRunId = packet.run_id;
    } else if (packet.type === "message.accepted" && typeof payload.content === "string") {
      this.messages.push({ role: "user", content: payload.content });
    } else if (packet.type === "tool.started") {
      this.startedReply = false;
      this.write(`\n${dim(`◇ tool › ${payload.name ?? "unknown"}`)}\n`);
    } else if (packet.type === "tool.completed") {
      this.write(`${dim(`  ${payload.status ?? "unknown"}: ${payload.summary ?? ""}`)}\n`);
    } else if (packet.type === "tool.approval_required") {
      void this.respondToApproval(payload);
    } else if (packet.type === "text.delta") {
      if (typeof payload.text !== "string") return;
      if (!this.startedReply) {
        this.startedReply = true;
        this.write(`\n${pink("◈ lain › ")}`);
      }
      this.write(payload.text);
    } else if (packet.type === "text.completed" && typeof payload.content === "string") {
      this.messages.push({ role: "assistant", content: payload.content });
    } else if (packet.type === "run.completed") {
      this.finish(null, payload.usage);
    } else if (packet.type === "run.cancelled") {
      this.finish(new Error("cancelled"));
    } else if (packet.type === "run.failed" || packet.type === "error") {
      this.finish(new Error(payload.code ?? packet.code ?? "run failed"));
    }
  }

  async respondToApproval(payload) {
    let approved = false;
    try {
      approved = Boolean(await this.approve(payload));
    } catch {}
    this.send({
      type: "tool.approval.respond",
      approval_id: payload.approval_id,
      approved,
    });
  }

  finish(error, usage) {
    const pending = this.pending;
    if (!pending) return;
    this.write("\n");
    this.activeRunId = null;
    this.pending = null;
    if (error) pending.reject(error);
    else pending.resolve(usage ?? null);
  }
}

async function connectClient(url) {
  const client = new LainClient(url);
  await client.connect();
  return client;
}

async function connectOrStartKernel() {
  const configuredUrl = process.env.LAIN_WS_URL;
  if (configuredUrl) {
    return { client: await connectClient(configuredUrl), kernel: null };
  }

  process.stdout.write(dim("starting workspace kernel...\n"));
  await run("cargo", ["build", "--locked", "--bin", "lain-kernel"], KERNEL_DIRECTORY);
  const port = await availablePort();
  let kernelError = "";
  let kernelSpawnError = null;
  const kernel = spawn(KERNEL_BINARY, [], {
    cwd: KERNEL_DIRECTORY,
    env: {
      ...process.env,
      LAIN_BIND_ADDR: `127.0.0.1:${port}`,
      LAIN_WORKSPACE: process.env.LAIN_WORKSPACE ?? process.cwd(),
    },
    stdio: ["ignore", "ignore", "pipe"],
    windowsHide: true,
  });
  kernel.stderr.setEncoding("utf8");
  kernel.stderr.on("data", (chunk) => {
    kernelError = `${kernelError}${chunk}`.slice(-2_000);
  });
  kernel.once("error", (error) => {
    kernelSpawnError = error;
  });
  const url = `ws://127.0.0.1:${port}/v1/ws`;
  for (let attempt = 0; attempt < 50; attempt += 1) {
    if (kernelSpawnError) throw kernelSpawnError;
    if (kernel.exitCode !== null) {
      throw new Error(kernelError.trim() || `kernel exited with code ${kernel.exitCode}`);
    }
    try {
      return { client: await connectClient(url), kernel };
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }
  kernel.kill();
  throw new Error(kernelError.trim() || "local kernel did not become ready");
}

function availablePort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => resolve(address.port));
    });
  });
}

function run(command, args, cwd) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd, stdio: "inherit", windowsHide: true });
    child.once("error", reject);
    child.once("exit", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`${command} exited with code ${code}`));
    });
  });
}

function usageLine(usage) {
  if (!usage) return "";
  const cached = usage.cached_input_tokens ?? 0;
  const cacheRate = usage.input_tokens ? ` (${Math.round((cached / usage.input_tokens) * 100)}%)` : "";
  return dim(
    `tokens: ${usage.input_tokens ?? "?"} in / ${usage.output_tokens ?? "?"} out / ${cached} cached${cacheRate}\n`,
  );
}

export function historyText(messages, limit = 8) {
  const visible = messages
    .filter((message) => ["user", "assistant"].includes(message.role))
    .slice(-limit);
  if (visible.length === 0) return dim("no previous messages\n");
  return `${visible
    .map((message) => {
      const label = message.role === "user" ? "you " : "lain";
      const content = String(message.content ?? "").replaceAll(/\s+/g, " ").trim();
      return `${pink(`${label} › `)}${content.length > 500 ? `${content.slice(0, 500)}...` : content}`;
    })
    .join("\n")}\n`;
}

function help() {
  return [
    "Commands:",
    "  /help    show this help",
    "  /history show recent persisted messages",
    "  /status  show kernel and session",
    "  /quit    close the connection",
  ].join("\n");
}

export async function main(args = process.argv.slice(2)) {
  if (args.includes("--help") || args.includes("-h")) {
    console.log("Usage: lain [-c] [-p \"prompt\"] [initial prompt]");
    return;
  }
  const continueSession = args.includes("-c") || args.includes("--continue");
  const oneShot = args.includes("-p");
  const initialPrompt = args
    .filter((argument) => !["-c", "--continue", "-p"].includes(argument))
    .join(" ")
    .trim();
  if (oneShot && !initialPrompt) throw new Error("-p requires a prompt");

  process.stdout.write(banner());
  const { client, kernel } = await connectOrStartKernel();
  const stopOwnedKernel = () => kernel?.kill();
  process.once("exit", stopOwnedKernel);
  process.stdout.write(`${dim(`connected: ${client.status()}`)}\n`);
  if (continueSession) process.stdout.write(historyText(client.messages));
  const readline = createInterface({ input: process.stdin, output: process.stdout });
  client.approve = async (approval) => {
    const { detail, question } = approvalPrompt(approval);
    const warning = typeof approval.warning === "string" ? `\n${dim(approval.warning)}` : "";
    process.stdout.write(
      `\n${pink(`鈼?approve 鈥?${approval.name ?? "tool"}`)}\n${detail}${warning}\n`,
    );
    if (!process.stdin.isTTY || !process.stdout.isTTY) return false;
    const answer = await readline.question(question);
    return /^(y|yes)$/i.test(answer.trim());
  };

  const ask = async (text) => {
    try {
      const usage = await client.ask(text);
      process.stdout.write(usageLine(usage));
    } catch (error) {
      process.stderr.write(`${error.message}\n`);
    }
  };
  const onInterrupt = () => {
    if (client.activeRunId) client.cancel();
    else readline.close();
  };
  process.on("SIGINT", onInterrupt);
  try {
    if (initialPrompt) await ask(initialPrompt);
    if (oneShot) return;
    while (true) {
      const text = (await readline.question(pink("\n◈ you  › "))).trim();
      if (!text) continue;
      if (text === "/quit" || text === "/exit") break;
      if (text === "/help") {
        console.log(help());
        continue;
      }
      if (text === "/history") {
        process.stdout.write(historyText(client.messages));
        continue;
      }
      if (text === "/status") {
        console.log(client.status());
        continue;
      }
      await ask(text);
    }
  } finally {
    process.off("SIGINT", onInterrupt);
    process.off("exit", stopOwnedKernel);
    readline.close();
    client.close();
    stopOwnedKernel();
  }
}

if (
  process.argv[1] &&
  realpathSync(process.argv[1]) === realpathSync(fileURLToPath(import.meta.url))
) {
  main().catch((error) => {
    process.stderr.write(`lain: ${error.message}\n`);
    process.exitCode = 1;
  });
}
