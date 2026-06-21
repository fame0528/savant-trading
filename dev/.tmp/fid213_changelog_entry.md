## [0.15.3] - 2026-06-20

### Fixed
- **FID-213:** Anvil fresh-startup balance override. Engine now adopts `starting_balance` over chain-truth on Anvil fresh-startup (when no persisted state file exists), emits a single audit ledger line showing adopted vs. chain-reported (warn at >$0.10 drift, info otherwise). `save_state()` now surfaces errors via warn! macro instead of silent `.ok()` to prevent disk-failure regressions. Fixes the phantom-$-50-at-fresh-restart operator reported on 2026-06-20.

### Tests
- Added `tests/fid213_anvil_balance_init.rs` (6 tests): covers helper-fn, struct marker presence, override-block structural presence, save_state ordering, threshold check presence, log-message audit. cargo baseline: 469 → 475.

### Docs
- `dev/fids/archive/FID-2026-0620-213-anvil-fresh-startup-balance-override.md` — full FID archive.
- `dev/LEARNINGS.md` — session row with 5 lessons.
- `README.md` — cargo count banner: 469 → 475.
- `VERSION` — bumped 0.15.2 → 0.15.3.

