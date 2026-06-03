//! Keyboard input handling and action dispatch.
//!
//! Maps [`crossterm::event::KeyEvent`] to [`KeyAction`] enum variants.
//! The main loop dispatches actions to either the active tab or global logic.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// ---------------------------------------------------------------------------
// KeyAction  — canonical actions that the TUI understands
// ---------------------------------------------------------------------------

/// All possible actions triggered by keyboard input.
///
/// Some variants are forward-looking (search, sort, detail) and not yet
/// wired in `handle_action`. They are defined here to document the full
/// keyboard contract and are silently ignored until implemented.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum KeyAction {
    // Navigation
    SwitchTab(usize),
    NextTab,
    PrevTab,

    // Scrolling
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    ScrollTop,
    ScrollEnd,

    // Search
    Search,
    SearchInput(char),
    SearchBackspace,
    SearchSubmit,
    SearchCancel,
    NextMatch,
    PrevMatch,

    // Table interaction
    SortBy(usize),
    EnterDetail,
    CloseDetail,
    SelectUp,
    SelectDown,

    // Global
    Quit,
    Refresh,
    Help,
    ToggleTail,
    ClearFilter,
    Noop,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Map a raw key event to a [`KeyAction`].
///
/// Returns `None` for unmapped keys (ignored by the TUI).
pub fn dispatch_key(key: KeyEvent) -> Option<KeyAction> {
    // Handle Ctrl+C globally
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(KeyAction::Quit);
    }

    // Handle search input mode separately — character keys pass through
    // (search mode is handled at the TUI loop level before dispatch)

    match key.code {
        // Quit
        KeyCode::Char('q') | KeyCode::Esc => Some(KeyAction::Quit),

        // Tab switching — number keys 0-9
        KeyCode::Char('0') => Some(KeyAction::SwitchTab(0)),
        KeyCode::Char('1') => Some(KeyAction::SwitchTab(1)),
        KeyCode::Char('2') => Some(KeyAction::SwitchTab(2)),
        KeyCode::Char('3') => Some(KeyAction::SwitchTab(3)),
        KeyCode::Char('4') => Some(KeyAction::SwitchTab(4)),
        KeyCode::Char('5') => Some(KeyAction::SwitchTab(5)),
        KeyCode::Char('6') => Some(KeyAction::SwitchTab(6)),
        KeyCode::Char('7') => Some(KeyAction::SwitchTab(7)),
        KeyCode::Char('8') => Some(KeyAction::SwitchTab(8)),
        KeyCode::Char('9') => Some(KeyAction::SwitchTab(9)),

        // Tab navigation
        KeyCode::Tab => Some(KeyAction::NextTab),
        KeyCode::BackTab => Some(KeyAction::PrevTab),

        // Scrolling
        KeyCode::Up => Some(KeyAction::ScrollUp),
        KeyCode::Down => Some(KeyAction::ScrollDown),
        KeyCode::PageUp => Some(KeyAction::PageUp),
        KeyCode::PageDown => Some(KeyAction::PageDown),
        KeyCode::Home => Some(KeyAction::ScrollTop),
        KeyCode::End => Some(KeyAction::ScrollEnd),

        // Search
        KeyCode::Char('/') => Some(KeyAction::Search),
        KeyCode::Char('n') => Some(KeyAction::NextMatch),
        KeyCode::Char('N') if key.modifiers.contains(KeyModifiers::SHIFT) => {
            Some(KeyAction::PrevMatch)
        }

        // Table interaction
        KeyCode::Enter => Some(KeyAction::EnterDetail),

        // Global actions
        KeyCode::Char('r') => Some(KeyAction::Refresh),
        KeyCode::Char('?') | KeyCode::F(1) => Some(KeyAction::Help),
        KeyCode::Char('t') => Some(KeyAction::ToggleTail),

        _ => None,
    }
}
