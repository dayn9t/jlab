// Shortcut data types: scope, category, action, modifiers, bindings, and the
// on-disk config format. Pure move from `shortcuts.rs`; re-exported from the
// `shortcuts` module root so `crate::shortcuts::*` paths are unchanged.

use serde::{Deserialize, Serialize};

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
            Self::OpenProject | Self::Save | Self::Quit => {
                KeyModifiers { ctrl: true, shift: false, alt: false }
            }
            Self::Copy | Self::Paste => KeyModifiers { ctrl: true, shift: false, alt: false },
            Self::SwitchToNormalMode | Self::SwitchToDrawingMode | Self::SwitchToEditingMode => {
                KeyModifiers { ctrl: false, shift: true, alt: false }
            }
            Self::CyclePreviousObject => KeyModifiers { ctrl: false, shift: true, alt: false },
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
            | Self::Zoom400 => KeyModifiers { ctrl: false, shift: false, alt: false },
            _ => KeyModifiers { ctrl: false, shift: false, alt: false },
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
        Self { ctrl: false, shift: false, alt: false }
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
