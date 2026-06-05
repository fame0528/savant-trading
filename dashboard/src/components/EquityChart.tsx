"use client";

import { AreaChart, Area, XAxis, YAxis, Tooltip, ResponsiveContainer } from "recharts";
import type { EquitySnapshot } from "@/lib/api";

const fmt = {
  usd: (v: number) =>
    (v < 0 ? "-$" : "$") + Math.abs(v).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 }),
  time: (ts: string) => {
    const d = new Date(ts);
    return `${d.getHours() % 12 || 12}:${String(d.getMinutes()).padStart(2, "0")} ${d.getHours() >= 12 ? "PM" : "AM"}`;
  },
};

export default function EquityChart({ data }: { data: EquitySnapshot[] }) {
  if (!data || data.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center text-[var(--dimmer)] text-xs">
        <i className="fa-solid fa-spinner fa-spin mr-2" /> Collecting equity data…
      </div>
    );
  }

  const chartData = data.map((d) => ({
    time: fmt.time(d.timestamp),
    equity: d.equity,
    balance: d.balance,
  }));

  const minVal = Math.min(...chartData.map((d) => d.equity)) * 0.98;
  const maxVal = Math.max(...chartData.map((d) => d.equity)) * 1.02;

  return (
    <div className="flex-1 px-1 pb-1">
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={chartData} margin={{ top: 4, right: 4, bottom: 0, left: 0 }}>
          <defs>
            <linearGradient id="equityGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="var(--cyan)" stopOpacity={0.3} />
              <stop offset="100%" stopColor="var(--cyan)" stopOpacity={0} />
            </linearGradient>
          </defs>
          <XAxis
            dataKey="time"
            tick={{ fill: "var(--dimmer)", fontSize: 8 }}
            axisLine={false}
            tickLine={false}
            interval="preserveStartEnd"
          />
          <YAxis
            domain={[minVal, maxVal]}
            tick={{ fill: "var(--dimmer)", fontSize: 8 }}
            axisLine={false}
            tickLine={false}
            tickFormatter={(v: number) => `$${v.toFixed(0)}`}
            width={36}
          />
          <Tooltip
            contentStyle={{
              background: "var(--panel-solid)",
              border: "1px solid var(--line)",
              borderRadius: "4px",
              fontSize: "10px",
              fontFamily: "monospace",
              color: "var(--txt)",
            }}
            formatter={(value) => [fmt.usd(Number(value)), "Equity"]}
            labelStyle={{ color: "var(--dim)", fontSize: "9px" }}
          />
          <Area
            type="monotone"
            dataKey="equity"
            stroke="var(--cyan)"
            strokeWidth={1.5}
            fill="url(#equityGrad)"
            dot={false}
            animationDuration={500}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
