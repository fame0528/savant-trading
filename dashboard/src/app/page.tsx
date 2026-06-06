"use client";

import Image from "next/image";
import dynamic from "next/dynamic";
import { formatTime12h, formatTimeShort } from "@/lib/time";
import {
  ProgressBarRoot,
  ProgressBarFill,
} from "@heroui/react";
import { useDashboard } from "@/hooks/useDashboard";
import { sounds } from "@/lib/sounds";
import { useEffect, useRef } from "react";
import toast, { Toaster } from "react-hot-toast";
import dayjs from "dayjs";
import relativeTime from "dayjs/plugin/relativeTime";
import ErrorBoundary from "@/components/ErrorBoundary";

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

function SectionHeader({ icon, title, tag, tagColor }: { icon: string; title: string; tag?: string; tagColor?: string }) {
  return (
    <div className="flex items-center gap-2 px-3 pt-2 pb-1 border-b border-[var(--line)]">
      <Icon name={icon} className="text-[var(--dim)] text-[10px]" />
      <span className="text-[10px] tracking-[2px] uppercase font-semibold text-[var(--dim)]">{title}</span>
      {tag && <span className={`ml-auto text-[9px] font-bold ${tagColor ?? "text-[var(--cyan)]"}`}>{tag}</span>}
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

  return (
    <div className="h-screen w-screen flex flex-col p-1.5 gap-1.5 overflow-hidden">
      <Toaster position="top-right" toastOptions={{
        className: "!bg-[var(--panel-solid)] !text-[var(--txt)] !border !border-[var(--line)] !text-xs !font-mono",
        duration: 4000,
      }} />

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

      {/* ── KPI Bar ── */}
      <div className="grid grid-cols-6 gap-1.5 shrink-0">
        <KPI icon="fa-wallet" label="Portfolio Value" value={fmt.usd(eq)} color="text-[var(--cyan)]" />
        <KPI icon="fa-bank" label="Cash Balance" value={fmt.usd(portfolio?.balance ?? 0)} sub="USD available" />
        <KPI icon="fa-sack-dollar" label="Profit" value={fmt.usd((portfolio?.unrealized_pnl ?? 0) + (session?.total_pnl ?? 0))} sub={`${fmt.usd(session?.total_pnl ?? 0)} locked · ${fmt.usd(portfolio?.unrealized_pnl ?? 0)} open`} color={pnlClass((portfolio?.unrealized_pnl ?? 0) + (session?.total_pnl ?? 0))} />
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
          <SectionHeader icon="fa-gauge-high" title="Performance" />
          <div className="flex-1 px-3 pb-2 overflow-y-auto space-y-1 text-[11px]">
            <div className="flex justify-between">
              <span className="text-[var(--green)] flex items-center gap-1"><Icon name="fa-circle-check" className="text-[9px]" />{session?.wins ?? 0}W</span>
              <span className="text-[var(--red)] flex items-center gap-1"><Icon name="fa-circle-xmark" className="text-[9px]" />{session?.losses ?? 0}L</span>
            </div>
            <ProgressBarRoot className="h-1.5 rounded bg-[var(--red)] overflow-hidden">
              <ProgressBarFill className="h-full bg-[var(--green)] rounded" style={{ width: `${(session?.wins ?? 0) / ((session?.wins ?? 0) + (session?.losses ?? 0) || 1) * 100}%` }} />
            </ProgressBarRoot>
            <div className="space-y-0.5 pt-1">
              <div className="flex justify-between"><span className="text-[var(--dim)] flex items-center gap-1"><Icon name="fa-brain" className="text-[8px]" />Decisions</span><span className="font-semibold">{session?.total_decisions ?? decisions.length}</span></div>
              <div className="flex justify-between"><span className="text-[var(--dim)] flex items-center gap-1"><Icon name="fa-right-left" className="text-[8px]" />Trades today</span><span className="font-semibold">{portfolio?.trades_today ?? 0}</span></div>
              <div className="flex justify-between"><span className="text-[var(--dim)] flex items-center gap-1"><Icon name="fa-shield-halved" className="text-[8px]" />Confidence cap</span><span className="font-semibold text-[var(--cyan)]">{memory?.confidence_cap ?? "—"}</span></div>
              <div className="flex justify-between"><span className="text-[var(--dim)] flex items-center gap-1"><Icon name="fa-crosshairs" className="text-[8px]" />Brier</span><span className="font-semibold">{memory?.brier_score?.toFixed(3) ?? "—"}{memory?.brier_label ? ` (${memory.brier_label})` : ""}</span></div>
              <div className="flex justify-between"><span className="text-[var(--dim)] flex items-center gap-1"><Icon name="fa-wave-square" className="text-[8px]" />CUSUM</span><span className="font-semibold">{memory?.cusum_status ?? "—"}</span></div>
            </div>
          </div>
        </div>

        <div className="bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md flex flex-col overflow-hidden">
          <SectionHeader icon="fa-globe" title="Market Insight" />
          <div className="flex-1 px-3 pb-2 overflow-y-auto">
            <div className="flex items-center gap-3">
              <div className="text-center shrink-0">
                <FearGauge value={insight?.fear_greed ?? null} />
                <p className="text-[8px] tracking-[1px] text-[var(--dim)] flex items-center justify-center gap-1"><Icon name="fa-face-grimace" className="text-[7px]" />FEAR &amp; GREED</p>
              </div>
              <div className="flex-1 space-y-0.5 text-[11px]">
                <div className="flex justify-between"><span className="text-[var(--dim)] flex items-center gap-1"><Icon name="fa-heart-pulse" className="text-[8px]" />Sentiment</span><span>{insight?.fear_greed_label ?? "—"}</span></div>
                <div className="flex justify-between"><span className="text-[var(--dim)] flex items-center gap-1"><Icon name="fa-faucet-drip" className="text-[8px]" />Funding</span><span className="font-mono">{insight?.funding_rate != null ? (insight.funding_rate * 100).toFixed(4) + "%" : "—"}</span></div>
                <div className="flex justify-between"><span className="text-[var(--dim)] flex items-center gap-1"><Icon name="fa-bitcoin-sign" className="text-[8px]" />BTC dom</span><span className="font-mono">{insight?.btc_dominance?.toFixed(1) ?? "—"}%</span></div>
                <div className="flex justify-between"><span className="text-[var(--dim)] flex items-center gap-1"><Icon name="fa-cube" className="text-[8px]" />Block</span><span className="font-mono">{insight?.block_height?.toLocaleString() ?? "—"}</span></div>
                <div className="flex justify-between"><span className="text-[var(--dim)] flex items-center gap-1"><Icon name="fa-newspaper" className="text-[8px]" />News</span><span className="font-mono">{insight?.rss_items ?? 0}</span></div>
              </div>
            </div>
            <div className="flex flex-wrap gap-1 mt-1.5">
              {insight?.trending_coins?.slice(0, 8).map((c) => (
                <span key={c} className="text-[8px] px-1.5 py-0.5 rounded bg-[var(--cyan)]/10 border border-[var(--cyan)]/20 text-[var(--cyan)] flex items-center gap-0.5">
                  <Icon name="fa-fire" className="text-[6px]" />{c}
                </span>
              ))}
            </div>
          </div>
        </div>

        {/* Row 2: Positions | Risk | Decisions */}
        <div className="bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md flex flex-col overflow-hidden">
          <SectionHeader icon="fa-briefcase" title="Open Positions" tag={`${positions.length}`} />
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
                      <span className={`text-sm font-bold font-mono ${pnlClass(upnl)}`}>{fmt.usd(upnl)} <span className="text-[9px]">({fmt.pct(upPct)})</span></span>
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
                    <div className="grid grid-cols-4 gap-1 mt-1 text-[9px]">
                      <div><span className="block text-[7px] text-[var(--dimmer)] uppercase flex items-center gap-0.5"><Icon name="fa-eye" className="text-[5px]" />Now</span>{fmt.price(p.current_price)}</div>
                      <div><span className="block text-[7px] text-[var(--dimmer)] uppercase flex items-center gap-0.5"><Icon name="fa-coins" className="text-[5px]" />Qty</span>{p.quantity.toPrecision(3)}</div>
                      <div><span className="block text-[7px] text-[var(--dimmer)] uppercase flex items-center gap-0.5"><Icon name="fa-coins" className="text-[5px]" />Size</span>{fmt.usd(p.entry_price * p.quantity)}</div>
                      <div><span className="block text-[7px] text-[var(--dimmer)] uppercase flex items-center gap-0.5"><Icon name="fa-shield" className="text-[5px]" />Risk</span>{fmt.usd(Math.abs(p.entry_price - p.stop_loss) * p.quantity)}</div>
                      <div><span className="block text-[7px] text-[var(--dimmer)] uppercase flex items-center gap-0.5"><Icon name="fa-hourglass-half" className="text-[5px]" />Age</span>{dayjs(p.opened_at).fromNow(true)}</div>
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>

        <div className="bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md flex flex-col overflow-hidden">
          <SectionHeader icon="fa-shield-halved" title="Risk Controls" />
          <div className="flex-1 px-3 pb-2 overflow-y-auto space-y-1.5 text-[11px]">
            <div className="flex justify-between items-center">
              <span className="text-[var(--dim)] flex items-center gap-1"><Icon name="fa-bolt" className="text-[8px]" />Circuit breaker</span>
              <span className={`text-[9px] px-1.5 py-0.5 rounded font-bold flex items-center gap-1 ${risk?.circuit_breaker === "OK" ? "text-[var(--green)] bg-[var(--green)]/10" : "text-[var(--red)] bg-[var(--red)]/10"}`}>
                <Icon name={risk?.circuit_breaker === "OK" ? "fa-check" : "fa-triangle-exclamation"} className="text-[7px]" />
                {risk?.circuit_breaker ?? "OK"}
              </span>
            </div>
            <div>
              <div className="flex justify-between mb-0.5"><span className="text-[var(--dim)] flex items-center gap-1"><Icon name="fa-arrow-trend-down" className="text-[8px]" />Drawdown</span><span className="font-mono">{((risk?.drawdown_pct ?? 0) * 100).toFixed(1)}% / {((risk?.max_drawdown ?? 0.1) * 100).toFixed(0)}%</span></div>
              <ProgressBarRoot className="h-1 rounded bg-white/5 overflow-hidden"><ProgressBarFill className="h-full bg-[var(--amber)] rounded" style={{ width: `${Math.min(100, ((risk?.drawdown_pct ?? 0) / (risk?.max_drawdown ?? 0.1)) * 100)}%` }} /></ProgressBarRoot>
            </div>
            <div>
              <div className="flex justify-between mb-0.5"><span className="text-[var(--dim)] flex items-center gap-1"><Icon name="fa-calendar-xmark" className="text-[8px]" />Daily loss</span><span className="font-mono">{Math.abs((risk?.daily_loss_pct ?? 0) * 100).toFixed(1)}% / {((risk?.max_daily_loss ?? 0.05) * 100).toFixed(0)}%</span></div>
              <ProgressBarRoot className="h-1 rounded bg-white/5 overflow-hidden"><ProgressBarFill className="h-full bg-[var(--red)] rounded" style={{ width: `${Math.min(100, (Math.abs(risk?.daily_loss_pct ?? 0) / (risk?.max_daily_loss ?? 0.05)) * 100)}%` }} /></ProgressBarRoot>
            </div>
            <div>
              <div className="flex justify-between mb-0.5"><span className="text-[var(--dim)] flex items-center gap-1"><Icon name="fa-grip" className="text-[8px]" />Positions</span><span className="font-mono">{risk?.open_positions ?? 0} / {risk?.max_positions ?? 3}</span></div>
              <ProgressBarRoot className="h-1 rounded bg-white/5 overflow-hidden"><ProgressBarFill className="h-full bg-[var(--cyan)] rounded" style={{ width: `${Math.min(100, ((risk?.open_positions ?? 0) / (risk?.max_positions ?? 3)) * 100)}%` }} /></ProgressBarRoot>
            </div>
            <div className="flex justify-between"><span className="text-[var(--dim)] flex items-center gap-1"><Icon name="fa-percent" className="text-[8px]" />Risk / trade</span><span className="font-mono">{((risk?.max_risk_per_trade ?? 0) * 100).toFixed(0)}%</span></div>
            <div className="flex justify-between"><span className="text-[var(--dim)] flex items-center gap-1"><Icon name="fa-book-open" className="text-[8px]" />Replay lessons</span><span className="font-mono text-[var(--violet)]">{memory?.replay_lesson_count ?? 0}</span></div>
          </div>
        </div>

        <div className="bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md flex flex-col overflow-hidden">
          <SectionHeader icon="fa-robot" title="AI Decisions" tag="live" />
          <div className="flex-1 px-3 pb-2 overflow-y-auto">
            {decisions.length === 0 ? (
              <p className="text-[var(--dimmer)] text-xs text-center py-4 flex items-center justify-center gap-1.5">
                <Icon name="fa-spinner fa-spin" />Waiting for first AI cycle…
              </p>
            ) : (
              decisions.slice(0, 10).map((d, i) => {
                const a = d.action.toUpperCase();
                const conf = d.confidence * 100;
                return (
                  <div key={i} className="border-l-2 border-[var(--line2)] pl-2 py-1 mb-1">
                    <div className="flex items-center gap-1.5">
                      <span className="font-semibold text-[11px]">{d.pair}</span>
                      <span className={`text-[8px] px-1 py-0.5 rounded font-bold flex items-center gap-0.5 ${
                        a === "BUY" ? "text-[var(--green)] bg-[var(--green)]/10" :
                        a === "SELL" || a === "CLOSE" ? "text-[var(--red)] bg-[var(--red)]/10" :
                        "text-[var(--dim)] bg-white/5"
                      }`}>
                        <Icon name={a === "BUY" ? "fa-circle-arrow-up" : a === "SELL" || a === "CLOSE" ? "fa-circle-arrow-down" : "fa-minus"} className="text-[6px]" />
                        {a}
                      </span>
                      <ProgressBarRoot className="flex-1 h-[3px] bg-white/5 rounded-full overflow-hidden">
                        <ProgressBarFill className="h-full bg-gradient-to-r from-[var(--violet)] to-[var(--cyan)] rounded-full" style={{ width: `${conf}%` }} />
                      </ProgressBarRoot>
                      <span className="text-[9px] text-[var(--dim)] font-mono">{conf.toFixed(0)}%</span>
                    </div>
                    <p className="text-[9px] text-[var(--dim)] mt-0.5 line-clamp-1 break-words">{d.reasoning}</p>
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
            <span className="text-[10px] text-[var(--dim)] tracking-wider font-mono">savant — terminal</span>
            <div className="flex-1" />
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
            <button
              onClick={() => {
                const text = [...activity].reverse().map(e =>
                  `${formatTime12h(e.timestamp)} [${e.pair}] ${e.message}`
                ).join("\n");
                navigator.clipboard.writeText(text);
              }}
              className="text-[var(--dim)] hover:text-[var(--cyan)] transition-colors cursor-pointer"
              title="Copy activity log"
            >
              <Icon name="fa-copy" className="text-[9px]" />
            </button>
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
          <SectionHeader icon="fa-receipt" title="Closed Trades" tag={`${trades.length}`} />
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
