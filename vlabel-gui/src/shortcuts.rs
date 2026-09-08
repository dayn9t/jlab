// Keyboard shortcuts system for VLabel v0.7
// Provides configurable keyboard shortcuts with scope-based conflict detection
//
// Types live in `shortcut_types.rs`, the manager and editor state in
// `shortcut_manager.rs`; both are declared here with `#[path]` (same pattern
// as `state_persistence.rs`) because `main.rs` maps modules at `src/` level
// and this keeps the module tree unchanged.

#[path = "shortcut_types.rs"]
mod shortcut_types;

#[path = "shortcut_manager.rs"]
mod shortcut_manager;

pub use shortcut_manager::{ShortcutEditorState, ShortcutManager};
pub use shortcut_types::{
    KeyModifiers, ShortcutAction, ShortcutBinding, ShortcutCategory, ShortcutConfig, ShortcutScope,
};

// Tests live in their own file (declared with `#[path]`, same pattern as
// `state_persistence.rs`) to keep this file away from the length hard limit.
#[cfg(test)]
#[path = "shortcuts_tests.rs"]
mod tests;
