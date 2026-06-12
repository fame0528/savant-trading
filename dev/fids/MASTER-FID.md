# MASTER-FID — FID Tracker

**Last updated:** 2026-06-12
**Active work streams:** 5 merged + 3 individual = 8
**Recently Completed:** 10 (6 in v0.13.9, 4 in prior versions)
**Archived FIDs:** 141

---

## Active Work Streams (8)

### Merged FIDs (5 work streams, consolidating 11 fragmented FIDs)

The 11 open FIDs (FID-126 through FID-136) overlap thematically and have dependency chains. They are grouped into 5 merged work streams for cleaner planning. The individual FID files remain in `dev/fids/` as reference material.

| ID | Title | Status | Severity | Component FIDs |
|---|---|---|---|---|
| **MS-1** | **Multi-Provider LLM Infrastructure** | ✅ **shipped (v0.13.9)** | medium | FID-122, FID-123 |
| **MS-2** | **Conviction-Weighted Decision System** | 🟡 partial (1/4 audit targets met) | critical | FID-126, FID-127, FID-132 |
| **MS-3** | **Sandbox Data Realism** | ❌ not started | high | FID-128, FID-134 |
| **MS-4** | **Prompt & Knowledge Hygiene** | 🟡 partial (KU scrub done, deep-asian removal pending) | high | FID-129, FID-131 |
| **MS-5** | **Sandbox Evaluation Suite** | ❌ not started | high | FID-130, FID-133, FID-135 |

### Individual FIDs (3, kept separate)

| ID | Title | Status | Severity |
|---|---|---|---|
| FID-106 | Agent Conversation & Query System | open, spec only | high |
| FID-110 | Engine Decomposition (Sessions 5-7 deferred) | partially-complete (Sessions 1-4 of 7) | critical |
| FID-136 | Release Coordination & Dependency Tracking | open, meta-FID | medium |

---

## Recently Completed (10, in v0.13.9 or prior)

| ID | Title | Version | Severity |
|---|---|---|---|
| FID-121 | 0x Liquidity Validation Gate | v0.13.8 (CHANGELOG fix in v0.13.9) | medium |
| FID-122 | TokenRouter Provider Integration | v0.13.9 | medium |
| FID-123 | Sandbox Multi-Provider Support | v0.13.9 | high |
| FID-124 | Sandbox Raw Response Capture | v0.13.9 | high |
| FID-125 | Dynamic Pair List (soul.md Stale Pair Fix) | v0.13.9 | critical |
| FID-137 | Close-Rounding Bug (f64→wei overflow) | v0.13.9 | critical |
| FID-113 | PnL Tracking (Fee Estimate) | v0.13.7 | medium |
| FID-118 | Pair Health Rotation | v0.13.5 | high |
| FID-119 | VolRatio=0 Misdiagnosis | v0.13.6 | critical |
| FID-120 | Dynamic Token Database | v0.13.8 | high |

---

## Implementation Status (from working tree vs v0.13.8)

| Component | Code in tree | Self-reported status | Independent verification |
|---|---|---|---|
| FID-121 (0x liquidity gate) | ✅ | implemented | commit `77a32fa2` (v0.13.8) |
| FID-122 (TokenRouter) | ✅ | verified | clippy+test clean |
| FID-123 (sandbox multi-provider) | ✅ | fixed | clippy+test clean |
| FID-124 (sandbox raw capture) | ✅ | fixed | clippy+test clean |
| FID-125 (dynamic pair list) | ✅ | done | clippy+test clean |
| FID-137 (close rounding) | ✅ | fixed | 299/299 tests pass + on-chain verified |
| FID-126 (conviction thresholds) | 🟡 prompts modified | partial pass (1/4 hard targets per `dev/audits/fid-126-verification-2026-06-12.md`) | audit by earlier Buffy (Opus-class) |
| FID-127 (conviction sizing) | 🟡 parser + position.rs | implemented but bypassed by `SAVANT_GATE_DISABLED=1` | session 03:00 |
| FID-128 (jump-diffusion) | ❌ not started | spec only | n/a |
| FID-129 (remove deep-asian) | ❌ not started | spec only | n/a |
| FID-130 (counterfactual grader) | ❌ not started | spec only | n/a |
| FID-131 (KU scrub) | 🟡 knowledge.rs partial | institutional penalty expanded + crypto-native 2.0x boost | per audit recs 1+2 |
| FID-132 (checklist matrix) | ❌ not started | spec only | n/a |
| FID-133 (A/B harness) | ❌ not started | spec only | n/a |
| FID-134 (adversarial scenarios) | ❌ not started | spec only | n/a |
| FID-135 (calibration loop) | ❌ not started | spec only | n/a |
| FID-136 (release coordination) | ❌ not started | spec only | n/a |

---

## Sandbox Volume Spike Fix (FID-126 partial, undocumented)

`src/sandbox/generator.rs` was modified beyond the FID-126 spec: 5x trend drift + 2.5x volume spike injection in last 10 candles of trade scenarios. This addresses the "Filter 4: Low Volume Penalty" identified in `research/prompt-sandbox-reasoning-action-divergence.md` but is NOT full jump-diffusion (FID-128).

---

## 0-Trades-Overnight Diagnosis (CLOSED)

The engine ran 60+ cycles overnight (2026-06-12 11:25–11:48) with **$0.00 USDC balance**. Root cause: the ARB position close failed due to f64→wei rounding overflow (48 wei above on-chain balance). 0x API returned dust (0 output), gasless fallback returned `INSUFFICIENT_BALANCE`. Position remained stranded.

**Fix:** FID-137 (`sell_entire_balance: bool` + 0.01% wei haircut) is in the v0.13.9 release. After v0.13.9 deploy, the engine will retry the close and succeed (`sellEntireBalance=true` ignores the sellAmount, uses actual on-chain balance).

**Reference:** `C:\Users\spenc\Desktop\savanttrading.txt` (cycle 62, lines 706-718) shows the exact failure: `amount=262540979419345780736` vs `balance=262540979419345732548` (48 wei overflow).

---

## Implementation Order (per Gemini Deep Research, FID-126-134 plan)

### Phase 1 (prompt + content only, no Rust changes) — Fast wins
- MS-2: Conviction-Weighted Decision System (FID-126 partial in v0.13.9 working tree)
- MS-4: Prompt & Knowledge Hygiene (FID-131 partial done)

### Phase 2 (Rust engine + new modules) — needs MS-1 done
- MS-2: Conviction-Weighted Sizing (FID-127 needs FID-126 stable)
- MS-5: Sandbox Evaluation Suite (depends on MS-1, MS-2)

### Phase 3 (data layer + sandbox extension) — needs MS-2/MS-4 stable
- MS-3: Sandbox Data Realism (FID-128 jump-diffusion)
- MS-4: Complete KU scrub (FID-131 review pass)
- MS-3: Adversarial Scenarios (FID-134 depends on MS-2)

**Release strategy:** Phase 1 → v0.14.0, Phase 2 → v0.15.0, Phase 3 → v0.16.0

---

## Reference Material in dev/fids/

The 11 individual FID files for the merged work streams remain in `dev/fids/`:
- MS-1: `FID-2026-0611-122-tokenrouter-provider.md` (archived), `FID-2026-0611-123-sandbox-multi-provider.md` (archived)
- MS-2: `FID-2026-0612-126-conviction-weighted-thresholds.md`, `FID-2026-0612-127-conviction-weighted-sizing.md`, `FID-2026-0612-132-checklist-evaluation-matrix.md`
- MS-3: `FID-2026-0612-128-sandbox-jump-diffusion-data.md`, `FID-2026-0612-134-adversarial-scenarios.md`
- MS-4: `FID-2026-0612-129-remove-deep-asian-penalty.md`, `FID-2026-0612-131-ku-absolute-language-scrub.md`
- MS-5: `FID-2026-0612-130-counterfactual-grader.md`, `FID-2026-0612-133-ab-test-harness.md`, `FID-2026-0612-135-checklist-modifier-calibration.md`

Plus 3 individual FIDs: `FID-2026-0609-106-...`, `FID-2026-0610-110-...`, `FID-2026-0612-136-...`

---

## Archive Index

141 FIDs in `dev/fids/archive/`. The most recent archivals (v0.13.9 batch):
- FID-121, FID-122, FID-123, FID-124, FID-125, FID-137
- Plus FID-120 cleanup (shipped in v0.13.8, archived now)

---

## Source Material

- `dev/audits/fid-126-verification-2026-06-12.md` — audit of FID-126 prompt changes
- `dev/audits/FID-2026-0612-knowledge-base-institutional-audit.md` — KB audit (74.5% institutional)
- `research/prompt-sandbox-reasoning-action-divergence.md` + `-v2.md` — source prompts for 11 FIDs
- `research/gemini-deep-research-sandbox-divergence.md` — Gemini research (52KB)
- `research/prompt-snipe-knowledge-generation.md` — snipe processing workflow
- `dev/SNIPE-TRANSCRIPT-PROCESSING.md` — snipe processing workflow (in-tree)
