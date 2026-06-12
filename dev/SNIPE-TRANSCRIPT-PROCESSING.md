# Snipe Transcript Processing Workflow

This document describes how to process new sniping transcripts/books into knowledge units for the Savant Trading knowledge base.

## Overview

**Goal:** Take raw transcripts from YouTube videos, podcasts, or book excerpts about crypto sniping, and convert them into structured knowledge units that integrate with the existing `knowledge/knowledge_crypto_native.json` file.

**Source files to look for:**
- YouTube videos from crypto snipers (CryptoGains, Altcoin Daily, crypto Twitter alpha threads, DEXScreener educators)
- Books about crypto day trading, scalping, or altcoin investing
- Discord/Telegram transcripts from sniping communities
- Twitter/X threads about specific altcoin trades

**Where to save the raw content:**
- Place YouTube transcripts in `knowledge/youtube-interviews/`
- Place book excerpts in `knowledge/books/`
- Place other transcripts in `knowledge/extracted/`

## How the Existing Pipeline Works

The existing knowledge generation pipeline has these scripts (see `knowledge/` directory):
- `extract_books.py` — extracts content from book files
- `generate_forex_knowledge.py` — generates knowledge units from forex content (we won't use this)

The YouTube interview files in `knowledge/youtube-interviews/` are already in markdown format and manually curated. The existing scalper interviews include:
- `Trading LIVE with the BEST Scalper in the World (PERFECT Accuracy).md`
- `Trading LIVE with TWO World Class Order Flow Scalpers.md`
- `Day Trading For Beginners MY COMPLETE BEGINNERS COURSE 2026.md`

## Processing Steps

### Step 1: Obtain the Raw Transcript

**For YouTube videos:**
1. Get the transcript (use youtube-transcript-api, yt-dlp, or manual copy-paste)
2. Save as markdown in `knowledge/youtube-interviews/[Video Title].md`
3. Add a header with source info:
   ```markdown
   Source: YouTube - [Channel Name]
   Video: [Title]
   URL: [URL]
   Date: [Upload Date]
   Snipe-Relevance: HIGH/MEDIUM/LOW
   ```

**For books:**
1. Extract the relevant chapters/sections
2. Save as text or markdown in `knowledge/books/[Book Title].md`
3. Add a header with source info

### Step 2: Extract Snipe-Relevant Insights

Read through the transcript and identify:
- **Entry signals** — what triggers a buy/sell
- **Exit signals** — when to take profit or cut loss
- **Regime detection** — how to identify market state
- **Position sizing** — how much to risk per trade
- **Timing** — best hours/days for sniping
- **Token selection** — what makes a good snipe target

Mark each insight with `[SNIPE]` tag. Ignore content that doesn't apply to crypto sniping (e.g., stock trading, forex, long-term investing).

### Step 3: Generate Knowledge Units

For each `[SNIPE]` insight, create a JSON knowledge unit with this exact structure:

```json
{
  "id": "snipe-XXX",
  "source": "snipe-transcript-[YYYY-MM-DD]",
  "topic": "CryptoNative",
  "conditions": ["Trending", "Ranging", "HighVolatility", "LowVolatility", "ExtremeFear", "ExtremeGreed", "VolumeExpansion"],
  "content": "1-3 sentences with specific thresholds, triggers, and actions",
  "priority": 3,
  "tags": ["scalp", "altcoin", "5m-candle"]
}
```

**Field requirements:**
- `id`: `snipe-XXX` where XXX is the next sequential number
- `source`: `snipe-transcript-YYYY-MM-DD` (use today's date)
- `topic`: always `"CryptoNative"`
- `conditions`: which market conditions this insight applies to
- `content`: 1-3 sentences, actionable, specific
- `priority`: 3-5 (5 = most important)
- `tags`: MUST include `"scalp"`. Add specific tags like `"volume-spike"`, `"5m-candle"`, `"altcoin"`, `"liquidity-grab"`, etc.

**Quality bar:**
- Good: "Enter when 1m volume exceeds 3x 20-period average within 5m candle. Stop = wick of spike candle. Target = 1.5-2x spike candle range. Skip if BTC dominance rising > 0.5% in last hour."
- Bad: "Markets are complex. Be patient. Use proper risk management."

### Step 4: Merge into Knowledge Base

Add the new units to `knowledge/knowledge_crypto_native.json`:

1. Open the file
2. Find the closing `]`
3. Remove the `]` and trailing newline
4. Add `,` then each new unit on its own line
5. Add `]` at the end

OR use this Python script:

```python
import json

# Load existing knowledge
with open('knowledge/knowledge_crypto_native.json') as f:
    existing = json.load(f)

# Load new units
new_units = [
    # ... your new knowledge units here
]

# Merge
existing.extend(new_units)

# Save
with open('knowledge/knowledge_crypto_native.json', 'w') as f:
    json.dump(existing, f, indent=2)

print(f"Added {len(new_units)} new units. Total: {len(existing)}")
```

### Step 5: Verify and Test

1. Run `cargo check` to ensure the project still compiles
2. Run the sandbox with `--save-responses` to measure divergence count change
3. Compare before/after Buy/Sell/Close action counts

## What Makes a Good Snipe Transcript

**Good sources:**
- Active crypto day traders who post real P&L
- Educators who focus on 5m-15m timeframes
- Traders who use DEX (not just CEX)
- People who trade altcoins (not just BTC/ETH)
- Content that includes specific entry/exit examples

**Avoid:**
- Stock trading content (even if "crypto adjacent")
- Long-term HODL advice
- Pure technical analysis theory without examples
- Forex content (different session dynamics)
- Generic "risk management" lectures

## Existing Snipe-Relevant Content in the Knowledge Base

These files already contain snipe-relevant knowledge (from the existing YouTube interviews):
- `yt_fabio_scalper.json` — Fabio/Valentina scalping approach
- `yt_juvier_daytrading.json` — Juvier day trading framework
- `yt_two_scalpers.json` — Two order flow scalpers
- `yt_claude_code_bot.json` — Claude Code trading bot
- `yt_ai_bots.json` — AI trading bots
- `yt_ai_crypto_money.json` — AI crypto trading

These have 189+ units tagged with "scalp" which activate the 5.0x MMR boost.

## Quality Over Quantity

Better to have 10 excellent knowledge units than 50 mediocre ones. Each unit should:
- Teach the model something it doesn't already know
- Be specific and actionable
- Include numerical thresholds
- Mention when NOT to use the signal
- Reference crypto-native concepts (altcoin, DEX, gas, etc.)

## File Locations

- Raw transcripts: `knowledge/youtube-interviews/`, `knowledge/books/`, `knowledge/extracted/`
- Generated knowledge units: `knowledge/knowledge_crypto_native.json`
- Existing scripts: `knowledge/extract_books.py`, `knowledge/generate_forex_knowledge.py`
- Audit report: `dev/audits/FID-2026-0612-knowledge-base-institutional-audit.md`

## Next Steps After Processing

1. Run sandbox with `--save-responses` flag
2. Compare divergence count to baseline (34/60 before MMR fix)
3. If divergence improved, keep the new units
4. If divergence worsened, remove or rewrite the problematic units
5. Update the audit report with results
