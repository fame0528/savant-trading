"use client";

import { useEffect, useState, useCallback } from "react";
import { api } from "@/lib/api";
import type {
  EngineStatus,
  Portfolio,
  Position,
  TradeRecord,
  DecisionRecord,
  MarketInsight,
  RiskData,
  SessionData,
  MemoryData,
  ActivityEntry,
  ConfigData,
} from "@/lib/api";

export interface DashboardState {
  status: EngineStatus | null;
  portfolio: Portfolio | null;
  positions: Position[];
  trades: TradeRecord[];
  decisions: DecisionRecord[];
  activity: ActivityEntry[];
  insight: MarketInsight | null;
  risk: RiskData | null;
  session: SessionData | null;
  memory: MemoryData | null;
  config: ConfigData | null;
  online: boolean;
  lastUpdate: Date | null;
}

const EMPTY: DashboardState = {
  status: null,
  portfolio: null,
  positions: [],
  trades: [],
  decisions: [],
  activity: [],
  insight: null,
  risk: null,
  session: null,
  memory: null,
  config: null,
  online: false,
  lastUpdate: null,
};

export function useDashboard(pollMs = 4000) {
  const [state, setState] = useState<DashboardState>(EMPTY);

  const refresh = useCallback(async () => {
    const [status, portfolio, positions, trades, decisions, activity, insight, risk, session, memory, config] =
      await Promise.all([
        api.getStatus(),
        api.getPortfolio(),
        api.getPositions(),
        api.getTrades(),
        api.getDecisions(),
        api.getActivity(),
        api.getInsight(),
        api.getRisk(),
        api.getSession(),
        api.getMemory(),
        api.getConfig(),
      ]);

    setState({
      status,
      portfolio,
      positions: positions ?? [],
      trades: trades ?? [],
      decisions: decisions ?? [],
      activity: activity ?? [],
      insight,
      risk,
      session,
      memory,
      config,
      online: !!status,
      lastUpdate: new Date(),
    });
  }, []);

  useEffect(() => {
    refresh();
    const timer = setInterval(refresh, pollMs);
    return () => clearInterval(timer);
  }, [refresh, pollMs]);

  return { ...state, refresh };
}
