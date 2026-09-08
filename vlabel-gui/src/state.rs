use crate::shortcuts::ShortcutManager;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use vlabel_core::{Label, LabelMeta, Object, Point, Polygon};
use vlabel_utils::Project;

// Config persistence (recent projects, language, auto-save, theme, UI
// settings) lives in its own module; declared here because `main.rs` maps
// modules at `src/` level and this keeps the module tree unchanged.
#[path = "state_persistence.rs"]
mod state_persistence;

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
        Self { mode: EditMode::Browse, selected_vertex: None }
    }
}

pub struct PendingDrawClick {
    pub position: Point<f32>,
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
    pub current_annotation: Option<Label>,

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
    pub temp_points: Vec<Point<f32>>,

    /// Pending single-clicks in Drawing mode (delayed for double-click detection)
    pub pending_draw_clicks: Vec<PendingDrawClick>,

    /// Clipboard objects for copy/paste operations
    pub clipboard_objects: Vec<Object>,

    /// Clipboard ROIs for copy/paste operations
    pub clipboard_rois: Vec<Polygon<f32>>,

    /// Locked ROIs applied to every image on switch (None = not locked)
    pub locked_rois: Option<Vec<Polygon<f32>>>,

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

    /// Show the delete-sample confirmation dialog
    pub show_delete_confirm: bool,

    /// Global auto-save setting
    pub auto_save_enabled: bool,

    /// Theme setting
    pub theme_color: ThemeColor,

    /// Show left sidebar
    pub show_left_panel: bool,

    /// Show right sidebar
    pub show_right_panel: bool,

    /// Font size setting
    pub font_size: f32,

    /// UI scale setting
    pub ui_scale: f32,

    /// Show scrollbar setting
    pub show_scrollbar: bool,
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
        let theme_color = saved_theme.unwrap_or(ThemeColor::Dark);

        // Try to load saved UI settings
        let saved_ui_settings = Self::load_ui_settings();
        let font_size = saved_ui_settings.0.unwrap_or(16.0);
        let ui_scale = saved_ui_settings.1.unwrap_or(1.0);
        let show_scrollbar = saved_ui_settings.2.unwrap_or(true);

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
            locked_rois: None,
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
            show_delete_confirm: false,
            auto_save_enabled,
            theme_color,
            show_left_panel: true,
            show_right_panel: true,
            font_size,
            ui_scale,
            show_scrollbar,
        };
        // Load recent projects from file
        let _ = state.load_recent_projects();
        state
    }

    /// Load a project
    pub fn load_project(&mut self, path: PathBuf) -> anyhow::Result<()> {
        // Validate before touching any state: a failed open must leave the
        // current session (project, locked ROIs, clipboard) exactly as it was.
        let mut project = Project::open(&path)?;
        let images = project.list_images()?;

        // Same save-on-exit contract as close_project: keep pending edits of
        // the project being left behind.
        self.save_if_needed()?;

        // Reset per-project state so nothing leaks from the previous project:
        // locked ROIs would be applied — and auto-saved — into the new
        // project's images, and a stale current_image/current_annotation
        // could save project A's annotation inside project B. Cleared
        // unconditionally (not only via load_current_image) so a new project
        // with an empty images/ dir cannot keep the previous sample alive.
        self.locked_rois = None;
        self.clipboard_objects.clear();
        self.clipboard_rois.clear();
        self.current_image = None;
        self.current_annotation = None;
        self.selected_object_id = None;
        self.editing_state.selected_vertex = None;
        self.has_unsaved_changes = false;
        self.clear_drawing_state();

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
        self.default_category_id =
            self.project.as_ref().and_then(|p| p.meta.categories.first().map(|c| c.id));
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
        self.locked_rois = None;
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
                    self.current_annotation = Some(vlabel_core::new_label("vlabel"));
                }
            }
        }

        self.has_unsaved_changes = false;
        self.selected_object_id = None;
        // A stale selected vertex would let arrow keys mutate the next image.
        self.editing_state.selected_vertex = None;
        self.clear_drawing_state();

        // Apply locked ROIs: replace this image's ROIs (objects are kept)
        if let Some(locked) = &self.locked_rois {
            if let Some(label) = &mut self.current_annotation {
                if apply_locked_rois(label, locked) {
                    self.has_unsaved_changes = true;
                }
            }
        }
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
        if let (Some(project), Some(label), Some(image)) =
            (&self.project, &self.current_annotation, &self.current_image)
        {
            if let Some(filename) = image.path.file_name().and_then(|s| s.to_str()) {
                project.save_annotation(filename, label)?;
                self.has_unsaved_changes = false;
            }
        }
        Ok(())
    }

    /// Copy selected target or all objects/ROIs to clipboard
    pub fn copy_selected(&mut self) {
        self.clipboard_objects.clear();
        self.clipboard_rois.clear();

        let label = match &self.current_annotation {
            Some(label) => label,
            None => return,
        };

        if let Some(selected_id) = self.selected_object_id {
            if let Some(roi_index) = roi_index_from_id(selected_id) {
                if let Some(roi) = label.rois.get(roi_index) {
                    self.clipboard_rois.push(roi.clone());
                    log::info!("Copied ROI #{} to clipboard", roi_index);
                    return;
                }
            } else if let Some(obj) = label.objects.iter().find(|o| o.id == selected_id) {
                self.clipboard_objects.push(obj.clone());
                log::info!("Copied object #{} to clipboard", obj.id);
                return;
            }
        }

        self.clipboard_objects = label.objects.clone();
        self.clipboard_rois = label.rois.clone();
        log::info!(
            "Copied all objects ({}) and ROIs ({}) to clipboard",
            self.clipboard_objects.len(),
            self.clipboard_rois.len()
        );
    }

    /// Paste objects/ROIs from clipboard
    pub fn paste_from_clipboard(&mut self) {
        let label = match &mut self.current_annotation {
            Some(label) => label,
            None => return,
        };

        if self.clipboard_objects.is_empty() && self.clipboard_rois.is_empty() {
            return;
        }

        let offset = 0.05;
        let mut last_object_id = None;
        if !self.clipboard_objects.is_empty() {
            let mut next_id = label.objects.iter().map(|o| o.id).max().unwrap_or(-1) + 1;
            for obj in &self.clipboard_objects {
                let mut new_obj = obj.clone();
                new_obj.id = next_id;
                next_id += 1;
                new_obj.polygon = Polygon::from(
                    new_obj
                        .polygon
                        .0
                        .iter()
                        .map(|p| Point { x: (p.x + offset).min(1.0), y: (p.y + offset).min(1.0) })
                        .collect::<Vec<_>>(),
                );
                label.objects.push(new_obj);
                last_object_id = Some(next_id - 1);
            }
        }

        let mut last_roi_id = None;
        if !self.clipboard_rois.is_empty() {
            for roi in &self.clipboard_rois {
                let new_roi = Polygon::from(
                    roi.0
                        .iter()
                        .map(|p| Point { x: (p.x + offset).min(1.0), y: (p.y + offset).min(1.0) })
                        .collect::<Vec<_>>(),
                );
                label.rois.push(new_roi);
                last_roi_id = Some(roi_id_from_index(label.rois.len() - 1));
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

    /// Lock current image's ROIs (selected ROI preferred, otherwise all).
    /// While locked, every image switch replaces the target image's ROIs.
    pub fn lock_rois(&mut self) {
        let rois = match &self.current_annotation {
            Some(label) => rois_to_lock(label, self.selected_object_id),
            None => Vec::new(),
        };
        if rois.is_empty() {
            log::warn!("Lock ROI: current image has no ROIs to lock");
            self.locked_rois = None;
            return;
        }
        log::info!("Locked {} ROI(s)", rois.len());
        self.locked_rois = Some(rois);
    }

    /// Unlock ROIs and stop applying them on image switch
    pub fn unlock_rois(&mut self) {
        if self.locked_rois.take().is_some() {
            log::info!("Unlocked ROIs");
        }
    }

    /// Drop `path` from the cached image list so state never points at a file
    /// already deleted on disk; clamps the index and clears the loaded image.
    /// Used when a sample was deleted but the list refresh failed.
    fn forget_image(&mut self, path: &Path) {
        self.images.retain(|p| p != path);
        if self.current_image_index >= self.images.len() {
            self.current_image_index = self.images.len().saturating_sub(1);
        }
        if self.current_image.as_ref().is_some_and(|img| img.path == path) {
            self.current_image = None;
            self.current_annotation = None;
            self.selected_object_id = None;
            self.editing_state.selected_vertex = None;
            self.has_unsaved_changes = false;
        }
    }

    /// Delete the current sample: image file + annotation file, then load
    /// the image that now sits at the same index (or clear state if none left).
    ///
    /// `Err` means the delete itself failed. Failures after a successful
    /// delete (list refresh, loading the successor) are logged loudly and
    /// leave clean state instead: reporting them as a failed delete would be
    /// false — the destructive operation already happened.
    pub fn delete_current_sample(&mut self) -> anyhow::Result<()> {
        let Some(image_path) = self.images.get(self.current_image_index).cloned() else {
            return Ok(());
        };
        let Some(project) = self.project.as_ref() else {
            return Ok(());
        };

        project.delete_sample(&image_path)?;
        let images = match project.list_images() {
            Ok(images) => images,
            Err(e) => {
                log::error!(
                    "Deleted {:?} but refreshing the image list failed: {:#}",
                    image_path,
                    anyhow::Error::from(e)
                );
                // The sample is gone from disk; don't leave state pointing
                // at the deleted file — drop it so a later refresh can recover.
                self.forget_image(&image_path);
                return Ok(());
            }
        };
        self.images = images;
        if self.current_image_index >= self.images.len() {
            self.current_image_index = self.images.len().saturating_sub(1);
        }
        // Drop loaded state pointing at the deleted file BEFORE loading the
        // successor: if that load fails, current_image/current_annotation
        // must not keep referencing the deleted sample — save_if_needed on
        // the next navigation would resurrect vlabels/<deleted-stem>.json5.
        self.forget_image(&image_path);

        if self.images.is_empty() {
            self.current_image = None;
            self.current_annotation = None;
            self.has_unsaved_changes = false;
            self.selected_object_id = None;
            self.editing_state.selected_vertex = None;
            self.clear_drawing_state();
        } else if let Err(e) = self.load_current_image() {
            log::error!("Deleted {:?} but loading the next image failed: {:#}", image_path, e);
        }
        log::info!("Deleted sample {:?}", image_path);
        Ok(())
    }

    /// Delete selected object
    pub fn delete_selected(&mut self) {
        if let (Some(label), Some(selected_id)) =
            (&mut self.current_annotation, self.selected_object_id)
        {
            if let Some(roi_index) = roi_index_from_id(selected_id) {
                if roi_index < label.rois.len() {
                    label.rois.remove(roi_index);
                    self.selected_object_id = None;
                    self.editing_state.selected_vertex = None;
                    self.has_unsaved_changes = true;
                    log::info!("Deleted ROI #{}", roi_index);
                }
            } else if let Some(pos) = label.objects.iter().position(|o| o.id == selected_id) {
                label.objects.remove(pos);
                self.selected_object_id = None;
                self.editing_state.selected_vertex = None;
                self.has_unsaved_changes = true;
                log::info!("Deleted object #{}", selected_id);
            }
        }
    }

    /// Get current metadata
    pub fn get_meta(&self) -> Option<&LabelMeta> {
        self.project.as_ref().map(|p| &p.meta)
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

/// Pick the ROIs to lock from a label: the selected ROI if one is selected,
/// otherwise all ROIs. (A selected object falls back to all ROIs — locking is
/// about ROIs, not objects.)
pub(crate) fn rois_to_lock(label: &Label, selected_id: Option<i32>) -> Vec<Polygon<f32>> {
    if let Some(id) = selected_id {
        if let Some(index) = roi_index_from_id(id) {
            if let Some(roi) = label.rois.get(index) {
                return vec![roi.clone()];
            }
        }
    }
    label.rois.clone()
}

/// Replace a label's ROIs with the locked ROIs. Returns whether anything changed.
pub(crate) fn apply_locked_rois(label: &mut Label, locked: &[Polygon<f32>]) -> bool {
    if label.rois.as_slice() == locked {
        return false;
    }
    label.rois = locked.to_vec();
    vlabel_core::touch(label);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poly(x: f32) -> Polygon<f32> {
        Polygon::from(vec![Point { x, y: 0.1 }, Point { x: x + 0.1, y: 0.2 }, Point { x, y: 0.3 }])
    }

    fn label_with_rois(n: usize) -> Label {
        let mut label = vlabel_core::new_label("test");
        for i in 0..n {
            label.rois.push(poly(0.1 * i as f32));
        }
        label
    }

    #[test]
    fn rois_to_lock_selected_roi_returns_only_that_roi() {
        let label = label_with_rois(3);
        // ROI ids are negative: -(index) - 1
        let selected = roi_id_from_index(1);
        let locked = rois_to_lock(&label, Some(selected));
        assert_eq!(locked, vec![label.rois[1].clone()]);
    }

    #[test]
    fn rois_to_lock_no_selection_returns_all_rois() {
        let label = label_with_rois(3);
        let locked = rois_to_lock(&label, None);
        assert_eq!(locked, label.rois);
    }

    #[test]
    fn rois_to_lock_selected_object_falls_back_to_all_rois() {
        let label = label_with_rois(2);
        // Object ids are non-negative, roi_index_from_id returns None
        let locked = rois_to_lock(&label, Some(5));
        assert_eq!(locked, label.rois);
    }

    #[test]
    fn apply_locked_rois_replaces_and_reports_change() {
        let mut label = label_with_rois(1);
        let old_modified = label.last_modified;

        let locked = vec![poly(0.9)];
        assert!(apply_locked_rois(&mut label, &locked));
        assert_eq!(label.rois, locked);
        assert!(label.last_modified >= old_modified);
    }

    #[test]
    fn apply_locked_rois_identical_is_noop() {
        let mut label = label_with_rois(2);
        let locked = label.rois.clone();

        assert!(!apply_locked_rois(&mut label, &locked));
        assert_eq!(label.rois, locked);
    }

    #[test]
    fn apply_locked_rois_empty_clears_existing() {
        let mut label = label_with_rois(2);
        assert!(apply_locked_rois(&mut label, &[]));
        assert!(label.rois.is_empty());
    }

    #[test]
    fn forget_image_drops_path_clamps_index_and_clears_current() {
        let mut state = AppState::new();
        state.images = vec![PathBuf::from("/t/a.png"), PathBuf::from("/t/b.png")];
        state.current_image_index = 1;
        state.current_image = Some(ImageData {
            path: PathBuf::from("/t/b.png"),
            width: 1,
            height: 1,
            pixels: vec![],
        });
        state.current_annotation = Some(vlabel_core::new_label("test"));

        state.forget_image(&PathBuf::from("/t/b.png"));

        assert_eq!(state.images, vec![PathBuf::from("/t/a.png")]);
        assert_eq!(state.current_image_index, 0);
        assert!(state.current_image.is_none());
        assert!(state.current_annotation.is_none());
    }
}
