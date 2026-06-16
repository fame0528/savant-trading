"use client";

import { useEffect, useRef, useCallback, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import "@xterm/xterm/css/xterm.css";

interface TerminalPanelProps {
  className?: string;
}

// FID-161: Module-level ref so the copy button in page.tsx can access
// the xterm Terminal API instead of scraping the DOM (which copies
// the entire scrollback buffer instead of visible content).
let _globalTerminal: Terminal | null = null;
export function getGlobalTerminal(): Terminal | null {
  return _globalTerminal;
}

export default function TerminalPanel({ className }: TerminalPanelProps) {
  const termRef = useRef<HTMLDivElement>(null);
  const terminal = useRef<Terminal | null>(null);
  const ws = useRef<WebSocket | null>(null);
  const fitAddon = useRef<FitAddon | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [input, setInput] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const connect = useCallback(() => {
    if (ws.current?.readyState === WebSocket.OPEN) return;

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const socket = new WebSocket(`${protocol}//localhost:8080/api/terminal`);

    socket.onopen = () => {
      terminal.current?.write("\x1b[32m[connected]\x1b[0m\r\n");
    };

    socket.onmessage = (event) => {
      const text = event.data as string;
      // Engine already includes timestamps — don't add client-side prefix.
      // Just pass through the raw output.
      terminal.current?.write(text);
    };

    socket.onclose = () => {
      terminal.current?.write("\r\n\x1b[33m[disconnected]\x1b[0m\r\n");
      reconnectTimer.current = setTimeout(connect, 3000);
    };

    socket.onerror = () => {
      terminal.current?.write("\r\n\x1b[31m[error]\x1b[0m connection failed\r\n");
    };

    ws.current = socket;
  }, []);

  useEffect(() => {
    if (!termRef.current) return;

    const term = new Terminal({
      cursorBlink: true,
      cursorStyle: "block",
      fontSize: 12,
      fontFamily: '"SF Mono", "JetBrains Mono", "Cascadia Code", Consolas, monospace',
      lineHeight: 1.15,
      scrollback: 3000,
      allowProposedApi: true,
      theme: {
        background: "#0a0c14",
        foreground: "#d7def0",
        cursor: "#00fbff",
        cursorAccent: "#0a0c14",
        selectionBackground: "rgba(0, 251, 255, 0.15)",
        black: "#07080d",
        red: "#ff5470",
        green: "#2bf59a",
        yellow: "#ffb347",
        blue: "#00fbff",
        magenta: "#a98cff",
        cyan: "#00fbff",
        white: "#d7def0",
        brightBlack: "#444c66",
        brightRed: "#ff5470",
        brightGreen: "#2bf59a",
        brightYellow: "#ffb347",
        brightBlue: "#00fbff",
        brightMagenta: "#a98cff",
        brightCyan: "#00fbff",
        brightWhite: "#ffffff",
      },
    });

    const fit = new FitAddon();
    const links = new WebLinksAddon();

    term.loadAddon(fit);
    term.loadAddon(links);
    term.open(termRef.current);

    requestAnimationFrame(() => {
      fit.fit();
    });

    terminal.current = term;
    fitAddon.current = fit;
    _globalTerminal = term;

    term.onData((data) => {
      if (ws.current?.readyState === WebSocket.OPEN) {
        ws.current.send(data);
      }
    });

    connect();

    const handleResize = () => {
      requestAnimationFrame(() => fit.fit());
    };
    window.addEventListener("resize", handleResize);

    const observer = new ResizeObserver(handleResize);
    observer.observe(termRef.current);

    return () => {
      window.removeEventListener("resize", handleResize);
      observer.disconnect();
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current);
      ws.current?.close();
      _globalTerminal = null;
      term.dispose();
    };
  }, [connect]);

  const handleSend = () => {
    if (!input.trim()) return;
    if (ws.current?.readyState === WebSocket.OPEN) {
      ws.current.send(input + "\r");
    }
    setInput("");
    inputRef.current?.focus();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      handleSend();
    }
    if (e.key === "c" && e.ctrlKey) {
      e.preventDefault();
      if (ws.current?.readyState === WebSocket.OPEN) {
        ws.current.send("\x03");
      }
    }
  };

  return (
    <div className={className} style={{ display: "flex", flexDirection: "column", height: "100%", width: "100%" }}>
      <div
        ref={termRef}
        style={{ flex: 1, minHeight: 0, padding: "2px 4px" }}
        onClick={() => terminal.current?.focus()}
      />
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
          placeholder="Engine log viewer — click terminal to scroll, Ctrl+C to interrupt"
          className="flex-1 bg-transparent text-[var(--txt)] font-mono text-xs outline-none placeholder:text-[var(--dimmer)]"
          autoFocus
        />
        <button
          onClick={handleSend}
          className="text-[var(--dim)] hover:text-[var(--cyan)] transition-colors px-1"
        >
          <i className="fa-solid fa-paper-plane text-[10px]" />
        </button>
      </div>
    </div>
  );
}
