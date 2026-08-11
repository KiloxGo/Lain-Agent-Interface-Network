import { create } from "zustand";

import type { ClientCommand } from "../protocol/generated/ClientCommand";
import type { EventEnvelope } from "../protocol/generated/EventEnvelope";
import type { MessageRecord } from "../protocol/generated/MessageRecord";
import type { SessionSnapshot } from "../protocol/generated/SessionSnapshot";
import type { StorageScope } from "../protocol/generated/StorageScope";

export type MessageStatus = "streaming" | "complete" | "cancelled" | "error";

export type ChatViewMessage = {
  id: string;
  runId: string | null;
  clientMessageId: string | null;
  role: "user" | "assistant";
  text: string;
  status: MessageStatus;
};

type ConnectionStatus = "disconnected" | "connecting" | "connected";

type ConnectionStore = {
  status: ConnectionStatus;
  scope: StorageScope | null;
  sessionId: string | null;
  lastSeq: number;
  messages: ChatViewMessage[];
  activeRunId: string | null;
  error: string | null;
  connect: () => void;
  disconnect: () => void;
  sendMessage: (text: string) => boolean;
  cancelRun: () => void;
};

const CURSOR_KEY = "lain.protocol.v1.cursor";
const MAX_BACKOFF_MS = 10_000;

let socket: WebSocket | null = null;
let generation = 0;
let retryAttempt = 0;
let retryTimer: number | null = null;
let heartbeatTimer: number | null = null;
let intentionallyClosed = false;

export const useConnectionStore = create<ConnectionStore>((_set, get) => ({
  status: "disconnected",
  scope: null,
  sessionId: null,
  lastSeq: 0,
  messages: [],
  activeRunId: null,
  error: null,
  connect: () => connectSocket(),
  disconnect: () => disconnectSocket(),
  sendMessage: (text) => {
    const content = text.trim();
    const { sessionId, status, activeRunId } = get();
    if (
      !content ||
      !sessionId ||
      status !== "connected" ||
      activeRunId ||
      socket?.readyState !== WebSocket.OPEN
    ) {
      return false;
    }
    return sendCommand({
      type: "message.create",
      session_id: sessionId,
      client_message_id: crypto.randomUUID(),
      text: content,
    });
  },
  cancelRun: () => {
    const runId = get().activeRunId;
    if (runId) {
      sendCommand({ type: "run.cancel", run_id: runId });
    }
  },
}));

function websocketUrl() {
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${window.location.host}/v1/ws`;
}

function connectSocket() {
  intentionallyClosed = false;
  clearRetry();
  if (socket?.readyState === WebSocket.OPEN || socket?.readyState === WebSocket.CONNECTING) {
    return;
  }
  const currentGeneration = ++generation;
  useConnectionStore.setState({ status: "connecting", error: null });
  const connection = new WebSocket(websocketUrl());
  socket = connection;

  connection.onopen = () => {
    if (currentGeneration !== generation) return;
    sendCommand({ type: "client.hello", versions: [1] });
  };
  connection.onmessage = (message) => {
    if (currentGeneration !== generation || typeof message.data !== "string") return;
    handlePacket(message.data);
  };
  connection.onerror = () => {
    if (currentGeneration === generation) {
      useConnectionStore.setState({ error: "connection_error" });
    }
  };
  connection.onclose = () => {
    if (currentGeneration !== generation) return;
    socket = null;
    clearHeartbeat();
    useConnectionStore.setState({ status: "disconnected" });
    if (!intentionallyClosed) scheduleReconnect();
  };
}

function disconnectSocket() {
  intentionallyClosed = true;
  generation += 1;
  clearRetry();
  clearHeartbeat();
  const connection = socket;
  socket = null;
  if (connection && connection.readyState < WebSocket.CLOSING) {
    connection.close(1000, "component unmounted");
  }
  useConnectionStore.setState({ status: "disconnected" });
}

function scheduleReconnect() {
  clearRetry();
  const base = Math.min(250 * 2 ** retryAttempt, MAX_BACKOFF_MS);
  const jitter = Math.floor(Math.random() * Math.max(1, base / 4));
  retryAttempt += 1;
  retryTimer = window.setTimeout(connectSocket, base + jitter);
}

function clearRetry() {
  if (retryTimer !== null) window.clearTimeout(retryTimer);
  retryTimer = null;
}

function startHeartbeat(intervalMs: number) {
  clearHeartbeat();
  heartbeatTimer = window.setInterval(() => {
    sendCommand({ type: "client.ping", nonce: crypto.randomUUID() });
  }, Math.max(5_000, intervalMs));
}

function clearHeartbeat() {
  if (heartbeatTimer !== null) window.clearInterval(heartbeatTimer);
  heartbeatTimer = null;
}

function sendCommand(command: ClientCommand) {
  if (socket?.readyState !== WebSocket.OPEN) return false;
  socket.send(JSON.stringify(command));
  return true;
}

function handlePacket(raw: string) {
  let packet: unknown;
  try {
    packet = JSON.parse(raw);
  } catch {
    protocolFailure("invalid_json");
    return;
  }
  if (!isRecord(packet) || typeof packet.type !== "string" || packet.v !== 1) {
    protocolFailure("invalid_packet");
    return;
  }

  if (packet.type === "server.hello") {
    if (!isScope(packet.scope) || typeof packet.heartbeat_interval_ms !== "number") {
      protocolFailure("invalid_server_hello");
      return;
    }
    retryAttempt = 0;
    const current = useConnectionStore.getState();
    const afterSeq =
      current.sessionId === packet.scope.session_id && current.messages.length > 0
        ? current.lastSeq
        : 0;
    useConnectionStore.setState({
      status: "connected",
      scope: packet.scope,
      sessionId: packet.scope.session_id,
      lastSeq: afterSeq,
      error: null,
    });
    sendCommand({
      type: "session.subscribe",
      session_id: packet.scope.session_id,
      after_seq: afterSeq,
    });
    startHeartbeat(packet.heartbeat_interval_ms);
    return;
  }

  if (packet.type === "session.snapshot") {
    if (!isSessionSnapshot(packet.snapshot)) {
      protocolFailure("invalid_snapshot");
      return;
    }
    applySnapshot(packet.snapshot);
    return;
  }

  if (packet.type === "server.pong") return;
  if (packet.type === "error") {
    useConnectionStore.setState({
      error: typeof packet.code === "string" ? packet.code : "protocol_error",
    });
    return;
  }
  if (isEventEnvelope(packet)) applyEvent(packet);
  else protocolFailure("invalid_event");
}

function applySnapshot(snapshot: SessionSnapshot) {
  const messages = snapshot.messages.flatMap(toViewMessage);
  useConnectionStore.setState({
    sessionId: snapshot.session_id,
    lastSeq: snapshot.last_seq,
    messages,
    activeRunId: snapshot.active_run?.id ?? null,
    error: null,
  });
  saveCursor(snapshot.session_id, snapshot.last_seq);
}

function applyEvent(event: EventEnvelope) {
  const state = useConnectionStore.getState();
  if (event.session_id !== state.sessionId) return;
  if (event.seq !== null) {
    if (event.seq <= state.lastSeq) return;
    if (event.seq !== state.lastSeq + 1) {
      protocolFailure("event_seq_gap");
      return;
    }
  }
  const payload = isRecord(event.payload) ? event.payload : {};
  const runId = event.run_id;

  useConnectionStore.setState((current) => {
    let messages = current.messages;
    let activeRunId = current.activeRunId;
    let error = current.error;
    if (
      event.type === "message.accepted" &&
      typeof payload.message_id === "string" &&
      typeof payload.client_message_id === "string" &&
      typeof payload.content === "string"
    ) {
      if (!messages.some((message) => message.id === payload.message_id)) {
        messages = [
          ...messages,
          {
            id: payload.message_id,
            runId,
            clientMessageId: payload.client_message_id,
            role: "user",
            text: payload.content,
            status: "complete",
          },
        ];
      }
    } else if (event.type === "run.queued" || event.type === "run.started") {
      activeRunId = runId;
    } else if (event.type === "model.started" && runId) {
      if (!messages.some((message) => message.runId === runId && message.role === "assistant")) {
        messages = [
          ...messages,
          {
            id: `${runId}:stream`,
            runId,
            clientMessageId: null,
            role: "assistant",
            text: "",
            status: "streaming",
          },
        ];
      }
    } else if (event.type === "text.delta" && runId && typeof payload.text === "string") {
      messages = messages.map((message) =>
        message.runId === runId && message.role === "assistant"
          ? { ...message, text: message.text + payload.text }
          : message,
      );
    } else if (
      event.type === "text.completed" &&
      runId &&
      typeof payload.message_id === "string" &&
      typeof payload.content === "string"
    ) {
      const found = messages.some(
        (message) => message.runId === runId && message.role === "assistant",
      );
      messages = found
        ? messages.map((message) =>
            message.runId === runId && message.role === "assistant"
              ? {
                  ...message,
                  id: payload.message_id as string,
                  text: payload.content as string,
                  status: "complete" as const,
                }
              : message,
          )
        : [
            ...messages,
            {
              id: payload.message_id,
              runId,
              clientMessageId: null,
              role: "assistant",
              text: payload.content,
              status: "complete",
            },
          ];
    } else if (event.type === "run.completed") {
      activeRunId = null;
    } else if (event.type === "run.cancelled" || event.type === "run.failed") {
      messages = messages.map((message) =>
        message.runId === runId && message.role === "assistant"
          ? {
              ...message,
              status: event.type === "run.cancelled" ? "cancelled" : "error",
            }
          : message,
      );
      activeRunId = null;
      error = event.type === "run.failed" && typeof payload.code === "string" ? payload.code : error;
    }
    return {
      messages,
      activeRunId,
      error,
      lastSeq: event.seq ?? current.lastSeq,
    };
  });
  if (event.seq !== null && event.session_id) saveCursor(event.session_id, event.seq);
}

function protocolFailure(code: string) {
  useConnectionStore.setState({ error: code });
  if (socket?.readyState === WebSocket.OPEN) socket.close(1002, code);
}

function toViewMessage(message: MessageRecord): ChatViewMessage[] {
  if (message.role !== "user" && message.role !== "assistant") return [];
  return [
    {
      id: message.id,
      runId: message.run_id,
      clientMessageId: message.client_message_id,
      role: message.role,
      text: message.content,
      status: message.status === "complete" ? "complete" : "error",
    },
  ];
}

function saveCursor(sessionId: string, lastSeq: number) {
  window.localStorage.setItem(CURSOR_KEY, JSON.stringify({ sessionId, lastSeq }));
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isScope(value: unknown): value is StorageScope {
  return (
    isRecord(value) &&
    typeof value.instance_id === "string" &&
    typeof value.user_id === "string" &&
    typeof value.session_id === "string"
  );
}

function isEventEnvelope(value: Record<string, unknown>): value is EventEnvelope {
  return (
    value.v === 1 &&
    typeof value.event_id === "string" &&
    typeof value.instance_id === "string" &&
    (value.session_id === null || typeof value.session_id === "string") &&
    (value.run_id === null || typeof value.run_id === "string") &&
    (value.seq === null || typeof value.seq === "number") &&
    typeof value.type === "string" &&
    typeof value.ts === "string" &&
    "payload" in value
  );
}

function isSessionSnapshot(value: unknown): value is SessionSnapshot {
  return (
    isRecord(value) &&
    typeof value.session_id === "string" &&
    typeof value.last_seq === "number" &&
    Array.isArray(value.messages) &&
    (value.active_run === null || isRecord(value.active_run))
  );
}
