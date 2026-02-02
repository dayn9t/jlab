use crate::shortcuts::ShortcutManager;
use lab_core::{Annotation, Meta, Object, Point};
use lab_utils::Project;
use serde::{Deserialize, Serialize};
use serde_json;
use std::path::PathBuf;

/// Theme color preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeColor {
    Light,
    Dark,
    System,
}

impl ThemeColor {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::System => "system",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            "system" => Some(Self::System),
            _ => None,
        }
    }

    pub fn name_key(&self) -> &'static str {
        match self {
            Self::Light => "options.theme_light",
            Self::Dark => "options.theme_dark",
            Self::System => "options.theme_system",
        }
    }
}

/// Edit mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMode {
    /// Browse mode: read-only selection and navigation
    Browse,
    /// Drawing mode: create new shapes
    Drawing,
    /// Editing mode: move shapes and edit vertices
    Editing,
}

/// Editing state
pub struct EditingState {
    /// Current edit mode
    pub mode: EditMode,

    /// Selected vertex (object_id, vertex_index)
    pub selected_vertex: Option<(i32, usize)>,
}

impl EditingState {
    pub fn new() -> Self {
        Self {
            mode: EditMode::Browse,
            selected_vertex: None,
        }
    }
}

pub struct PendingDrawClick {
    pub position: Point,
    pub time: f64,
}

/// Application state
pub struct AppState {
    /// Current project
    pub project: Option<Project>,

    /// List of image paths
    pub images: Vec<PathBuf>,

    /// Current image index
    pub current_image_index: usize,

    /// Current annotation
    pub current_annotation: Option<Annotation>,

    /// Loaded image data
    pub current_image: Option<ImageData>,

    /// Has unsaved changes
    pub has_unsaved_changes: bool,

    /// Target type for new shapes
    pub draw_target: DrawTarget,

    /// Default category for new objects
    pub default_category_id: Option<i32>,

    /// Selected object ID
    pub selected_object_id: Option<i32>,

    /// Temporary points (for drawing new shapes)
    pub temp_points: Vec<Point>,

    /// Pending single-clicks in Drawing mode (delayed for double-click detection)
    pub pending_draw_clicks: Vec<PendingDrawClick>,

    /// Clipboard objects for copy/paste operations
    pub clipboard_objects: Vec<Object>,

    /// Clipboard ROIs for copy/paste operations
    pub clipboard_rois: Vec<Vec<Point>>,

    /// Editing state
    pub editing_state: EditingState,

    /// Recent projects (max 10)
    pub recent_projects: Vec<PathBuf>,

    /// Current language
    pub language: crate::i18n::Language,

    /// Internationalization
    pub i18n: crate::i18n::I18n,

    /// Shortcut manager
    pub shortcut_manager: ShortcutManager,

    /// Show shortcut settings window
    pub show_shortcut_settings: bool,

    // Dialog states (新增)
    pub show_options_dialog: bool,
    pub show_about_dialog: bool,

    /// Global auto-save setting
    pub auto_save_enabled: bool,

    /// Theme setting
    pub theme_color: ThemeColor,

    /// Show left sidebar
    pub show_left_panel: bool,

    /// Show right sidebar
    pub show_right_panel: bool,
}

impl AppState {
    pub fn new() -> Self {
        // Detect system language
        let language = crate::i18n::Language::detect_system();

        // Try to load saved language setting
        let saved_language = Self::load_language_setting();
        let language = saved_language.unwrap_or(language);

        // Initialize i18n
        let i18n = crate::i18n::I18n::new(language).expect("Failed to load language resources");

        // Try to load saved auto-save setting
        let saved_auto_save = Self::load_auto_save_setting();
        let auto_save_enabled = saved_auto_save.unwrap_or(true);

        // Try to load saved theme setting
        let saved_theme = Self::load_theme_setting();
        let theme_color = saved_theme.unwrap_or(ThemeColor::System);

        let mut state = Self {
            project: None,
            images: Vec::new(),
            current_image_index: 0,
            current_annotation: None,
            current_image: None,
            has_unsaved_changes: false,
            draw_target: DrawTarget::Object,
            default_category_id: None,
            selected_object_id: None,
            temp_points: Vec::new(),
            pending_draw_clicks: Vec::new(),
            clipboard_objects: Vec::new(),
            clipboard_rois: Vec::new(),
            editing_state: EditingState::new(),
            recent_projects: Vec::new(),
            language,
            i18n,
            shortcut_manager: ShortcutManager::load_user_config().unwrap_or_else(|e| {
                log::warn!("Failed to load user shortcuts config: {}", e);
                ShortcutManager::new()
            }),
            show_shortcut_settings: false,
            show_options_dialog: false,
            show_about_dialog: false,
            auto_save_enabled,
            theme_color,
            show_left_panel: true,
            show_right_panel: true,
        };
        // Load recent projects from file
        let _ = state.load_recent_projects();
        state
    }

    /// Load a project
    pub fn load_project(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let mut project = Project::open(&path)?;
        let images = project.list_images()?;

        // Apply global auto-save setting to project
        project.meta.shape.auto_save = self.auto_save_enabled;

        // Load project-specific shortcuts config if exists
        self.shortcut_manager =
            ShortcutManager::load_with_project_config(&path).unwrap_or_else(|e| {
                log::warn!("Failed to load project shortcuts config: {}", e);
                ShortcutManager::load_user_config().unwrap_or_else(|e| {
                    log::warn!("Failed to load user shortcuts config: {}", e);
                    ShortcutManager::new()
                })
            });

        self.project = Some(project);
        self.default_category_id = self
            .project
            .as_ref()
            .and_then(|p| p.meta.categories.first().map(|c| c.id));
        self.images = images;
        self.current_image_index = 0;

        if !self.images.is_empty() {
            self.load_current_image()?;
        }

        // Add to recent projects
        self.add_recent_project(path);

        Ok(())
    }

    /// Close the current project
    pub fn close_project(&mut self) -> anyhow::Result<()> {
        // Save if needed before closing
        if self.has_unsaved_changes {
            if let Some(project) = &self.project {
                if project.meta.shape.auto_save {
                    self.save_annotation()?;
                }
            }
        }

        // Clear all state
        self.project = None;
        self.images.clear();
        self.current_image_index = 0;
        self.current_annotation = None;
        self.current_image = None;
        self.has_unsaved_changes = false;
        self.default_category_id = None;
        self.selected_object_id = None;
        self.clear_drawing_state();
        self.clipboard_objects.clear();
        self.clipboard_rois.clear();
        self.editing_state = EditingState::new();

        log::info!("Project closed");
        Ok(())
    }

    /// Load the current image and its annotation
    pub fn load_current_image(&mut self) -> anyhow::Result<()> {
        if self.images.is_empty() {
            return Ok(());
        }

        let image_path = &self.images[self.current_image_index];

        // Load image
        let img = image::open(image_path)?;
        let rgba = img.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let pixels = rgba.into_raw();

        self.current_image = Some(ImageData {
            path: image_path.clone(),
            width: size[0] as u32,
            height: size[1] as u32,
            pixels,
        });

        // Load annotation if exists
        if let Some(project) = &self.project {
            if let Some(filename) = image_path.file_name().and_then(|s| s.to_str()) {
                self.current_annotation = project.load_annotation(filename)?;

                if self.current_annotation.is_none() {
                    // Create new annotation
                    self.current_annotation = Some(Annotation::new("jlab-gui"));
                }
            }
        }

        self.has_unsaved_changes = false;
        self.selected_object_id = None;
        self.clear_drawing_state();
        Ok(())
    }

    /// Navigate to next image
    pub fn next_image(&mut self) -> anyhow::Result<()> {
        if self.current_image_index + 1 < self.images.len() {
            self.save_if_needed()?;
            self.current_image_index += 1;
            self.load_current_image()?;
        }
        Ok(())
    }

    /// Navigate to previous image
    pub fn prev_image(&mut self) -> anyhow::Result<()> {
        if self.current_image_index > 0 {
            self.save_if_needed()?;
            self.current_image_index -= 1;
            self.load_current_image()?;
        }
        Ok(())
    }

    /// Jump forward by N images
    pub fn jump_forward(&mut self, n: usize) -> anyhow::Result<()> {
        if self.current_image_index + n < self.images.len() {
            self.save_if_needed()?;
            self.current_image_index += n;
            self.load_current_image()?;
        }
        Ok(())
    }

    pub fn clear_drawing_state(&mut self) {
        self.temp_points.clear();
        self.pending_draw_clicks.clear();
    }

    /// Jump backward by N images
    pub fn jump_backward(&mut self, n: usize) -> anyhow::Result<()> {
        if self.current_image_index >= n {
            self.save_if_needed()?;
            self.current_image_index -= n;
            self.load_current_image()?;
        }
        Ok(())
    }

    /// Save current annotation if there are unsaved changes and auto-save is enabled
    pub fn save_if_needed(&mut self) -> anyhow::Result<()> {
        if self.has_unsaved_changes {
            // Check if auto-save is enabled
            let auto_save = self.get_meta().map(|m| m.shape.auto_save).unwrap_or(true); // Default to true

            if auto_save {
                self.save_annotation()?;
            }
        }
        Ok(())
    }

    /// Save current annotation
    pub fn save_annotation(&mut self) -> anyhow::Result<()> {
        if let (Some(project), Some(annotation), Some(image)) =
            (&self.project, &self.current_annotation, &self.current_image)
        {
            if let Some(filename) = image.path.file_name().and_then(|s| s.to_str()) {
                project.save_annotation(filename, annotation)?;
                self.has_unsaved_changes = false;
            }
        }
        Ok(())
    }

    /// Copy selected target or all objects/ROIs to clipboard
    pub fn copy_selected(&mut self) {
        self.clipboard_objects.clear();
        self.clipboard_rois.clear();

        let annotation = match &self.current_annotation {
            Some(annotation) => annotation,
            None => return,
        };

        if let Some(selected_id) = self.selected_object_id {
            if let Some(roi_index) = roi_index_from_id(selected_id) {
                if let Some(roi_points) = annotation.rois.get(roi_index) {
                    self.clipboard_rois.push(roi_points.clone());
                    log::info!("Copied ROI #{} to clipboard", roi_index);
                    return;
                }
            } else if let Some(obj) = annotation.objects.iter().find(|o| o.id == selected_id) {
                self.clipboard_objects.push(obj.clone());
                log::info!("Copied object #{} to clipboard", obj.id);
                return;
            }
        }

        self.clipboard_objects = annotation.objects.clone();
        self.clipboard_rois = annotation.rois.clone();
        log::info!(
            "Copied all objects ({}) and ROIs ({}) to clipboard",
            self.clipboard_objects.len(),
            self.clipboard_rois.len()
        );
    }

    /// Paste objects/ROIs from clipboard
    pub fn paste_from_clipboard(&mut self) {
        let annotation = match &mut self.current_annotation {
            Some(annotation) => annotation,
            None => return,
        };

        if self.clipboard_objects.is_empty() && self.clipboard_rois.is_empty() {
            return;
        }

        let offset = 0.05;
        let mut last_object_id = None;
        if !self.clipboard_objects.is_empty() {
            let mut next_id = annotation.objects.iter().map(|o| o.id).max().unwrap_or(-1) + 1;
            for obj in &self.clipboard_objects {
                let mut new_obj = obj.clone();
                new_obj.id = next_id;
                next_id += 1;
                new_obj.polygon = new_obj
                    .polygon
                    .iter()
                    .map(|p| Point::new((p.x + offset).min(1.0), (p.y + offset).min(1.0)))
                    .collect();
                annotation.objects.push(new_obj);
                last_object_id = Some(next_id - 1);
            }
        }

        let mut last_roi_id = None;
        if !self.clipboard_rois.is_empty() {
            for roi_points in &self.clipboard_rois {
                let new_roi: Vec<Point> = roi_points
                    .iter()
                    .map(|p| Point::new((p.x + offset).min(1.0), (p.y + offset).min(1.0)))
                    .collect();
                annotation.rois.push(new_roi);
                last_roi_id = Some(roi_id_from_index(annotation.rois.len() - 1));
            }
        }

        if let Some(object_id) = last_object_id {
            self.selected_object_id = Some(object_id);
        } else if let Some(roi_id) = last_roi_id {
            self.selected_object_id = Some(roi_id);
        }

        if last_object_id.is_some() || last_roi_id.is_some() {
            self.has_unsaved_changes = true;
        }
    }

    /// Delete selected object
    pub fn delete_selected(&mut self) {
        if let (Some(annotation), Some(selected_id)) =
            (&mut self.current_annotation, self.selected_object_id)
        {
            if let Some(roi_index) = roi_index_from_id(selected_id) {
                if roi_index < annotation.rois.len() {
                    annotation.rois.remove(roi_index);
                    self.selected_object_id = None;
                    self.editing_state.selected_vertex = None;
                    self.has_unsaved_changes = true;
                    log::info!("Deleted ROI #{}", roi_index);
                }
            } else if let Some(pos) = annotation.objects.iter().position(|o| o.id == selected_id) {
                annotation.objects.remove(pos);
                self.selected_object_id = None;
                self.editing_state.selected_vertex = None;
                self.has_unsaved_changes = true;
                log::info!("Deleted object #{}", selected_id);
            }
        }
    }

    /// Get current metadata
    pub fn get_meta(&self) -> Option<&Meta> {
        self.project.as_ref().map(|p| &p.meta)
    }

    /// Add project to recent projects list
    pub fn add_recent_project(&mut self, path: PathBuf) {
        // Remove if already exists
        self.recent_projects.retain(|p| p != &path);
        // Add to front
        self.recent_projects.insert(0, path);
        // Limit to 10
        self.recent_projects.truncate(10);
        // Save to file
        let _ = self.save_recent_projects();
    }

    /// Load recent projects from config file
    pub fn load_recent_projects(&mut self) -> anyhow::Result<()> {
        let config_path = self.get_recent_projects_path()?;
        if !config_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&config_path)?;
        self.recent_projects = serde_json::from_str(&content)?;
        Ok(())
    }

    /// Save recent projects to config file
    pub fn save_recent_projects(&self) -> anyhow::Result<()> {
        let config_path = self.get_recent_projects_path()?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&self.recent_projects)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Get path to recent projects config file
    fn get_recent_projects_path(&self) -> anyhow::Result<PathBuf> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| anyhow::anyhow!("Cannot find home directory"))?;

        let config_dir = PathBuf::from(home).join(".config").join("jlab");
        Ok(config_dir.join("recent_projects.json"))
    }

    /// Set language
    pub fn set_language(&mut self, language: crate::i18n::Language) -> anyhow::Result<()> {
        self.i18n.set_language(language)?;
        self.language = language;
        let _ = self.save_language_setting();
        Ok(())
    }

    /// Load language setting from config file
    fn load_language_setting() -> Option<crate::i18n::Language> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()?;
        let config_path = PathBuf::from(home)
            .join(".config")
            .join("jlab")
            .join("language.json");

        if !config_path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&config_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Save language setting to config file
    fn save_language_setting(&self) -> anyhow::Result<()> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| anyhow::anyhow!("Cannot find home directory"))?;

        let config_dir = PathBuf::from(home).join(".config").join("jlab");
        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("language.json");
        let content = serde_json::to_string_pretty(&self.language)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Load auto-save setting from config file
    fn load_auto_save_setting() -> Option<bool> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()?;
        let config_path = PathBuf::from(home)
            .join(".config")
            .join("jlab")
            .join("auto_save.json");

        if !config_path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&config_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Save auto-save setting to config file
    pub fn save_auto_save_setting(&self) -> anyhow::Result<()> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| anyhow::anyhow!("Cannot find home directory"))?;

        let config_dir = PathBuf::from(home).join(".config").join("jlab");
        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("auto_save.json");
        let content = serde_json::to_string_pretty(&self.auto_save_enabled)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Load theme setting from config file
    fn load_theme_setting() -> Option<ThemeColor> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()?;
        let config_path = PathBuf::from(home)
            .join(".config")
            .join("jlab")
            .join("theme.json");

        if !config_path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&config_path).ok()?;
        let theme_str: String = serde_json::from_str(&content).ok()?;
        ThemeColor::from_str(&theme_str)
    }

    /// Save theme setting to config file
    pub fn save_theme_setting(&self) -> anyhow::Result<()> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| anyhow::anyhow!("Cannot find home directory"))?;

        let config_dir = PathBuf::from(home).join(".config").join("jlab");
        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("theme.json");
        let content = serde_json::to_string_pretty(&self.theme_color.as_str())?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }
}

/// Image data
pub struct ImageData {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Target type for new shapes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawTarget {
    Object,
    Roi,
}

pub fn roi_id_from_index(index: usize) -> i32 {
    -(index as i32) - 1
}

pub fn roi_index_from_id(id: i32) -> Option<usize> {
    if id < 0 {
        Some((-id - 1) as usize)
    } else {
        None
    }
}
