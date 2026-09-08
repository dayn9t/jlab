// Shortcut runtime: `ShortcutManager` (config load/save, input dispatch,
// conflict detection, default and binding construction) plus the
// `ShortcutEditorState` UI state. Pure move from `shortcuts.rs`; re-exported
// from the `shortcuts` module root so `crate::shortcuts::*` paths are
// unchanged.
//
// Kept in a single module with `ShortcutEditorState` because it constructs
// `ShortcutManager` via struct literal (private fields) and the private
// `default_config`/`build_bindings` helpers are shared across constructors.

use super::{
    KeyModifiers, ShortcutAction, ShortcutBinding, ShortcutCategory, ShortcutConfig, ShortcutScope,
};
use crate::state::EditMode;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

/// Shortcut manager - handles loading, saving, and runtime detection
pub struct ShortcutManager {
    bindings: HashMap<(egui::Key, KeyModifiers, ShortcutScope), ShortcutAction>,
    config: ShortcutConfig,
}

impl ShortcutManager {
    /// Create new manager with default configuration
    pub fn new() -> Self {
        let config = Self::default_config();
        let bindings = Self::build_bindings(&config);

        Self { bindings, config }
    }

    /// Load configuration from JSON5 file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read shortcuts config from {:?}", path))?;

        let config: ShortcutConfig = json5::from_str(&content)
            .with_context(|| format!("Failed to parse shortcuts config from {:?}", path))?;

        let bindings = Self::build_bindings(&config);

        Ok(Self { bindings, config })
    }

    /// Save configuration to JSON5 file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {:?}", parent))?;
        }

        let content =
            json5::to_string(&self.config).context("Failed to serialize shortcuts config")?;

        std::fs::write(path, content)
            .with_context(|| format!("Failed to write shortcuts config to {:?}", path))?;

        Ok(())
    }

    /// Handle input and return triggered action
    pub fn handle_input(&self, ctx: &egui::Context, mode: EditMode) -> Option<ShortcutAction> {
        let scope = match mode {
            EditMode::Browse => ShortcutScope::Normal,
            EditMode::Drawing => ShortcutScope::Drawing,
            EditMode::Editing => ShortcutScope::Editing,
        };

        ctx.input(|i| {
            // Check all pressed keys
            for key in &[
                egui::Key::A,
                egui::Key::B,
                egui::Key::C,
                egui::Key::D,
                egui::Key::E,
                egui::Key::F,
                egui::Key::G,
                egui::Key::H,
                egui::Key::I,
                egui::Key::J,
                egui::Key::K,
                egui::Key::L,
                egui::Key::M,
                egui::Key::N,
                egui::Key::O,
                egui::Key::P,
                egui::Key::Q,
                egui::Key::R,
                egui::Key::S,
                egui::Key::T,
                egui::Key::U,
                egui::Key::V,
                egui::Key::W,
                egui::Key::X,
                egui::Key::Y,
                egui::Key::Z,
                egui::Key::Num0,
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
                egui::Key::Num5,
                egui::Key::Num6,
                egui::Key::Num7,
                egui::Key::Num8,
                egui::Key::Num9,
                egui::Key::Space,
                egui::Key::Tab,
                egui::Key::Escape,
                egui::Key::Delete,
                egui::Key::ArrowLeft,
                egui::Key::ArrowRight,
                egui::Key::ArrowUp,
                egui::Key::ArrowDown,
                egui::Key::Plus,
                egui::Key::Minus,
                egui::Key::Equals,
            ] {
                if i.key_pressed(*key) {
                    let modifiers = KeyModifiers {
                        ctrl: i.modifiers.ctrl,
                        shift: i.modifiers.shift,
                        alt: i.modifiers.alt,
                    };

                    // Check mode-specific binding first
                    if let Some(action) = self.bindings.get(&(*key, modifiers, scope)) {
                        return Some(*action);
                    }

                    // Check global binding
                    if let Some(action) =
                        self.bindings.get(&(*key, modifiers, ShortcutScope::Global))
                    {
                        return Some(*action);
                    }
                }
            }

            None
        })
    }

    /// Get binding for an action
    pub fn get_binding(&self, action: ShortcutAction) -> Option<&ShortcutBinding> {
        self.config.shortcuts.iter().find(|b| b.action == action.as_str())
    }

    /// Merge configuration (for project-specific overrides)
    pub fn merge_config(&mut self, config: ShortcutConfig) {
        // Merge shortcuts - project config overrides user config
        for binding in config.shortcuts {
            // Remove old binding for this action if exists
            self.config.shortcuts.retain(|b| b.action != binding.action);
            self.config.shortcuts.push(binding);
        }
        // Rebuild bindings after merge
        self.bindings = Self::build_bindings(&self.config);
    }

    /// Detect conflicts in current configuration
    pub fn detect_conflicts(&self) -> usize {
        let mut conflict_count = 0;
        let mut seen: HashMap<(egui::Key, KeyModifiers, ShortcutScope), ShortcutAction> =
            HashMap::new();

        for binding in &self.config.shortcuts {
            // Parse action
            let action = match binding.action.as_str() {
                "OpenProject" => ShortcutAction::OpenProject,
                "Save" => ShortcutAction::Save,
                "CloseProject" => ShortcutAction::CloseProject,
                "Quit" => ShortcutAction::Quit,
                "Copy" => ShortcutAction::Copy,
                "Paste" => ShortcutAction::Paste,
                "Delete" => ShortcutAction::Delete,
                "Deselect" => ShortcutAction::Deselect,
                "FinishDrawing" => ShortcutAction::FinishDrawing,
                "ConvertToRectangle" => ShortcutAction::ConvertToRectangle,
                "FixSelfIntersection" => ShortcutAction::FixSelfIntersection,
                "SwitchToNormalMode" => ShortcutAction::SwitchToNormalMode,
                "SwitchToDrawingMode" => ShortcutAction::SwitchToDrawingMode,
                "SwitchToEditingMode" => ShortcutAction::SwitchToEditingMode,
                "PreviousImage" => ShortcutAction::PreviousImage,
                "NextImage" => ShortcutAction::NextImage,
                "JumpBackward10" => ShortcutAction::JumpBackward10,
                "JumpForward10" => ShortcutAction::JumpForward10,
                "CycleNextObject" => ShortcutAction::CycleNextObject,
                "CyclePreviousObject" => ShortcutAction::CyclePreviousObject,
                "FitToCanvas" => ShortcutAction::FitToCanvas,
                "ResetZoom" => ShortcutAction::ResetZoom,
                "MoveLeft" => ShortcutAction::MoveLeft,
                "MoveRight" => ShortcutAction::MoveRight,
                "MoveUp" => ShortcutAction::MoveUp,
                "MoveDown" => ShortcutAction::MoveDown,
                "ScaleUp" => ShortcutAction::ScaleUp,
                "ScaleDown" => ShortcutAction::ScaleDown,
                "Cancel" => ShortcutAction::Cancel,
                "ToggleAutoSave" => ShortcutAction::ToggleAutoSave,
                "ToggleLeftPanel" => ShortcutAction::ToggleLeftPanel,
                "ToggleRightPanel" => ShortcutAction::ToggleRightPanel,
                "Zoom25" => ShortcutAction::Zoom25,
                "Zoom50" => ShortcutAction::Zoom50,
                "Zoom75" => ShortcutAction::Zoom75,
                "Zoom100" => ShortcutAction::Zoom100,
                "Zoom125" => ShortcutAction::Zoom125,
                "Zoom150" => ShortcutAction::Zoom150,
                "Zoom200" => ShortcutAction::Zoom200,
                "Zoom300" => ShortcutAction::Zoom300,
                "Zoom400" => ShortcutAction::Zoom400,
                _ => continue,
            };

            // Parse key
            let key = match binding.key.as_str() {
                "A" => egui::Key::A,
                "B" => egui::Key::B,
                "C" => egui::Key::C,
                "D" => egui::Key::D,
                "E" => egui::Key::E,
                "F" => egui::Key::F,
                "G" => egui::Key::G,
                "H" => egui::Key::H,
                "I" => egui::Key::I,
                "J" => egui::Key::J,
                "K" => egui::Key::K,
                "L" => egui::Key::L,
                "M" => egui::Key::M,
                "N" => egui::Key::N,
                "O" => egui::Key::O,
                "P" => egui::Key::P,
                "Q" => egui::Key::Q,
                "R" => egui::Key::R,
                "S" => egui::Key::S,
                "T" => egui::Key::T,
                "U" => egui::Key::U,
                "V" => egui::Key::V,
                "W" => egui::Key::W,
                "X" => egui::Key::X,
                "Y" => egui::Key::Y,
                "Z" => egui::Key::Z,
                "Num0" => egui::Key::Num0,
                "Num1" => egui::Key::Num1,
                "Num2" => egui::Key::Num2,
                "Num3" => egui::Key::Num3,
                "Num4" => egui::Key::Num4,
                "Num5" => egui::Key::Num5,
                "Num6" => egui::Key::Num6,
                "Num7" => egui::Key::Num7,
                "Num8" => egui::Key::Num8,
                "Num9" => egui::Key::Num9,
                "Space" => egui::Key::Space,
                "Tab" => egui::Key::Tab,
                "Escape" => egui::Key::Escape,
                "Delete" => egui::Key::Delete,
                "ArrowLeft" => egui::Key::ArrowLeft,
                "ArrowRight" => egui::Key::ArrowRight,
                "ArrowUp" => egui::Key::ArrowUp,
                "ArrowDown" => egui::Key::ArrowDown,
                "Plus" => egui::Key::Plus,
                "Minus" => egui::Key::Minus,
                "Equals" => egui::Key::Equals,
                _ => continue,
            };

            let modifiers =
                KeyModifiers { ctrl: binding.ctrl, shift: binding.shift, alt: binding.alt };

            let scope = binding.scope;

            // Check for conflicts based on strict mode rules
            if scope == ShortcutScope::Global {
                // Global conflicts with everything
                for check_scope in [
                    ShortcutScope::Global,
                    ShortcutScope::Normal,
                    ShortcutScope::Drawing,
                    ShortcutScope::Editing,
                ] {
                    if let Some(&other_action) = seen.get(&(key, modifiers, check_scope)) {
                        if action != other_action {
                            conflict_count += 1;
                        }
                    }
                }
            } else {
                // Mode-specific conflicts with global and same mode
                if let Some(&other_action) = seen.get(&(key, modifiers, ShortcutScope::Global)) {
                    if action != other_action {
                        conflict_count += 1;
                    }
                }
                if let Some(&other_action) = seen.get(&(key, modifiers, scope)) {
                    if action != other_action {
                        conflict_count += 1;
                    }
                }
            }

            seen.insert((key, modifiers, scope), action);
        }

        conflict_count
    }

    /// Format shortcut text for display in menus (e.g., "Ctrl+S")
    pub fn format_shortcut(&self, action: ShortcutAction) -> Option<String> {
        self.get_binding(action).map(|binding| {
            let mut parts = Vec::new();

            if binding.ctrl {
                parts.push("Ctrl");
            }
            if binding.shift {
                parts.push("Shift");
            }
            if binding.alt {
                parts.push("Alt");
            }

            // Format key name
            let key_name = match binding.key.as_str() {
                "Space" => "Space",
                "Tab" => "Tab",
                "Escape" => "Esc",
                "Delete" => "Del",
                "ArrowLeft" => "←",
                "ArrowRight" => "→",
                "ArrowUp" => "↑",
                "ArrowDown" => "↓",
                "Plus" => "+",
                "Minus" => "-",
                "Equals" => "=",
                key if key.starts_with("Num") => &key[3..], // "Num1" -> "1"
                key => key,
            };

            parts.push(key_name);
            parts.join("+")
        })
    }

    /// Get user config directory path
    pub fn user_config_dir() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|mut path| {
            path.push("vlabel");
            path
        })
    }

    /// Get user config file path
    pub fn user_config_path() -> Option<std::path::PathBuf> {
        Self::user_config_dir().map(|mut path| {
            path.push("shortcuts.json5");
            path
        })
    }

    /// Load user configuration if it exists
    pub fn load_user_config() -> Result<Self> {
        if let Some(path) = Self::user_config_path() {
            if path.exists() {
                log::info!("Loading user shortcuts config from {:?}", path);
                return Self::load_from_file(&path);
            }
        }

        // No user config, use defaults
        log::info!("No user shortcuts config found, using defaults");
        Ok(Self::new())
    }

    /// Load with project config override
    pub fn load_with_project_config(project_dir: &Path) -> Result<Self> {
        // Start with user config (or defaults)
        let mut manager = Self::load_user_config()?;

        // Try to load project config
        let project_config_path = project_dir.join("shortcuts.json5");
        if project_config_path.exists() {
            log::info!("Loading project shortcuts config from {:?}", project_config_path);
            let project_config: ShortcutConfig = {
                let content = std::fs::read_to_string(&project_config_path).with_context(|| {
                    format!(
                        "Failed to read project shortcuts config from {:?}",
                        project_config_path
                    )
                })?;
                json5::from_str(&content).with_context(|| {
                    format!(
                        "Failed to parse project shortcuts config from {:?}",
                        project_config_path
                    )
                })?
            };
            manager.merge_config(project_config);
        }

        Ok(manager)
    }

    /// Get the current configuration
    pub fn get_config(&self) -> &ShortcutConfig {
        &self.config
    }

    /// Apply a new configuration
    pub fn apply_config(&mut self, config: ShortcutConfig) {
        self.config = config;
        self.bindings = Self::build_bindings(&self.config);
    }

    /// Build default configuration
    fn default_config() -> ShortcutConfig {
        let actions = [
            ShortcutAction::OpenProject,
            ShortcutAction::Save,
            ShortcutAction::CloseProject,
            ShortcutAction::Quit,
            ShortcutAction::Copy,
            ShortcutAction::Paste,
            ShortcutAction::Delete,
            ShortcutAction::Deselect,
            ShortcutAction::FinishDrawing,
            ShortcutAction::ConvertToRectangle,
            ShortcutAction::FixSelfIntersection,
            ShortcutAction::SwitchToNormalMode,
            ShortcutAction::SwitchToDrawingMode,
            ShortcutAction::SwitchToEditingMode,
            ShortcutAction::PreviousImage,
            ShortcutAction::NextImage,
            ShortcutAction::JumpBackward10,
            ShortcutAction::JumpForward10,
            ShortcutAction::CycleNextObject,
            ShortcutAction::CyclePreviousObject,
            ShortcutAction::FitToCanvas,
            ShortcutAction::ResetZoom,
            ShortcutAction::MoveLeft,
            ShortcutAction::MoveRight,
            ShortcutAction::MoveUp,
            ShortcutAction::MoveDown,
            ShortcutAction::ScaleUp,
            ShortcutAction::ScaleDown,
            ShortcutAction::Cancel,
            ShortcutAction::ToggleAutoSave,
            ShortcutAction::ToggleLeftPanel,
            ShortcutAction::ToggleRightPanel,
            ShortcutAction::Zoom25,
            ShortcutAction::Zoom50,
            ShortcutAction::Zoom75,
            ShortcutAction::Zoom100,
            ShortcutAction::Zoom125,
            ShortcutAction::Zoom150,
            ShortcutAction::Zoom200,
            ShortcutAction::Zoom300,
            ShortcutAction::Zoom400,
        ];

        let shortcuts = actions
            .iter()
            .filter_map(|action| {
                action.default_key().map(|key| ShortcutBinding {
                    action: action.as_str().to_string(),
                    key: format!("{:?}", key),
                    ctrl: action.default_modifiers().ctrl,
                    shift: action.default_modifiers().shift,
                    alt: action.default_modifiers().alt,
                    scope: action.scope(),
                    description: action.description_key().to_string(),
                    category: action.category().as_str().to_string(),
                })
            })
            .collect();

        ShortcutConfig { version: "1.0".to_string(), shortcuts }
    }

    /// Build bindings map from config
    fn build_bindings(
        config: &ShortcutConfig,
    ) -> HashMap<(egui::Key, KeyModifiers, ShortcutScope), ShortcutAction> {
        let mut bindings = HashMap::new();

        for binding in &config.shortcuts {
            // Parse action
            let action = match binding.action.as_str() {
                "OpenProject" => ShortcutAction::OpenProject,
                "Save" => ShortcutAction::Save,
                "CloseProject" => ShortcutAction::CloseProject,
                "Quit" => ShortcutAction::Quit,
                "Copy" => ShortcutAction::Copy,
                "Paste" => ShortcutAction::Paste,
                "Delete" => ShortcutAction::Delete,
                "Deselect" => ShortcutAction::Deselect,
                "FinishDrawing" => ShortcutAction::FinishDrawing,
                "ConvertToRectangle" => ShortcutAction::ConvertToRectangle,
                "FixSelfIntersection" => ShortcutAction::FixSelfIntersection,
                "SwitchToNormalMode" => ShortcutAction::SwitchToNormalMode,
                "SwitchToDrawingMode" => ShortcutAction::SwitchToDrawingMode,
                "SwitchToEditingMode" => ShortcutAction::SwitchToEditingMode,
                "PreviousImage" => ShortcutAction::PreviousImage,
                "NextImage" => ShortcutAction::NextImage,
                "JumpBackward10" => ShortcutAction::JumpBackward10,
                "JumpForward10" => ShortcutAction::JumpForward10,
                "CycleNextObject" => ShortcutAction::CycleNextObject,
                "CyclePreviousObject" => ShortcutAction::CyclePreviousObject,
                "FitToCanvas" => ShortcutAction::FitToCanvas,
                "ResetZoom" => ShortcutAction::ResetZoom,
                "MoveLeft" => ShortcutAction::MoveLeft,
                "MoveRight" => ShortcutAction::MoveRight,
                "MoveUp" => ShortcutAction::MoveUp,
                "MoveDown" => ShortcutAction::MoveDown,
                "ScaleUp" => ShortcutAction::ScaleUp,
                "ScaleDown" => ShortcutAction::ScaleDown,
                "Cancel" => ShortcutAction::Cancel,
                "ToggleAutoSave" => ShortcutAction::ToggleAutoSave,
                "ToggleLeftPanel" => ShortcutAction::ToggleLeftPanel,
                "ToggleRightPanel" => ShortcutAction::ToggleRightPanel,
                "Zoom25" => ShortcutAction::Zoom25,
                "Zoom50" => ShortcutAction::Zoom50,
                "Zoom75" => ShortcutAction::Zoom75,
                "Zoom100" => ShortcutAction::Zoom100,
                "Zoom125" => ShortcutAction::Zoom125,
                "Zoom150" => ShortcutAction::Zoom150,
                "Zoom200" => ShortcutAction::Zoom200,
                "Zoom300" => ShortcutAction::Zoom300,
                "Zoom400" => ShortcutAction::Zoom400,
                _ => continue, // Skip unknown actions
            };

            // Parse key
            let key = match binding.key.as_str() {
                "A" => egui::Key::A,
                "B" => egui::Key::B,
                "C" => egui::Key::C,
                "D" => egui::Key::D,
                "E" => egui::Key::E,
                "F" => egui::Key::F,
                "G" => egui::Key::G,
                "H" => egui::Key::H,
                "I" => egui::Key::I,
                "J" => egui::Key::J,
                "K" => egui::Key::K,
                "L" => egui::Key::L,
                "M" => egui::Key::M,
                "N" => egui::Key::N,
                "O" => egui::Key::O,
                "P" => egui::Key::P,
                "Q" => egui::Key::Q,
                "R" => egui::Key::R,
                "S" => egui::Key::S,
                "T" => egui::Key::T,
                "U" => egui::Key::U,
                "V" => egui::Key::V,
                "W" => egui::Key::W,
                "X" => egui::Key::X,
                "Y" => egui::Key::Y,
                "Z" => egui::Key::Z,
                "Num0" => egui::Key::Num0,
                "Num1" => egui::Key::Num1,
                "Num2" => egui::Key::Num2,
                "Num3" => egui::Key::Num3,
                "Num4" => egui::Key::Num4,
                "Num5" => egui::Key::Num5,
                "Num6" => egui::Key::Num6,
                "Num7" => egui::Key::Num7,
                "Num8" => egui::Key::Num8,
                "Num9" => egui::Key::Num9,
                "Space" => egui::Key::Space,
                "Tab" => egui::Key::Tab,
                "Escape" => egui::Key::Escape,
                "Delete" => egui::Key::Delete,
                "ArrowLeft" => egui::Key::ArrowLeft,
                "ArrowRight" => egui::Key::ArrowRight,
                "ArrowUp" => egui::Key::ArrowUp,
                "ArrowDown" => egui::Key::ArrowDown,
                "Plus" => egui::Key::Plus,
                "Minus" => egui::Key::Minus,
                "Equals" => egui::Key::Equals,
                _ => continue, // Skip unknown keys
            };

            let modifiers =
                KeyModifiers { ctrl: binding.ctrl, shift: binding.shift, alt: binding.alt };

            bindings.insert((key, modifiers, binding.scope), action);
        }

        bindings
    }
}

/// State for the shortcut editor UI
pub struct ShortcutEditorState {
    pub working_config: ShortcutConfig,
    pub editing_action: Option<ShortcutAction>,
    pub captured_key: Option<egui::Key>,
    pub captured_modifiers: KeyModifiers,
    pub search_filter: String,
    pub category_filter: Option<ShortcutCategory>,
    pub conflict_count: usize,
    pub is_capturing: bool,
}

impl ShortcutEditorState {
    pub fn new(current_config: &ShortcutConfig) -> Self {
        let mut state = Self {
            working_config: current_config.clone(),
            editing_action: None,
            captured_key: None,
            captured_modifiers: KeyModifiers { ctrl: false, shift: false, alt: false },
            search_filter: String::new(),
            category_filter: None,
            conflict_count: 0,
            is_capturing: false,
        };
        state.refresh_conflicts();
        state
    }

    pub fn start_editing(&mut self, action: ShortcutAction) {
        self.editing_action = Some(action);
        self.captured_key = None;
        self.captured_modifiers = KeyModifiers { ctrl: false, shift: false, alt: false };
        self.is_capturing = true;
    }

    pub fn cancel_editing(&mut self) {
        self.editing_action = None;
        self.is_capturing = false;
    }

    pub fn apply_edit(&mut self) {
        if let (Some(action), Some(key)) = (self.editing_action, self.captured_key) {
            self.update_binding(action, key, self.captured_modifiers);
            self.cancel_editing();
            self.refresh_conflicts();
        }
    }

    fn update_binding(&mut self, action: ShortcutAction, key: egui::Key, modifiers: KeyModifiers) {
        if let Some(binding) =
            self.working_config.shortcuts.iter_mut().find(|b| b.action == action.as_str())
        {
            binding.key = format!("{:?}", key);
            binding.ctrl = modifiers.ctrl;
            binding.shift = modifiers.shift;
            binding.alt = modifiers.alt;
        }
    }

    pub fn refresh_conflicts(&mut self) {
        let temp_manager = ShortcutManager {
            bindings: std::collections::HashMap::new(),
            config: self.working_config.clone(),
        };
        self.conflict_count = temp_manager.detect_conflicts();
    }

    pub fn get_filtered_shortcuts(&self) -> Vec<(ShortcutAction, &ShortcutBinding)> {
        ShortcutAction::all_actions()
            .into_iter()
            .filter_map(|action| {
                if let Some(cat) = self.category_filter {
                    if action.category() != cat {
                        return None;
                    }
                }

                if !self.search_filter.is_empty() {
                    let search_lower = self.search_filter.to_lowercase();
                    let action_name = action.as_str().to_lowercase();
                    if !action_name.contains(&search_lower) {
                        return None;
                    }
                }

                self.working_config
                    .shortcuts
                    .iter()
                    .find(|b| b.action == action.as_str())
                    .map(|b| (action, b))
            })
            .collect()
    }
}

impl Default for ShortcutManager {
    fn default() -> Self {
        Self::new()
    }
}
