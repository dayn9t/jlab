use super::LabApp;
use egui::{Context, TopBottomPanel};

impl LabApp {
    pub(super) fn show_toolbar(&mut self, ctx: &Context, status_hint: &mut Option<String>) {
        // Toolbar
        TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Mode buttons
                ui.label(self.state.i18n.t("toolbar.mode"));

                let normal_mode_label = self.state.i18n.t("toolbar.mode_normal");
                let normal_mode_hint = self.hint_with_shortcut(
                    self.state.i18n.t("hint.edit_mode_browse"),
                    crate::shortcuts::ShortcutAction::SwitchToNormalMode,
                );
                let normal_mode_response = ui.selectable_label(
                    self.state.editing_state.mode == crate::state::EditMode::Browse,
                    normal_mode_label.clone(),
                );
                Self::update_status_hint(status_hint, &normal_mode_response, normal_mode_hint);
                if normal_mode_response.clicked() {
                    self.state.editing_state.mode = crate::state::EditMode::Browse;
                }

                let drawing_mode_label = self.state.i18n.t("toolbar.mode_drawing");
                let drawing_mode_hint = self.hint_with_shortcut(
                    self.state.i18n.t("hint.edit_mode_drawing"),
                    crate::shortcuts::ShortcutAction::SwitchToDrawingMode,
                );
                let drawing_mode_response = ui.selectable_label(
                    self.state.editing_state.mode == crate::state::EditMode::Drawing,
                    drawing_mode_label.clone(),
                );
                Self::update_status_hint(status_hint, &drawing_mode_response, drawing_mode_hint);
                if drawing_mode_response.clicked() {
                    self.state.editing_state.mode = crate::state::EditMode::Drawing;
                    self.state.clear_drawing_state();
                }

                let editing_mode_label = self.state.i18n.t("toolbar.mode_editing");
                let editing_mode_hint = self.hint_with_shortcut(
                    self.state.i18n.t("hint.edit_mode_editing"),
                    crate::shortcuts::ShortcutAction::SwitchToEditingMode,
                );
                let editing_mode_response = ui.selectable_label(
                    self.state.editing_state.mode == crate::state::EditMode::Editing,
                    editing_mode_label.clone(),
                );
                Self::update_status_hint(status_hint, &editing_mode_response, editing_mode_hint);
                if editing_mode_response.clicked() {
                    // Only allow entering Editing mode if an object is selected
                    if self.state.selected_object_id.is_some() {
                        self.state.editing_state.mode = crate::state::EditMode::Editing;
                        self.state.editing_state.selected_vertex = None;
                    }
                }

                ui.separator();

                ui.label(self.state.i18n.t("toolbar.draw_target"));
                let target_object_label = self.state.i18n.t("toolbar.draw_target_object");
                let target_object_hint = self.state.i18n.t("hint.draw_target_object");
                let target_object_response = ui.selectable_label(
                    self.state.draw_target == crate::state::DrawTarget::Object,
                    target_object_label.clone(),
                );
                Self::update_status_hint(status_hint, &target_object_response, target_object_hint);
                if target_object_response.clicked() {
                    self.state.draw_target = crate::state::DrawTarget::Object;
                }

                let target_roi_label = self.state.i18n.t("toolbar.draw_target_roi");
                let target_roi_hint = self.state.i18n.t("hint.draw_target_roi");
                let target_roi_response = ui.selectable_label(
                    self.state.draw_target == crate::state::DrawTarget::Roi,
                    target_roi_label.clone(),
                );
                Self::update_status_hint(status_hint, &target_roi_response, target_roi_hint);
                if target_roi_response.clicked() {
                    self.state.draw_target = crate::state::DrawTarget::Roi;
                }

                ui.separator();

                let zoom_label = self.state.i18n.t("toolbar.zoom");
                let zoom_hint = self.state.i18n.t("hint.toolbar_zoom");
                let zoom_hint_prefix = self.state.i18n.t("hint.zoom_to");
                ui.label(zoom_label.clone());
                let zoom_response = egui::ComboBox::from_id_salt("toolbar_zoom")
                    .selected_text(format!("{:.0}%", self.canvas.zoom * 100.0))
                    .show_ui(ui, |ui| {
                        let current_zoom = (self.canvas.zoom * 100.0).round();
                        for zoom in super::ZOOM_LEVELS {
                            let item_label = format!("{:.0}%", zoom);
                            let item_response =
                                ui.selectable_label((current_zoom - zoom).abs() < 0.5, item_label);
                            Self::update_status_hint(
                                status_hint,
                                &item_response,
                                format!("{} {:.0}%", zoom_hint_prefix, zoom),
                            );
                            if item_response.clicked() {
                                self.set_zoom(zoom);
                            }
                        }
                    })
                    .response;
                Self::update_status_hint(status_hint, &zoom_response, zoom_hint);

                ui.separator();

                let left_panel_label = self.state.i18n.t("menu.view_left_panel");
                let left_panel_hint = self.state.i18n.t("hint.view_left_panel");
                let left_panel_response =
                    ui.checkbox(&mut self.state.show_left_panel, left_panel_label.clone());
                Self::update_status_hint(status_hint, &left_panel_response, left_panel_hint);

                let right_panel_label = self.state.i18n.t("menu.view_right_panel");
                let right_panel_hint = self.state.i18n.t("hint.view_right_panel");
                let right_panel_response =
                    ui.checkbox(&mut self.state.show_right_panel, right_panel_label.clone());
                Self::update_status_hint(status_hint, &right_panel_response, right_panel_hint);
            });
        });
    }
}
