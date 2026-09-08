//! Tests for [`crate::shortcuts::ShortcutManager`] JSON5 persistence.
//!
//! Extracted from `shortcuts.rs` (which sits near the file-length hard limit)
//! as a pure move. The module is declared with `#[cfg(test)] #[path]` inside
//! `shortcuts.rs`, so it is a child of `shortcuts` even though it sits at
//! `src/` level.

use super::*;

#[test]
fn test_shortcuts_roundtrip_and_hand_edit_tolerance() {
    let dir = std::env::temp_dir().join("vlabel_shortcuts_json5_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("shortcuts.json5");

    let manager = ShortcutManager::new();
    manager.save_to_file(&path).unwrap();
    let loaded = ShortcutManager::load_from_file(&path).unwrap();
    assert_eq!(loaded.get_config().shortcuts.len(), manager.get_config().shortcuts.len());

    // 手改：文件头加注释（JSON5 宽容读）
    let hand_edited = format!("// edited by hand\n{}", std::fs::read_to_string(&path).unwrap());
    std::fs::write(&path, hand_edited).unwrap();
    ShortcutManager::load_from_file(&path).unwrap();

    let _ = std::fs::remove_dir_all(&dir);
}
