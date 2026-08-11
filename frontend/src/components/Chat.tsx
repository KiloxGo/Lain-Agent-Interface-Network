import { useEffect, useRef, useState } from "react";

type MessageStatus = "streaming" | "complete" | "cancelled" | "error";

type ChatMessage = {
  id: string;
  role: "user" | "assistant";
  text: string;
  status: MessageStatus;
};

type ServerEvent =
  | { type: "started"; request_id: string; model: string }
  | { type: "delta"; text: string }
  | {
      type: "finished";
      reason: string;
      usage?: {
        input_tokens: number;
        output_tokens: number;
        cached_input_tokens: number;
      };
    }
  | { type: "error"; code: string; partial: boolean };

function socketUrl() {
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${window.location.host}/ws/stream`;
}

function Chat() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [text, setText] = useState("");
  const [connected, setConnected] = useState(false);
  const [running, setRunning] = useState(false);
  const socket = useRef<WebSocket | null>(null);
  const activeAssistantId = useRef<string | null>(null);

  useEffect(() => {
    const connection = new WebSocket(socketUrl());
    socket.current = connection;
    connection.onopen = () => setConnected(true);
    connection.onclose = () => {
      setConnected(false);
      setRunning(false);
    };
    connection.onerror = () => setConnected(false);
    connection.onmessage = (message) => {
      let event: ServerEvent;
      try {
        event = JSON.parse(message.data) as ServerEvent;
      } catch {
        return;
      }

      if (event.type === "started") {
        const id = event.request_id;
        activeAssistantId.current = id;
        setMessages((current) => [
          ...current,
          { id, role: "assistant", text: "", status: "streaming" },
        ]);
      } else if (event.type === "delta") {
        const id = activeAssistantId.current;
        if (!id) return;
        setMessages((current) =>
          current.map((item) =>
            item.id === id ? { ...item, text: item.text + event.text } : item,
          ),
        );
      } else if (event.type === "finished") {
        const id = activeAssistantId.current;
        if (id) {
          const status = event.reason === "cancelled" ? "cancelled" : "complete";
          setMessages((current) =>
            current.map((item) => (item.id === id ? { ...item, status } : item)),
          );
        }
        activeAssistantId.current = null;
        setRunning(false);
      } else if (event.type === "error") {
        const id = activeAssistantId.current ?? crypto.randomUUID();
        setMessages((current) => {
          const existing = current.some((item) => item.id === id);
          if (existing) {
            return current.map((item) =>
              item.id === id
                ? {
                    ...item,
                    text: item.text || `...Noise...[${event.code}]`,
                    status: "error",
                  }
                : item,
            );
          }
          return [
            ...current,
            {
              id,
              role: "assistant",
              text: `...Noise...[${event.code}]`,
              status: "error",
            },
          ];
        });
        activeAssistantId.current = null;
        setRunning(false);
      }
    };

    return () => {
      connection.close();
      socket.current = null;
    };
  }, []);

  const send = () => {
    const content = text.trim();
    if (!content || !connected || running || socket.current?.readyState !== WebSocket.OPEN) {
      return;
    }
    setMessages((current) => [
      ...current,
      {
        id: crypto.randomUUID(),
        role: "user",
        text: content,
        status: "complete",
      },
    ]);
    socket.current.send(JSON.stringify({ type: "message", text: content }));
    setText("");
    setRunning(true);
  };

  const cancel = () => {
    if (running && socket.current?.readyState === WebSocket.OPEN) {
      socket.current.send(JSON.stringify({ type: "cancel" }));
    }
  };

  return (
    <section aria-label="Chat">
      <p>{connected ? "Connected" : "Disconnected"}</p>
      <div aria-live="polite">
        {messages.map((message) => (
          <p key={message.id} data-role={message.role} data-status={message.status}>
            <strong>{message.role === "user" ? "You" : "Lain"}:</strong>{" "}
            {message.text || (message.status === "streaming" ? "…" : "")}
          </p>
        ))}
      </div>
      <input
        value={text}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") send();
        }}
        disabled={!connected || running}
        aria-label="Message"
      />
      <button type="button" onClick={send} disabled={!connected || running || !text.trim()}>
        Send
      </button>
      <button type="button" onClick={cancel} disabled={!running}>
        Stop
      </button>
    </section>
  );
}

export default Chat;
