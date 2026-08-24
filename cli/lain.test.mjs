import assert from "node:assert/strict";
import test from "node:test";

import { LainClient } from "./lain.mjs";

class FakeSocket {
  constructor() {
    this.handlers = new Map();
    this.sent = [];
  }

  addEventListener(name, handler) {
    this.handlers.set(name, handler);
  }

  emit(name, data) {
    this.handlers.get(name)?.({ data });
  }

  packet(packet) {
    this.emit("message", JSON.stringify(packet));
  }

  send(message) {
    this.sent.push(JSON.parse(message));
  }

  close() {
    this.emit("close");
  }
}

test("negotiates v1 and streams one turn", async () => {
  const socket = new FakeSocket();
  let output = "";
  const client = new LainClient("ws://test", {
    socketFactory: () => socket,
    write: (text) => {
      output += text;
    },
  });

  const connected = client.connect();
  socket.emit("open");
  assert.deepEqual(socket.sent[0], { type: "client.hello", versions: [1] });
  socket.packet({ v: 1, type: "server.hello", scope: { session_id: "session-1" } });
  assert.equal(socket.sent[1].type, "session.subscribe");
  socket.packet({ v: 1, type: "session.snapshot", snapshot: {} });
  await connected;

  const answer = client.ask("hello");
  assert.equal(socket.sent[2].type, "message.create");
  socket.packet({ v: 1, type: "run.started", run_id: "run-1", payload: {} });
  socket.packet({ v: 1, type: "model.started", run_id: "run-1", payload: {} });
  socket.packet({ v: 1, type: "text.delta", run_id: "run-1", payload: { text: "hi" } });
  socket.packet({
    v: 1,
    type: "run.completed",
    run_id: "run-1",
    payload: { usage: { input_tokens: 3, output_tokens: 1, cached_input_tokens: 2 } },
  });

  assert.deepEqual(await answer, { input_tokens: 3, output_tokens: 1, cached_input_tokens: 2 });
  assert.match(output, /lain › hi/);
});
