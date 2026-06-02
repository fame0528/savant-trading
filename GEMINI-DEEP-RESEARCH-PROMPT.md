# Gemini Deep Research: Savant Trading Agent Optimization

## Research Objective

We are building an autonomous AI-powered crypto trading agent called **Savant** that trades on Kraken exchange 24/7. We need optimal configuration settings for the LLM and trading parameters to maximize profitability while maintaining strict risk management.

We are NOT asking for code. We are asking for **expert-level guidance on configuring an LLM-based trading agent** — temperature, token budgets, prompt architecture, risk parameters, and any other settings that affect decision quality.

---

## Model Specifications

| Spec | Value |
|------|-------|
| Model | mimo-v2.5-pro (Xiaomi MiMo) |
| Context window | 1,048,576 tokens (1M) |
| Max output | 131,072 tokens (128K) |
| Architecture | Mixture-of-Experts (MoE), 309B total / 15B active params |
| Capabilities | Deep thinking, streaming, function calling, structured output |
| Rate limits | 100 RPM, 10M TPM |
| Pricing (≤256K ctx) | $1/M input, $0.2/M cached, $3/M output |
| Pricing (≤1M ctx) | $2/M input, $0.4/M cached, $6/M output |
| API | OpenAI-compatible via OpenGateway proxy |
| Access | https://opengateway.gitlawb.com/v1 |
| API key | `ogw_live_56a8ce019141ea5a32d26860281b7dbb` |

### Model Behavior Notes

- The model has a "thinking" mode that produces reasoning in a separate `reasoning` field before emitting content in the `content` field
- With `stream: true`, both fields accumulate via SSE chunks
- With `stream: false`, the full response includes both `reasoning` and `content` in a single JSON response
- The model returns responses in the format: `choices[0].message.content` (actual answer) and `choices[0].message.reasoning` (chain-of-thought)
- Our parser tries `content` first, falls back to `reasoning` if content is empty

---

## Current Configuration

### AI Settings (config/default.toml)

```toml
[ai]
provider = "opengateway"
endpoint = "https://opengateway.gitlawb.com/v1"
model = "mimo-v2.5-pro"
api_key_env = "OPENGATEWAY_API_KEY"
autonomy_level = 3              # 1=Guided, 2=Supervised, 3=Autonomous
max_decisions_per_hour = 20
context_window_candles = 100    # Number of candles sent in prompt
knowledge_token_budget = 8000   # Max chars of knowledge units in prompt
price_tolerance_pct = 10.0      # Entry price must be within 10% of market
max_retries = 3
temperature = 0.7
max_tokens = 131072             # Set to model's actual max output (128K)
timeout_secs = 180
```

### Risk Settings

```toml
[risk]
max_risk_per_trade = 0.20       # 20% of portfolio per trade (5 positions)
dynamic_risk_tiers = [
    { balance = 500.0, risk_pct = 0.20 },    # 5 × 20% = 100% deployed
    { balance = 5000.0, risk_pct = 0.10 },   # 5 × 10% = 50% deployed
    { balance = 50000.0, risk_pct = 0.05 },  # 5 × 5% = 25% deployed
    { balance = 999999.0, risk_pct = 0.02 }, # 5 × 2% = 10% deployed
]
max_daily_loss = 0.20           # 20% daily loss = halt
max_drawdown = 0.40             # 40% drawdown from peak = stop
max_positions = 5
min_rr_ratio = 1.5              # Minimum risk-reward ratio
```

### Trading Settings

```toml
[trading]
pairs = ["BTC/USD", "ETH/USD", "SOL/USD", "XRP/USD", "DOGE/USD"]
scan_all_pairs = true           # Discover all Kraken USD pairs
timeframe = "5m"
timeframes = ["5m", "1h", "4h"] # Multi-timeframe analysis
starting_balance = 50.0         # $50 paper budget
fee_rate = 0.0026               # 0.26% Kraken taker
slippage_pct = 0.0005           # 0.05% slippage estimate
```

### Training Settings

```toml
[training]
min_sample_size = 5
failure_win_rate = 0.30         # Below 30% = anti-pattern
max_portfolio_heat = 0.40       # Max total risk / equity
utility_learning_rate = 0.05    # Knowledge utility adjustment per episode
utility_archive_threshold = 0.30
brier_cap_threshold = 0.25      # Brier score threshold for confidence cap
memory_context_min_trades = 5   # Min episodes before memory activates
```

---

## System Prompt Architecture

The system prompt is ~42,000 characters and consists of 5 layers:

### Layer 1: Base Identity (~1,200 chars)
```
You are Savant — an autonomous crypto trading agent operating on Kraken exchange.

Core Principles:
- You are a rigorous trading agent. You do not guess.
- Every decision must be backed by data from the provided market context.
- You optimize for mathematical correctness, extreme robustness, and long-term maintainability.
- You never take positions you cannot justify with specific technical or fundamental reasoning.
- You are not a chatbot. You are a systematic trader. Be concise. Be precise. Be profitable.

Operating Rules:
- Always specify exact entry, stop-loss, and take-profit prices.
- Never risk more than the configured max risk per trade.
- Always provide a confidence score (0.0 to 1.0) based on setup quality.
- If no high-quality setup exists, output a HOLD decision.
- Favor high R:R setups (minimum 1.5:1). Reject anything below.
- Consider fees (0.26% Kraken taker) and slippage (0.05%) in all calculations.
```

### Layer 2: Risk Constraints (~800 chars)
```
Risk Constraints (Hard Limits — You Cannot Override These):
- Max risk per trade: 1% of portfolio
- Max daily loss: 3% — all trading halts if breached
- Max drawdown from peak: 10% — all positions closed, bot stops
- Max concurrent positions: 3
- Minimum risk-reward ratio: 1.5:1
- Circuit breakers are INDEPENDENT of you — they will close positions regardless

Position Sizing:
- Formula: size = (balance * max_risk_pct) / (entry - stop_loss)
- Stop loss is MANDATORY on every position — no exceptions
- Move to break-even after 1R profit
- Trail stop after 2R profit using ATR-based trailing
```

### Layer 3: Strategy Knowledge (~1,500 chars)
```
Scale-Out: TP1=50%, TP2=30%, TP3=20%
Trailing: Break-even at 1R, ATR trail at 2R
Fees: 0.26% taker, 0.52% round trip
Regime: Trending=favor momentum, Ranging=favor mean reversion, Volatile=reduce size
```

### Layer 4: ECHO Protocol Rules (~2,500 chars)
```
Trading rules from professional traders:
- Sell into strength (Pradeep Bondi): 10-20% in 2-3 days → sell 80%
- 3 losses = stop for the day (TJR/Fabio)
- Don't marry your position (Fabio Valentina)
- Four-factor model for losses: Setup, Process, Market, Trader
- Session awareness: London 2-5 AM EST, NY 7-10 AM EST
- Compound strategy: risk daily profits on directional days
```

### Layer 5: Knowledge Units (~8,000 chars budget)
2,959 tagged knowledge units from 22 curated sources covering:
- Technical analysis (RSI, ADX, EMA, volume profile, order flow)
- On-chain analytics (MVRV, SOPR, NUPL, exchange flows)
- Risk management (position sizing, drawdown control, correlation)
- Market microstructure (order book, funding rates, liquidations)
- Trading psychology (discipline, FOMO, revenge trading)

Knowledge is selected via MMR (Maximal Marginal Relevance) with utility scoring. Units with higher utility scores get priority. The selection is context-aware — it matches knowledge units to current market conditions (regime, sentiment, on-chain metrics).

### Layer 6: Memory Context (dynamic, ~500-2,000 chars)
After 5+ episodes, the agent receives:
- Recent episode outcomes (win/loss, confidence, category)
- Semantic patterns (e.g., "Trend Bull: 65% win rate, PF 1.83")
- Anti-patterns (e.g., "On-Chain: 29% win rate — reduce conviction")
- Active lessons from experience replay

---

## User Message Structure

The user message (~2,500 chars) contains:

1. **Market Data**: Current candle (OHLCV), 100 historical candles
2. **Indicators**: EMA fast/slow, RSI, ATR, ADX, VWAP, volume profile
3. **Regime**: Trending/Ranging/Volatile (detected from ADX + ATR)
4. **Insight**: Fear & Greed index, funding rate, MVRV, SOPR, NUPL, liquidation risk
5. **RSS News**: Top 10 headlines with relevance scoring
6. **Conditions Summary**: Actionable assessments using SOUL.md thresholds
7. **Order Book**: Imbalance metric (bid-heavy vs ask-heavy)
8. **Session**: Asian/European/US/Late US/Weekend
9. **Multi-Timeframe**: 1h candles for higher-TF context
10. **Positions**: Current open positions (if any)
11. **Account**: Balance, equity, drawdown

---

## Required Output Format

The agent must respond with ONLY a JSON object:
```json
{
    "action": "BUY" | "SELL" | "HOLD" | "CLOSE" | "ADJUST_STOP",
    "pair": "BTC/USD",
    "side": "Long" | "Short",
    "entry_price": 0.0,
    "stop_loss": 0.0,
    "take_profit_1": 0.0,
    "take_profit_2": 0.0,
    "take_profit_3": 0.0,
    "position_size_pct": 0.0,
    "confidence": 0.0,
    "reasoning": "Your reasoning here — cite specific data points",
    "knowledge_sources": ["source-id-001"],
    "risk_reward": 0.0
}
```

---

## Training Results & Benchmarks

### v0.4.3 Training (1,134 episodes)

| Metric | Value | Target |
|--------|-------|--------|
| Brier Score | 0.24-0.28 | < 0.25 |
| 50-75% Confidence Accuracy | 100% | > 80% |
| 75-100% Confidence Accuracy | 80% | > 85% |
| 0-25% Confidence Bucket | 18% accuracy (noise) | 0 trades |
| Action Rate | 35% | 40-60% |
| Error Rate | 0% | 0% |
| Knowledge Utility | Trending up | Trending up |
| Auto-Lessons | 1-2 per run | 3-7 per run |

### Confidence Distribution (last run)
| Range | Count | Accuracy | Avg Conf |
|-------|-------|----------|----------|
| 0-25% | 11 | 18% | 0% |
| 25-50% | 1 | 100% | 35% |
| 50-75% | 3 | 100% | 60% |
| 75-100% | 5 | 80% | 78% |

### Category Edge (last run)
| Category | Win Rate | Notes |
|----------|----------|-------|
| Catalyst | 100% | Small sample |
| Trend Bear | 100% | Small sample |
| Microstructure | 100% | Small sample |
| Correlation | 67% | |
| Sentiment | 50% | |
| Session | 50% | |
| Trend Bull | 25% | Weakest — needs more training |

### Known Behavioral Issues
1. **Short bias** — Agent historically only shorts. Recently fixed by boosting capitulation buy signals in training scenarios. One LONG trade appeared at 65% confidence.
2. **0-25% confidence noise** — Agent takes trades at near-zero confidence. Confidence floor (40%) implemented to downgrade these to Hold.
3. **Over-conservative** — With real data, agent holds 65% of the time. It correctly identifies conflicting signals but may be missing valid setups.
4. **Stream parse errors** — ~5% of LLM calls fail on SSE streaming. Non-streaming fallback handles it.

---

## SOUL.md Action Triggers (Key Decision Framework)

The agent uses these thresholds for decision-making:

### On-Chain Triggers
- MVRV < 1.0 + SOPR < 1.0 = **Capitulation Buy** (HIGH conviction)
- MVRV < 0.8 = **Deep Undervaluation** (HIGH conviction)
- MVRV > 3.5 = **Euphoria Sell** (HIGH conviction)
- SOPR reset (0.98-1.02 in bull market) = **Buy Zone** (MEDIUM conviction)

### Sentiment Triggers
- Fear & Greed ≤ 15 = **Extreme Fear** → contrarian buy (HIGH conviction)
- Fear & Greed ≤ 30 = **Fear** → contrarian buy (MEDIUM conviction)
- Fear & Greed ≥ 85 = **Extreme Greed** → contrarian sell (HIGH conviction)
- Fear & Greed ≥ 70 = **Greed** → caution, tighten stops (MEDIUM conviction)

### Funding Rate Triggers
- Funding > 0.05%/8hr = **Overleveraged Longs** → squeeze risk, avoid longs
- Funding < -0.01%/8hr = **Overleveraged Shorts** → squeeze setup, watch for long entries
- Funding > 0.5%/8hr = **Crisis Level** → do not trade, wait for resolution

### Regime Triggers
- ADX > 25 = **Trending** → favor momentum entries
- ADX < 20 = **Ranging** → favor mean reversion, volume profile entries
- ATR > 1.5x average = **Volatile** → reduce position size, widen stops

### Conflict Resolution
When signals conflict, the agent should:
1. Count aligned triggers per direction
2. If 3+ triggers align → trade with conviction
3. If triggers conflict equally → HOLD
4. If crisis-level anomaly detected → HOLD, wait 3+ candles

---

## Research Questions

### 1. Temperature Optimization
- Current: 0.7
- For a trading agent that needs consistent, well-reasoned JSON output with a chain-of-thought model, what temperature produces the best decisions?
- Consider: Too low = not enough exploration of edge cases. Too high = inconsistent JSON output, hallucinated prices.
- The model has separate thinking/reasoning — does temperature affect thinking quality differently than output quality?

### 2. Token Budget Allocation
- Current: max_tokens = 131072 (128K), knowledge_token_budget = 8000 chars
- With a 1M context window and 128K output, how should we allocate tokens?
- Is 8000 chars of knowledge (~2000 tokens) too little? Too much?
- Should we send more candles (currently 100 × 5m = 8.3 hours of data)?
- How much of the 128K output budget should we reserve for thinking vs. content?

### 3. Prompt Architecture
- Current: 6 layers totaling ~42K chars (~10-12K tokens)
- Is the prompt too long? Too short? Missing critical information?
- Should knowledge units be injected differently (e.g., as a separate message, or in the system prompt)?
- Should we use few-shot examples in the prompt?
- Should the conditions_summary be more or less detailed?

### 4. Risk Parameters
- Current: 20% per trade, 20% daily loss, 40% drawdown, 5 positions
- For a $50 starting balance, are these too aggressive? Too conservative?
- Should position sizing scale differently with balance?
- Is the 1.5:1 minimum R:R too low? Too high?

### 5. Output Quality
- Current: JSON-only output with 3-pass parsing (strict → repair → partial extraction)
- The model sometimes returns reasoning before JSON (thinking mode). How should we handle this?
- Should we ask for structured output differently?
- Should we use function calling instead of JSON-in-content?

### 6. Training Methodology
- Current: Random scenarios with weighted categories, Brier score calibration, experience replay
- Is the training approach sound? What improvements would increase calibration?
- Should we train on real market data instead of synthetic scenarios?
- How many episodes are needed for stable calibration?

### 7. Multi-Timeframe Analysis
- Current: 5m primary + 1h + 4h candles sent in prompt
- Is this the right set of timeframes for crypto?
- Should we add daily/weekly for macro context?
- How should we weight different timeframes in the prompt?

### 8. Session Awareness
- Current: Asian/European/US/Late US/Weekend with size multipliers
- Should the agent trade differently in different sessions?
- Are the session boundaries correct for crypto (24/7 market)?
- Should we disable trading during certain sessions?

### 9. Confidence Calibration
- Current: Agent reports confidence 0.0-1.0, but 0-25% bucket has 18% accuracy
- How should we calibrate confidence scores?
- Should we use a different confidence model (e.g., ensemble of indicators)?
- Is the 40% confidence floor the right threshold?

### 10. Model-Specific Optimization
- MiMo v2.5 Pro has "deep thinking" capability — how do we best leverage this?
- Should we disable thinking for speed, or keep it for quality?
- Are there specific prompt patterns that work better with MoE models?
- How does the model handle conflicting signals in its reasoning?

---

## Deliverable

Provide a comprehensive configuration recommendation covering:

1. **Optimal temperature** for trading decisions (with reasoning)
2. **Token budget allocation** (context window, output, knowledge, candles)
3. **Prompt architecture improvements** (structure, length, examples)
4. **Risk parameter recommendations** (position sizing, drawdown limits, R:R)
5. **Output format recommendations** (JSON structure, parsing strategy)
6. **Training methodology improvements** (scenario design, calibration)
7. **Model-specific settings** (thinking mode, streaming, retry strategy)
8. **Any other settings** that would improve the agent's trading performance

For each recommendation, explain:
- What the current setting is
- What you recommend
- Why (with reference to trading theory, LLM behavior, or both)
- Expected impact on performance

We are optimizing for **profitability** over the long term (hundreds of trades), not for any single trade. The agent must be robust, calibrated, and disciplined.
