"use client";

import Image from "next/image";
import dynamic from "next/dynamic";
import { formatTime12h, formatTimeShort } from "@/lib/time";
import {
  ProgressBar,
  ProgressBarRoot,
  ProgressBarFill,
  Chip,
  Table,
  Tooltip,
  Spinner,
  Separator,
  Label,
} from "@heroui/react";
import { useDashboard } from "@/hooks/useDashboard";
import { copyFormatters, downloadTradesCSV } from "@/lib/copy";
import { sounds } from "@/lib/sounds";
import { useEffect, useRef } from "react";
import toast, { Toaster } from "react-hot-toast";
import dayjs from "dayjs";
import relativeTime from "dayjs/plugin/relativeTime";
import ErrorBoundary from "@/components/ErrorBoundary";
import Ticker from "@/components/Ticker";

dayjs.extend(relativeTime);

const TerminalPanel = dynamic(() => import("@/components/Terminal"), { ssr: false });
const EquityChart = dynamic(() => import("@/components/EquityChart"), { ssr: false });

const fmt = {
  usd: (v: number) =>
    (v < 0 ? "-$" : "$") + Math.abs(v).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 }),
  pct: (v: number) => (v >= 0 ? "+" : "") + v.toFixed(2) + "%",
  price: (p: number) => {
    if (p === 0) return "0";
    if (p >= 1000) return p.toLocaleString("en-US", { maximumFractionDigits: 2 });
    if (p >= 1) return p.toFixed(4);
    if (p >= 0.001) return p.toFixed(6);
    return p.toPrecision(3);
  },
  uptime: (sec: number) => {
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    return h ? `${h}h ${m}m` : `${m}m`;
  },
};

const pnlClass = (v: number) => (v >= 0 ? "text-[var(--green)]" : "text-[var(--red)]");

function Icon({ name, className = "" }: { name: string; className?: string }) {
  return <i className={`fa-solid ${name} ${className}`} />;
}

function KPI({ icon, label, value, sub, color }: { icon: string; label: string; value: string; sub?: string; color?: string }) {
  return (
    <div className="bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md p-2 flex flex-col justify-center">
      <div className="flex items-center gap-1.5 mb-1">
        <Icon name={icon} className={`text-[9px] ${color ?? "text-[var(--dim)]"}`} />
        <p className="text-[9px] tracking-[1.5px] uppercase text-[var(--dim)]">{label}</p>
      </div>
      <p className="text-xl font-bold font-mono tabular-nums leading-tight">{value}</p>
      {sub && <p className="text-[10px] mt-0.5 text-[var(--dim)]">{sub}</p>}
    </div>
  );
}

function FearGauge({ value }: { value: number | null }) {
  if (value == null) return <div className="text-[var(--dimmer)] text-xs"><Icon name="fa-chart-simple" className="mr-1" />No data</div>;
  const frac = Math.max(0, Math.min(100, value)) / 100;
  const hue = frac * 120;
  return (
    <div className="relative w-[100px] h-[60px]">
      <svg viewBox="0 0 130 80" className="w-full h-full">
        <path d="M 15 72 A 52 52 0 0 1 115 72" fill="none" stroke="rgba(255,255,255,0.07)" strokeWidth="10" strokeLinecap="round" />
        <path d="M 15 72 A 52 52 0 0 1 115 72" fill="none" stroke={`hsl(${hue},85%,55%)`} strokeWidth="10" strokeLinecap="round" strokeDasharray={`${frac * 163} 163`} />
        <text x="65" y="66" textAnchor="middle" fill="#fff" fontSize="22" fontWeight="700">{value}</text>
      </svg>
    </div>
  );
}

function MetricRow({ icon, label, value, color }: { icon: string; label: string; value: React.ReactNode; color?: string }) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-[10px] text-[var(--dim)] flex items-center gap-1.5">
        <Icon name={icon} className="text-[7px]" />{label}
      </span>
      <span className={`text-[13px] font-mono font-semibold ${color ?? ""}`}>{value}</span>
    </div>
  );
}

function RiskBar({ icon, label, value, max, tooltip }: { icon: string; label: string; value: number; max: number; tooltip?: string }) {
  const pct = max > 0 ? (value / max) * 100 : 0;
  const color = pct < 50 ? "success" : pct < 80 ? "warning" : "danger";
  return (
    <Tooltip delay={300}>
      <div className="cursor-help">
        <ProgressBar aria-label={label} size="md" value={pct} color={color}>
          <Label>
            <span className="text-[10px] text-[var(--dim)] flex items-center gap-1.5">
              <Icon name={icon} className="text-[7px]" />{label}
            </span>
          </Label>
          <ProgressBar.Output>
            <span className="text-[13px] font-mono font-semibold">
              {(value * 100).toFixed(1)}% / {(max * 100).toFixed(0)}%
            </span>
          </ProgressBar.Output>
          <ProgressBar.Track>
            <ProgressBar.Fill />
          </ProgressBar.Track>
        </ProgressBar>
      </div>
      {tooltip && <Tooltip.Content showArrow><p className="text-[10px]">{tooltip}</p></Tooltip.Content>}
    </Tooltip>
  );
}

function CopyButton({ text, title }: { text: () => string; title?: string }) {
  return (
    <button
      onClick={() => navigator.clipboard.writeText(text())}
      className="inline-flex items-center justify-center text-[var(--dim)] hover:text-[var(--cyan)] transition-colors cursor-pointer leading-none"
      title={title ?? "Copy to clipboard"}
    >
      <Icon name="fa-copy" className="text-[9px]" />
    </button>
  );
}

function SectionHeader({ icon, title, tag, tagColor, onCopy }: { icon: string; title: string; tag?: string; tagColor?: string; onCopy?: () => string }) {
  return (
    <div className="flex items-center gap-2 px-3 pt-2 pb-1 border-b border-[var(--line)]">
      <span className="inline-flex items-center"><Icon name={icon} className="text-[var(--dim)] text-[10px]" /></span>
      <span className="text-[10px] tracking-[2px] uppercase font-semibold text-[var(--dim)] leading-none">{title}</span>
      {tag && <span className={`ml-auto text-[9px] font-bold leading-none ${tagColor ?? "text-[var(--cyan)]"}`}>{tag}</span>}
      {onCopy && <span className="ml-auto inline-flex items-center"><CopyButton text={onCopy} title={`Copy ${title.toLowerCase()}`} /></span>}
    </div>
  );
}

export default function Dashboard() {
  const state = useDashboard(4000);
  const { status, portfolio, positions, trades, decisions, activity, insight, risk, session, memory, config, online } = state;
  const prevTradeCount = useRef(trades.length);
  const prevPosCount = useRef(positions.length);
  const prevStops = useRef<Record<string, number>>({});

  const live = (status?.mode ?? "").toUpperCase() === "LIVE";
  const eq = portfolio?.equity ?? portfolio?.balance ?? 0;

  // Sound effects on state changes
  useEffect(() => {
    if (trades.length > prevTradeCount.current) {
      const latest = trades[0];
      if (latest) {
        if (latest.notes?.includes("Stop loss")) {
          sounds.stopLoss();
          toast.error(`Stop loss hit: ${latest.pair} — ${fmt.usd(latest.pnl)}`, { icon: "🛑" });
        } else if (latest.notes?.includes("TP")) {
          sounds.takeProfit();
          toast.success(`Take profit: ${latest.pair} — ${fmt.usd(latest.pnl)}`, { icon: "🎯" });
        } else {
          sounds.tradeClose();
          toast(`Closed: ${latest.pair} — ${fmt.usd(latest.pnl)}`, { icon: "📊" });
        }
      }
    }
    prevTradeCount.current = trades.length;
  }, [trades]);

  useEffect(() => {
    if (positions.length > prevPosCount.current) {
      sounds.tradeOpen();
      const latest = positions[positions.length - 1];
      if (latest) toast.success(`Opened: ${latest.side} ${latest.pair} @ ${fmt.price(latest.entry_price)}`, { icon: "🚀" });
    }
    prevPosCount.current = positions.length;
  }, [positions]);

  // Detect stop-loss changes on existing positions (manual override or trailing)
  useEffect(() => {
    for (const p of positions) {
      const prev = prevStops.current[p.pair];
      if (prev !== undefined && Math.abs(prev - p.stop_loss) > 0.0001) {
        const direction = p.stop_loss > prev ? "Trailed up" : "Tightened";
        toast(`🛡️ ${direction}: ${p.pair} SL ${fmt.price(prev)} → ${fmt.price(p.stop_loss)}`, {
          duration: 5000,
          icon: "🛡️",
        });
      }
      prevStops.current[p.pair] = p.stop_loss;
    }
  }, [positions]);

  useEffect(() => {
    if (online) sounds.connected();
  }, [online]);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Ctrl+Shift+C — copy all sections
      if (e.ctrlKey && e.shiftKey && e.key === "C") {
        e.preventDefault();
        const sections = [
          copyFormatters.performance(session),
          copyFormatters.marketInsight(insight),
          copyFormatters.positions(positions),
          copyFormatters.risk(risk),
          copyFormatters.decisions(decisions),
          copyFormatters.trades(trades),
          copyFormatters.activity(activity),
        ].join("\n\n---\n\n");
        navigator.clipboard.writeText(sections);
        toast.success("All sections copied", { duration: 2000, id: 'clipboard' });
      }
      // Ctrl+Shift+E — export trades CSV
      if (e.ctrlKey && e.shiftKey && e.key === "E") {
        e.preventDefault();
        downloadTradesCSV(trades);
        toast.success("Trades CSV downloaded", { duration: 2000, id: 'csv-download' });
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [session, insight, positions, risk, decisions, trades, activity]);

  return (
    <div className="h-screen w-screen flex flex-col p-1.5 gap-1.5 overflow-hidden">
      <Toaster position="bottom-right" gutter={12} containerStyle={{ bottom: 16, right: 16 }} toastOptions={{
        className: "!bg-[var(--panel-solid)] !text-[var(--txt)] !border !border-[var(--line)] !text-xs !font-mono",
        duration: 4000,
        removeDelay: 500,
        success: {
          iconTheme: { primary: 'var(--green)', secondary: 'var(--bg)' },
        },
        error: {
          duration: 5000,
          iconTheme: { primary: 'var(--red)', secondary: 'var(--bg)' },
        },
      }} />

      {/* ── Connection Overlay ── */}
      {!online && (
        <div className="fixed inset-0 z-50 flex flex-col items-center justify-center bg-[var(--bg)]/90 backdrop-blur-sm">
          <Image src="/savant.png" alt="SAVANT" width={64} height={64} className="rounded-lg mb-4 opacity-50" />
          <span className="text-xl font-extrabold tracking-[8px] bg-gradient-to-r from-white/50 to-[var(--cyan)]/50 bg-clip-text text-transparent mb-2">SAVANT</span>
          <span className="text-[10px] tracking-[3px] uppercase text-[var(--dim)] mb-6">Autonomous Trading Agent</span>
          <div className="flex items-center gap-2 text-[var(--dim)] text-[11px] font-mono">
            <span className="w-1.5 h-1.5 rounded-full bg-[var(--red)] shadow-[0_0_6px_var(--red)] animate-pulse" />
            <span>Disconnected from engine — data may be stale. Retrying...</span>
          </div>
        </div>
      )}

      {/* ── Header ── */}
      <div className="flex items-center gap-3 bg-gradient-to-b from-[rgba(20,24,40,0.7)] to-[rgba(12,14,24,0.5)] border border-[var(--line)] backdrop-blur-xl px-3 py-1.5 shrink-0">
        <Image src="/savant.png" alt="SAVANT" width={36} height={36} className="rounded" />
        <span className="text-base font-extrabold tracking-[6px] bg-gradient-to-r from-white to-[var(--cyan)] bg-clip-text text-transparent">SAVANT</span>
        <span className="text-[9px] tracking-[3px] uppercase text-[var(--dim)]">Autonomous Trading Agent</span>
        <span className={`inline-flex items-center gap-1.5 rounded border px-2 py-0.5 text-[9px] font-bold tracking-wider uppercase ${
          live ? "border-[var(--cyan)]/30 bg-[var(--cyan)]/10 text-[var(--cyan)]" : "border-[var(--amber)]/30 bg-[var(--amber)]/10 text-[var(--amber)]"
        }`}>
          <Icon name={live ? "fa-satellite-dish" : "fa-moon"} className="text-[8px]" />
          {status?.mode ?? "—"} · {status?.running ? "RUNNING" : "IDLE"}
        </span>
        {portfolio?.hunt_mode && (
          <span className="inline-flex items-center gap-1 rounded border px-2 py-0.5 text-[9px] font-bold tracking-wider uppercase" style={{ color: 'var(--neon-red)', borderColor: 'rgba(255, 45, 85, 0.3)', backgroundColor: 'rgba(255, 45, 85, 0.1)', textShadow: 'var(--neon-red-glow)' }}>
            <Icon name="fa-crosshairs" className="text-[8px]" />
            HUNT MODE
          </span>
        )}
        {portfolio?.monitoring_mode && !portfolio?.hunt_mode && (
          <span className="inline-flex items-center gap-1 rounded border px-2 py-0.5 text-[9px] font-bold tracking-wider uppercase" style={{ color: 'var(--neon-amber)', borderColor: 'rgba(255, 179, 71, 0.3)', backgroundColor: 'rgba(255, 179, 71, 0.1)', textShadow: 'var(--neon-amber-glow)' }}>
            <Icon name="fa-eye" className="text-[8px]" />
            MONITORING
          </span>
        )}
        {(portfolio?.price_staleness_secs ?? 0) > 300 && (
          <span className="inline-flex items-center gap-1 rounded border px-2 py-0.5 text-[9px] font-bold tracking-wider uppercase" style={{ color: 'var(--red)', borderColor: 'rgba(255, 45, 85, 0.3)', backgroundColor: 'rgba(255, 45, 85, 0.1)', textShadow: 'var(--neon-red-glow)' }}>
            <Icon name="fa-triangle-exclamation" className="text-[8px]" />
            STALE PRICES {Math.floor((portfolio?.price_staleness_secs ?? 0) / 60)}m
          </span>
        )}
        <div className="flex-1" />
        <div className="flex gap-4 items-center text-[10px] text-[var(--dim)]">
          <span className="flex items-center gap-1"><Icon name="fa-microchip" className="text-[8px]" /> <b className="text-[var(--txt)]">{config?.model ?? "—"}</b></span>
          <span className="flex items-center gap-1"><Icon name="fa-clock" className="text-[8px]" /> <b className="text-[var(--txt)]">{fmt.uptime(status?.uptime_seconds ?? 0)}</b></span>
          <span className="flex items-center gap-1"><Icon name="fa-layer-group" className="text-[8px]" /> <b className="text-[var(--txt)]">{status?.pairs?.length ?? 0}</b></span>
          <span className="flex items-center gap-1.5">
            <span className={`w-1.5 h-1.5 rounded-full ${online ? "bg-[var(--green)] shadow-[0_0_8px_var(--green)]" : "bg-[var(--red)] shadow-[0_0_6px_var(--red)]"}`} />
            <Icon name={online ? "fa-link" : "fa-link-slash"} className="text-[8px]" />
            {online ? "connected" : "offline"}
          </span>
        </div>
      </div>

      {/* ── News Ticker ── */}
      <Ticker speed={50}>
        {insight?.trending_coins?.slice(0, 8).map((c, i) => (
          <span key={`t-${i}`} className="text-[9px] flex items-center gap-1 text-[var(--cyan)]"><Icon name="fa-fire" className="text-[7px]" />{c}</span>
        ))}
        <span className="text-[9px] flex items-center gap-1.5">
          <Icon name={(insight?.fear_greed ?? 50) < 30 ? "fa-arrow-trend-down" : (insight?.fear_greed ?? 50) > 70 ? "fa-arrow-trend-up" : "fa-minus"} className="text-[7px]" />
          <span className="text-[var(--amber)]">F&amp;G: {insight?.fear_greed ?? "—"}</span>
          <span className="text-[var(--dim)]">({insight?.fear_greed_label ?? "—"})</span>
        </span>
        <span className="text-[9px] flex items-center gap-1.5">
          <Icon name={(insight?.funding_rate ?? 0) < 0 ? "fa-arrow-up" : "fa-arrow-down"} className={`text-[7px] ${(insight?.funding_rate ?? 0) < 0 ? "text-[var(--green)]" : "text-[var(--red)]"}`} />
          <span className="text-[var(--dim)]">Funding:</span>
          <span className={`font-mono ${(insight?.funding_rate ?? 0) < -0.005 ? "text-[var(--green)]" : (insight?.funding_rate ?? 0) > 0.005 ? "text-[var(--red)]" : "text-[var(--dim)]"}`}>{insight?.funding_rate != null ? (insight.funding_rate * 100).toFixed(4) + "%" : "—"}</span>
        </span>
        <span className="text-[9px] flex items-center gap-1.5">
          <Icon name="fa-bitcoin-sign" className="text-[7px] text-[var(--dim)]" />
          <span className="text-[var(--dim)]">BTC Dom:</span>
          <span className="font-mono text-[var(--txt)]">{insight?.btc_dominance?.toFixed(1) ?? "—"}%</span>
        </span>
        <span className="text-[9px] flex items-center gap-1.5">
          <Icon name="fa-cube" className="text-[7px] text-[var(--dim)]" />
          <span className="text-[var(--dim)]">Block:</span>
          <span className="font-mono text-[var(--txt)]">{insight?.block_height?.toLocaleString() ?? "—"}</span>
        </span>
        <span className="text-[9px] flex items-center gap-1.5">
          <Icon name="fa-newspaper" className="text-[7px] text-[var(--dim)]" />
          <span className="text-[var(--dim)]">News:</span>
          <span className="font-mono text-[var(--txt)]">{insight?.rss_items ?? 0}</span>
        </span>
        {positions.map((p) => (
          <span key={`pos-${p.id}`} className="text-[9px] flex items-center gap-1.5">
            <Icon name={p.side === "Long" ? "fa-arrow-trend-up" : "fa-arrow-trend-down"} className={`text-[7px] ${(p.unrealized_pnl ?? 0) >= 0 ? "text-[var(--green)]" : "text-[var(--red)]"}`} />
            <span className="text-[var(--txt)] font-semibold">{p.pair.split("/")[0]}</span>
            <span className="font-mono text-[var(--dim)]">{fmt.price(p.current_price)}</span>
            <span className={`font-mono ${(p.unrealized_pnl ?? 0) >= 0 ? "text-[var(--green)]" : "text-[var(--red)]"}`}>{fmt.pct(p.entry_price ? ((p.side === "Long" ? (p.current_price - p.entry_price) : (p.entry_price - p.current_price)) / p.entry_price * 100) : 0)}</span>
          </span>
        ))}
      </Ticker>

      {/* ── KPI Bar ── */}
      <div className="grid grid-cols-6 gap-1.5 shrink-0">
        <KPI icon="fa-wallet" label="Portfolio Value" value={fmt.usd(eq)} color="text-[var(--cyan)]" />
        <KPI icon="fa-bank" label="Cash Balance" value={fmt.usd(portfolio?.balance ?? 0)} sub="USD available" />
        <KPI icon="fa-sack-dollar" label="Profit" value={fmt.usd(session?.total_pnl ?? 0)} sub={`${fmt.usd(session?.starting_balance ?? 30)} invested → ${fmt.usd(portfolio?.equity ?? 0)} on-chain`} color={pnlClass(session?.total_pnl ?? 0)} />
        <KPI icon="fa-bullseye" label="Win Rate" value={`${((session?.win_rate ?? 0) * 100).toFixed(0)}%`} sub={`${session?.wins ?? 0}W / ${session?.losses ?? 0}L`} color="text-[var(--green)]" />
        <KPI icon="fa-rotate" label="Trades Today" value={`${portfolio?.trades_today ?? 0}`} sub={`${session?.total_trades ?? 0} total`} color="text-[var(--violet)]" />
        <KPI icon="fa-layer-group" label="Positions" value={`${positions.length} / ${risk?.max_positions ?? 3}`} sub={positions.length > 0 ? positions.map(p => p.pair.split("/")[0]).join(", ") : "none open"} />
      </div>

      {/* ── Bento Grid ── */}
      <div className="flex-1 grid grid-cols-[1.6fr_1fr_1fr] grid-rows-[1.2fr_1fr_1fr] gap-1.5 min-h-0">

        {/* Row 1: Equity | Performance | Market Insight */}
        <div className="bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md flex flex-col overflow-hidden">
          <SectionHeader icon="fa-chart-area" title="Equity Curve" tag="live" />
          <ErrorBoundary label="Equity Curve">
            <EquityChart data={state.equity} />
          </ErrorBoundary>
        </div>

        <div className="bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md flex flex-col overflow-hidden">
          <SectionHeader icon="fa-gauge-high" title="Performance" onCopy={() => copyFormatters.performance(session)} />
          <div className="flex-1 px-3 pb-2 overflow-y-auto space-y-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-1.5">
                <Chip color="success" size="sm" variant="soft">
                  <i className="fa-solid fa-circle-check text-[7px] mr-0.5" />
                  <Chip.Label>{session?.wins ?? 0}W</Chip.Label>
                </Chip>
                <Chip color="danger" size="sm" variant="soft">
                  <i className="fa-solid fa-circle-xmark text-[7px] mr-0.5" />
                  <Chip.Label>{session?.losses ?? 0}L</Chip.Label>
                </Chip>
              </div>
              <span className={`font-mono font-bold text-[13px] ${((session?.win_rate ?? 0) >= 0.5) ? "text-[var(--green)]" : "text-[var(--red)]"}`}>
                {((session?.win_rate ?? 0) * 100).toFixed(0)}%
              </span>
            </div>
            <ProgressBarRoot className="h-1.5 rounded bg-[var(--red)] overflow-hidden">
              <ProgressBarFill className="h-full bg-[var(--green)] rounded" style={{ width: `${(session?.wins ?? 0) / ((session?.wins ?? 0) + (session?.losses ?? 0) || 1) * 100}%` }} />
            </ProgressBarRoot>
            <Separator className="my-1" />
            <div className="grid grid-cols-2 gap-x-3 gap-y-1.5">
              <MetricRow icon="fa-brain" label="Decisions" value={session?.total_decisions ?? decisions.length} />
              <MetricRow icon="fa-right-left" label="Trades" value={portfolio?.trades_today ?? 0} />
              <div className="flex items-center justify-between">
                <span className="text-[10px] text-[var(--dim)] flex items-center gap-1.5"><Icon name="fa-shield-halved" className="text-[7px]" />Conf cap</span>
                <Chip size="sm" variant="soft" color={(memory?.confidence_cap ?? "") === "LOW" ? "success" : (memory?.confidence_cap ?? "") === "HIGH" ? "danger" : "accent"}>
                  <Chip.Label>{memory?.confidence_cap ?? "—"}</Chip.Label>
                </Chip>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[10px] text-[var(--dim)] flex items-center gap-1.5"><Icon name="fa-crosshairs" className="text-[7px]" />Brier</span>
                <Tooltip delay={300}>
                  <span className={`text-[13px] font-mono font-semibold cursor-help ${(() => { const b = memory?.brier_score; if (b == null) return "text-[var(--dim)]"; return b < 0.20 ? "text-[var(--green)]" : b < 0.30 ? "text-[var(--amber)]" : "text-[var(--red)]"; })()}`}>
                    {memory?.brier_score?.toFixed(3) ?? "—"}
                  </span>
                  <Tooltip.Content showArrow>
                    <p className="text-[10px]">Calibration score. Lower is better. &lt;0.20 = well calibrated. {memory?.brier_label ?? ""}</p>
                  </Tooltip.Content>
                </Tooltip>
              </div>
              <div className="flex items-center justify-between col-span-2">
                <span className="text-[10px] text-[var(--dim)] flex items-center gap-1.5"><Icon name="fa-wave-square" className="text-[7px]" />CUSUM</span>
                <Tooltip delay={300}>
                  <Chip size="sm" variant="soft" color={String(memory?.cusum_status ?? "").toLowerCase().includes("positive") ? "success" : String(memory?.cusum_status ?? "").toLowerCase().includes("negative") ? "danger" : "default"}>
                    <Chip.Label>{memory?.cusum_status ?? "—"}</Chip.Label>
                  </Chip>
                  <Tooltip.Content showArrow>
                    <p className="text-[10px]">Cumulative sum control chart. Detects edge decay over time.</p>
                  </Tooltip.Content>
                </Tooltip>
              </div>
            </div>
            {(portfolio?.hunt_mode || (portfolio?.monitoring_mode && !portfolio?.hunt_mode)) && (
              <>
                <Separator className="my-1" />
                <div className="flex items-center justify-center">
                  {portfolio?.hunt_mode ? (
                    <Chip size="sm" variant="soft" color="danger" style={{ color: 'var(--neon-red)', textShadow: 'var(--neon-red-glow)' }}>
                      <i className="fa-solid fa-crosshairs text-[7px] mr-0.5" />
                      <Chip.Label>HUNT MODE</Chip.Label>
                    </Chip>
                  ) : (
                    <Chip size="sm" variant="soft" color="warning" style={{ color: 'var(--neon-amber)', textShadow: 'var(--neon-amber-glow)' }}>
                      <i className="fa-solid fa-eye text-[7px] mr-0.5" />
                      <Chip.Label>MONITORING</Chip.Label>
                    </Chip>
                  )}
                </div>
              </>
            )}
          </div>
        </div>

        <div className="bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md flex flex-col overflow-hidden">
          <SectionHeader icon="fa-globe" title="Market Insight" onCopy={() => copyFormatters.marketInsight(insight)} />
          <div className="flex-1 px-3 pb-2 overflow-y-auto">
            <div className="flex items-center gap-2.5 mb-2">
              <div className="text-center shrink-0">
                <FearGauge value={insight?.fear_greed ?? null} />
                <p className="text-[8px] tracking-[1px] text-[var(--dim)] flex items-center justify-center gap-1"><Icon name="fa-face-grimace" className="text-[7px]" />FEAR &amp; GREED</p>
              </div>
              <div className="flex-1 space-y-1">
                <div className="flex items-center justify-between">
                  <span className="text-[10px] text-[var(--dim)] flex items-center gap-1.5"><Icon name="fa-heart-pulse" className="text-[7px]" />Sentiment</span>
                  <Chip size="sm" variant="soft" color={(() => { const fg = insight?.fear_greed; if (fg == null) return "default"; return fg <= 25 ? "danger" : fg <= 45 ? "warning" : fg <= 55 ? "default" : fg <= 75 ? "success" : "danger"; })()}>
                    <Chip.Label>{insight?.fear_greed_label ?? "—"}</Chip.Label>
                  </Chip>
                </div>
                <MetricRow icon="fa-faucet-drip" label="Funding" value={insight?.funding_rate != null ? (insight.funding_rate * 100).toFixed(4) + "%" : "—"} color={(() => { const f = insight?.funding_rate; if (f == null) return ""; return f < -0.01 ? "text-[var(--green)]" : f > 0.01 ? "text-[var(--red)]" : "text-[var(--dim)]"; })()} />
                <MetricRow icon="fa-bitcoin-sign" label="BTC dom" value={`${insight?.btc_dominance?.toFixed(1) ?? "—"}%`} />
                <MetricRow icon="fa-cube" label="Block" value={insight?.block_height?.toLocaleString() ?? "—"} />
                <MetricRow icon="fa-newspaper" label="News" value={insight?.rss_items ?? 0} />
              </div>
            </div>
            <Separator className="my-1" />
            <div className="flex flex-wrap gap-1">
              {insight?.trending_coins?.slice(0, 8).map((c) => (
                <Chip key={c} size="sm" variant="soft" color="accent">
                  <i className="fa-solid fa-fire text-[6px] mr-0.5" />
                  <Chip.Label>{c}</Chip.Label>
                </Chip>
              ))}
            </div>
          </div>
        </div>

        {/* Row 2: Positions | Risk | Decisions */}
        <div className="bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md flex flex-col overflow-hidden">
          <SectionHeader icon="fa-briefcase" title="Open Positions" tag={`${positions.length}`} onCopy={() => copyFormatters.positions(positions)} />
          <div className="flex-1 px-3 pb-2 overflow-y-auto">
            {positions.length === 0 ? (
              <p className="text-[var(--dimmer)] text-xs text-center py-4 flex items-center justify-center gap-1.5">
                <Icon name="fa-inbox" />No open positions
              </p>
            ) : (
              positions.map((p) => {
                const upnl = p.unrealized_pnl;
                const upPct = p.entry_price ? (upnl / (p.entry_price * p.quantity)) * 100 : 0;
                const lo = Math.min(p.stop_loss, p.entry_price, p.current_price);
                const hi = Math.max(p.take_profit_1, p.entry_price, p.current_price);
                const span = hi - lo || 1;
                const at = (v: number) => Math.max(0, Math.min(100, ((v - lo) / span) * 100));
                return (
                  <div key={p.id} className="border border-[var(--line)] p-2 mb-1.5 bg-[rgba(8,10,18,0.6)]">
                    <div className="flex justify-between items-center mb-1.5">
                      <div className="flex items-center gap-1.5">
                        <span className="font-bold text-white text-xs">{p.pair}</span>
                        <span className={`text-[8px] px-1 py-0.5 rounded flex items-center gap-0.5 ${p.side === "Long" ? "text-[var(--green)] bg-[var(--green)]/10" : "text-[var(--red)] bg-[var(--red)]/10"}`}>
                          <Icon name={p.side === "Long" ? "fa-arrow-up" : "fa-arrow-down"} className="text-[6px]" />{p.side}
                        </span>
                      </div>
                      <div className="flex items-center gap-1.5">
                        <span className={`text-sm font-bold font-mono ${pnlClass(upnl)}`}>{fmt.usd(upnl)} <span className="text-[9px]">({fmt.pct(upPct)})</span></span>
                        <button
                          onClick={() => {
                            if (window.confirm(`Close ${p.pair} ${p.side}? This will execute an on-chain swap.`)) {
                              fetch(`/api/positions/${p.pair.replace("/", "-")}/close`, { method: "POST" });
                            }
                          }}
                          className="text-[8px] text-[var(--dim)] hover:text-[var(--red)] transition-colors cursor-pointer px-1"
                          title="Close position"
                        >
                          <Icon name="fa-xmark" className="text-[8px]" />
                        </button>
                      </div>
                    </div>
                    <div className="relative h-1 rounded-full bg-gradient-to-r from-[var(--red)]/50 via-[var(--dim)]/20 to-[var(--green)]/50 mb-1">
                      <div className="absolute -top-0.5 w-0.5 h-2 bg-[var(--red)]" style={{ left: `${at(p.stop_loss)}%` }} />
                      <div className="absolute -top-0.5 w-0.5 h-2 bg-[var(--dim)]" style={{ left: `${at(p.entry_price)}%` }} />
                      <div className="absolute -top-0.5 w-0.5 h-2 bg-[var(--green)]" style={{ left: `${at(p.take_profit_1)}%` }} />
                      <div className="absolute -top-1 w-[2px] h-3 bg-white shadow-[0_0_6px_#fff]" style={{ left: `${at(p.current_price)}%` }} />
                    </div>
                    <div className="flex justify-between text-[8px] text-[var(--dimmer)]">
                      <span className="text-[var(--red)] flex items-center gap-0.5"><Icon name="fa-shield" className="text-[6px]" />SL {fmt.price(p.stop_loss)}</span>
                      <span>entry {fmt.price(p.entry_price)}</span>
                      <span className="text-[var(--green)] flex items-center gap-0.5">TP {fmt.price(p.take_profit_1)} <Icon name="fa-flag-checkered" className="text-[6px]" /></span>
                    </div>
                    <div className="grid grid-cols-4 gap-1 mt-1">
                      <MetricRow icon="fa-eye" label="Now" value={fmt.price(p.current_price)} />
                      <MetricRow icon="fa-coins" label="Qty" value={p.quantity.toPrecision(3)} />
                      <MetricRow icon="fa-coins" label="Size" value={fmt.usd(p.entry_price * p.quantity)} />
                      <MetricRow icon="fa-shield" label="Risk" value={fmt.usd(Math.abs(p.entry_price - p.stop_loss) * p.quantity)} />
                      <MetricRow icon="fa-hourglass-half" label="Age" value={dayjs(p.opened_at).fromNow(true)} />
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>

        <div className="bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md flex flex-col overflow-hidden">
          <SectionHeader icon="fa-shield-halved" title="Risk Controls" onCopy={() => copyFormatters.risk(risk)} />
          <div className="flex-1 px-3 pb-2 overflow-y-auto space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-[10px] text-[var(--dim)] flex items-center gap-1.5"><Icon name="fa-bolt" className="text-[7px]" />Circuit breaker</span>
              <Chip size="sm" variant="soft" color={risk?.circuit_breaker === "OK" ? "success" : "danger"}>
                <i className={`fa-solid ${risk?.circuit_breaker === "OK" ? "fa-check" : "fa-triangle-exclamation"} text-[7px] mr-0.5`} />
                <Chip.Label>{risk?.circuit_breaker ?? "OK"}</Chip.Label>
              </Chip>
            </div>
            <RiskBar icon="fa-arrow-trend-down" label="Drawdown" value={risk?.drawdown_pct ?? 0} max={risk?.max_drawdown ?? 0.1} tooltip={`Max drawdown from peak equity. Halt at ${((risk?.max_drawdown ?? 0.1) * 100).toFixed(0)}%. Floor: $10.`} />
            <RiskBar icon="fa-calendar-xmark" label="Daily loss" value={Math.abs(risk?.daily_loss_pct ?? 0)} max={risk?.max_daily_loss ?? 0.05} tooltip={`Max daily loss. Halt at ${((risk?.max_daily_loss ?? 0.05) * 100).toFixed(0)}%. Floor: $5. Resets at midnight UTC.`} />
            <RiskBar icon="fa-grip" label="Positions" value={risk?.open_positions ?? 0} max={risk?.max_positions ?? 3} />
            <Separator className="my-1" />
            <div className="grid grid-cols-2 gap-x-3 gap-y-1">
              <MetricRow icon="fa-percent" label="Risk/trade" value={`${((risk?.max_risk_per_trade ?? 0) * 100).toFixed(0)}%`} />
              <div className="flex items-center justify-between">
                <span className="text-[10px] text-[var(--dim)] flex items-center gap-1.5"><Icon name="fa-book-open" className="text-[7px]" />Replays</span>
                <Chip size="sm" variant="soft" color="accent">
                  <Chip.Label>{memory?.replay_lesson_count ?? 0}</Chip.Label>
                </Chip>
              </div>
            </div>
          </div>
        </div>

        <div className="bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md flex flex-col overflow-hidden">
          <SectionHeader icon="fa-robot" title="AI Decisions" tag={(() => {
            if (decisions.length === 0) return "live";
            const last = new Date(decisions[0].timestamp).getTime();
            const elapsed = Date.now() - last;
            const interval = 15 * 60 * 1000; // 15 min cycle
            const remaining = Math.max(0, interval - elapsed);
            if (remaining === 0) return `${dayjs(decisions[0].timestamp).fromNow(true)} ago · next cycle soon`;
            const mins = Math.ceil(remaining / 60000);
            return `${dayjs(decisions[0].timestamp).fromNow(true)} ago · next in ${mins}m`;
          })()} onCopy={() => copyFormatters.decisions(decisions)} />
          <div className="flex-1 px-3 pb-2 overflow-y-auto">
            {decisions.length === 0 ? (
              <p className="text-[var(--dimmer)] text-xs text-center py-4 flex items-center justify-center gap-1.5">
                <Icon name={portfolio?.monitoring_mode ? "fa-eye" : "fa-spinner fa-spin"} />
                {portfolio?.monitoring_mode ? "Monitoring — LLM not active while fully deployed" : "Waiting for first AI cycle…"}
              </p>
            ) : (
              decisions.slice(0, 10).map((d, i) => {
                const a = d.action.toUpperCase();
                const conf = d.confidence * 100;
                const age = Date.now() - new Date(d.timestamp).getTime();
                const isStale = age > 30 * 60 * 1000; // 30 min
                return (
                  <div key={i} className={`border-l-2 border-[var(--line2)] pl-2 py-1 mb-1 ${isStale ? "opacity-50" : ""}`}>
                    <div className="flex items-center gap-1.5">
                      <span className="font-semibold text-[11px]">{d.pair}</span>
                      <span className="text-[8px] text-[var(--dimmer)]">{dayjs(d.timestamp).fromNow(true)}</span>
                      <span className={`text-[8px] px-1 py-0.5 rounded font-bold flex items-center gap-0.5 ${
                        a === "BUY" ? "text-[var(--green)] bg-[var(--green)]/10" :
                        a === "SELL" || a === "CLOSE" ? "text-[var(--red)] bg-[var(--red)]/10" :
                        a === "ADJUST" || a === "ADJUSTSTOP" ? "text-[var(--amber)] bg-[var(--amber)]/10" :
                        "text-[var(--dim)] bg-white/5"
                      }`}>
                        <Icon name={a === "BUY" ? "fa-circle-arrow-up" : a === "SELL" || a === "CLOSE" ? "fa-circle-arrow-down" : a === "ADJUST" || a === "ADJUSTSTOP" ? "fa-sliders" : "fa-minus"} className="text-[6px]" />
                        {a.replace("_", " ")}
                      </span>
                      <ProgressBar aria-label="Confidence" size="sm" value={conf} color={conf >= 67 ? "success" : conf >= 34 ? "warning" : "danger"} className="flex-1">
                        <ProgressBar.Track>
                          <ProgressBar.Fill />
                        </ProgressBar.Track>
                      </ProgressBar>
                      <span className={`text-[9px] font-mono font-bold ${conf >= 67 ? "text-[var(--green)]" : conf >= 34 ? "text-[var(--amber)]" : "text-[var(--red)]"}`}>{conf.toFixed(0)}%</span>
                    </div>
                    <p className="text-[9px] text-[var(--dim)] mt-0.5 break-words">{d.reasoning}</p>
                  </div>
                );
              })
            )}
          </div>
        </div>

        {/* Row 3: Console | Activity | Trades */}
        <div className="bg-[#0a0c14] border border-[var(--line)] flex flex-col overflow-hidden">
          <div className="flex items-center gap-2 px-3 py-1.5 border-b border-[var(--line)]">
            <i className="fa-solid fa-terminal text-[var(--dim)] text-[9px]" />
            <span className="text-[10px] text-[var(--dim)] tracking-wider font-mono leading-none">savant — terminal</span>
            <div className="flex-1" />
            <CopyButton text={() => {
              try {
                const el = document.querySelector('.xterm-screen');
                if (!el) return "Terminal not available";
                const rows = el.querySelectorAll('.xterm-rows > div');
                const lines: string[] = [];
                rows.forEach((row) => {
                  const spans = row.querySelectorAll('span');
                  let line = "";
                  spans.forEach((s) => { line += s.textContent ?? ""; });
                  if (line.trim()) lines.push(line);
                });
                return lines.join("\n") || "No terminal output";
              } catch { return "Could not read terminal"; }
            }} title="Copy terminal output" />
            <span className="w-2.5 h-2.5 rounded-full bg-[var(--green)]/80"></span>
            <span className="w-2.5 h-2.5 rounded-full bg-[var(--amber)]/80"></span>
            <span className="w-2.5 h-2.5 rounded-full bg-[var(--red)]/80"></span>
          </div>
          <div className="flex-1 min-h-0">
            <ErrorBoundary label="Terminal">
              <TerminalPanel className="h-full" />
            </ErrorBoundary>
          </div>
        </div>

        <div className="bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md flex flex-col overflow-hidden">
          <div className="flex items-center gap-2 px-3 pt-2 pb-1 border-b border-[var(--line)]">
            <Icon name="fa-timeline" className="text-[var(--dim)] text-[10px]" />
            <span className="text-[10px] tracking-[2px] uppercase font-semibold text-[var(--dim)]">Activity</span>
            <span className="ml-auto text-[9px] font-bold text-[var(--cyan)]">{activity.length}</span>
            <span className="inline-flex items-center">
              <CopyButton text={() => copyFormatters.activity(activity)} title="Copy activity log" />
            </span>
          </div>
          <div className="flex-1 px-3 pb-2 overflow-y-auto font-mono text-[10px]">
            {activity.length === 0 ? (
              <p className="text-[var(--dimmer)] text-xs text-center py-4"><Icon name="fa-inbox" className="mr-1" />No activity yet.</p>
            ) : (
              [...activity].reverse().slice(0, 30).map((e, i) => (
                <div key={i} className={`flex gap-2 py-px border-b border-white/[0.02] whitespace-nowrap ${
                  e.level === "Trade" ? "text-[var(--green)]" : e.level === "Decision" ? "text-[var(--violet)]" : e.level === "Warning" || e.level === "Error" ? "text-[var(--red)]" : e.level === "Thinking" ? "text-[var(--amber)]" : "text-[var(--txt)]"
                }`}>
                  <span className="text-[var(--dimmer)] shrink-0">{formatTime12h(e.timestamp)}</span>
                  <span className="text-[var(--cyan)] shrink-0 w-[60px] overflow-hidden text-ellipsis">{e.pair}</span>
                  <span className="overflow-hidden text-ellipsis">{e.message}</span>
                </div>
              ))
            )}
          </div>
        </div>

        <div className="bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md flex flex-col overflow-hidden">
          <div className="flex items-center gap-2 px-3 pt-2 pb-1 border-b border-[var(--line)]">
            <span className="inline-flex items-center"><Icon name="fa-receipt" className="text-[var(--dim)] text-[10px]" /></span>
            <span className="text-[10px] tracking-[2px] uppercase font-semibold text-[var(--dim)] leading-none">Closed Trades</span>
            <span className="ml-auto text-[9px] font-bold leading-none text-[var(--cyan)]">{trades.length}</span>
            <span className="ml-auto inline-flex items-center">
              <CopyButton text={() => copyFormatters.trades(trades)} title="Copy closed trades" />
            </span>
            {trades.length > 0 && (
              <button
                onClick={() => downloadTradesCSV(trades)}
                className="inline-flex items-center justify-center text-[var(--dim)] hover:text-[var(--cyan)] transition-colors cursor-pointer leading-none"
                title="Download CSV"
              >
                <Icon name="fa-download" className="text-[9px]" />
              </button>
            )}
          </div>
          <div className="flex-1 px-3 pb-2 overflow-y-auto">
            {trades.length === 0 ? (
              <p className="text-[var(--dimmer)] text-xs text-center py-4"><Icon name="fa-inbox" className="mr-1" />No closed trades yet.</p>
            ) : (
              <table className="w-full text-[10px]">
                <thead>
                  <tr className="text-[var(--dimmer)] text-left">
                    <th className="py-0.5 pr-2"><Icon name="fa-hashtag" className="mr-0.5 text-[7px]" />PAIR</th>
                    <th className="py-0.5 pr-2"><Icon name="fa-arrow-right-arrow-left" className="mr-0.5 text-[7px]" />SIDE</th>
                    <th className="py-0.5 pr-2"><Icon name="fa-door-open" className="mr-0.5 text-[7px]" />ENTRY</th>
                    <th className="py-0.5 pr-2"><Icon name="fa-door-closed" className="mr-0.5 text-[7px]" />EXIT</th>
                    <th className="py-0.5 pr-2"><Icon name="fa-sack-dollar" className="mr-0.5 text-[7px]" />P&L</th>
                    <th className="py-0.5"><Icon name="fa-percent" className="mr-0.5 text-[7px]" />%</th>
                  </tr>
                </thead>
                <tbody>
                  {trades.slice(0, 10).map((t) => (
                    <tr key={t.id} className="border-t border-white/[0.03] even:bg-white/[0.015]">
                      <td className="py-0.5 pr-2 font-semibold">{t.pair}</td>
                      <td className={`py-0.5 pr-2 ${pnlClass(t.side === "Long" ? 1 : -1)}`}>
                        <span className="flex items-center gap-0.5"><Icon name={t.side === "Long" ? "fa-arrow-up" : "fa-arrow-down"} className="text-[7px]" />{t.side}</span>
                      </td>
                      <td className="py-0.5 pr-2 font-mono">{fmt.price(t.entry_price)}</td>
                      <td className="py-0.5 pr-2 font-mono">{fmt.price(t.exit_price)}</td>
                      <td className={`py-0.5 pr-2 font-mono ${pnlClass(t.pnl)}`}>{fmt.usd(t.pnl)}</td>
                      <td className={`py-0.5 font-mono ${pnlClass(t.pnl_pct)}`}>{t.pnl_pct.toFixed(2)}%</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </div>

      </div>
    </div>
  );
}
