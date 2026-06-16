import type {
  SessionData,
  MarketInsight,
  Position,
  RiskData,
  DecisionRecord,
  TradeRecord,
  ActivityEntry,
  JuryStateSnapshot,
  JuryCycleRecord,
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
          .map((e) => {
            const src = e.source ? `[${e.source}] ` : "";
            return `[${e.timestamp}] [${e.level}] ${src}${e.pair}: ${e.message}`;
          })
          .join("\n"),

  // FID-162: Jury status snapshot
  jury: (j: JuryStateSnapshot | null) =>
    j
      ? [
          "Jury Status",
          line("Source", j.source),
          line("Enabled", j.enabled ? "yes" : "no"),
          line("Jury size", j.jury_size),
          line("M3 control", j.m3_control_active ? "active" : "off"),
          line("Free models", j.free_models_used.join(", ") || "—"),
          line("Veto enabled", j.veto_enabled ? "yes" : "no"),
          line("Veto threshold", j.veto_threshold),
          line("Regime sizes", `T:${j.regime_sizes.trending} R:${j.regime_sizes.ranging} V:${j.regime_sizes.volatile}`),
          line("Evaluations", j.cumulative.total_evaluations),
          line("Quorum failures", j.cumulative.quorum_failures),
          line("Total verdicts", j.cumulative.total_verdicts),
          line("Total failures", j.cumulative.total_failures),
          line("Avg latency", `${Math.round(j.cumulative.total_latency_ms / Math.max(1, j.cumulative.total_evaluations))}ms`),
          line("Key health", `${j.key_health.healthy}/${j.key_health.total} healthy, ${j.key_health.rotating} rotating`),
          line("M3 calls", j.estimated_m3_calls),
          line("Free model calls", j.estimated_free_model_calls),
          line("Veto flag now", j.veto_flag_active_now ? "ACTIVE" : "clear"),
          line("Last cycle", j.last_cycle_at ?? "never"),
        ].join("\n")
      : "Jury: no data",

  juryRecent: (c: JuryCycleRecord[]) =>
    c.length === 0
      ? "No jury cycles recorded"
      : c
          .map((cy) => {
            const v = cy.verdict_breakdown;
            const veto = cy.veto_detected
              ? cy.veto_enforced
                ? ` VETO ENFORCED[${cy.veto_enforced_pairs.join(",")}]`
                : " VETO DETECTED"
              : "";
            return `#${cy.cycle_id} ${cy.timestamp} | ${v.buy}B/${v.sell}S/${v.hold}H/${v.failed}F | consensus ${(cy.consensus_strength * 100).toFixed(0)}% | ${cy.consensus_action}${cy.judge_action ? ` (judge: ${cy.judge_action})` : ""}${veto}`;
          })
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
