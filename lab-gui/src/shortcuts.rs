// Keyboard shortcuts system for JLab v0.7
// Provides configurable keyboard shortcuts with scope-based conflict detection

use crate::state::EditMode;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Shortcut scope - determines when a shortcut is active
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShortcutScope {
    /// Active in all modes
    Global,
    /// Active only in Browse mode
    Normal,
    /// Active only in Drawing mode
    Drawing,
    /// Active only in Editing mode
    Editing,
}

impl ShortcutScope {
    pub fn name_key(&self) -> &'static str {
        match self {
            Self::Global => "shortcuts.scope_global",
            Self::Normal => "shortcuts.scope_normal",
            Self::Drawing => "shortcuts.scope_drawing",
            Self::Editing => "shortcuts.scope_editing",
        }
    }
}

/// Shortcut category for UI grouping
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutCategory {
    File,
    Edit,
    Mode,
    Navigation,
    View,
    Tools,
    Other,
}

impl ShortcutCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Edit => "edit",
            Self::Mode => "mode",
            Self::Navigation => "navigation",
            Self::View => "view",
            Self::Tools => "tools",
            Self::Other => "other",
        }
    }

    pub fn all_categories() -> Vec<Self> {
        vec![
            Self::File,
            Self::Edit,
            Self::Mode,
            Self::Navigation,
            Self::View,
            Self::Tools,
            Self::Other,
        ]
    }

    pub fn name_key(&self) -> &'static str {
        match self {
            Self::File => "shortcuts.category_file",
            Self::Edit => "shortcuts.category_edit",
            Self::Mode => "shortcuts.category_mode",
            Self::Navigation => "shortcuts.category_navigation",
            Self::View => "shortcuts.category_view",
            Self::Tools => "shortcuts.category_tools",
            Self::Other => "shortcuts.category_other",
        }
    }
}

/// All possible shortcut actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutAction {
    // File operations
    OpenProject,
    Save,
    CloseProject,
    Quit,

    // Edit operations
    Copy,
    Paste,
    Delete,
    Deselect,
    FinishDrawing,
    ConvertToRectangle,
    FixSelfIntersection,

    // Mode switching
    SwitchToNormalMode,
    SwitchToDrawingMode,
    SwitchToEditingMode,

    // Navigation
    PreviousImage,
    NextImage,
    JumpBackward10,
    JumpForward10,
    CycleNextObject,
    CyclePreviousObject,

    // View
    FitToCanvas,
    ResetZoom,

    // Movement
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,

    // Scaling
    ScaleUp,
    ScaleDown,

    // Other
    Cancel,

    // Toggle Panels
    ToggleAutoSave,
    ToggleLeftPanel,
    ToggleRightPanel,

    // Zoom Levels
    Zoom25,
    Zoom50,
    Zoom75,
    Zoom100,
    Zoom125,
    Zoom150,
    Zoom200,
    Zoom300,
    Zoom400,
}

impl ShortcutAction {
    /// Get all possible actions
    pub fn all_actions() -> Vec<Self> {
        vec![
            Self::OpenProject,
            Self::Save,
            Self::CloseProject,
            Self::Quit,
            Self::Copy,
            Self::Paste,
            Self::Delete,
            Self::Deselect,
            Self::FinishDrawing,
            Self::ConvertToRectangle,
            Self::FixSelfIntersection,
            Self::SwitchToNormalMode,
            Self::SwitchToDrawingMode,
            Self::SwitchToEditingMode,
            Self::PreviousImage,
            Self::NextImage,
            Self::JumpBackward10,
            Self::JumpForward10,
            Self::CycleNextObject,
            Self::CyclePreviousObject,
            Self::FitToCanvas,
            Self::ResetZoom,
            Self::MoveLeft,
            Self::MoveRight,
            Self::MoveUp,
            Self::MoveDown,
            Self::ScaleUp,
            Self::ScaleDown,
            Self::Cancel,
            Self::ToggleAutoSave,
            Self::ToggleLeftPanel,
            Self::ToggleRightPanel,
            Self::Zoom25,
            Self::Zoom50,
            Self::Zoom75,
            Self::Zoom100,
            Self::Zoom125,
            Self::Zoom150,
            Self::Zoom200,
            Self::Zoom300,
            Self::Zoom400,
        ]
    }

    /// Get default key for this action
    pub fn default_key(&self) -> Option<egui::Key> {
        use egui::Key::*;
        match self {
            Self::OpenProject => Some(O),
            Self::Save => Some(S),
            Self::CloseProject => None,
            Self::Quit => Some(Q),
            Self::Copy => Some(C),
            Self::Paste => Some(V),
            Self::Delete => Some(Delete),
            Self::Deselect => Some(Escape),
            Self::FinishDrawing => Some(Space),
            Self::ConvertToRectangle => Some(R),
            Self::FixSelfIntersection => Some(B),
            Self::SwitchToNormalMode => Some(Num1),
            Self::SwitchToDrawingMode => Some(Num2),
            Self::SwitchToEditingMode => Some(Num3),
            Self::PreviousImage => Some(A),
            Self::NextImage => Some(D),
            Self::JumpBackward10 => Some(S),
            Self::JumpForward10 => Some(W),
            Self::CycleNextObject => Some(Tab),
            Self::CyclePreviousObject => Some(Tab),
            Self::FitToCanvas => Some(F),
            Self::ResetZoom => None,
            Self::MoveLeft => Some(ArrowLeft),
            Self::MoveRight => Some(ArrowRight),
            Self::MoveUp => Some(ArrowUp),
            Self::MoveDown => Some(ArrowDown),
            Self::ScaleUp => Some(Plus),
            Self::ScaleDown => Some(Minus),
            Self::Cancel => Some(Escape),
            Self::ToggleAutoSave => None,
            Self::ToggleLeftPanel => None,
            Self::ToggleRightPanel => None,
            Self::Zoom25 => None,
            Self::Zoom50 => None,
            Self::Zoom75 => None,
            Self::Zoom100 => Some(Z),
            Self::Zoom125 => None,
            Self::Zoom150 => None,
            Self::Zoom200 => None,
            Self::Zoom300 => None,
            Self::Zoom400 => None,
        }
    }

    /// Get default modifiers for this action
    pub fn default_modifiers(&self) -> KeyModifiers {
        match self {
            Self::OpenProject | Self::Save | Self::Quit => KeyModifiers {
                ctrl: true,
                shift: false,
                alt: false,
            },
            Self::Copy | Self::Paste => KeyModifiers {
                ctrl: true,
                shift: false,
                alt: false,
            },
            Self::SwitchToNormalMode | Self::SwitchToDrawingMode | Self::SwitchToEditingMode => {
                KeyModifiers {
                    ctrl: false,
                    shift: true,
                    alt: false,
                }
            }
            Self::CyclePreviousObject => KeyModifiers {
                ctrl: false,
                shift: true,
                alt: false,
            },
            Self::ToggleAutoSave
            | Self::ToggleLeftPanel
            | Self::ToggleRightPanel
            | Self::Zoom25
            | Self::Zoom50
            | Self::Zoom75
            | Self::Zoom100
            | Self::Zoom125
            | Self::Zoom150
            | Self::Zoom200
            | Self::Zoom300
            | Self::Zoom400 => KeyModifiers {
                ctrl: false,
                shift: false,
                alt: false,
            },
            _ => KeyModifiers {
                ctrl: false,
                shift: false,
                alt: false,
            },
        }
    }

    /// Get scope for this action
    pub fn scope(&self) -> ShortcutScope {
        match self {
            Self::OpenProject | Self::Save | Self::CloseProject | Self::Quit => {
                ShortcutScope::Global
            }
            Self::Delete => ShortcutScope::Global,
            Self::Copy | Self::Paste => ShortcutScope::Editing,
            Self::SwitchToNormalMode | Self::SwitchToDrawingMode | Self::SwitchToEditingMode => {
                ShortcutScope::Global
            }
            Self::PreviousImage | Self::NextImage | Self::JumpBackward10 | Self::JumpForward10 => {
                ShortcutScope::Global
            }
            Self::FitToCanvas | Self::ResetZoom => ShortcutScope::Global,
            Self::CycleNextObject | Self::CyclePreviousObject => ShortcutScope::Normal,
            Self::Deselect => ShortcutScope::Normal,
            Self::FinishDrawing => ShortcutScope::Drawing,
            Self::ConvertToRectangle | Self::FixSelfIntersection => ShortcutScope::Editing,
            Self::MoveLeft | Self::MoveRight | Self::MoveUp | Self::MoveDown => {
                ShortcutScope::Editing
            }
            Self::ScaleUp | Self::ScaleDown => ShortcutScope::Editing,
            Self::Cancel => ShortcutScope::Global,
            Self::ToggleAutoSave
            | Self::ToggleLeftPanel
            | Self::ToggleRightPanel
            | Self::Zoom25
            | Self::Zoom50
            | Self::Zoom75
            | Self::Zoom100
            | Self::Zoom125
            | Self::Zoom150
            | Self::Zoom200
            | Self::Zoom300
            | Self::Zoom400 => ShortcutScope::Global,
        }
    }

    /// Get category for this action
    pub fn category(&self) -> ShortcutCategory {
        match self {
            Self::OpenProject | Self::Save | Self::CloseProject | Self::Quit => {
                ShortcutCategory::File
            }
            Self::Copy
            | Self::Paste
            | Self::Delete
            | Self::Deselect
            | Self::FinishDrawing
            | Self::ConvertToRectangle
            | Self::FixSelfIntersection => ShortcutCategory::Edit,
            Self::SwitchToNormalMode | Self::SwitchToDrawingMode | Self::SwitchToEditingMode => {
                ShortcutCategory::Mode
            }
            Self::PreviousImage
            | Self::NextImage
            | Self::JumpBackward10
            | Self::JumpForward10
            | Self::CycleNextObject
            | Self::CyclePreviousObject => ShortcutCategory::Navigation,
            Self::FitToCanvas | Self::ResetZoom => ShortcutCategory::View,
            Self::MoveLeft
            | Self::MoveRight
            | Self::MoveUp
            | Self::MoveDown
            | Self::ScaleUp
            | Self::ScaleDown => ShortcutCategory::Tools,
            Self::Cancel => ShortcutCategory::Other,
            Self::ToggleAutoSave
            | Self::ToggleLeftPanel
            | Self::ToggleRightPanel
            | Self::Zoom25
            | Self::Zoom50
            | Self::Zoom75
            | Self::Zoom100
            | Self::Zoom125
            | Self::Zoom150
            | Self::Zoom200
            | Self::Zoom300
            | Self::Zoom400 => ShortcutCategory::View,
        }
    }

    /// Get i18n key for description
    pub fn description_key(&self) -> &'static str {
        match self {
            Self::OpenProject => "shortcut_actions.open_project",
            Self::Save => "shortcut_actions.save",
            Self::CloseProject => "shortcut_actions.close_project",
            Self::Quit => "shortcut_actions.quit",
            Self::Copy => "shortcut_actions.copy",
            Self::Paste => "shortcut_actions.paste",
            Self::Delete => "shortcut_actions.delete",
            Self::Deselect => "shortcut_actions.deselect",
            Self::FinishDrawing => "shortcut_actions.finish_drawing",
            Self::ConvertToRectangle => "shortcut_actions.convert_to_rectangle",
            Self::FixSelfIntersection => "shortcut_actions.fix_self_intersection",
            Self::SwitchToNormalMode => "shortcut_actions.switch_to_normal",
            Self::SwitchToDrawingMode => "shortcut_actions.switch_to_drawing",
            Self::SwitchToEditingMode => "shortcut_actions.switch_to_editing",
            Self::PreviousImage => "shortcut_actions.previous_image",
            Self::NextImage => "shortcut_actions.next_image",
            Self::JumpBackward10 => "shortcut_actions.jump_backward_10",
            Self::JumpForward10 => "shortcut_actions.jump_forward_10",
            Self::CycleNextObject => "shortcut_actions.cycle_next",
            Self::CyclePreviousObject => "shortcut_actions.cycle_previous",
            Self::FitToCanvas => "shortcut_actions.fit_to_canvas",
            Self::ResetZoom => "shortcut_actions.reset_zoom",
            Self::MoveLeft => "shortcut_actions.move_left",
            Self::MoveRight => "shortcut_actions.move_right",
            Self::MoveUp => "shortcut_actions.move_up",
            Self::MoveDown => "shortcut_actions.move_down",
            Self::ScaleUp => "shortcut_actions.scale_up",
            Self::ScaleDown => "shortcut_actions.scale_down",
            Self::Cancel => "shortcut_actions.cancel",
            Self::ToggleAutoSave => "shortcut_actions.toggle_auto_save",
            Self::ToggleLeftPanel => "shortcut_actions.toggle_left_panel",
            Self::ToggleRightPanel => "shortcut_actions.toggle_right_panel",
            Self::Zoom25 => "shortcut_actions.zoom_25",
            Self::Zoom50 => "shortcut_actions.zoom_50",
            Self::Zoom75 => "shortcut_actions.zoom_75",
            Self::Zoom100 => "shortcut_actions.zoom_100",
            Self::Zoom125 => "shortcut_actions.zoom_125",
            Self::Zoom150 => "shortcut_actions.zoom_150",
            Self::Zoom200 => "shortcut_actions.zoom_200",
            Self::Zoom300 => "shortcut_actions.zoom_300",
            Self::Zoom400 => "shortcut_actions.zoom_400",
        }
    }

    /// Get action name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpenProject => "OpenProject",
            Self::Save => "Save",
            Self::CloseProject => "CloseProject",
            Self::Quit => "Quit",
            Self::Copy => "Copy",
            Self::Paste => "Paste",
            Self::Delete => "Delete",
            Self::Deselect => "Deselect",
            Self::FinishDrawing => "FinishDrawing",
            Self::ConvertToRectangle => "ConvertToRectangle",
            Self::FixSelfIntersection => "FixSelfIntersection",
            Self::SwitchToNormalMode => "SwitchToNormalMode",
            Self::SwitchToDrawingMode => "SwitchToDrawingMode",
            Self::SwitchToEditingMode => "SwitchToEditingMode",
            Self::PreviousImage => "PreviousImage",
            Self::NextImage => "NextImage",
            Self::JumpBackward10 => "JumpBackward10",
            Self::JumpForward10 => "JumpForward10",
            Self::CycleNextObject => "CycleNextObject",
            Self::CyclePreviousObject => "CyclePreviousObject",
            Self::FitToCanvas => "FitToCanvas",
            Self::ResetZoom => "ResetZoom",
            Self::MoveLeft => "MoveLeft",
            Self::MoveRight => "MoveRight",
            Self::MoveUp => "MoveUp",
            Self::MoveDown => "MoveDown",
            Self::ScaleUp => "ScaleUp",
            Self::ScaleDown => "ScaleDown",
            Self::Cancel => "Cancel",
            Self::ToggleAutoSave => "ToggleAutoSave",
            Self::ToggleLeftPanel => "ToggleLeftPanel",
            Self::ToggleRightPanel => "ToggleRightPanel",
            Self::Zoom25 => "Zoom25",
            Self::Zoom50 => "Zoom50",
            Self::Zoom75 => "Zoom75",
            Self::Zoom100 => "Zoom100",
            Self::Zoom125 => "Zoom125",
            Self::Zoom150 => "Zoom150",
            Self::Zoom200 => "Zoom200",
            Self::Zoom300 => "Zoom300",
            Self::Zoom400 => "Zoom400",
        }
    }
}

/// Key modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Default for KeyModifiers {
    fn default() -> Self {
        Self {
            ctrl: false,
            shift: false,
            alt: false,
        }
    }
}

/// Shortcut binding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutBinding {
    pub action: String,
    pub key: String,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    pub scope: ShortcutScope,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: String,
}

/// Shortcut configuration file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutConfig {
    pub version: String,
    pub shortcuts: Vec<ShortcutBinding>,
}

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

    /// Load configuration from YAML file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read shortcuts config from {:?}", path))?;

        let config: ShortcutConfig = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse shortcuts config from {:?}", path))?;

        let bindings = Self::build_bindings(&config);

        Ok(Self { bindings, config })
    }

    /// Save configuration to YAML file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {:?}", parent))?;
        }

        let content =
            serde_yaml::to_string(&self.config).context("Failed to serialize shortcuts config")?;

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
        self.config
            .shortcuts
            .iter()
            .find(|b| b.action == action.as_str())
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

            let modifiers = KeyModifiers {
                ctrl: binding.ctrl,
                shift: binding.shift,
                alt: binding.alt,
            };

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
            path.push("jlab");
            path
        })
    }

    /// Get user config file path
    pub fn user_config_path() -> Option<std::path::PathBuf> {
        Self::user_config_dir().map(|mut path| {
            path.push("shortcuts.yaml");
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
        let project_config_path = project_dir.join("shortcuts.yaml");
        if project_config_path.exists() {
            log::info!(
                "Loading project shortcuts config from {:?}",
                project_config_path
            );
            let project_config: ShortcutConfig = {
                let content = std::fs::read_to_string(&project_config_path).with_context(|| {
                    format!(
                        "Failed to read project shortcuts config from {:?}",
                        project_config_path
                    )
                })?;
                serde_yaml::from_str(&content).with_context(|| {
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

        ShortcutConfig {
            version: "1.0".to_string(),
            shortcuts,
        }
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

            let modifiers = KeyModifiers {
                ctrl: binding.ctrl,
                shift: binding.shift,
                alt: binding.alt,
            };

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
            captured_modifiers: KeyModifiers {
                ctrl: false,
                shift: false,
                alt: false,
            },
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
        self.captured_modifiers = KeyModifiers {
            ctrl: false,
            shift: false,
            alt: false,
        };
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
        if let Some(binding) = self
            .working_config
            .shortcuts
            .iter_mut()
            .find(|b| b.action == action.as_str())
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
