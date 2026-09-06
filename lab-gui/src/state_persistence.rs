//! Config persistence for [`AppState`]: recent projects, language, auto-save,
//! theme, and UI settings (stored as JSON under `~/.config/jlab/`).
//!
//! Extracted from `state.rs` as a pure move (impl block split) to keep that
//! file under the length limit. The module is declared with `#[path]` inside
//! `state.rs`, so it is a child of `state` even though it sits at `src/` level.

use crate::state::{AppState, ThemeColor};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

impl AppState {
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
    pub(crate) fn load_language_setting() -> Option<crate::i18n::Language> {
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).ok()?;
        let config_path = PathBuf::from(home).join(".config").join("jlab").join("language.json");

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
    pub(crate) fn load_auto_save_setting() -> Option<bool> {
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).ok()?;
        let config_path = PathBuf::from(home).join(".config").join("jlab").join("auto_save.json");

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
    pub(crate) fn load_theme_setting() -> Option<ThemeColor> {
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).ok()?;
        let config_path = PathBuf::from(home).join(".config").join("jlab").join("theme.json");

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

    /// Load UI settings from config file
    pub(crate) fn load_ui_settings() -> (Option<f32>, Option<f32>, Option<bool>) {
        let home = match std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            Ok(h) => h,
            Err(_) => return (None, None, None),
        };
        let config_path = PathBuf::from(home).join(".config").join("jlab").join("ui_settings.json");

        if !config_path.exists() {
            return (None, None, None);
        }

        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(_) => return (None, None, None),
        };

        #[derive(Deserialize)]
        struct UISettings {
            font_size: Option<f32>,
            ui_scale: Option<f32>,
            show_scrollbar: Option<bool>,
        }

        let settings: UISettings = match serde_json::from_str(&content) {
            Ok(s) => s,
            Err(_) => return (None, None, None),
        };

        (settings.font_size, settings.ui_scale, settings.show_scrollbar)
    }

    /// Save UI settings to config file
    pub fn save_ui_settings(&self) -> anyhow::Result<()> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| anyhow::anyhow!("Cannot find home directory"))?;

        let config_dir = PathBuf::from(home).join(".config").join("jlab");
        std::fs::create_dir_all(&config_dir)?;

        #[derive(Serialize)]
        struct UISettings {
            font_size: f32,
            ui_scale: f32,
            show_scrollbar: bool,
        }

        let settings = UISettings {
            font_size: self.font_size,
            ui_scale: self.ui_scale,
            show_scrollbar: self.show_scrollbar,
        };

        let config_path = config_dir.join("ui_settings.json");
        let content = serde_json::to_string_pretty(&settings)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // AppState project-lifecycle integration tests. They exercise state.rs
    // logic but live here to keep state.rs under the file-length limit.
    use crate::state::AppState;
    use lab_core::{CatDef, LabelMeta, Point, Polygon, RoiConfig, ShapeConfig};
    use std::fs;
    use std::path::PathBuf;

    fn poly(x: f32) -> Polygon<f32> {
        Polygon::from(vec![Point { x, y: 0.1 }, Point { x: x + 0.1, y: 0.2 }, Point { x, y: 0.3 }])
    }

    /// Create a temp project (meta.yaml + optionally one real PNG) usable by
    /// load_project. `image_name: None` creates a project with empty images/.
    fn temp_project(tag: &str, image_name: Option<&str>) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("lab-gui-state-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("images")).unwrap();
        let meta = LabelMeta {
            id: 1,
            name: "test".to_string(),
            description: String::new(),
            shape: ShapeConfig {
                title_style: 0,
                thickness: 2,
                auto_save: false,
                vertex_radius: 10.0,
            },
            roi: RoiConfig { color: "#800080".to_string() },
            categories: vec![CatDef {
                id: 0,
                name: "person".to_string(),
                description: String::new(),
                hotkey: "1".to_string(),
                color: "#FF0000".to_string(),
                properties: vec![],
            }],
            property_types: vec![],
            property_special_values: vec![],
        };
        lab_core::io::save_meta(root.join("meta.yaml"), &meta).unwrap();
        if let Some(image_name) = image_name {
            image::RgbaImage::from_pixel(4, 4, image::Rgba([8, 8, 8, 255]))
                .save(root.join("images").join(image_name))
                .unwrap();
        }
        root
    }

    /// Point HOME at a temp dir so config writes during tests don't touch the
    /// real user config (recent_projects.json etc.).
    fn redirect_home(tag: &str) -> PathBuf {
        let home =
            std::env::temp_dir().join(format!("lab-gui-home-{}-{}", tag, std::process::id()));
        fs::create_dir_all(&home).unwrap();
        std::env::set_var("HOME", &home);
        home
    }

    #[test]
    fn load_project_resets_per_project_state() {
        let home = redirect_home("f1");
        let mut state = AppState::new();
        state.locked_rois = Some(vec![poly(0.5)]);
        state.clipboard_objects = vec![lab_core::new_object(0, 0, poly(0.1))];
        state.clipboard_rois = vec![poly(0.7)];
        state.editing_state.selected_vertex = Some((1, 2));

        state.load_project(temp_project("f1", Some("a.png"))).unwrap();

        assert!(state.locked_rois.is_none());
        assert!(state.clipboard_objects.is_empty());
        assert!(state.clipboard_rois.is_empty());
        assert!(state.editing_state.selected_vertex.is_none());
        // Project B's fresh annotation must not inherit project A's ROIs.
        assert!(state.current_annotation.as_ref().unwrap().rois.is_empty());
        assert!(!state.has_unsaved_changes);
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn delete_last_sample_clears_editing_selection() {
        let home = redirect_home("f6");
        let mut state = AppState::new();
        state.load_project(temp_project("f6", Some("only.png"))).unwrap();
        state.selected_object_id = Some(0);
        state.editing_state.selected_vertex = Some((0, 1));

        state.delete_current_sample().unwrap();

        assert!(state.images.is_empty());
        assert!(state.selected_object_id.is_none());
        assert!(state.editing_state.selected_vertex.is_none());
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn load_project_failure_keeps_session_state() {
        let home = redirect_home("f7");
        let mut state = AppState::new();
        state.locked_rois = Some(vec![poly(0.5)]);
        state.clipboard_rois = vec![poly(0.7)];
        state.clipboard_objects = vec![lab_core::new_object(0, 0, poly(0.2))];

        // Directory without meta.yaml: Project::open fails, the user stays in
        // the previously opened project.
        let bad = temp_project("f7-bad", None).join("no-meta");
        fs::create_dir_all(&bad).unwrap();
        assert!(state.load_project(bad).is_err());

        // The failed open must not have wiped the live session's state.
        assert_eq!(state.locked_rois.as_deref(), Some(&[poly(0.5)][..]));
        assert_eq!(state.clipboard_rois.len(), 1);
        assert_eq!(state.clipboard_objects.len(), 1);
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn load_project_empty_images_clears_previous_sample() {
        let home = redirect_home("f8");
        let root_a = temp_project("f8a", Some("a.png"));
        let mut state = AppState::new();
        state.load_project(root_a.clone()).unwrap();
        assert!(state.current_image.is_some());
        state.selected_object_id = Some(3);
        state.has_unsaved_changes = true;

        // Project B has meta.yaml but an empty images/ dir: nothing from
        // project A's loaded sample may survive the switch.
        state.load_project(temp_project("f8b", None)).unwrap();

        assert!(state.images.is_empty());
        assert!(state.current_image.is_none());
        assert!(state.current_annotation.is_none());
        assert!(state.selected_object_id.is_none());
        assert!(!state.has_unsaved_changes);
        let _ = fs::remove_dir_all(&home);
        let _ = fs::remove_dir_all(&root_a);
    }

    #[test]
    fn delete_current_sample_corrupt_successor_is_ok_and_does_not_resurrect() {
        let home = redirect_home("f9");
        let root = temp_project("f9", Some("a.png"));
        // b.png is listed but undecodable: it becomes the successor after
        // deleting a.png (list_images sorts, so a.png is loaded first).
        fs::write(root.join("images/b.png"), b"not a real image").unwrap();
        let mut state = AppState::new();
        state.load_project(root.clone()).unwrap();
        assert_eq!(state.current_image.as_ref().unwrap().path, root.join("images/a.png"));
        // Unsaved edits made before the delete must never be written back.
        state.has_unsaved_changes = true;

        // The delete itself succeeded, so it must not be reported as failure.
        state.delete_current_sample().unwrap();

        assert!(!root.join("images/a.png").exists());
        assert!(state.current_image.is_none(), "no state may reference the deleted sample");
        assert!(state.current_annotation.is_none());
        assert!(!state.has_unsaved_changes);
        // A later auto-save must not resurrect the deleted sample's annotation.
        state.save_if_needed().unwrap();
        assert!(!root.join("labels/a.yaml").exists());
        let _ = fs::remove_dir_all(&home);
        let _ = fs::remove_dir_all(&root);
    }
}
