#!/usr/bin/env python3
"""Analyze M3 sandbox capture files for FID-126 verification metrics.

Extracts per-scenario action, conviction_score, sizing_multiplier, regime_label,
and trigger_weights from the raw LLM response text, then computes:
- Action distribution
- Conviction score distribution (mean, std dev, min, max)
- Regime distribution
- Anti-pattern compliance (count of conviction=0.50 or 0.65)
- Trigger weight distribution

Usage:
    python scripts/analyze_fid126_captures.py                          # auto-detect latest
    python scripts/analyze_fid126_captures.py --capture-dir <PATH>    # specific dir
    python scripts/analyze_fid126_captures.py <PATH>                  # positional
"""
import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path


def find_latest_capture_dir() -> Path:
    """Auto-detect the most recent sandbox_responses/sandbox_YYYY-MM-DD_HH-MM-SS dir."""
    base = Path("data/sandbox_responses")
    if not base.exists():
        return None
    candidates = sorted([d for d in base.iterdir() if d.is_dir() and d.name.startswith("sandbox_")])
    if not candidates:
        return None
    return candidates[-1]


def extract_decision_json(raw: str) -> dict | None:
    """Extract the LLM's decision JSON from raw response text.

    Handles markdown ```json``` wrappers, leading prose, and reasoning text.
    Returns the parsed dict, or None if no valid JSON found.
    """
    if not raw:
        return None

    # Strip thinking tags (Qwen/DeepSeek style)
    raw = re.sub(r"<think>.*?</think>", "", raw, flags=re.DOTALL)
    raw = re.sub(r"<think>.*", "", raw, flags=re.DOTALL)
    raw = raw.strip()

    # Try markdown ```json ... ``` block
    m = re.search(r"```(?:json)?\s*(\{.*?\})\s*```", raw, flags=re.DOTALL)
    if m:
        try:
            return json.loads(m.group(1))
        except json.JSONDecodeError:
            pass

    # Try bare JSON object (find first { and last })
    if "{" in raw:
        start = raw.find("{")
        # Find matching closing brace via depth tracking
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
    parser = argparse.ArgumentParser(description="FID-126 verification metrics")
    parser.add_argument(
        "capture_dir_pos",
        nargs="?",
        default=None,
        help="Capture directory (positional, optional)",
    )
    parser.add_argument(
        "--capture-dir",
        default=None,
        help="Capture directory (named, optional)",
    )
    args = parser.parse_args()

    if args.capture_dir:
        CAPTURE_DIR = Path(args.capture_dir)
    elif args.capture_dir_pos:
        CAPTURE_DIR = Path(args.capture_dir_pos)
    else:
        CAPTURE_DIR = find_latest_capture_dir()
        if CAPTURE_DIR is None:
            print("ERROR: No capture directory found in data/sandbox_responses/")
            sys.exit(1)
        print(f"Auto-detected latest capture dir: {CAPTURE_DIR}\n")

    if not CAPTURE_DIR.exists():
        print(f"ERROR: {CAPTURE_DIR} does not exist")
        sys.exit(1)

    capture_files = sorted(CAPTURE_DIR.glob("*.json"))
    print(f"Found {len(capture_files)} capture files\n")

    # Aggregators
    actions = Counter()
    regimes = Counter()
    trigger_strong = []
    trigger_moderate = []
    trigger_weak = []
    convictions = []
    sizing_multipliers = []
    parse_failures = []
    decisions = []  # per-scenario records for the table

    for cf in capture_files:
        with open(cf, encoding='utf-8', errors='replace') as f:
            data = json.load(f)
        # Some capture files wrap the record in a list; unwrap if so
        if isinstance(data, list):
            if len(data) == 0:
                parse_failures.append(cf.stem)
                actions["PARSE_FAIL"] += 1
                continue
            data = data[0] if isinstance(data[0], dict) else {}
        if not isinstance(data, dict):
            parse_failures.append(cf.stem)
            actions["PARSE_FAIL"] += 1
            continue
        scen = data.get("scenario_id", cf.stem)
        cat = data.get("category", "?")
        diff = data.get("difficulty", "?")
        exp = data.get("expected_action", "?")
        raw = data.get("raw_response", "")

        decision = extract_decision_json(raw)
        if decision is None:
            parse_failures.append(scen)
            actions["PARSE_FAIL"] += 1
            decisions.append({
                "scenario": scen,
                "category": cat,
                "difficulty": diff,
                "expected": exp,
                "action": "PARSE_FAIL",
                "conviction": None,
                "sizing_mult": None,
                "regime": None,
                "tw_strong": None,
                "tw_moderate": None,
                "tw_weak": None,
            })
            continue

        action = decision.get("action", "?").upper()
        conviction = decision.get("conviction_score")
        sizing_mult = decision.get("sizing_multiplier")
        regime = decision.get("regime_label", "?").upper() if decision.get("regime_label") else None
        tw = decision.get("trigger_weights", {}) or {}
        tw_s = tw.get("strong")
        tw_m = tw.get("moderate")
        tw_w = tw.get("weak")

        actions[action] += 1
        if regime:
            regimes[regime] += 1
        if conviction is not None:
            convictions.append(conviction)
        if sizing_mult is not None:
            sizing_multipliers.append(sizing_mult)
        if tw_s is not None:
            trigger_strong.append(tw_s)
        if tw_m is not None:
            trigger_moderate.append(tw_m)
        if tw_w is not None:
            trigger_weak.append(tw_w)

        decisions.append({
            "scenario": scen,
            "category": cat,
            "difficulty": diff,
            "expected": exp,
            "action": action,
            "conviction": conviction,
            "sizing_mult": sizing_mult,
            "regime": regime,
            "tw_strong": tw_s,
            "tw_moderate": tw_m,
            "tw_weak": tw_w,
        })

    n = len(capture_files)
    n_parsed = n - len(parse_failures)
    n_buy = actions.get("BUY", 0)
    n_sell = actions.get("SELL", 0)
    n_hold = actions.get("HOLD", 0) + actions.get("PASS", 0)
    n_close = actions.get("CLOSE", 0)
    n_adjust = actions.get("ADJUST_STOP", 0)
    n_parse_fail = actions.get("PARSE_FAIL", 0)

    print("=" * 70)
    print(f"FID-126 VERIFICATION METRICS — M3 SANDBOX RUN {CAPTURE_DIR.name}")
    print("=" * 70)
    print(f"\nTotal scenarios:         {n}")
    print(f"Parsed successfully:     {n_parsed}")
    print(f"Parse failures:          {n_parse_fail}  ({100*n_parse_fail/n:.0f}%)")
    print(f"\nAction distribution:")
    print(f"  BUY:           {n_buy:3d}  ({100*n_buy/n:.0f}%)   [target: 9-18 / 15-30%]")
    print(f"  SELL:          {n_sell:3d}  ({100*n_sell/n:.0f}%)")
    print(f"  HOLD/PASS:     {n_hold:3d}  ({100*n_hold/n:.0f}%)")
    print(f"  CLOSE:         {n_close:3d}  ({100*n_close/n:.0f}%)")
    print(f"  ADJUST_STOP:   {n_adjust:3d}  ({100*n_adjust/n:.0f}%)")

    if convictions:
        n_conv = len(convictions)
        mean_conv = sum(convictions) / n_conv
        var_conv = sum((c - mean_conv) ** 2 for c in convictions) / n_conv
        std_conv = var_conv ** 0.5
        min_conv = min(convictions)
        max_conv = max(convictions)
        n_at_050 = sum(1 for c in convictions if abs(c - 0.50) < 0.001)
        n_at_065 = sum(1 for c in convictions if abs(c - 0.65) < 0.001)
        n_at_threshold = n_at_050 + n_at_065
        print(f"\nConviction score distribution ({n_conv} samples):")
        print(f"  Mean:  {mean_conv:.3f}")
        print(f"  Std:   {std_conv:.3f}   [target: > 0.15]")
        print(f"  Min:   {min_conv:.3f}")
        print(f"  Max:   {max_conv:.3f}")
        print(f"  At 0.50: {n_at_050}   [anti-pattern: should be 0]")
        print(f"  At 0.65: {n_at_065}   [anti-pattern: should be 0]")
        print(f"  At either threshold: {n_at_threshold}   [anti-pattern: should be 0]")
    else:
        mean_conv = std_conv = 0
        print(f"\nConviction score distribution: NO DATA (all parses failed or no conviction_score)")

    if regimes:
        n_reg = sum(regimes.values())
        print(f"\nRegime distribution ({n_reg} samples):")
        for r in ["TRENDING", "VOLATILE", "RANGING", "GREYZONE"]:
            count = regimes.get(r, 0)
            pct = 100 * count / n_reg if n_reg else 0
            print(f"  {r:10s}: {count:3d}  ({pct:.0f}%)")
        gz_pct = 100 * regimes.get("GREYZONE", 0) / n_reg if n_reg else 0
        print(f"  GreyZone %:  {gz_pct:.0f}%   [target: 30-50%]")
    else:
        print(f"\nRegime distribution: NO DATA")

    if trigger_strong:
        print(f"\nTrigger weight counts (strong):")
        for v in sorted(set(trigger_strong)):
            c = trigger_strong.count(v)
            print(f"  {v}: {c} scenarios")
    else:
        print(f"\nTrigger weights: NO DATA")

    # Buy count by regime (regime coverage)
    print(f"\n--- Per-scenario decisions ---")
    print(f"{'Scenario':<10} {'Cat':<12} {'Action':<14} {'Conv':<6} {'Regime':<10} {'TW(s/m/w)':<10} {'Expected':<25}")
    print("-" * 90)
    for d in decisions:
        conv = f"{d['conviction']:.2f}" if d['conviction'] is not None else "—"
        tw = f"{d['tw_strong']}/{d['tw_moderate']}/{d['tw_weak']}" if d['tw_strong'] is not None else "—"
        regime = d['regime'] or "—"
        print(f"{d['scenario']:<10} {d['category'][:11]:<12} {d['action']:<14} {conv:<6} {regime:<10} {tw:<10} {d['expected'][:24]:<25}")

    # VERDICT
    print(f"\n{'='*70}")
    print("VERDICT (vs FID-126 targets)")
    print(f"{'='*70}")
    buy_pct = 100 * n_buy / n
    print(f"  Buy count {n_buy}/{n} ({buy_pct:.0f}%): {'PASS' if 9 <= n_buy <= 18 else 'FAIL'} (target 15-30%)")
    if convictions:
        print(f"  Conviction std {std_conv:.3f}: {'PASS' if std_conv > 0.15 else 'FAIL'} (target > 0.15)")
        print(f"  Anti-pattern (conv=0.50/0.65): {'PASS' if n_at_threshold == 0 else 'FAIL — {} threshold outputs'.format(n_at_threshold)}")
    if regimes:
        gz_pct = 100 * regimes.get("GREYZONE", 0) / sum(regimes.values()) if sum(regimes.values()) else 0
        print(f"  GreyZone % {gz_pct:.0f}%: {'PASS' if 30 <= gz_pct <= 50 else 'WARN'} (target 30-50%)")
    print(f"  Regime coverage (≥1 Buy per regime): ", end="")
    if n_buy > 0 and regimes:
        buy_regimes = set()
        for d in decisions:
            if d['action'] == 'BUY' and d['regime']:
                buy_regimes.add(d['regime'])
        if len(buy_regimes) >= 4:
            print(f"PASS (BUYs in {buy_regimes})")
        else:
            print(f"FAIL (BUYs in only {buy_regimes})")
    else:
        print(f"N/A (0 BUYs)")


if __name__ == "__main__":
    main()
