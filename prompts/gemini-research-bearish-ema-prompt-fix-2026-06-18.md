# Gemini Deep Research Prompt: v0.14.9 Rate Limit + Bearish-EMA Fix

**Created:** 2026-06-18 13:26 EST
**Author:** Vera (sponsored by Spencer)
**Purpose:** v0.14.9 will ship FID-204 (10x NVIDIA keys), FID-205 (per-model cooldown), FID-206 (bearish-EMA prompt fix), FID-207 (LLM timeout log), FID-208 (decision log cap). Research validates the approach BEFORE we code, per the research-first protocol. Previous Gemini research (`gemini-research-2026-06-17.md`, `llm-default-to-pass-2026-06-17.md`) covered strategy calibration and default-to-PASS diagnosis; this follow-up focuses on the specific implementation choices for v0.14.9.

---

## Instructions for Spencer

1. Copy the entire prompt below (everything between the `---` lines marked "PROMPT START" and "PROMPT END")
2. Paste into Gemini Deep Research
3. Save the full response to `C:\Users\spenc\dev\savant-trading\prompts\prompt-results\gemini-bearish-ema-prompt-fix-2026-06-18.md`
4. Tell me the path. I'll read it before finalizing FID-206's prompt wording.

While you run the research, I'll prep FID-204 and FID-205 implementation scaffolds (the config + key loading is mechanical, doesn't need research). We'll meet back when the research lands.

---

## PROMPT START

# Deep Research: NVIDIA NIM Multi-Key Rate-Limit Isolation, LLM Retry Herd Behavior, and Bearish-EMA Veto in Trading Prompts

## Context

I am building an autonomous crypto trading engine (Savant Trading) that runs 24/7 on decentralized exchanges via the 0x API on Arbitrum. The engine makes 5-minute cycle decisions on ~15 trading pairs, using a multi-model "jury" architecture: 1 M3 control model (1T MoE) does the primary batched decision call, then 10 free-model jurors (Llama 3.3 70B, DeepSeek V4 Pro 1T, Nemotron Super 120B, Qwen 3.5 397B, Mistral Large 3 675B, GLM 5.1, Kimi K2.6 1T, etc.) provide shadow verdicts for future override comparison.

All inference runs through NVIDIA NIM free tier (`integrate.api.nvidia.com/v1/chat/completions`). The engine does NOT use a paid LLM endpoint — every API call must succeed on free tier or the cycle stalls.

**Overnight run v0.14.8 (2026-06-18, 8h58m, 169 cycles):**
- 2 successful batched LLM calls (3:55 AM and 4:05 AM EST, 138s + 167s latency)
- 167 cycles got rate-limited (HTTP 429) and never produced a verdict
- 22 PASS decisions captured, 0 BUY, 0 SELL, 0 executed trades
- 682 WARN log lines, all "Stream rate limited" / "Fallback rate limited" retries
- Engine process survived the entire run with zero crashes (rate-limit handling kept it alive but unproductive)

I now have 11 NVIDIA API keys (1 legacy, 10 new — same email can create multiple free-tier keys per account, each with independent rate-limit buckets). Empirical burst test confirmed: 5 successful M3 calls per key before HTTP 429.

## The Three Concrete Problems

### Problem 1 (FID-204): NVIDIA NIM Rate Limit — 10x Key Solution

**Empirical observation (live test, 2026-06-18 12:25 EST):**

```
10 sequential M3 calls with single NVIDIA_API_KEY:
Latencies: 53.57, 0.58, 0.54, 1.13, 1.01, 0.12, 60.01, 6.56, 0.55, 60.02s
Statuses:  200,    200, 200, 200, 200, 429,  TIMEOUT, 200,  200,  TIMEOUT

→ 5 successful calls, then 429. ~5 RPM per model per key. 60s recovery window.
```

11 separate NVIDIA API keys all return 200 on a small llama-3.1-8b ping. 3 of 10 new keys confirmed can hit M3 model directly (cold start 20-35s, warm 1-2s). 11 keys stored in `.env` as `NVIDIA_API_KEY` (legacy) and `NVIDIA_API_KEY_1` through `NVIDIA_API_KEY_10` (new).

**Proposed implementation:**

- `src/core/config.rs:194-222` — Extend `NvidiaConfig` with `api_key_envs: Vec<String>` (default empty, backward-compat with single `api_key_env`)
- `src/agent/jury/pool.rs:38-66` — `JuryPool` gains `nvidia_api_keys: Vec<String>`; in `evaluate()`, juror N (N >= 1) uses `keys[(N-1) % keys.len()]`
- `src/agent/provider.rs` — `LlmProvider::chat_with_key()` accepts optional per-call API key override
- Round-robin rotation when juror count > key count

### Problem 2 (FID-205): Per-Model Cooldown on 429

**Current behavior:** When a juror hits 429, it retries immediately with the same model. If 5 jurors hit 429 on `deepseek-v4-flash` simultaneously, they all retry simultaneously and pile more requests on the same model. Exponential backoff helps but doesn't prevent herd behavior.

**Proposed implementation:**

- `JuryPool` tracks `model_cooldowns: HashMap<String, Instant>` — when model X returns 429, add to cooldown for 60s
- In `evaluate()`, skip models with active cooldown; if all models for a juror are in cooldown, log warning and continue without that juror (fail-open, fail-soft)
- After cooldown expires, model is re-eligible
- Random jitter in cooldown expiration (±10s) to prevent sync at cooldown boundaries

### Problem 3 (FID-206): Bearish-EMA Prompt Fix

**The actual finding (NOT what Gemini predicted earlier on 2026-06-17):**

Gemini predicted "LLM defaults to PASS" via negative constraint priming. The data shows a different mechanism: M3 outputs non-zero conviction scores (up to 0.52!) but applies a CUSTOM bearish-EMA veto not in the prompt:

```
BTC/USD | Trending ADX 20.7 | EMA_F < EMA_S | Conviction: 0.22 | Action: PASS
Reasoning: "Trending ADX 20.7 borderline, EMA_F < EMA_S bearish... No momentum
trigger confirmed - EMA cross is against. Hold."

UNI/USD | Ranging ADX 14.1 | Z-score -1.99 oversold bounce | Conviction: 0.18 | Action: PASS
Reasoning: "EMA_F < EMA_S bearish... Z-score -1.99 suggests oversold bounce
potential... Below Ranging 0.25 threshold - HOLD."
```

**Distribution of "PASS despite non-zero conviction" (32 cases at conviction ≥ 0.20):**
- 30/32 = 94% PASS
- 1/32 = BUY (ENA/USD at conviction 0.52, from old binary)
- Reasoning consistently cites "EMA cross is against" or "below threshold"

**Current prompt structure (`src/agent/prompts/output_format.md` line 73):**

> "0. **PASS is NOT a default (FID-192 / FID-198).** PASS means 'I have zero directional view on this pair.' Most pairs have SOME directional lean. Output Buy or Sell with conviction_score (0.05-1.0) and let the engine's regime gate filter it. Below the threshold, the gate downgrades to HOLD. If conviction is between the probe threshold and main threshold, output `is_probe: true`."

This is a NEGATIVE constraint (telling the model what NOT to do) plus an affirmative instruction. The model is reading "Output Buy or Sell" and ignoring the threshold-downgrade mechanism — it's adding a stricter rule on its own ("don't fight the trend").

## What I Need Researched

### Question 1 (FID-204): Per-Key Rate-Limit Isolation

1. **Is per-key rate-limit isolation a standard pattern?** Confirm via research that NVIDIA NIM (or similar inference gateways: OpenAI, Anthropic, Together AI, Fireworks, OpenRouter) enforce RPM caps per API key, not per account. If per-account, our fix is useless and we need a different approach (self-hosted NIM, paid tier, request queuing at the gateway).

2. **What's the optimal rotation strategy for a 10-juror + 1-batch model pool?** Round-robin (`juror N → keys[N-1]`) is simple but means if key 3 hits 429, juror 3 is stuck waiting even if keys 1-2 are healthy. Compare alternatives:
   - Random pick from healthy keys
   - Least-recently-used
   - Per-model fixed mapping (juror 3 always uses key 3)
   - Adaptive: track per-key cooldown, skip recently-429'd keys

3. **Are there edge cases we should handle?** E.g., M3 batch call taking 138-167s blocks the cycle. Should we have a separate "long-running" key pool for M3 and a "short-running" pool for shadow jurors? Different timeout configs per key?

4. **Alternatives we might be missing:** Self-hosted NIM containers on DGX Cloud (would bypass free-tier caps entirely). NVIDIA NGC "organization-level" rate limits vs per-key. Token-bucket vs leaky-bucket vs fixed-window rate-limit semantics.

### Question 2 (FID-205): Herd-Retry Mitigation

5. **Is herd-retry a known problem in LLM orchestration?** Confirm via research that staggering retries across multiple keys/models is standard practice. Cite examples from LangChain, LlamaIndex, LiteLLM, Semantic Kernel, portkey.ai, or production case studies.

6. **Optimal cooldown duration?** 60s is a guess based on the NVIDIA NIM burst test. Should it scale with the 429 response (if NVIDIA ever sent Retry-After header, honor it)? Should we extend cooldown after repeated 429s (exponential backoff on cooldown)?

7. **Jitter strategy:** All jurors in cooldown reset at the same moment → thundering herd. Recommend random jitter in cooldown expiration. What distribution (uniform ±10s, exponential, Gaussian)? Cite research on jitter for distributed systems.

8. **Telemetry needs:** Should we expose `model_cooldowns_active_count` metric so the dashboard can show "3 jurors currently in cooldown" instead of just "rate limited" warnings? What's the standard observability pattern for LLM gateway rate limits?

### Question 3 (FID-206): Bearish-EMA Prompt Fix

9. **Why does the model add a bearish-EMA veto?** This is the core question. Theories to investigate:
   - Training data bias: most financial training data is cautious about counter-trend entries (academic: "Trend-Following, Risk Parity, and Momentum Strategies" literature)
   - Autoregressive context: after generating "EMA_F < EMA_S", the model is "in" bearish mode and PASS feels safe (autoregressive token probability)
   - Token probability: PASS is a high-frequency token in fine-tuning data (modeled responses tend to be cautious — see Anthropic's "Constitutional AI" research)
   - RLHF / safety training: model is trained to avoid aggressive trading recommendations
   - Specific named pattern: the "don't fight the Fed" / "the trend is your friend" folk wisdom embedded in training data

10. **Does affirmative phrasing actually work better than negative constraints for trading prompts?** The semantic gravity well research from the 2026-06-17 Gemini paper said yes, but for this specific scenario (bearish trend, conviction > 0), is it enough? Are there published examples of "Buy the dip" / "Sell the rip" prompts that successfully override model caution bias?

11. **Few-shot examples — how many needed?** Single example or 3-5 examples covering different regime/trigger combos? Does including a CONTRARIAN example (bearish trend → Sell) help or hurt? What about LONG/SHORT balance in the examples — does showing 3 Buys + 3 Sells in examples reduce directional bias vs showing 5 Buys + 1 Sell?

12. **Should we change the action vocabulary?** Currently: BUY / SELL / PASS / HOLD / ADJUST_STOP / CLOSE. Alternatives:
    - LONG / SHORT / NO_TRADE / HOLD_POSITION (clearer directional vocabulary)
    - ENTER_LONG / ENTER_SHORT / EXIT / STAY (more imperative, less interpretive)
    - Keep current but rename PASS to NO_SIGNAL to make the "zero direction" meaning explicit
    
    Research: how do other production LLM trading systems name their action tokens? (Examples: Hummingbot, FreqAI, Jesse, Backtrader LLM extensions, FinRL)

13. **Engine-level fix alternatives:** Currently the engine downgrades to PASS when conviction < threshold. What if:
    - Engine treats "PASS at conviction 0.0" as no-decision (skips cycle)
    - Engine treats "PASS at conviction > 0 but below threshold" as a soft signal (logs but doesn't execute)
    - Engine requires affirmative action token (Buy/Sell/Adjust/Close), absence = engine default HOLD
    - Engine validates "PASS" reasoning for contradictory signals (e.g., conviction > 0.10 but action=PASS = engine override to HOLD or NO_SIGNAL)

14. **Other LLM training bias mitigations worth knowing?**
    - Constitutional AI techniques (Anthropic)
    - Self-consistency sampling (multiple inference passes + majority vote)
    - Chain-of-thought BEFORE action token (force reasoning to commit to direction first)
    - Temperature/top-p tuning for trading prompts (lower temperature = more deterministic = less PASS-default)
    - Logit bias: explicitly suppress PASS token probability when conviction > threshold
    - System prompt vs user prompt separation (which one carries the action vocabulary)

## Constraints

- All 10 NVIDIA NIM keys are real (Spencer provisioned them 2026-06-18). Per-key rate limits are empirically confirmed (~5 RPM per M3 per key).
- We have 22 verified PASS decisions from the v0.14.8 overnight run to validate any prompt fix against.
- The fix must NOT regress existing working paths: OpenRouter fallback, 10-juror shadow mode, conviction scoring, regime classification.
- Engine is currently stopped (Spencer stopped it manually after overnight run).
- v0.14.9 scope is FIXED at FID-204 + FID-205 + FID-206 + FID-207 (LLM timeout log) + FID-208 (decision log cap raise). Don't suggest FIDs outside this scope.

## Deliverable

For each problem, please provide:

1. **Confirmation of the diagnosis** (with citations to research papers, blog posts, GitHub issues, or production case studies where possible — URLs are valuable)
2. **Validation of the proposed implementation** OR a better alternative with rationale
3. **Concrete wording recommendations** for FID-206 prompt changes (this is the highest-priority research deliverable — show me the actual prompt text to use, not just principles)
4. **Failure modes to watch for** during the v0.14.9 release (what could go wrong, how to detect it)

## Research Output Path

Save your full response to: `C:\Users\spenc\dev\savant-trading\prompts\prompt-results\gemini-bearish-ema-prompt-fix-2026-06-18.md`

(You don't need to do this — Spencer will save the response. But include a closing summary I can use as the FID-206 implementation checklist.)

## PROMPT END
