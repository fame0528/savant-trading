//! FID-146: Jury veto state — shared between jury block and per-pair decision loop.
//! This module exists at the library level so sandbox tests can access the flag.

use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag set by jury block when ≥70% of jury disagrees with primary Buy/Sell.
/// Per-pair decision loop reads this flag and overrides Buy/Sell → Pass.
/// Reset to false after override fires.
pub static FID_146_JURY_VETO: AtomicBool = AtomicBool::new(false);

/// Store true if jury supermajority disagrees with primary action.
pub fn set_veto() {
    FID_146_JURY_VETO.store(true, Ordering::Relaxed);
}

/// Load current veto state.
pub fn is_vetoed() -> bool {
    FID_146_JURY_VETO.load(Ordering::Relaxed)
}

/// Reset veto flag to false (called after override fires).
pub fn clear_veto() {
    FID_146_JURY_VETO.store(false, Ordering::Relaxed);
}
