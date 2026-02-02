// Options dialog for JLab
use crate::i18n::Language;
use crate::shortcuts::ShortcutEditorState;
use crate::state::ThemeColor;
use egui::Context;

// Dialog result type for passing button actions back
#[derive(Clone)]
pub enum DialogButtonAction {
    Apply,
    Ok,
    Cancel,
}

/// Options dialog tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsTab {
    General,
    Shortcuts,
}

impl OptionsTab {
    pub fn name_key(&self) -> &'static str {
        match self {
            Self::General => "options.tab_general",
            Self::Shortcuts => "options.tab_shortcuts",
        }
    }
}

/// Options dialog state
pub struct OptionsDialogState {
    pub show: bool,
    pub active_tab: OptionsTab,
    pub selected_language: Language,
    pub auto_save_enabled: bool,
    pub shortcut_editor: Option<ShortcutEditorState>,
    pub pending_changes: bool,
    // New options
    pub font_size: f32,
    pub ui_scale: f32,
    pub theme_color: ThemeColor,
    pub show_scrollbar: bool,
}

impl OptionsDialogState {
    pub fn new(language: Language, auto_save: bool) -> Self {
        Self {
            show: false,
            active_tab: OptionsTab::General,
            selected_language: language,
            auto_save_enabled: auto_save,
            shortcut_editor: None,
            pending_changes: false,
            font_size: 16.0,
            ui_scale: 1.0,
            theme_color: ThemeColor::System,
            show_scrollbar: true,
        }
    }

    /// Show the options dialog
    pub fn show_dialog(&mut self, current_config: &crate::shortcuts::ShortcutConfig) {
        self.show = true;
        self.active_tab = OptionsTab::General;
        self.shortcut_editor = Some(ShortcutEditorState::new(current_config));
        self.pending_changes = false;
        // Settings will be refreshed from current state before showing
    }

    /// Refresh settings from current application state
    pub fn refresh_from_state(&mut self, language: Language, auto_save: bool, theme: ThemeColor) {
        self.selected_language = language;
        self.auto_save_enabled = auto_save;
        self.theme_color = theme;
    }

    /// Render the options dialog
    pub fn show(
        &mut self,
        ctx: &Context,
        i18n: &crate::i18n::I18n,
    ) -> (bool, bool, Option<DialogButtonAction>) {
        if !self.show {
            return (false, false, None);
        }

        let mut open = true;
        let mut button_action = None;

        egui::Window::new(i18n.t("options.title"))
            .open(&mut open)
            .default_size([700.0, 450.0])
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                self.show_content(ui, i18n, &mut button_action);
            });

        if !open {
            self.show = false;
            self.shortcut_editor = None;
        }

        (self.show, open, button_action)
    }

    fn show_content(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &crate::i18n::I18n,
        button_action: &mut Option<DialogButtonAction>,
    ) {
        // Top row: search bar and buttons
        ui.horizontal(|ui| {
            // Search bar on the left
            ui.label(i18n.t("options.search"));
            let search_text = if let Some(ref mut editor) = self.shortcut_editor {
                &mut editor.search_filter
            } else {
                &mut String::new()
            };
            ui.text_edit_singleline(search_text);

            // Use available width to push buttons to the right
            ui.add_space(ui.available_width() - 120.0);

            // Buttons on the right (Apply, OK, Cancel)
            if ui.button(i18n.t("options.apply")).clicked() {
                *button_action = Some(DialogButtonAction::Apply);
            }
            if ui.button(i18n.t("options.ok")).clicked() {
                *button_action = Some(DialogButtonAction::Ok);
            }
            if ui.button(i18n.t("options.cancel")).clicked() {
                *button_action = Some(DialogButtonAction::Cancel);
            }
        });

        ui.separator();

        // Tab selector
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active_tab, OptionsTab::General, i18n.t(OptionsTab::General.name_key()));
            ui.selectable_value(&mut self.active_tab, OptionsTab::Shortcuts, i18n.t(OptionsTab::Shortcuts.name_key()));
        });

        ui.separator();

        // Tab content
        match self.active_tab {
            OptionsTab::General => self.show_general_tab(ui, i18n),
            OptionsTab::Shortcuts => self.show_shortcuts_tab(ui, i18n),
        }
    }

    fn show_general_tab(&mut self, ui: &mut egui::Ui, i18n: &crate::i18n::I18n) {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(10.0);

                // Language section
                ui.label(i18n.t("options.section_language"));
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(i18n.t("options.language"));
                    let lang_text = self.selected_language.name();
                    let response = egui::ComboBox::from_id_salt("options_language")
                        .selected_text(lang_text)
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(self.selected_language == Language::ZhCN, Language::ZhCN.name()).clicked() {
                                self.selected_language = Language::ZhCN;
                            }
                            if ui.selectable_label(self.selected_language == Language::EnUS, Language::EnUS.name()).clicked() {
                                self.selected_language = Language::EnUS;
                            }
                        });
                    if response.response.changed() {
                        self.pending_changes = true;
                    }
                });

                ui.add_space(15.0);

                // Appearance section
                ui.label(i18n.t("options.section_appearance"));
                ui.separator();

                // Font size
                ui.horizontal(|ui| {
                    ui.label(i18n.t("options.font_size"));
                    ui.add(egui::Slider::new(&mut self.font_size, 10.0..=30.0).step_by(1.0));
                    ui.label(format!("{:.0}", self.font_size));
                    if ui.response().changed() {
                        self.pending_changes = true;
                    }
                });

                ui.add_space(10.0);

                // UI scale
                ui.horizontal(|ui| {
                    ui.label(i18n.t("options.ui_scale"));
                    ui.add(egui::Slider::new(&mut self.ui_scale, 0.5..=2.0).step_by(0.1));
                    ui.label(format!("x{:.1}", self.ui_scale));
                    if ui.response().changed() {
                        self.pending_changes = true;
                    }
                });

                ui.add_space(10.0);

                // Theme color
                ui.horizontal(|ui| {
                    ui.label(i18n.t("options.theme_color"));
                    let theme_text = i18n.t(self.theme_color.name_key());
                    egui::ComboBox::from_id_salt("options_theme")
                        .selected_text(theme_text)
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(self.theme_color == ThemeColor::Light, i18n.t("options.theme_light")).clicked() {
                                self.theme_color = ThemeColor::Light;
                            }
                            if ui.selectable_label(self.theme_color == ThemeColor::Dark, i18n.t("options.theme_dark")).clicked() {
                                self.theme_color = ThemeColor::Dark;
                            }
                            if ui.selectable_label(self.theme_color == ThemeColor::System, i18n.t("options.theme_system")).clicked() {
                                self.theme_color = ThemeColor::System;
                            }
                        });
                    if ui.response().changed() {
                        self.pending_changes = true;
                    }
                });

                ui.add_space(15.0);

                // Interface section
                ui.label(i18n.t("options.section_interface"));
                ui.separator();

                // Show scrollbar
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.show_scrollbar, i18n.t("options.show_scrollbar"));
                    if ui.response().changed() {
                        self.pending_changes = true;
                    }
                });

                ui.add_space(15.0);

                // Project section
                ui.label(i18n.t("options.section_project"));
                ui.separator();

                // Auto save
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.auto_save_enabled, i18n.t("options.auto_save"));
                    if ui.response().changed() {
                        self.pending_changes = true;
                    }
                });
        });
    }

    fn show_shortcuts_tab(&mut self, ui: &mut egui::Ui, i18n: &crate::i18n::I18n) {
        if let Some(ref mut editor) = self.shortcut_editor {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.add_space(10.0);

                // Category filter for shortcuts
                ui.horizontal(|ui| {
                    ui.label(i18n.t("options.category"));
                    let category_filter = editor.category_filter;
                    if ui.button(i18n.t("options.all_categories")).clicked() {
                        // Toggle between all and none
                    }
                    // Show category buttons here if needed
                });

                ui.separator();

                let filtered = editor.get_filtered_shortcuts();
                for (action, binding) in filtered {
                    ui.horizontal(|ui| {
                        ui.label(i18n.t(action.description_key()));
                        ui.separator();
                        let shortcut_text = format_shortcut(binding);
                        ui.label(shortcut_text);
                        ui.separator();
                        if ui.small_button(i18n.t("shortcuts.edit")).clicked() {
                            // TODO: open edit dialog
                        }
                    });
                }
            });
        }
    }

    /// Get current dialog settings
    pub fn get_current_settings(&self) -> DialogSettings {
        DialogSettings {
            language: self.selected_language,
            auto_save: self.auto_save_enabled,
            shortcut_config: self.shortcut_editor.as_ref().map(|e| e.working_config.clone()),
            font_size: self.font_size,
            ui_scale: self.ui_scale,
            theme_color: self.theme_color,
            show_scrollbar: self.show_scrollbar,
        }
    }
}

/// Dialog settings
#[derive(Clone)]
pub struct DialogSettings {
    pub language: Language,
    pub auto_save: bool,
    pub shortcut_config: Option<crate::shortcuts::ShortcutConfig>,
    pub font_size: f32,
    pub ui_scale: f32,
    pub theme_color: ThemeColor,
    pub show_scrollbar: bool,
}

fn format_shortcut(binding: &crate::shortcuts::ShortcutBinding) -> String {
    let mut parts = Vec::new();
    if binding.ctrl { parts.push("Ctrl"); }
    if binding.shift { parts.push("Shift"); }
    if binding.alt { parts.push("Alt"); }
    parts.push(&binding.key);
    parts.join("+")
}
