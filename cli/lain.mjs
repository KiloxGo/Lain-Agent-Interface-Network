#!/usr/bin/env node

import { randomUUID } from "node:crypto";
import { createInterface } from "node:readline/promises";
import { pathToFileURL } from "node:url";

const DEFAULT_URL = "ws://127.0.0.1:3000/v1/ws";
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

export class LainClient {
  constructor(url = DEFAULT_URL, { socketFactory, write = (text) => process.stdout.write(text) } = {}) {
    this.url = url;
    this.socketFactory = socketFactory ?? ((address) => new WebSocket(address));
    this.write = write;
    this.socket = null;
    this.sessionId = null;
    this.activeRunId = null;
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
    } else if (packet.type === "model.started") {
      this.startedReply = true;
      this.write(`\n${pink("◈ lain › ")}`);
    } else if (packet.type === "text.delta") {
      if (typeof payload.text !== "string") return;
      if (!this.startedReply) {
        this.startedReply = true;
        this.write(`\n${pink("◈ lain › ")}`);
      }
      this.write(payload.text);
    } else if (packet.type === "run.completed") {
      this.finish(null, payload.usage);
    } else if (packet.type === "run.cancelled") {
      this.finish(new Error("cancelled"));
    } else if (packet.type === "run.failed" || packet.type === "error") {
      this.finish(new Error(payload.code ?? packet.code ?? "run failed"));
    }
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

function usageLine(usage) {
  if (!usage) return "";
  return dim(
    `tokens: ${usage.input_tokens ?? "?"} in / ${usage.output_tokens ?? "?"} out / ${usage.cached_input_tokens ?? 0} cached\n`,
  );
}

function help() {
  return [
    "Commands:",
    "  /help    show this help",
    "  /status  show kernel and session",
    "  /quit    close the connection",
  ].join("\n");
}

export async function main(args = process.argv.slice(2)) {
  if (args.includes("--help") || args.includes("-h")) {
    console.log("Usage: node cli/lain.mjs [-p \"prompt\"] [initial prompt]");
    return;
  }
  const oneShot = args[0] === "-p";
  const initialPrompt = (oneShot ? args.slice(1) : args).join(" ").trim();
  if (oneShot && !initialPrompt) throw new Error("-p requires a prompt");

  process.stdout.write(banner());
  const client = new LainClient(process.env.LAIN_WS_URL ?? DEFAULT_URL);
  await client.connect();
  process.stdout.write(`${dim(`connected: ${client.status()}`)}\n`);

  const ask = async (text) => {
    try {
      const usage = await client.ask(text);
      process.stdout.write(usageLine(usage));
    } catch (error) {
      process.stderr.write(`${error.message}\n`);
    }
  };
  if (initialPrompt) await ask(initialPrompt);
  if (oneShot) {
    client.close();
    return;
  }

  const readline = createInterface({ input: process.stdin, output: process.stdout });
  const onInterrupt = () => {
    if (client.activeRunId) client.cancel();
    else readline.close();
  };
  process.on("SIGINT", onInterrupt);
  try {
    while (true) {
      const text = (await readline.question(pink("\n◈ you  › "))).trim();
      if (!text) continue;
      if (text === "/quit" || text === "/exit") break;
      if (text === "/help") {
        console.log(help());
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
    readline.close();
    client.close();
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(`lain: ${error.message}\n`);
    process.exitCode = 1;
  });
}
