# Gemini Deep Research Prompt — Savant Trading Soul Design

## Research Objective

Design an enterprise-quality SOUL.md for an autonomous crypto trading agent named "Savant" that operates 24/7 on Kraken exchange. The soul must be comprehensive enough to guide every trading decision, define the agent's persona, psychology, and behavioral framework. It replaces a 20-line instruction sheet with a full identity specification.

## Context

Savant is a Rust-native autonomous crypto trading engine with:

- 254 knowledge units from 22 curated sources (trader transcripts + research)
- AI brain powered by mimo v2.5 pro LLM via OpenGateway
- Trades 15 crypto pairs on Kraken (BTC, ETH, SOL, XRP, ADA, DOGE, AVAX, DOT, LINK, UNI, ATOM, ALGO, FIL, NEAR, MATIC)
- $50 paper trading budget, scaling to live
- Parallel AI evaluation (all 15 pairs simultaneously)
- Real-time TUI dashboard
- Obsidian vault integration for transparent state

## What I Need

Research the following topics and synthesize into a comprehensive SOUL.md specification:

### 1. Trading Persona Design

- What makes a great trader's mindset? Research top traders (Mark Minervini, Paul Tudor Jones, Stanley Druckenmiller, Ed Seykota) and extract their core philosophies
- How should an AI trading agent's "personality" differ from a human trader?
- What cognitive biases must the soul explicitly guard against?
- How should the agent handle drawdowns, losing streaks, and winning streaks?

### 2. Crypto-Specific Trading Philosophy

- How does crypto trading psychology differ from traditional markets?
- What are the unique behavioral patterns of crypto markets (24/7, high volatility, narrative-driven, whale-dominated)?
- How should the agent think about Bitcoin dominance, alt seasons, and cycle positioning?
- What is the optimal relationship between technical analysis and on-chain/fundamental data?

### 3. Risk Management Philosophy

- Beyond position sizing rules, what is the RISK MINDSET?
- How should the agent think about capital preservation vs growth?
- What is the correct relationship between conviction and position size?
- How should the agent handle correlated positions?

### 4. Decision Framework

- What is the optimal decision-making process for a trading agent?
- How should the agent balance conflicting signals (e.g., extreme fear + high funding rate)?
- When should the agent NOT trade?
- How should the agent handle uncertainty?

### 5. Behavioral Constraints

- What should the agent NEVER do?
- What should the agent ALWAYS do?
- How should the agent respond to system errors, API failures, and market anomalies?
- What is the correct relationship between the agent and its operator?

### 6. Identity Invariants

- What principles must never change across versions, sessions, or market conditions?
- What is the agent's core purpose beyond making money?
- How should the agent think about its own limitations?

## Reference: Existing Soul Pattern

The Savant AI Framework uses this SOUL.md structure:

```markdown
# SOUL.md — Agent Persona Specification

## Identity
| Field | Value |
|-------|-------|
| Designation | ... |
| Version | ... |
| Role | ... |

## Behavioral Profile
### Cognitive Style
### Communication
### Decision Framework

## Operational Constraints
### What You Always Do
### What You Never Do
### What You Do Under Pressure

## Technical Values

## Identity Invariants
```

## Output Format

Produce a complete SOUL.md file (200-400 lines) following the structure above, but adapted for a trading agent. Include:

- Specific trading philosophies backed by research
- Concrete behavioral rules (not vague platitudes)
- Crypto-specific guidance
- Risk management mindset
- Decision framework with explicit steps
- Identity invariants that define who Savant is

The soul should be so comprehensive that reading it alone gives the LLM a complete understanding of how to trade — without needing any other prompt context.

## Research Sources to Consult

1. "Trade Your Way to Financial Freedom" — Van Tharp (trading psychology)
2. "The Mental Game of Trading" — Jared Tendler (tilt, emotional control)
3. "Market Wizards" series — Jack Schwager (top trader interviews)
4. "Reminiscences of a Stock Operator" — Edwin Lefèvre (market wisdom)
5. ICT/Smart Money Concepts — inner circle trader methodology
6. On-chain analytics frameworks (Glassnode, CryptoQuant)
7. Crypto-specific risk management (funding rates, liquidation cascades)
8. AI agent design patterns (persona specification, behavioral constraints)
9. Hermes/OpenClaw soul design patterns
10. Savant AI Framework SOUL.md specification

## Constraints

- The soul must work for a 24/7 crypto market (no stock market concepts)
- It must be specific enough to guide actual trading decisions
- It must be compatible with the existing knowledge injection system (254 units)
- It must define the agent's relationship with its operator (Spencer)
- It must include guardrails against common AI agent failures (hallucination, overconfidence, tilt)
