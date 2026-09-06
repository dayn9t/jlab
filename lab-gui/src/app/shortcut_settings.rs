// Shortcut settings window: editor UI for keybindings
use super::LabApp;
use egui::Context;

impl LabApp {
    pub(super) fn show_shortcut_settings(&mut self, ctx: &Context) {
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

    fn render_shortcut_editor_content(
        &mut self,
        ui: &mut egui::Ui,
        editor: &mut crate::shortcuts::ShortcutEditorState,
        ctx: &Context,
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
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            self.render_shortcut_list(ui, editor);
        });

        ui.separator();

        // Bottom buttons
        ui.horizontal(|ui| {
            if ui.button(self.state.i18n.t("shortcuts.reset_defaults")).clicked() {
                editor.working_config =
                    crate::shortcuts::ShortcutManager::new().get_config().clone();
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

                if ui.small_button(self.state.i18n.t("shortcuts.edit")).clicked() {
                    action_to_edit = Some(*action);
                }

                if ui.small_button(self.state.i18n.t("shortcuts.clear")).clicked() {
                    action_to_clear = Some(*action);
                }
            });
        }

        // Apply actions after iteration
        if let Some(action) = action_to_edit {
            editor.start_editing(action);
        }
        if let Some(action) = action_to_clear {
            editor.working_config.shortcuts.retain(|b| b.action != action.as_str());
            editor.refresh_conflicts();
        }
    }

    fn show_edit_shortcut_dialog(
        &mut self,
        ctx: &Context,
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
            if let Some(binding) =
                editor.working_config.shortcuts.iter().find(|b| b.action == action.as_str())
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
                    editor.captured_modifiers =
                        crate::shortcuts::KeyModifiers { ctrl: false, shift: false, alt: false };
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
        ctx: &Context,
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
}
