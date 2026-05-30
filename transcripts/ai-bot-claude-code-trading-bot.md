# How To Actually Build a Trading Bot With Claude Code (Fully Automated)

> **Source:** YouTube — Full step-by-step tutorial on building an automated trading bot using Claude Code, Hidden Markov Models, and the Alpaca brokerage API.
>
> **Topics Covered:** Claude Code setup, project scaffolding, Hidden Markov Models (HMM), regime detection (crash/bear/neutral/bull/euphoria), volatility-based allocation strategies, walk-forward backtesting, risk management with circuit breakers, Alpaca brokerage integration, order execution, Streamlit dashboard, paper trading, live trading transition

---

## Table of Contents

1. [Introduction — What We Are Building](#introduction--what-we-are-building)
2. [System Architecture — Five Components](#system-architecture--five-components)
3. [Getting Started — IDE and Claude Code Setup](#getting-started--ide-and-claude-code-setup)
4. [Phase 1: Project Scaffolding](#phase-1-project-scaffolding)
5. [Phase 2: The Brain — Hidden Markov Models](#phase-2-the-brain--hidden-markov-models)
6. [Phase 3: Allocation Strategies](#phase-3-allocation-strategies)
7. [Phase 4: Walk-Forward Backtesting Engine](#phase-4-walk-forward-backtesting-engine)
8. [Phase 5: Risk Management Layer](#phase-5-risk-management-layer)
9. [Phase 6: Alpaca Brokerage Integration](#phase-6-alpaca-brokerage-integration)
10. [Phase 7: Main Loop and Orchestration](#phase-7-main-loop-and-orchestration)
11. [Phase 8: Dashboard and Monitoring](#phase-8-dashboard-and-monitoring)
12. [Important Notes and Next Steps](#important-notes-and-next-steps)

---

## Introduction — What We Are Building

This guide covers building a **fully automated trading bot** from scratch using Claude Code (a coding AI agent). The bot detects the current market regime, adjusts portfolio allocations automatically, and places real trades through a brokerage.

This is **not** a basic indicator bot that buys when RSI crosses above 30. This is a system that:

1. Connects to a **real brokerage** (Alpaca) for automatic trade execution
2. Manages risk with **circuit breakers** and drawdown limits
3. Adapts to **changing market conditions** automatically using Hidden Markov Models

> **Disclaimer:** This is not financial advice. Trading involves real risk. The content shows concepts for building a systemic trading framework to help become a more disciplined trader. Validate and extensively test your own strategies.

---

## System Architecture — Five Components

The entire system consists of five components:

| Component | Purpose | Description |
|-----------|---------|-------------|
| **Brain** | Decision-making | Hidden Markov Models classify the market into regimes (crash, bear, neutral, bull, euphoria) |
| **Allocation** | Position sizing | Volatility-based strategies determine how much capital to allocate per regime |
| **Safety** | Risk management | Circuit breakers, drawdown limits, and leverage controls — independent of the AI model |
| **Brokerage** | Trade execution | Alpaca API connection for placing real orders |
| **Dashboard** | Visualization | Streamlit UI showing regime detection, portfolio, signals, and risk status |

### How the Regime System Works

The HMM brain classifies the market into five regimes. Depending on the detected regime, allocation strategies and trading behavior change dynamically:

| Regime | Market Condition | Allocation Behavior |
|--------|-----------------|---------------------|
| **Crash** | Extreme downside volatility | Minimal exposure, capital preservation |
| **Bear** | Sustained downtrend | Reduced allocation, defensive positioning |
| **Neutral** | Low volatility, range-bound | Moderate allocation, balanced approach |
| **Bull** | Sustained uptrend | Higher allocation, trend-following |
| **Euphoria** | Extreme upside volatility | Caution — potential reversal zone |

---

## Getting Started — IDE and Claude Code Setup

### Step 1: Download Visual Studio Code

Download and install VS Code. Open a blank folder for the project.

### Step 2: Install Claude Code Extension

1. Search for **"Claude Code"** in the VS Code extensions panel
2. Install **"Claude Code for VS Code"** (first result)
3. The extension appears on the right-hand sidebar for chatting with Claude Code

> **Key Insight:** Claude Code works best when given the full project structure before diving into logic. Build scaffolding first, then implement components one by one.

---

## Phase 1: Project Scaffolding

The first prompt creates the project skeleton — no actual logic, just file structure. This tells Claude Code how the codebase is organized.

### Project Structure

```
regime-trader/
├── settings/
│   ├── config.yaml          # All parameters (broker, HMM, strategies, risk, backtest, monitoring)
│   └── credentials.env       # API keys (gitignored)
├── core/
│   ├── hmm_engine.py         # The brain — HMM regime detection
│   ├── feature_engineering.py # Technical indicators and features
│   ├── regime_strategies.py   # Volume-based allocation per regime
│   └── risk_manager.py        # Position sizing, leverage, drawdown limits
├── broker/
│   ├── alpaca_client.py       # API wrapper for Alpaca connection
│   └── order_executor.py      # Place, modify, cancel trades
├── data/
│   └── market_data.py         # Real-time and historical data feeds
├── backtest/
│   ├── backtester.py          # Walk-forward backtesting engine
│   └── performance.py         # Sharpe ratio, drawdowns, regime breakdowns
├── monitoring/
│   ├── dashboard.py           # Streamlit UI
│   └── alerts.py              # Email/webhook alerts for critical events
├── tests/
│   ├── test_hmm.py
│   ├── test_strategies.py
│   ├── test_risk.py
│   └── test_orders.py
├── main.py                    # Entry point and orchestration
├── requirements.txt           # Dependencies
└── .env                       # Environment variables (API keys)
```

### Settings File Parameters

The settings file contains all configurable parameters:

- **Broker settings** — API endpoint, connection parameters
- **HMM parameters** — Number of regimes to test (3–7), training window, features
- **Strategy parameters** — Allocation percentages per regime, leverage limits
- **Risk parameters** — Circuit breaker thresholds, max drawdown, position limits
- **Backtest parameters** — Window sizes, benchmark comparisons
- **Monitoring parameters** — Alert thresholds, dashboard refresh rate

> **Key Insight:** This is where you customize tickers, strategies, and risk tolerances. The structure is standardized regardless of what you trade.

---

## Phase 2: The Brain — Hidden Markov Models

HMMs do not predict prices — they predict the **type of environment** the market is in. By detecting regimes (calm, volatile, bear, bull), the system adjusts exposure and strategies before applying any trading logic.

### How HMM Regime Detection Works

1. **Feature Engineering** — Technical indicators (price action, volume, volatility metrics) are computed as input features
2. **Model Training** — HMM is trained on approximately 2 years of daily data
3. **Regime Classification** — The model classifies each period into a regime state
4. **Automatic Regime Count Selection** — Testing 3–7 regimes, the model picks the optimal count using statistical criteria (not hardcoded)

### Regime Labeling

Regimes are sorted by mean return and labeled:

| Regime Count | Labels |
|-------------|--------|
| 3 | Bear, Neutral, Bull |
| 4 | Crash, Bear, Bull, Euphoria |
| 5 | Crash, Bear, Neutral, Bull, Euphoria |

### Critical: Avoiding Look-Ahead Bias

The default `predict` function in the HMM library processes the **entire sequence** of data, which creates look-ahead bias. Instead, the system uses the **forward algorithm only** — processing data point by point without seeing future data.

> **Key Insight:** This is a mandatory requirement. Using the default predict function will produce backtest results that cannot be replicated in live trading.

### Regime Stability Filter

A regime must persist for at least **3 consecutive bars** before the system acts on it. If the regime flickers between states more than 4 times in the past 20 bars, it's logged as uncertainty and position sizes are reduced.

### Tests and Validation

After building the HMM engine, mandatory tests verify:
- HMM fits correctly on training data
- Predictions work on out-of-sample data
- No look-ahead bias exists
- Regime stability filter functions correctly

---

## Phase 3: Allocation Strategies

The allocation layer is directly influenced by the brain. Depending on the detected volatility regime, it changes how much of the portfolio is invested and how.

### Strategy Framework

Three main allocation tiers based on volatility:

| Volatility Level | Allocation | Leverage | Behavior |
|-----------------|------------|----------|----------|
| **Low** | 95% of portfolio | 1.25x | Fully invested, maximum exposure |
| **Medium** | Variable | 1.0x | Stay invested if trend is intact |
| **High** | Minimal | None | Reduce exposure, capital preservation |

### Customization

The allocation strategies are the most customizable component. Replace the default strategies with your own based on:
- Your risk tolerance
- The asset class or ticker you're trading
- Your specific trading style (trend-following, mean-reversion, etc.)

### Strategy Orchestrator

The orchestrator combines:
- **Base case** allocation for each regime
- **Confidence adjustments** — uncertain regimes get reduced sizing
- **Rebalancing logic** — when and how to adjust positions
- **Signal data classes** — structured data for trade signals

> **Key Insight:** This is where you'll spend the most time. The framework is general-purpose — customize the strategies to your specific use case.

---

## Phase 4: Walk-Forward Backtesting Engine

The backtester validates that strategies work on blind historical data — not just in-sample optimization.

### How It Differs from Traditional Backtesting

| Approach | Method | Problem |
|----------|--------|---------|
| **Traditional** | Use all data, find perfect settings in hindsight | Results cannot be replicated — overfitted |
| **Walk-Forward** | Split data into rolling windows, test on unseen data | Realistic out-of-sample performance |

### Walk-Forward Parameters

- **In-sample window:** 252 trading days (1 year)
- **Out-of-sample window:** ~6 months for evaluation
- **Rolling forward:** The window slides forward, retraining and testing iteratively

### Realistic Simulation

The backtester includes:
- **Slippage modeling** — realistic execution costs
- **Transaction costs** — commissions and spread

### Performance Metrics

| Metric | Description |
|--------|-------------|
| **Total Return** | Cumulative return over the backtest period |
| **Sharpe Ratio** | Risk-adjusted return |
| **Max Drawdown** | Largest peak-to-trough decline |
| **Win Rate** | Percentage of profitable trades |
| **Total Trades** | Number of executed trades |

Results are broken down by:
- **Regime** — how the strategy performs in each market condition
- **Confidence bucket** — whether high-confidence trades outperform low-confidence trades

### Benchmark Comparisons

The backtester compares the strategy against three benchmarks:

| Benchmark | Description |
|-----------|-------------|
| **Buy and Hold** | Simply holding the asset for the entire period |
| **200-Day SMA Trend Following** | One of the most common systemic strategies |
| **Random Entry** | Random entries and allocation changes with same risk rules |

### Stress Tests

Random crash events are injected (10–15% single-day drops) to verify the system handles extreme conditions.

---

## Phase 5: Risk Management Layer

The risk management layer is the **most important system** in the entire framework — more important than the HMMs. A mediocre strategy with great risk management only loses money a little bit. A great strategy with bad risk management can blow up an account.

### Circuit Breakers

| Condition | Action |
|-----------|--------|
| Down **2%** in a single day | All position sizes cut in half |
| Down **3%** in a single day | Close everything |
| Down **5%** in a week | Half position sizes |
| Down **10%** from peak | Bot stops completely, writes a block file |

The 10% block file is **deliberate** — you must manually investigate what happened, understand why, and consciously delete the file to resume. This forces accountability.

### Position-Level Risk

| Parameter | Default |
|-----------|---------|
| **Max risk per trade** | 1% of portfolio |
| **Leverage limits** | Configurable per strategy |
| **Order validation** | Pre-trade checks |
| **Correlation checks** | Prevents entering correlated positions |

> **Key Insight:** These safety nets work independently of the AI model. Even if strategies are technically correct, these hardcoded limits protect against catastrophic losses.

---

## Phase 6: Alpaca Brokerage Integration

### Setting Up Alpaca

1. Go to **alpaca.markets** and sign up for a free account
2. You'll be placed in a **paper trading account** automatically
3. Navigate to **API Keys** at the bottom of the dashboard
4. Copy three values:
   - **Base URL** (endpoint)
   - **API Key**
   - **Secret Key**

### Security — Never Share API Keys

API keys are stored in the `.env` file, which is gitignored. **Never paste API keys into the Claude Code chat.** Type them manually into the `.env` file:

```
ALPACA_API_KEY=your_key_here
ALPACA_SECRET_KEY=your_secret_here
ALPACA_PAPER=true
```

### Broker Components

| Component | Function |
|-----------|----------|
| **Alpaca Client** | API wrapper — connects to Alpaca, reads account info |
| **Order Executor** | Submit orders, cancel orders, modify stops |
| **Position Tracker** | View open positions and order history |
| **Market Data** | Real-time and historical price feeds |

### Verification Test

After connecting, place a test trade (e.g., buy Nvidia) and verify it appears on the Alpaca dashboard. This confirms the system-to-broker pipeline works.

---

## Phase 7: Main Loop and Orchestration

The main loop ties everything together:

1. **Load configuration** from settings
2. **Connect to Alpaca** and verify account
3. **Check market hours** (open/closed)
4. **Train HMMs** on historical data
5. **Initialize risk manager** and position tracker
6. **Start data feeds**
7. **Main loop** — runs on each bar close (default: 5-minute bars)

### Error Handling

The system handles:
- Alpaca API downtime or errors
- HMM computation errors
- Data feed drops or disconnections

All errors are logged and the engine continues running.

---

## Phase 8: Dashboard and Monitoring

The Streamlit dashboard provides real-time visualization:

| Section | Information Displayed |
|---------|----------------------|
| **Portfolio** | Portfolio value, buying power, open positions |
| **Regime Detection** | Current regime, confidence score, regime history |
| **Risk Status** | Circuit breaker states, leverage usage, drawdown levels |
| **Signal Feed** | Historical trades with allocation, entry prices, stops, P&L |
| **Charts** | Price action with regime overlays, volume, confidence over time |

### Monitoring and Alerts

All trades are logged with structured monitoring for positions, recent signals, and risk events. Email/webhook alerts can be configured for critical events.

---

## Important Notes and Next Steps

### Paper Trading First

Paper trade for **at least one month** before going live. Watch every decision the bot makes — understand why it rebalanced, why it stayed put, when the risk manager overruled something.

### Iterative Improvement

- Review every single rebalance decision
- Run backtests across different tickers and time periods
- Iterate on allocation parameters and strategies
- Use Claude Code to continually improve the system

### Customization Points

| Component | What to Change |
|-----------|---------------|
| **Tickers** | Settings file — change which assets the bot trades |
| **Strategies** | Regime strategies file — replace allocation logic |
| **Risk Limits** | Risk manager — adjust circuit breaker thresholds |
| **Time Frames** | Main loop — change bar size (1min, 5min, 15min, etc.) |

### Documentation

Claude Code automatically generates documentation for every file. The README file contains a complete system overview for reference.

> **Key Insight:** Tools like Claude Code have opened up possibilities for retail traders to build quantitative systems that were previously never accessible. Anyone can now build complex trading systems, test strategies, and become a more disciplined trader.
