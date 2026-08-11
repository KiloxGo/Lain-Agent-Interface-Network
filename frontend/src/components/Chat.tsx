import { useEffect, useState } from "react";

import { useConnectionStore } from "../stores/connectionStore";

function Chat() {
  const [text, setText] = useState("");
  const status = useConnectionStore((state) => state.status);
  const messages = useConnectionStore((state) => state.messages);
  const activeRunId = useConnectionStore((state) => state.activeRunId);
  const error = useConnectionStore((state) => state.error);
  const connect = useConnectionStore((state) => state.connect);
  const disconnect = useConnectionStore((state) => state.disconnect);
  const sendMessage = useConnectionStore((state) => state.sendMessage);
  const cancelRun = useConnectionStore((state) => state.cancelRun);
  const connected = status === "connected";

  useEffect(() => {
    connect();
    return disconnect;
  }, [connect, disconnect]);

  const send = () => {
    if (sendMessage(text)) setText("");
  };

  return (
    <section aria-label="Chat">
      <p>{status === "connected" ? "Connected" : status === "connecting" ? "Connecting…" : "Disconnected"}</p>
      {error ? <p role="status">...Noise...[{error}]</p> : null}
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
        disabled={!connected || Boolean(activeRunId)}
        aria-label="Message"
      />
      <button
        type="button"
        onClick={send}
        disabled={!connected || Boolean(activeRunId) || !text.trim()}
      >
        Send
      </button>
      <button type="button" onClick={cancelRun} disabled={!activeRunId}>
        Stop
      </button>
    </section>
  );
}

export default Chat;
