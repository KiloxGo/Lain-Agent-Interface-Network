import { useState, useEffect, useRef } from "react";

function Chat() {
  const [msgs, setMsgs] = useState<string[]>([]);
  const [text, setText] = useState("");
  const ws = useRef<WebSocket | null>(null);

  useEffect(() => {
    ws.current = new WebSocket("ws://localhost:5173/ws");
    ws.current.onmessage = (e) => setMsgs((prev) => [...prev, e.data]);
    return () => ws.current?.close();
  }, []);

  return (
    <>
      {msgs.map((m, i) => (
        <p key={i}>{m}</p>
      ))}
      <input value={text} onChange={(e) => setText(e.target.value)} />
      <button
        type="button"
        onClick={() => {
          ws.current?.send(text);
          setText("");
        }}
      >
        发送
      </button>
    </>
  );
}

export default Chat;
