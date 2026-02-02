use crate::canvas::Canvas;
use crate::state::AppState;
use crate::tools::DrawingTools;
use egui::{Context, Vec2};
use std::path::PathBuf;

mod about_dialog;
mod canvas_view;
mod import_export;
mod menu;
mod options_dialog;
mod panels;
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

    fn with_polygon_mut<F>(annotation: &mut lab_core::Annotation, shape_id: i32, mutator: F) -> bool
    where
        F: FnOnce(&mut Vec<lab_core::Point>),
    {
        if let Some(roi_index) = crate::state::roi_index_from_id(shape_id) {
            if let Some(roi_points) = annotation.rois.get_mut(roi_index) {
                mutator(roi_points);
                return true;
            }
        } else if let Some(obj) = annotation.objects.iter_mut().find(|o| o.id == shape_id) {
            mutator(&mut obj.polygon);
            return true;
        }

        false
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
            format!(
                "{} ({}: {})",
                hint,
                self.state.i18n.t("hint.shortcut"),
                shortcut
            )
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

    fn show_shortcut_settings(&mut self, ctx: &egui::Context) {
        if !self.state.show_shortcut_settings {
            return;
        }

        if self.shortcut_editor.is_none() {
            self.shortcut_editor = Some(crate::shortcuts::ShortcutEditorState::new(
                self.state.shortcut_manager.get_config(),
            ));
        }

        let mut open = true;
        let mut should_close = false;
        let mut should_save = false;
        let mut new_config = None;

        // Take editor out temporarily to avoid borrow checker issues
        let mut editor = self.shortcut_editor.take();

        egui::Window::new(self.state.i18n.t("shortcuts.title"))
            .open(&mut open)
            .default_size([800.0, 600.0])
            .resizable(true)
            .show(ctx, |ui| {
                if let Some(ref mut ed) = editor {
                    let result = self.render_shortcut_editor_content(ui, ed, ctx);
                    should_close = result.0;
                    should_save = result.1;
                    new_config = result.2;
                }
            });

        // Put editor back
        self.shortcut_editor = editor;

        if should_save {
            if let Some(config) = new_config {
                self.state.shortcut_manager.apply_config(config);
                if let Some(path) = crate::shortcuts::ShortcutManager::user_config_path() {
                    if let Err(e) = self.state.shortcut_manager.save_to_file(&path) {
                        log::error!("Failed to save shortcuts: {}", e);
                    }
                }
            }
        }

        if !open || should_close {
            self.state.show_shortcut_settings = false;
            self.shortcut_editor = None;
        }
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
                self.state.ui_scale
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

        log::info!("Options applied: auto_save={}, font_size={}, ui_scale={}, theme={:?}, scrollbar={}",
            settings.auto_save, settings.font_size, settings.ui_scale, settings.theme_color, settings.show_scrollbar);
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

    fn render_shortcut_editor_content(
        &mut self,
        ui: &mut egui::Ui,
        editor: &mut crate::shortcuts::ShortcutEditorState,
        ctx: &egui::Context,
    ) -> (bool, bool, Option<crate::shortcuts::ShortcutConfig>) {
        let mut should_close = false;
        let mut should_save = false;
        let mut new_config = None;

        // Top: search and filter
        ui.horizontal(|ui| {
            ui.label(self.state.i18n.t("shortcuts.search"));
            ui.text_edit_singleline(&mut editor.search_filter);

            ui.separator();

            ui.label(self.state.i18n.t("shortcuts.category"));
            egui::ComboBox::from_id_salt("category_filter")
                .selected_text(
                    editor
                        .category_filter
                        .map(|c| self.state.i18n.t(c.name_key()))
                        .unwrap_or_else(|| self.state.i18n.t("shortcuts.all_categories")),
                )
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            editor.category_filter.is_none(),
                            self.state.i18n.t("shortcuts.all_categories"),
                        )
                        .clicked()
                    {
                        editor.category_filter = None;
                    }

                    for cat in crate::shortcuts::ShortcutCategory::all_categories() {
                        if ui
                            .selectable_label(
                                editor.category_filter == Some(cat),
                                self.state.i18n.t(cat.name_key()),
                            )
                            .clicked()
                        {
                            editor.category_filter = Some(cat);
                        }
                    }
                });
        });

        ui.separator();

        // Conflict warning
        if editor.conflict_count > 0 {
            ui.colored_label(
                egui::Color32::from_rgb(255, 165, 0),
                format!(
                    "⚠ {}: {}",
                    self.state.i18n.t("shortcuts.conflicts_detected"),
                    editor.conflict_count
                ),
            );
            ui.separator();
        }

        // Shortcut list
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.render_shortcut_list(ui, editor);
            });

        ui.separator();

        // Bottom buttons
        ui.horizontal(|ui| {
            if ui
                .button(self.state.i18n.t("shortcuts.reset_defaults"))
                .clicked()
            {
                editor.working_config = crate::shortcuts::ShortcutManager::new()
                    .get_config()
                    .clone();
                editor.refresh_conflicts();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(self.state.i18n.t("shortcuts.save")).clicked() {
                    should_save = true;
                    should_close = true;
                    new_config = Some(editor.working_config.clone());
                }

                if ui.button(self.state.i18n.t("shortcuts.cancel")).clicked() {
                    should_close = true;
                }
            });
        });

        // Edit dialog
        if editor.editing_action.is_some() {
            self.show_edit_shortcut_dialog(ctx, editor);
        }

        (should_close, should_save, new_config)
    }

    fn render_shortcut_list(
        &mut self,
        ui: &mut egui::Ui,
        editor: &mut crate::shortcuts::ShortcutEditorState,
    ) {
        let filtered = editor.get_filtered_shortcuts();

        // Table header
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(self.state.i18n.t("shortcuts.action")).strong());
            ui.separator();
            ui.label(egui::RichText::new(self.state.i18n.t("shortcuts.shortcut")).strong());
            ui.separator();
            ui.label(egui::RichText::new(self.state.i18n.t("shortcuts.scope")).strong());
        });

        ui.separator();

        // Collect actions to edit/clear
        let mut action_to_edit = None;
        let mut action_to_clear = None;

        // List items
        for (action, binding) in &filtered {
            ui.horizontal(|ui| {
                ui.label(self.state.i18n.t(action.description_key()));
                ui.separator();

                let shortcut_text = self.format_binding_display(binding);
                ui.label(shortcut_text);
                ui.separator();

                ui.label(self.state.i18n.t(binding.scope.name_key()));
                ui.separator();

                if ui
                    .small_button(self.state.i18n.t("shortcuts.edit"))
                    .clicked()
                {
                    action_to_edit = Some(*action);
                }

                if ui
                    .small_button(self.state.i18n.t("shortcuts.clear"))
                    .clicked()
                {
                    action_to_clear = Some(*action);
                }
            });
        }

        // Apply actions after iteration
        if let Some(action) = action_to_edit {
            editor.start_editing(action);
        }
        if let Some(action) = action_to_clear {
            editor
                .working_config
                .shortcuts
                .retain(|b| b.action != action.as_str());
            editor.refresh_conflicts();
        }
    }

    fn show_edit_shortcut_dialog(
        &mut self,
        ctx: &egui::Context,
        editor: &mut crate::shortcuts::ShortcutEditorState,
    ) {
        let action = editor.editing_action.unwrap();

        egui::Window::new(format!(
            "{}: {}",
            self.state.i18n.t("shortcuts.edit_shortcut"),
            self.state.i18n.t(action.description_key())
        ))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(self.state.i18n.t("shortcuts.press_key"));
            ui.separator();

            // Current binding
            if let Some(binding) = editor
                .working_config
                .shortcuts
                .iter()
                .find(|b| b.action == action.as_str())
            {
                ui.label(format!(
                    "{}: {}",
                    self.state.i18n.t("shortcuts.current"),
                    self.format_binding_display(binding)
                ));
            }

            // New binding
            ui.label(format!(
                "{}: {}",
                self.state.i18n.t("shortcuts.new"),
                if let Some(key) = editor.captured_key {
                    self.format_modifiers_and_key(&editor.captured_modifiers, key)
                } else {
                    self.state.i18n.t("shortcuts.capturing")
                }
            ));

            ui.separator();

            // Capture input
            if editor.is_capturing {
                self.capture_shortcut_input(ctx, editor);
            }

            ui.separator();

            // Buttons
            ui.horizontal(|ui| {
                if ui.button(self.state.i18n.t("shortcuts.clear")).clicked() {
                    editor.captured_key = None;
                    editor.captured_modifiers = crate::shortcuts::KeyModifiers {
                        ctrl: false,
                        shift: false,
                        alt: false,
                    };
                }

                if ui.button(self.state.i18n.t("shortcuts.cancel")).clicked() {
                    editor.cancel_editing();
                }

                if ui.button(self.state.i18n.t("shortcuts.apply")).clicked() {
                    editor.apply_edit();
                }
            });
        });
    }

    fn capture_shortcut_input(
        &self,
        ctx: &egui::Context,
        editor: &mut crate::shortcuts::ShortcutEditorState,
    ) {
        ctx.input(|i| {
            editor.captured_modifiers = crate::shortcuts::KeyModifiers {
                ctrl: i.modifiers.ctrl,
                shift: i.modifiers.shift,
                alt: i.modifiers.alt,
            };

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
                egui::Key::F1,
                egui::Key::F2,
                egui::Key::F3,
                egui::Key::F4,
                egui::Key::F5,
                egui::Key::F6,
                egui::Key::F7,
                egui::Key::F8,
                egui::Key::F9,
                egui::Key::F10,
                egui::Key::F11,
                egui::Key::F12,
            ] {
                if i.key_pressed(*key) {
                    editor.captured_key = Some(*key);
                    break;
                }
            }
        });
    }

    fn format_binding_display(&self, binding: &crate::shortcuts::ShortcutBinding) -> String {
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

        let key_name = match binding.key.as_str() {
            "Space" => "Space",
            "Tab" => "Tab",
            "Escape" => "Esc",
            "Delete" => "Del",
            "ArrowLeft" => "←",
            "ArrowRight" => "→",
            "ArrowUp" => "↑",
            "ArrowDown" => "↓",
            key if key.starts_with("Num") => &key[3..],
            key => key,
        };

        parts.push(key_name);
        parts.join("+")
    }

    fn format_modifiers_and_key(
        &self,
        modifiers: &crate::shortcuts::KeyModifiers,
        key: egui::Key,
    ) -> String {
        let mut parts = Vec::new();

        if modifiers.ctrl {
            parts.push("Ctrl".to_string());
        }
        if modifiers.shift {
            parts.push("Shift".to_string());
        }
        if modifiers.alt {
            parts.push("Alt".to_string());
        }

        parts.push(format!("{:?}", key));
        parts.join("+")
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        // Handle shortcuts through ShortcutManager
        if let Some(action) = self
            .state
            .shortcut_manager
            .handle_input(ctx, self.state.editing_state.mode)
        {
            self.handle_shortcut_action(action, ctx);
        }

        // Category hotkeys (1-9) - these are dynamic and not in ShortcutManager
        let category_hotkeys: Vec<(i32, u32)> = self
            .state
            .get_meta()
            .map(|meta| {
                meta.categories
                    .iter()
                    .filter_map(|c| {
                        c.hotkey
                            .parse::<u32>()
                            .ok()
                            .filter(|&d| d >= 1 && d <= 9)
                            .map(|d| (c.id, d))
                    })
                    .collect()
            })
            .unwrap_or_default();

        for (category_id, digit) in category_hotkeys {
            let key = match digit {
                1 => egui::Key::Num1,
                2 => egui::Key::Num2,
                3 => egui::Key::Num3,
                4 => egui::Key::Num4,
                5 => egui::Key::Num5,
                6 => egui::Key::Num6,
                7 => egui::Key::Num7,
                8 => egui::Key::Num8,
                9 => egui::Key::Num9,
                _ => continue,
            };

            if ctx.input(|i| i.key_pressed(key)) {
                if self.state.editing_state.mode == crate::state::EditMode::Editing {
                    DrawingTools::change_selected_category(&mut self.state, category_id);
                }
            }
        }
    }

    fn handle_shortcut_action(&mut self, action: crate::shortcuts::ShortcutAction, ctx: &Context) {
        use crate::shortcuts::ShortcutAction;
        let read_only = self.state.editing_state.mode == crate::state::EditMode::Browse;
        let editing_mode = self.state.editing_state.mode == crate::state::EditMode::Editing;

        match action {
            // File operations
            ShortcutAction::OpenProject => {
                self.open_project_dialog();
            }
            ShortcutAction::Save => {
                if self.state.editing_state.mode == crate::state::EditMode::Drawing {
                    self.finish_drawing();
                }
                let _ = self.state.save_annotation();
            }
            ShortcutAction::CloseProject => {
                let _ = self.state.close_project();
            }
            ShortcutAction::Quit => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

            // Edit operations
            ShortcutAction::Copy => {
                if editing_mode {
                    self.state.copy_selected();
                }
            }
            ShortcutAction::Paste => {
                if editing_mode {
                    self.state.paste_from_clipboard();
                }
            }
            ShortcutAction::Delete => {
                if !read_only {
                    self.state.delete_selected();
                }
            }
            ShortcutAction::Deselect => {
                self.state.selected_object_id = None;
                self.state.editing_state.selected_vertex = None;
            }
            ShortcutAction::FinishDrawing => {
                if self.state.editing_state.mode == crate::state::EditMode::Drawing {
                    self.finish_drawing();
                }
            }
            ShortcutAction::ConvertToRectangle => {
                self.convert_selected_to_rectangle();
            }
            ShortcutAction::FixSelfIntersection => {
                self.fix_selected_self_intersections();
            }

            // Mode switching
            ShortcutAction::SwitchToNormalMode => {
                self.state.editing_state.mode = crate::state::EditMode::Browse;
            }
            ShortcutAction::SwitchToDrawingMode => {
                self.state.editing_state.mode = crate::state::EditMode::Drawing;
                self.state.clear_drawing_state();
            }
            ShortcutAction::SwitchToEditingMode => {
                if self.state.selected_object_id.is_some() {
                    self.state.editing_state.mode = crate::state::EditMode::Editing;
                    self.state.editing_state.selected_vertex = None;
                }
            }

            // Navigation
            ShortcutAction::PreviousImage => {
                if self.state.editing_state.mode == crate::state::EditMode::Drawing {
                    self.finish_drawing();
                }
                let _ = self.state.prev_image();
                self.canvas.reset_view();
            }
            ShortcutAction::NextImage => {
                if self.state.editing_state.mode == crate::state::EditMode::Drawing {
                    self.finish_drawing();
                }
                let _ = self.state.next_image();
                self.canvas.reset_view();
            }
            ShortcutAction::JumpBackward10 => {
                if self.state.editing_state.mode == crate::state::EditMode::Drawing {
                    self.finish_drawing();
                }
                let _ = self.state.jump_backward(10);
                self.canvas.reset_view();
            }
            ShortcutAction::JumpForward10 => {
                if self.state.editing_state.mode == crate::state::EditMode::Drawing {
                    self.finish_drawing();
                }
                let _ = self.state.jump_forward(10);
                self.canvas.reset_view();
            }
            ShortcutAction::CycleNextObject => {
                if let Some(annotation) = &self.state.current_annotation {
                    if !annotation.objects.is_empty() {
                        let current_idx = self
                            .state
                            .selected_object_id
                            .and_then(|id| annotation.objects.iter().position(|o| o.id == id))
                            .unwrap_or(0);

                        let next_idx = (current_idx + 1) % annotation.objects.len();
                        self.state.selected_object_id = Some(annotation.objects[next_idx].id);
                    }
                }
            }
            ShortcutAction::CyclePreviousObject => {
                if let Some(annotation) = &self.state.current_annotation {
                    if !annotation.objects.is_empty() {
                        let current_idx = self
                            .state
                            .selected_object_id
                            .and_then(|id| annotation.objects.iter().position(|o| o.id == id))
                            .unwrap_or(0);

                        let prev_idx = if current_idx == 0 {
                            annotation.objects.len() - 1
                        } else {
                            current_idx - 1
                        };
                        self.state.selected_object_id = Some(annotation.objects[prev_idx].id);
                    }
                }
            }

            // View
            ShortcutAction::FitToCanvas => {
                if let Some(image) = &self.state.current_image {
                    let canvas_size = ctx.screen_rect().size();
                    let image_size = Vec2::new(image.width as f32, image.height as f32);
                    self.canvas.fit_to_canvas(canvas_size, image_size);
                }
            }
            ShortcutAction::ResetZoom => {
                self.canvas.reset_view();
            }

            // Movement
            ShortcutAction::MoveLeft => {
                if editing_mode {
                    self.move_selected((-1.0, 0.0), ctx);
                }
            }
            ShortcutAction::MoveRight => {
                if editing_mode {
                    self.move_selected((1.0, 0.0), ctx);
                }
            }
            ShortcutAction::MoveUp => {
                if editing_mode {
                    self.move_selected((0.0, -1.0), ctx);
                }
            }
            ShortcutAction::MoveDown => {
                if editing_mode {
                    self.move_selected((0.0, 1.0), ctx);
                }
            }

            // Scaling
            ShortcutAction::ScaleUp => {
                self.scale_selected_object(1.1);
            }
            ShortcutAction::ScaleDown => {
                self.scale_selected_object(0.9);
            }

            // Other
            ShortcutAction::Cancel => {
                if self.state.editing_state.mode == crate::state::EditMode::Drawing {
                    DrawingTools::cancel_drawing(&mut self.state);
                } else if self.state.editing_state.mode == crate::state::EditMode::Editing {
                    self.state.editing_state.mode = crate::state::EditMode::Browse;
                    self.state.selected_object_id = None;
                    self.state.editing_state.selected_vertex = None;
                    log::info!("Exited Editing mode");
                } else {
                    self.state.selected_object_id = None;
                    self.state.editing_state.selected_vertex = None;
                    log::info!("Deselected object");
                }
            }

            // Toggle Panels
            ShortcutAction::ToggleAutoSave => {
                // TODO: Implement auto-save toggle
                log::info!("Toggle Auto Save (not yet implemented)");
            }
            ShortcutAction::ToggleLeftPanel => {
                // TODO: Implement left panel toggle
                log::info!("Toggle Left Panel (not yet implemented)");
            }
            ShortcutAction::ToggleRightPanel => {
                // TODO: Implement right panel toggle
                log::info!("Toggle Right Panel (not yet implemented)");
            }

            // Zoom Levels
            ShortcutAction::Zoom25 => {
                self.canvas.set_zoom(0.25);
            }
            ShortcutAction::Zoom50 => {
                self.canvas.set_zoom(0.50);
            }
            ShortcutAction::Zoom75 => {
                self.canvas.set_zoom(0.75);
            }
            ShortcutAction::Zoom100 => {
                self.canvas.set_zoom(1.0);
            }
            ShortcutAction::Zoom125 => {
                self.canvas.set_zoom(1.25);
            }
            ShortcutAction::Zoom150 => {
                self.canvas.set_zoom(1.50);
            }
            ShortcutAction::Zoom200 => {
                self.canvas.set_zoom(2.0);
            }
            ShortcutAction::Zoom300 => {
                self.canvas.set_zoom(3.0);
            }
            ShortcutAction::Zoom400 => {
                self.canvas.set_zoom(4.0);
            }
        }
    }

    fn move_selected(&mut self, direction: (f32, f32), ctx: &Context) {
        if self.state.editing_state.mode != crate::state::EditMode::Editing {
            return;
        }

        if let Some(image) = &self.state.current_image {
            let shift_held = ctx.input(|i| i.modifiers.shift);
            let move_distance = if shift_held { 10.0 } else { 1.0 };

            // Convert pixel distance to normalized coordinates
            let dx_norm = (move_distance * direction.0) / image.width as f32;
            let dy_norm = (move_distance * direction.1) / image.height as f32;

            if let Some(annotation) = &mut self.state.current_annotation {
                // Priority 1: Move selected vertex if any
                if let Some((obj_id, vertex_idx)) = self.state.editing_state.selected_vertex {
                    let mut updated = false;
                    Self::with_polygon_mut(annotation, obj_id, |polygon| {
                        if vertex_idx < polygon.len() {
                            polygon[vertex_idx].x =
                                (polygon[vertex_idx].x + dx_norm).clamp(0.0, 1.0);
                            polygon[vertex_idx].y =
                                (polygon[vertex_idx].y + dy_norm).clamp(0.0, 1.0);
                            updated = true;
                        }
                    });
                    if updated {
                        self.state.has_unsaved_changes = true;
                    }
                } else if let Some(obj_id) = self.state.selected_object_id {
                    // Priority 2: Move selected object
                    Self::with_polygon_mut(annotation, obj_id, |polygon| {
                        for vertex in polygon {
                            vertex.x = (vertex.x + dx_norm).clamp(0.0, 1.0);
                            vertex.y = (vertex.y + dy_norm).clamp(0.0, 1.0);
                        }
                    });
                    self.state.has_unsaved_changes = true;
                }
            }
        }
    }

    fn apply_pending_draw_clicks(&mut self, now: f64, double_click_delay: f64) {
        if self.state.editing_state.mode != crate::state::EditMode::Drawing {
            self.state.pending_draw_clicks.clear();
            return;
        }

        let mut ready = Vec::new();
        self.state.pending_draw_clicks.retain(|pending| {
            if now - pending.time >= double_click_delay {
                ready.push(pending.position);
                false
            } else {
                true
            }
        });

        for position in ready {
            DrawingTools::handle_click(&mut self.state, position);
        }
    }

    fn flush_pending_draw_clicks(&mut self) {
        if self.state.pending_draw_clicks.is_empty() {
            return;
        }

        let pending: Vec<_> = self
            .state
            .pending_draw_clicks
            .drain(..)
            .map(|item| item.position)
            .collect();
        for position in pending {
            DrawingTools::handle_click(&mut self.state, position);
        }
    }

    fn drop_recent_pending_draw_click(&mut self, now: f64, double_click_delay: f64) {
        if let Some(last) = self.state.pending_draw_clicks.last() {
            if now - last.time <= double_click_delay {
                self.state.pending_draw_clicks.pop();
            }
        }
    }

    /// Finish drawing a new object
    fn finish_drawing(&mut self) -> bool {
        self.flush_pending_draw_clicks();

        if self.state.temp_points.len() < 3 {
            log::warn!("Need at least 3 points to create a shape");
            return false;
        }

        if self.state.draw_target == crate::state::DrawTarget::Roi {
            if let Some(annotation) = &mut self.state.current_annotation {
                annotation.rois.push(self.state.temp_points.clone());
                annotation.touch();
                self.state.has_unsaved_changes = true;
                let roi_id = crate::state::roi_id_from_index(annotation.rois.len() - 1);
                self.state.selected_object_id = Some(roi_id);
            }

            self.state.clear_drawing_state();
            log::info!("Updated ROI");
            return true;
        }

        let polygon = self.state.temp_points.clone();

        // Get next object ID
        let new_id = if let Some(annotation) = &self.state.current_annotation {
            annotation.objects.iter().map(|o| o.id).max().unwrap_or(-1) + 1
        } else {
            0
        };

        // Get default category
        let default_category = self
            .state
            .get_meta()
            .and_then(|m| m.categories.first())
            .map(|c| c.id)
            .unwrap_or(0);

        // Create new object
        let new_object = lab_core::Object {
            id: new_id,
            category: default_category,
            confidence: 1.0,
            polygon,
            properties: std::collections::HashMap::new(),
        };

        // Add to annotation
        if let Some(annotation) = &mut self.state.current_annotation {
            annotation.objects.push(new_object);
            self.state.has_unsaved_changes = true;

            // Select the new object
            self.state.selected_object_id = Some(new_id);
        }

        self.state.clear_drawing_state();

        log::info!("Created new object #{}", new_id);
        true
    }

    fn open_project_dialog(&mut self) {
        // Use rfd to open a folder picker dialog
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select Project Directory")
            .pick_folder()
        {
            log::info!("Selected project directory: {:?}", path);

            // Load the project
            match self.state.load_project(path.clone()) {
                Ok(()) => {
                    log::info!("Successfully loaded project from {:?}", path);
                    self.canvas.reset_view();
                }
                Err(e) => {
                    log::error!("Failed to load project from {:?}: {}", path, e);
                }
            }
        } else {
            log::info!("Project selection cancelled");
        }
    }

    fn convert_selected_to_rectangle(&mut self) {
        if self.state.editing_state.mode != crate::state::EditMode::Editing {
            return;
        }

        if let (Some(annotation), Some(obj_id)) = (
            &mut self.state.current_annotation,
            self.state.selected_object_id,
        ) {
            let mut updated = false;
            Self::with_polygon_mut(annotation, obj_id, |polygon| {
                if let Some((min, max)) = crate::geometry::bounding_box(polygon) {
                    *polygon = vec![
                        lab_core::Point::new(min.x, min.y),
                        lab_core::Point::new(max.x, min.y),
                        lab_core::Point::new(max.x, max.y),
                        lab_core::Point::new(min.x, max.y),
                    ];
                    updated = true;
                }
            });
            if updated {
                self.state.has_unsaved_changes = true;
                self.state.editing_state.selected_vertex = None;
            }
        }
    }

    fn fix_selected_self_intersections(&mut self) {
        if self.state.editing_state.mode != crate::state::EditMode::Editing {
            return;
        }

        if let (Some(annotation), Some(obj_id)) = (
            &mut self.state.current_annotation,
            self.state.selected_object_id,
        ) {
            let mut updated = false;
            Self::with_polygon_mut(annotation, obj_id, |polygon| {
                if crate::geometry::fix_self_intersections(polygon) {
                    updated = true;
                }
            });
            if updated {
                self.state.has_unsaved_changes = true;
                self.state.editing_state.selected_vertex = None;
            }
        }
    }

    /// Scale selected objects by a factor
    fn scale_selected_object(&mut self, scale_factor: f32) {
        if self.state.editing_state.mode != crate::state::EditMode::Editing {
            return;
        }

        if let (Some(annotation), Some(obj_id)) = (
            &mut self.state.current_annotation,
            self.state.selected_object_id,
        ) {
            Self::with_polygon_mut(annotation, obj_id, |polygon| {
                if let Some((min, max)) = crate::geometry::bounding_box(polygon) {
                    let center_x = (min.x + max.x) / 2.0;
                    let center_y = (min.y + max.y) / 2.0;

                    for vertex in polygon {
                        let dx = vertex.x - center_x;
                        let dy = vertex.y - center_y;
                        vertex.x = (center_x + dx * scale_factor).clamp(0.0, 1.0);
                        vertex.y = (center_y + dy * scale_factor).clamp(0.0, 1.0);
                    }
                }
            });
            self.state.has_unsaved_changes = true;
        }
    }
}
