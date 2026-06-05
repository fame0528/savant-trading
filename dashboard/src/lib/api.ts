const API_BASE = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080/api";

export interface ApiResponse<T> {
  data: T;
  error: string | null;
  timestamp: string;
}

export interface EngineStatus {
  running: boolean;
  mode: string;
  uptime_seconds: number;
  pairs: string[];
  autonomy_level: number;
  ai_status: string;
}

export interface Portfolio {
  balance: number;
  equity: number;
  drawdown_pct: number;
  daily_pnl: number;
  unrealized_pnl: number;
  peak_equity: number;
  open_positions: number;
  trades_today: number;
}

export interface Position {
  id: string;
  pair: string;
  side: string;
  entry_price: number;
  current_price: number;
  quantity: number;
  stop_loss: number;
  take_profit_1: number;
  take_profit_2: number;
  take_profit_3: number;
  unrealized_pnl: number;
  risk_amount: number;
  strategy_name: string;
  scale_level: string;
  opened_at: string;
}

export interface TradeRecord {
  id: string;
  pair: string;
  side: string;
  entry_price: number;
  exit_price: number;
  quantity: number;
  pnl: number;
  pnl_pct: number;
  strategy_name: string;
  opened_at: string;
  closed_at: string;
  notes: string;
}

export interface DecisionRecord {
  timestamp: string;
  pair: string;
  action: string;
  side: string;
  entry_price: number;
  stop_loss: number;
  take_profit_1: number;
  take_profit_2: number;
  take_profit_3: number;
  confidence: number;
  reasoning: string;
}

export interface MarketInsight {
  fear_greed: number | null;
  fear_greed_label: string | null;
  btc_dominance: number | null;
  funding_rate: number | null;
  open_interest: number | null;
  block_height: number | null;
  mempool_size: number | null;
  rss_items: number;
  trending_coins: string[];
  summary: string;
}

export interface RiskData {
  circuit_breaker: string;
  drawdown_pct: number;
  max_drawdown: number;
  daily_loss_pct: number;
  max_daily_loss: number;
  open_positions: number;
  max_positions: number;
  max_risk_per_trade: number;
}

export interface SessionData {
  total_pnl: number;
  total_trades: number;
  wins: number;
  losses: number;
  win_rate: number;
  total_decisions: number;
}

export interface MemoryData {
  brier_score: number | null;
  brier_label: string;
  confidence_cap: string;
  total_trades: number;
  cusum_status: string;
  replay_lesson_count: number;
}

export interface ActivityEntry {
  timestamp: string;
  level: string;
  pair: string;
  message: string;
}

export interface ConfigData {
  model: string;
  pairs: string[];
  starting_balance: number;
}

async function get<T>(path: string): Promise<T | null> {
  try {
    const res = await fetch(`${API_BASE}${path}`, { cache: "no-store" });
    if (!res.ok) return null;
    const json = await res.json();
    return json.data ?? json;
  } catch {
    return null;
  }
}

export const api = {
  getStatus: () => get<EngineStatus>("/status"),
  getPortfolio: () => get<Portfolio>("/portfolio"),
  getPositions: () => get<Position[]>("/positions"),
  getTrades: () => get<TradeRecord[]>("/trades"),
  getDecisions: () => get<DecisionRecord[]>("/decisions"),
  getActivity: () => get<ActivityEntry[]>("/activity"),
  getInsight: () => get<MarketInsight>("/insight"),
  getRisk: () => get<RiskData>("/risk"),
  getSession: () => get<SessionData>("/session"),
  getMemory: () => get<MemoryData>("/memory"),
  getEquity: () => get<EquitySnapshot[]>("/equity"),
  getConfig: () => get<ConfigData>("/config"),
};

export interface EquitySnapshot {
  timestamp: string;
  balance: number;
  equity: number;
  drawdown_pct: number;
  open_positions: number;
}
