
## Session 2026-06-21 — FID-222.7 + FID-222.8 + FID-219

### Key Learnings

1. **Text-anchored `grep -nF` is cascade-impossible.** When applying multi-section patches to a single file, resolve each anchor's live line number at insert time (NOT in a single upfront batch) and insert in reverse line order. This eliminates the cascade bug we hit twice: A2 → A3a → A3b cascade shifted PairData.push close brace from L2448 → L2449, breaking downstream anchors.

2. **Sed text substitution is dangerous for multi-line patches.** When fixing brace pairing mid-block, write the fix to /tmp/x.txt FIRST, delete the broken lines with `sed -i M,N d`, then `sed -i M-1 r /tmp/x.txt`. Avoid `sed -i 's|...|...|'` with embedded `\n` escapes — BSD vs GNU sed breaks this.

3. **`std::fs::write("savant.blocked", ...)` must appear on ONE LINE for FID-219 test assertions.** Brittle-by-design regression tests check exact substrings. Multi-line formatting in `{}`-block bodies shifts the literal pattern across lines and the test fails.

4. **`MarketRegime` is a Rust enum (Trending | Ranging | Volatile), NOT a struct. No `Default` impl exists.** Twice we attempted `MarketRegime::default()` and got E0599. Hardcode + TODO is the right pattern for placeholder states pending real implementation.

5. **Test file literals are sensitive to em-dash `—` vs hyphen `-`.** Test 7 expected `config.chains — skipping 5-min sync` (em-dash). Early drafts had `config.chains — skipping 5-min reconciliation sync` (wrong wording + extra word). Always `grep -F` the EXACT expected string against the implementation before declaring done.

### What to do next session
- **FID-222.9** — RegimeDetector integration (real `cycle_regime` from `regime_detector.observe(&market_stores)` ADX/vol signals).
- **FID-222.10** — Lock-order discipline in `src/strategy/pre_scorer.rs` — combine `record_funnel_runtime` + `record_funnel_heartbeat` into a single write via `FunnelRuntimeState::update_from_result(result, regime, hunt_mode, enabled)`.
