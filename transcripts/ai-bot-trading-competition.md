# I Made AI Trading Bots Compete To Make Money — OpenClaw Trading Olympics

> **Source:** YouTube — Experiment running autonomous AI trading bots on OpenClaw (Claude-powered) competing on Hyperliquid in a survival-of-the-fittest tournament.
>
> **Topics Covered:** OpenClaw (Claude Code), AI trading agents, Hyperliquid, natural selection for trading strategies, bot personas and strategies (breakout trader, mean reversion, liquidation hunter, pairs trader, sentiment trader, whale follower, YOLO bot), wallet funding, trading competition format, results analysis

---

## Table of Contents

1. [Introduction — OpenClaw and AI Trading](#introduction--openclaw-and-ai-trading)
2. [System Architecture — How It Works](#system-architecture--how-it-works)
3. [The Strategy Input Problem](#the-strategy-input-problem)
4. [Natural Selection for Trading Bots](#natural-selection-for-trading-bots)
5. [Race 1 — Batch One (Moderate Risk)](#race-1--batch-one-moderate-risk)
6. [Race 2 — Batch Two (Higher Risk)](#race-2--batch-two-higher-risk)
7. [Race 3 — Batch Three (Maximum Risk)](#race-3--batch-three-maximum-risk)
8. [Final Results and Analysis](#final-results-and-analysis)
9. [Next Steps — Evolution of Bots](#next-steps--evolution-of-bots)

---

## Introduction — OpenClaw and AI Trading

OpenClaw is an AI agent that controls your entire computer through the terminal. People have been running it on Mac Minis, server racks, and virtual servers. For the first time, this AI can autonomously execute tasks including trading crypto.

The experiment: fund multiple AI agents with trading capital, give them different strategies, and have them compete in a survival-of-the-fittest tournament. The bottom performers get eliminated and replaced in the next round.

> **Disclaimer:** This content is for entertainment purposes only. Modern financial instruments including crypto are highly volatile and the majority of retail clients will lose money. Do not invest any capital you're not prepared to go to zero.

---

## System Architecture — How It Works

The setup has four components:

| Component | Options | Description |
|-----------|---------|-------------|
| **Compute** | Mac Mini, current laptop, virtual server (~$5/month) | Where OpenClaw runs |
| **AI Agent** | OpenClaw (Claude, Gemini, ChatGPT) | The bot that analyzes and trades |
| **Trading Platform** | Hyperliquid, Asta | Where trades are executed on-chain |
| **Strategy Inputs** | Custom prompts, influencer strategies, leaderboard analysis | What guides the bot's decisions |

### Flow

1. OpenClaw runs on your chosen compute hardware
2. It's powered by an LLM (Claude recommended)
3. It connects to a trading platform (Hyperliquid) with a funded wallet
4. Strategy inputs tell it how to trade

---

## The Strategy Input Problem

The critical question: **what strategy do you give the bot?**

The bot can use any strategy in the world — from extremely profitable to account-destroying. The goal is to find the most profitable strategies possible.

### Approaches Being Explored

| Approach | Description |
|----------|-------------|
| **Research** | Have OpenClaw research the most profitable strategies autonomously |
| **Copy Trading** | Download a profitable trader's brain — their trades, commentary, stop-losses, and reasoning — and feed it to the bot |
| **Leaderboard Mining** | Pull the strategies of Hyperliquid's most profitable traders (those making tens of millions) and replicate their approach |
| **Natural Selection** | Run multiple bots with different strategies, eliminate losers, iterate winners |

---

## Natural Selection for Trading Bots

The concept: put 10 bots head-to-head. The least profitable drop out, the most profitable advance. Over thousands of iterations, the best strategies emerge organically — like biological evolution.

### Tournament Format

| Parameter | Value |
|-----------|-------|
| **Bots per race** | 5 |
| **Starting capital** | $1,000 per bot |
| **Time limit** | 3 hours per race |
| **Risk tiers** | Moderate → Higher → Maximum |
| **Platform** | Hyperliquid |

Each bot was given its own persona and strategy that it believed would be effective.

---

## Race 1 — Batch One (Moderate Risk)

### Bots and Strategies

| Bot | Strategy |
|-----|----------|
| Funding Harvester | Exploit funding rate differentials |
| Rubber Band | Mean reversion from extremes |
| Basis Bot | Cash-and-carry arbitrage |
| Fade Machine | Counter-trend fading |
| Old Guard | Classic trend following |

### Result

**All five bots were unprofitable.** Every bot lost money. The Basis Bot was the worst performer — its $1,000 wallet was devastated.

> **Key Insight:** The first batch of AI-generated strategies was universally terrible. The bots could not generate profitable strategies on their own from scratch.

---

## Race 2 — Batch Two (Higher Risk)

### Bots and Strategies

| Bot | Strategy |
|-----|----------|
| Breakout Trader | Momentum breakout plays |
| Old Diverge | Divergence-based entries |
| Liquidation Hunt | Trade liquidation clusters |
| Pairs Trader | Correlated asset spread trading |
| Sentiment Trader | News/sentiment-driven entries |

### Highlights

- **Old Diverge** had extreme volatility — fell off a cliff, recovered dramatically, then crashed again
- **Breakout Trader** made a massive trade on WIF (meme coin) — went from $2 in negative to **$78 profit** by going long at $0.233 and exiting at $0.252 with leverage
- **Pairs Trader** also finished in profit

### Result

**First profitable batch.** Two bots (Breakout Trader and Pairs Trader) were profitable, making this the first successful round.

---

## Race 3 — Batch Three (Maximum Risk)

### Bots and Strategies

| Bot | Strategy |
|-----|----------|
| Scalper | High-frequency scalping |
| YOLO Bot | Maximum leverage, all-in trades |
| Contrarian | Counter-trend positions |
| Whale Follower | Copy-trade profitable wallets |
| Meme Coin Sniper | Target meme coin breakouts |

### Highlights

- **YOLO Bot** made a **40x leverage long on Bitcoin** with $1,030 — the biggest position of the entire competition
- The trade soared to **$175.20 profit**
- **Whale Follower** barely traded — spent most of the time looking for profitable wallets to follow
- **Scalper** burned through capital in fees

### Result

**Profitable batch** thanks to the YOLO Bot's massive Bitcoin trade offsetting losses from other bots.

---

## Final Results and Analysis

### Aggregate Performance

| Metric | Value |
|--------|-------|
| **Total bots** | 15 (across 3 races) |
| **Total starting capital** | $15,000 |
| **Final value** | $15,074 |
| **Net profit** | **$74** |
| **Profitable bots** | 4 out of 15 |
| **Losing bots** | 11 out of 15 |

### Winner

**Bot 15 — YOLO Bot** with $175 profit from the 40x Bitcoin long in Race 3.

### Key Observations

- Most bots were negative — only 4 out of 15 made money
- The overall portfolio was saved by a few heavy hitters (pareto distribution)
- Higher risk tiers produced the biggest individual winners
- AI bots struggle to create profitable strategies from scratch without guidance

### Advancing to Next Round

The four profitable bots advance: **YOLO Bot, Breakout Trader, Meme Coin Sniper, and Pairs Trader**. They'll compete against each other and new challengers in subsequent rounds.

---

## Next Steps — Evolution of Bots

The plan is to run hundreds of these competitions, creating an evolution of trading bots over time:

1. **Feed profitable strategies** — Take winning bot configurations and use them as starting points for new bots
2. **Iterate with better inputs** — Provide more sophisticated strategy prompts based on what worked
3. **Scale up** — More bots, more races, more iterations
4. **Combine winners** — Merge the best elements of multiple winning strategies

> **Key Insight:** This is the beginning of using natural selection to discover profitable trading strategies. The potential is infinitely powerful — the best strategies emerge organically through competition and elimination.
