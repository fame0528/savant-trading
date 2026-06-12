"""Extract LLM-emitted Buy/Sell details and check price validity + position sizing.

This is a one-off investigation script (FID-126 environment check) to determine
whether the test wallet balance + MIN_NOTIONAL_USD settings are artificially
constraining trade execution, rather than the gate or LLM being the bottleneck.
"""
import json
import re
from pathlib import Path


def extract_decision_json(raw):
    if not raw:
        return None
    raw = re.sub(r"<think>.*?</think>", "", raw, flags=re.DOTALL)
    raw = re.sub(r"<think>.*", "", raw, flags=re.DOTALL)
    raw = raw.strip()
    m = re.search(r"```(?:json)?\s*(\{.*?\})\s*```", raw, flags=re.DOTALL)
    if m:
        try:
            return json.loads(m.group(1))
        except json.JSONDecodeError:
            pass
    if "{" in raw:
        start = raw.find("{")
        depth = 0
        in_string = False
        escape = False
        end = start
        for i in range(start, len(raw)):
            c = raw[i]
            if escape:
                escape = False
                continue
            if c == "\\":
                escape = True
                continue
            if c == '"':
                in_string = not in_string
                continue
            if in_string:
                continue
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
                if depth == 0:
                    end = i + 1
                    break
        if depth == 0:
            try:
                return json.loads(raw[start:end])
            except json.JSONDecodeError:
                pass
    return None


def main():
    all_trades = []
    for run_dir in [
        "sandbox_2026-06-12_06-02-44",
        "sandbox_2026-06-12_06-28-50",
    ]:
        dir_path = Path(f"data/sandbox_responses/{run_dir}")
        if not dir_path.exists():
            continue
        for cf in sorted(dir_path.glob("*.json")):
            with open(cf, encoding="utf-8", errors="replace") as f:
                data = json.load(f)
            if isinstance(data, list):
                data = data[0] if (data and isinstance(data[0], dict)) else {}
            if not isinstance(data, dict):
                continue
            scen = data.get("scenario_id", cf.stem)
            current_price = data.get("current_price", 0.0)
            raw = data.get("raw_response", "")
            d = extract_decision_json(raw)
            if d is None:
                continue
            a = d.get("action", "?").upper()
            if a not in ("BUY", "SELL"):
                continue
            all_trades.append(
                {
                    "scenario": scen,
                    "run": run_dir[-8:],
                    "action": a,
                    "pair": d.get("pair", "?"),
                    "entry_price": d.get("entry_price", 0.0),
                    "stop_loss": d.get("stop_loss", 0.0),
                    "tp1": d.get("take_profit_1", 0.0),
                    "position_size_pct": d.get("position_size_pct", 0.0),
                    "current_price": current_price,
                    "conviction": d.get("conviction_score", 0.0),
                    "sizing_mult": d.get("sizing_multiplier", 0.0),
                    "confidence": d.get("confidence", 0.0),
                }
            )

    print(f"Found {len(all_trades)} LLM-emitted Buy/Sell actions across both runs")
    print()
    print(
        f"{'Scenario':<12} {'Run':<10} {'Act':<5} {'Pair':<10} {'Entry':<10} "
        f"{'Stop':<10} {'TP1':<10} {'Size%':<6} {'Price':<10} {'Conv':<5} {'SizeM':<5}"
    )
    print("=" * 110)
    for t in all_trades:
        print(
            f"{t['scenario']:<12} {t['run']:<10} {t['action']:<5} {t['pair']:<10} "
            f"{t['entry_price']:<10.2f} {t['stop_loss']:<10.2f} {t['tp1']:<10.2f} "
            f"{t['position_size_pct']:<6.1f} {t['current_price']:<10.2f} "
            f"{t['conviction']:<5.2f} {t['sizing_mult']:<5.2f}"
        )

    # Price validity
    print()
    print("=" * 80)
    print("Price validity (entry vs current, >5% = hallucinated):")
    print("=" * 80)
    valid = [t for t in all_trades if t["current_price"] > 0 and t["entry_price"] > 0]
    for t in valid:
        pct_off = abs(t["entry_price"] - t["current_price"]) / t["current_price"] * 100
        flag = " <-- HALLUCINATED" if pct_off > 5 else ""
        print(
            f"  {t['scenario']:<10} {t['run']:<8} entry={t['entry_price']:<10.2f} "
            f"curr={t['current_price']:<10.2f} ({pct_off:>5.2f}% off){flag}"
        )

    # Position sizing at various balances
    # Formula: scaled_risk = balance * tier_risk * BASE_RISK_PCT(0.02) * KELLY(0.5) * sizing_mult * conviction_scaler
    # Default tier_risk = 0.10 for balance in [$500, $5000)
    # conviction_scaler = (conviction - 0.50) * 2.0, clamped to [0, 1]
    print()
    print("=" * 80)
    print("Position sizing at $1000 test balance (tier_risk=0.10, BASE=0.02, KELLY=0.5):")
    print("=" * 80)
    MIN_NOTIONAL_USD = 1.0
    for t in all_trades:
        if t["entry_price"] <= 0 or t["stop_loss"] <= 0:
            continue
        conv_scaler = max(0.0, min(1.0, (t["conviction"] - 0.50) * 2.0))
        size_m = t["sizing_mult"] if t["sizing_mult"] > 0 else 0.5
        # Use LLM's emitted position_size_pct as a cross-check
        for bal, tier in [(100, 1.00), (500, 1.00), (1000, 0.10), (5000, 0.10), (10000, 0.05)]:
            scaled_risk = bal * tier * 0.02 * 0.5 * size_m * conv_scaler
            notional = bal * (t["position_size_pct"] or 1.0) / 100.0
            below = " <-- BELOW MIN_NOTIONAL" if scaled_risk < MIN_NOTIONAL_USD else ""
            print(
                f"  {t['scenario']:<10} {t['run']:<8} bal=${bal:>5} tier={tier:.2f} "
                f"conv_scaler={conv_scaler:.2f} size_m={size_m:.2f} -> "
                f"scaled_risk=${scaled_risk:.2f} notional=${notional:.2f}{below}"
            )
        print()

    # Summary: how many trades would pass MIN_NOTIONAL at each balance?
    print()
    print("=" * 80)
    print("Summary: trades passing MIN_NOTIONAL_USD=$1.0 at each balance:")
    print("=" * 80)
    for bal, tier in [(100, 1.00), (500, 1.00), (1000, 0.10), (5000, 0.10), (10000, 0.05)]:
        passing = 0
        for t in all_trades:
            if t["entry_price"] <= 0 or t["stop_loss"] <= 0:
                continue
            conv_scaler = max(0.0, min(1.0, (t["conviction"] - 0.50) * 2.0))
            size_m = t["sizing_mult"] if t["sizing_mult"] > 0 else 0.5
            scaled_risk = bal * tier * 0.02 * 0.5 * size_m * conv_scaler
            if scaled_risk >= MIN_NOTIONAL_USD:
                passing += 1
        total = sum(1 for t in all_trades if t["entry_price"] > 0 and t["stop_loss"] > 0)
        print(f"  bal=${bal:>5} (tier={tier:.2f}): {passing}/{total} trades pass MIN_NOTIONAL")


if __name__ == "__main__":
    main()
