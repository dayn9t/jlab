use crate::canvas::Canvas;
use crate::state::AppState;
use egui::Context;
use std::path::PathBuf;

mod about_dialog;
mod canvas_view;
mod delete_confirm_dialog;
mod editing_actions;
mod import_export;
mod menu;
mod options_dialog;
mod panels;
mod shortcut_settings;
mod status_bar;
mod toolbar;

use about_dialog::AboutDialogState;
use options_dialog::{DialogButtonAction, OptionsDialogState};

const ZOOM_LEVELS: [f32; 9] = [25.0, 50.0, 75.0, 100.0, 125.0, 150.0, 200.0, 300.0, 400.0];

pub struct LabApp {
    state: AppState,
    canvas: Canvas,
    shortcut_editor: Option<crate::shortcuts::ShortcutEditorState>,
    options_dialog: OptionsDialogState,
    about_dialog: AboutDialogState,
    // UI settings
    ui_scale: f32,
    pixels_per_point: f32,
}

impl LabApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Create state first to get saved settings
        let state = AppState::new();
        let font_size = state.font_size;
        let ui_scale = state.ui_scale;

        // Configure fonts for Chinese support
        let mut fonts = egui::FontDefinitions::default();

        // Load Noto Sans CJK font
        fonts.font_data.insert(
            "noto_sans_cjk".to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                "../assets/NotoSansCJK-Regular.ttc"
            ))),
        );

        // Put Noto Sans CJK first in the proportional font family
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "noto_sans_cjk".to_owned());

        // Put Noto Sans CJK first in the monospace font family
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "noto_sans_cjk".to_owned());

        cc.egui_ctx.set_fonts(fonts);

        // Set initial pixels_per_point based on font size
        // Base font size is 14.0, so scale = font_size / 14.0
        let pixels_per_point = (font_size / 14.0) * ui_scale;
        cc.egui_ctx.set_pixels_per_point(pixels_per_point);

        Self {
            state,
            canvas: Canvas::new(),
            shortcut_editor: None,
            options_dialog: OptionsDialogState::new(crate::i18n::Language::ZhCN, false),
            about_dialog: AboutDialogState::new(),
            ui_scale,
            pixels_per_point,
        }
    }

    /// Load project from path (for command-line auto-open)
    pub fn load_project_from_path(&mut self, path: PathBuf) -> anyhow::Result<()> {
        self.state.load_project(path)?;
        self.canvas.reset_view();
        Ok(())
    }
}

impl eframe::App for LabApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Apply theme
        self.apply_theme(ctx);
        // Apply UI scale (font size and zoom)
        self.apply_ui_scale(ctx);

        let mut status_hint: Option<String> = None;
        let mut cursor_pixel_pos: Option<(i32, i32)> = None;

        self.show_top_menu(ctx, &mut status_hint);
        self.show_toolbar(ctx, &mut status_hint);
        self.show_left_panel(ctx);
        self.show_right_panel(ctx);
        self.show_canvas(ctx, &mut cursor_pixel_pos);
        self.show_status_bar(ctx, &status_hint, cursor_pixel_pos);

        // Handle keyboard shortcuts
        self.handle_shortcuts(ctx);

        // Show dialogs
        self.show_shortcut_settings(ctx);
        self.show_options_dialog(ctx);
        self.show_about_dialog(ctx);
        self.show_delete_confirm_dialog(ctx);
    }
}

impl LabApp {
    /// Helper method to format menu text with shortcut
    fn menu_text(&self, text: String, action: crate::shortcuts::ShortcutAction) -> String {
        if let Some(shortcut) = self.state.shortcut_manager.format_shortcut(action) {
            format!("{}\t{}", text, shortcut)
        } else {
            text
        }
    }

    fn hint_with_shortcut(&self, hint: String, action: crate::shortcuts::ShortcutAction) -> String {
        if let Some(shortcut) = self.state.shortcut_manager.format_shortcut(action) {
            format!("{} ({}: {})", hint, self.state.i18n.t("hint.shortcut"), shortcut)
        } else {
            hint
        }
    }

    fn set_zoom(&mut self, zoom_percent: f32) {
        self.canvas.zoom = (zoom_percent / 100.0).clamp(0.1, 10.0);
    }

    fn update_status_hint(hint: &mut Option<String>, response: &egui::Response, text: String) {
        if response.hovered() {
            *hint = Some(text);
        }
    }

    fn status_add_label(ui: &mut egui::Ui, first: &mut bool, text: String) {
        if !*first {
            ui.separator();
        }
        *first = false;
        ui.label(text);
    }

    fn status_add_colored(ui: &mut egui::Ui, first: &mut bool, color: egui::Color32, text: String) {
        if !*first {
            ui.separator();
        }
        *first = false;
        ui.colored_label(color, text);
    }

    fn show_options_dialog(&mut self, ctx: &egui::Context) {
        // Show dialog only if it was just triggered
        if self.state.show_options_dialog && !self.options_dialog.show {
            // Refresh current settings before showing dialog
            self.options_dialog.refresh_from_state(
                self.state.language,
                self.state.auto_save_enabled,
                self.state.theme_color,
                self.state.font_size,
                self.state.ui_scale,
            );
            self.options_dialog.show_dialog(self.state.shortcut_manager.get_config());
        }

        let (_show, open, button_action) = self.options_dialog.show(ctx, &self.state.i18n);

        if let Some(action) = button_action {
            match action {
                DialogButtonAction::Apply => {
                    // Apply settings and keep dialog open
                    self.apply_settings();
                }
                DialogButtonAction::Ok => {
                    // Apply settings and close dialog
                    self.apply_settings();
                    self.state.show_options_dialog = false;
                    self.options_dialog.shortcut_editor = None;
                }
                DialogButtonAction::Cancel => {
                    // Discard changes and close dialog
                    self.state.show_options_dialog = false;
                    self.options_dialog.shortcut_editor = None;
                }
                DialogButtonAction::RestoreDefaults => {
                    // Restore default settings and keep dialog open
                    self.options_dialog.restore_defaults();
                }
            }
        }

        if !open {
            self.state.show_options_dialog = false;
            self.options_dialog.shortcut_editor = None;
        }

        // Sync state with dialog's actual visibility
        if !self.options_dialog.show {
            self.state.show_options_dialog = false;
            self.options_dialog.shortcut_editor = None;
        }
    }

    fn apply_settings(&mut self) {
        let settings = self.options_dialog.get_current_settings();

        // Apply language change
        if settings.language != self.state.language {
            let _ = self.state.set_language(settings.language);
        }

        // Apply global auto-save change
        self.state.auto_save_enabled = settings.auto_save;
        if let Err(e) = self.state.save_auto_save_setting() {
            log::error!("Failed to save auto-save setting: {}", e);
        }

        // Apply auto-save to current project
        if let Some(project) = &mut self.state.project {
            project.meta.shape.auto_save = settings.auto_save;
        }

        // Apply theme change
        self.state.theme_color = settings.theme_color;
        if let Err(e) = self.state.save_theme_setting() {
            log::error!("Failed to save theme setting: {}", e);
        }

        // Apply font size and UI scale to state
        self.state.font_size = settings.font_size;
        self.state.ui_scale = settings.ui_scale;
        self.state.show_scrollbar = settings.show_scrollbar;

        // Apply to app
        self.ui_scale = settings.ui_scale;
        // pixels_per_point = (font_size / 14.0) * ui_scale
        self.pixels_per_point = (settings.font_size / 14.0) * settings.ui_scale;

        // Save UI settings
        if let Err(e) = self.state.save_ui_settings() {
            log::error!("Failed to save UI settings: {}", e);
        }

        // Apply shortcut changes
        if let Some(config) = settings.shortcut_config {
            self.state.shortcut_manager.apply_config(config);
            if let Some(path) = crate::shortcuts::ShortcutManager::user_config_path() {
                let _ = self.state.shortcut_manager.save_to_file(&path);
            }
        }

        log::info!(
            "Options applied: auto_save={}, font_size={}, ui_scale={}, theme={:?}, scrollbar={}",
            settings.auto_save,
            settings.font_size,
            settings.ui_scale,
            settings.theme_color,
            settings.show_scrollbar
        );
    }

    fn apply_ui_scale(&self, ctx: &egui::Context) {
        ctx.set_pixels_per_point(self.pixels_per_point);
    }

    fn show_about_dialog(&mut self, ctx: &egui::Context) {
        // Show dialog only if it was just triggered
        if self.state.show_about_dialog && !self.about_dialog.show {
            self.about_dialog.show_dialog();
        }
        self.about_dialog.show(ctx, &self.state.i18n);

        // Sync state with dialog's actual visibility
        if !self.about_dialog.show {
            self.state.show_about_dialog = false;
        }
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        match self.state.theme_color {
            crate::state::ThemeColor::Light => {
                ctx.set_visuals(egui::Visuals::light());
            }
            crate::state::ThemeColor::Dark => {
                ctx.set_visuals(egui::Visuals::dark());
            }
            crate::state::ThemeColor::System => {
                // Detect system theme preference
                #[cfg(target_os = "linux")]
                {
                    // On Linux, try to read from gsettings or use dark as default
                    let is_dark = std::process::Command::new("gsettings")
                        .args(&["get", "org.gnome.desktop.interface", "gtk-theme"])
                        .output()
                        .ok()
                        .map(|output| {
                            let theme = String::from_utf8_lossy(&output.stdout);
                            theme.to_lowercase().contains("dark")
                        })
                        .unwrap_or(false);

                    if is_dark {
                        ctx.set_visuals(egui::Visuals::dark());
                    } else {
                        ctx.set_visuals(egui::Visuals::light());
                    }
                }
                #[cfg(not(target_os = "linux"))]
                {
                    // On other platforms, default to dark theme
                    ctx.set_visuals(egui::Visuals::dark());
                }
            }
        }
    }
}
