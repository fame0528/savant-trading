import type {
  SessionData,
  MarketInsight,
  Position,
  RiskData,
  DecisionRecord,
  TradeRecord,
  ActivityEntry,
} from "./api";

const line = (label: string, value: string | number | null | undefined) =>
  `${label}: ${value ?? "—"}`;

export const copyFormatters = {
  performance: (s: SessionData | null) =>
    [
      "Performance",
      line("Win/Loss", `${s?.wins ?? 0}W / ${s?.losses ?? 0}L`),
      line("Decisions", s?.total_decisions ?? 0),
      line("Trades today", s?.total_trades ?? 0),
    ].join("\n"),

  marketInsight: (i: MarketInsight | null) =>
    [
      "Market Insight",
      line("Fear & Greed", `${i?.fear_greed ?? "—"} (${i?.fear_greed_label ?? "—"})`),
      line("Funding", i?.funding_rate ?? "—"),
      line("BTC Dom", i?.btc_dominance ?? "—"),
      line("Block", i?.block_height ?? "—"),
      line("News", i?.rss_items ?? 0),
      line("Trending", i?.trending_coins?.join(", ") ?? "—"),
    ].join("\n"),

  positions: (p: Position[]) =>
    p.length === 0
      ? "No positions"
      : p
          .map(
            (pos) =>
              `${pos.pair} ${pos.side} @ ${pos.entry_price} | SL: ${pos.stop_loss} | TP1: ${pos.take_profit_1} | PnL: ${pos.unrealized_pnl?.toFixed(2) ?? "—"}`
          )
          .join("\n"),

  risk: (r: RiskData | null) =>
    [
      "Risk Controls",
      line("Circuit breaker", r?.circuit_breaker ?? "OK"),
      line("Drawdown", `${((r?.drawdown_pct ?? 0) * 100).toFixed(1)}% / ${((r?.max_drawdown ?? 0.1) * 100).toFixed(0)}%`),
      line("Daily loss", `${Math.abs((r?.daily_loss_pct ?? 0) * 100).toFixed(1)}% / ${((r?.max_daily_loss ?? 0.05) * 100).toFixed(0)}%`),
      line("Positions", `${r?.open_positions ?? 0} / ${r?.max_positions ?? 3}`),
      line("Risk/trade", `${((r?.max_risk_per_trade ?? 0) * 100).toFixed(0)}%`),
    ].join("\n"),

  decisions: (d: DecisionRecord[]) =>
    d.length === 0
      ? "No decisions"
      : d
          .map(
            (dec) =>
              `${dec.pair} ${dec.action} ${(dec.confidence * 100).toFixed(0)}% — ${dec.reasoning ?? ""}`
          )
          .join("\n"),

  trades: (t: TradeRecord[]) =>
    t.length === 0
      ? "No trades"
      : t
          .map(
            (tr) =>
              `${tr.pair} ${tr.side} ${tr.entry_price}→${tr.exit_price} ${tr.pnl >= 0 ? "+" : ""}${tr.pnl?.toFixed(2)} (${tr.pnl_pct?.toFixed(2)}%)`
          )
          .join("\n"),

  activity: (a: ActivityEntry[]) =>
    a.length === 0
      ? "No activity"
      : a
          .map((e) => `[${e.timestamp}] [${e.level}] ${e.pair}: ${e.message}`)
          .join("\n"),
};

export function downloadTradesCSV(t: TradeRecord[]) {
  if (t.length === 0) return;
  const header = "pair,side,entry,exit,qty,pnl,pnl_pct,closed_at,notes";
  const rows = t.map((tr) =>
    `${tr.pair},${tr.side},${tr.entry_price},${tr.exit_price},${tr.quantity},${tr.pnl.toFixed(2)},${tr.pnl_pct.toFixed(2)},${tr.closed_at},"${(tr.notes ?? "").replace(/"/g, '""')}"`
  );
  const csv = [header, ...rows].join("\n");
  const blob = new Blob([csv], { type: "text/csv" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `savant-trades-${new Date().toISOString().slice(0, 10)}.csv`;
  a.click();
  URL.revokeObjectURL(url);
}
