"use client";

import { useState, useRef, useEffect, useCallback } from "react";

interface CommandMessage {
  id: number;
  type: "user" | "response" | "agent_msg" | "agent_notify";
  text: string;
  ok?: boolean;
  data?: Record<string, unknown>;
  timestamp: Date;
  severity?: string;
}

interface CommandTerminalProps {
  className?: string;
}

export default function CommandTerminal({ className }: CommandTerminalProps) {
  const [messages, setMessages] = useState<CommandMessage[]>([]);
  const [input, setInput] = useState("");
  const [connected, setConnected] = useState(false);
  const ws = useRef<WebSocket | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const msgId = useRef(0);
  const historyRef = useRef<string[]>([]);
  const historyIdx = useRef(-1);

  const connect = useCallback(() => {
    if (ws.current?.readyState === WebSocket.OPEN) return;

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const socket = new WebSocket(`${protocol}//localhost:8080/api/terminal/cmd`);

    socket.onopen = () => {
      setConnected(true);
    };

    socket.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data as string);
        const msg: CommandMessage = {
          id: ++msgId.current,
          type: data.type === "agent_notify" ? "agent_notify" : data.type === "agent_msg" ? "agent_msg" : "response",
          text: data.message || data.error || JSON.stringify(data.data || {}, null, 2),
          ok: data.ok,
          data: data.data,
          timestamp: new Date(),
          severity: data.severity,
        };
        setMessages((prev) => [...prev, msg]);
      } catch {
        const msg: CommandMessage = {
          id: ++msgId.current,
          type: "response",
          text: event.data as string,
          timestamp: new Date(),
        };
        setMessages((prev) => [...prev, msg]);
      }
    };

    socket.onclose = () => {
      setConnected(false);
      reconnectTimer.current = setTimeout(connect, 3000);
    };

    socket.onerror = () => {
      setConnected(false);
    };

    ws.current = socket;
  }, []);

  useEffect(() => {
    connect();
    return () => {
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current);
      ws.current?.close();
    };
  }, [connect]);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages]);

  const handleSend = () => {
    const trimmed = input.trim();
    if (!trimmed) return;
    if (ws.current?.readyState !== WebSocket.OPEN) return;

    // Add to history
    historyRef.current = [trimmed, ...historyRef.current.filter((h) => h !== trimmed)].slice(0, 50);
    historyIdx.current = -1;

    // Add user message to display
    const userMsg: CommandMessage = {
      id: ++msgId.current,
      type: "user",
      text: trimmed,
      timestamp: new Date(),
    };
    setMessages((prev) => [...prev, userMsg]);

    ws.current.send(trimmed);
    setInput("");
    inputRef.current?.focus();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      handleSend();
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      const history = historyRef.current;
      if (history.length > 0) {
        const next = Math.min(historyIdx.current + 1, history.length - 1);
        historyIdx.current = next;
        setInput(history[next]);
      }
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      const history = historyRef.current;
      if (historyIdx.current > 0) {
        historyIdx.current--;
        setInput(history[historyIdx.current]);
      } else {
        historyIdx.current = -1;
        setInput("");
      }
    }
  };

  const getSeverityColor = (severity?: string) => {
    switch (severity) {
      case "critical": return "var(--red)";
      case "warning": return "var(--amber)";
      case "info": return "var(--cyan)";
      default: return "var(--dim)";
    }
  };

  return (
    <div className={className} style={{ display: "flex", flexDirection: "column", height: "100%", width: "100%" }}>
      <div
        ref={scrollRef}
        style={{
          flex: 1,
          minHeight: 0,
          overflowY: "auto",
          padding: "8px 12px",
          fontFamily: '"SF Mono", "JetBrains Mono", "Cascadia Code", Consolas, monospace',
          fontSize: "11px",
          lineHeight: "1.5",
          background: "#0a0c14",
        }}
      >
        {messages.length === 0 && (
          <div style={{ color: "var(--dim)", fontStyle: "italic", padding: "16px 0" }}>
            Command channel ready. Type a command or natural language below.
            <br />
            <br />
            Examples:{" "}
            <span style={{ color: "var(--cyan)" }}>close weth</span> ·{" "}
            <span style={{ color: "var(--cyan)" }}>status</span> ·{" "}
            <span style={{ color: "var(--cyan)" }}>pause</span> ·{" "}
            <span style={{ color: "var(--cyan)" }}>set stop link 7.50</span> ·{" "}
            <span style={{ color: "var(--cyan)" }}>what{"'"}s happening with btc</span>
          </div>
        )}
        {messages.map((msg) => (
          <div key={msg.id} style={{ marginBottom: "6px" }}>
            {msg.type === "user" && (
              <div>
                <span style={{ color: "var(--dim)", marginRight: "8px" }}>
                  {msg.timestamp.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" })}
                </span>
                <span style={{ color: "var(--cyan)" }}>$</span>{" "}
                <span style={{ color: "var(--txt)" }}>{msg.text}</span>
              </div>
            )}
            {msg.type === "response" && (
              <div style={{ paddingLeft: "16px" }}>
                <span style={{ color: "var(--dim)", marginRight: "8px" }}>
                  {msg.timestamp.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" })}
                </span>
                {msg.ok ? (
                  <span>
                    <i className="fa-solid fa-check" style={{ color: "var(--green)", marginRight: "6px", fontSize: "10px" }} />
                    <span style={{ color: "var(--green)" }}>{msg.text}</span>
                  </span>
                ) : (
                  <span>
                    <i className="fa-solid fa-xmark" style={{ color: "var(--red)", marginRight: "6px", fontSize: "10px" }} />
                    <span style={{ color: "var(--red)" }}>{msg.text}</span>
                  </span>
                )}
                {msg.data && (
                  <pre style={{
                    margin: "4px 0 0 22px",
                    padding: "6px 8px",
                    background: "rgba(0,251,255,0.05)",
                    border: "1px solid var(--line)",
                    borderRadius: "4px",
                    color: "var(--dim)",
                    fontSize: "10px",
                    overflowX: "auto",
                    maxHeight: "120px",
                  }}>
                    {JSON.stringify(msg.data, null, 2)}
                  </pre>
                )}
              </div>
            )}
            {(msg.type === "agent_msg" || msg.type === "agent_notify") && (
              <div style={{ paddingLeft: "16px" }}>
                <span style={{ color: "var(--dim)", marginRight: "8px" }}>
                  {msg.timestamp.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" })}
                </span>
                <i className="fa-solid fa-robot" style={{ color: getSeverityColor(msg.severity), marginRight: "6px", fontSize: "10px" }} />
                <span style={{ color: getSeverityColor(msg.severity) }}>{msg.text}</span>
              </div>
            )}
          </div>
        ))}
      </div>
      <div className="flex items-center gap-1.5 px-2 py-1 border-t border-[var(--line)] bg-[#080a10]">
        <span className="text-[var(--cyan)] font-mono text-[10px] shrink-0">
          <i className="fa-solid fa-chevron-right text-[8px] mr-1" />$
        </span>
        <input
          ref={inputRef}
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={connected ? "Type a command... (close weth, status, pause)" : "Connecting..."}
          className="flex-1 bg-transparent text-[var(--txt)] font-mono text-xs outline-none placeholder:text-[var(--dimmer)]"
          disabled={!connected}
          autoFocus
        />
        <button
          onClick={handleSend}
          disabled={!connected}
          className="text-[var(--dim)] hover:text-[var(--cyan)] transition-colors px-1 disabled:opacity-30"
        >
          <i className="fa-solid fa-paper-plane text-[10px]" />
        </button>
      </div>
    </div>
  );
}
