"use client";

import { useState, useEffect } from "react";
import TerminalPanel from "./Terminal";
import CommandTerminal from "./CommandTerminal";

type TabId = "logs" | "command";

interface TerminalContainerProps {
  className?: string;
}

export default function TerminalContainer({ className }: TerminalContainerProps) {
  const [activeTab, setActiveTab] = useState<TabId>("logs");
  const [cmdUnread, setCmdUnread] = useState(0);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === "l") {
        e.preventDefault();
        setActiveTab("logs");
      }
      if (e.ctrlKey && e.key === "k") {
        e.preventDefault();
        setActiveTab("command");
        setCmdUnread(0);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const switchTab = (tab: TabId) => {
    setActiveTab(tab);
    if (tab === "command") setCmdUnread(0);
  };

  return (
    <div className={className} style={{ display: "flex", flexDirection: "column", height: "100%", width: "100%" }}>
      {/* Tab bar */}
      <div style={{
        display: "flex",
        alignItems: "center",
        gap: "0px",
        borderBottom: "1px solid var(--line)",
        background: "#080a10",
        padding: "0 8px",
        minHeight: "28px",
      }}>
        <TabButton
          label="Logs"
          icon="fa-terminal"
          active={activeTab === "logs"}
          onClick={() => switchTab("logs")}
          shortcut="Ctrl+L"
        />
        <TabButton
          label="Command"
          icon="fa-keyboard"
          active={activeTab === "command"}
          onClick={() => switchTab("command")}
          shortcut="Ctrl+K"
          badge={cmdUnread > 0 ? cmdUnread : undefined}
        />
        <div style={{ flex: 1 }} />
        <div style={{
          width: "6px",
          height: "6px",
          borderRadius: "50%",
          background: activeTab === "logs" ? "var(--green)" : "var(--cyan)",
          boxShadow: `0 0 4px ${activeTab === "logs" ? "var(--green)" : "var(--cyan)"}`,
        }} />
      </div>

      {/* Tab content */}
      <div style={{ flex: 1, minHeight: 0, display: activeTab === "logs" ? "block" : "none" }}>
        <TerminalPanel className="h-full" />
      </div>
      <div style={{ flex: 1, minHeight: 0, display: activeTab === "command" ? "block" : "none" }}>
        <CommandTerminal className="h-full" />
      </div>
    </div>
  );
}

function TabButton({
  label,
  icon,
  active,
  onClick,
  shortcut,
  badge,
}: {
  label: string;
  icon: string;
  active: boolean;
  onClick: () => void;
  shortcut: string;
  badge?: number;
}) {
  return (
    <button
      onClick={onClick}
      title={`${label} (${shortcut})`}
      style={{
        display: "flex",
        alignItems: "center",
        gap: "5px",
        padding: "4px 12px",
        background: "transparent",
        border: "none",
        borderBottom: active ? "2px solid var(--cyan)" : "2px solid transparent",
        color: active ? "var(--cyan)" : "var(--dim)",
        fontFamily: '"SF Mono", "JetBrains Mono", Consolas, monospace',
        fontSize: "10px",
        fontWeight: active ? 600 : 400,
        cursor: "pointer",
        transition: "all 0.15s ease",
        textTransform: "uppercase",
        letterSpacing: "0.5px",
      }}
    >
      <i className={`fa-solid ${icon}`} style={{ fontSize: "9px" }} />
      {label}
      {badge !== undefined && (
        <span style={{
          background: "var(--red)",
          color: "#fff",
          fontSize: "8px",
          padding: "1px 4px",
          borderRadius: "8px",
          minWidth: "14px",
          textAlign: "center",
        }}>
          {badge}
        </span>
      )}
    </button>
  );
}
