// Delete-sample confirmation dialog
use super::LabApp;
use egui::Context;

impl LabApp {
    pub(super) fn show_delete_confirm_dialog(&mut self, ctx: &Context) {
        if !self.state.show_delete_confirm {
            return;
        }

        // Bail out (and reset the flag) if the current sample vanished
        let Some(image_name) = self
            .state
            .images
            .get(self.state.current_image_index)
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
        else {
            self.state.show_delete_confirm = false;
            return;
        };

        let mut open = true;
        let mut confirm_clicked = false;
        let mut cancel_clicked = false;

        egui::Window::new(self.state.i18n.t("dialog.delete_sample_title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(format!(
                    "{}: {}",
                    self.state.i18n.t("dialog.delete_sample_file"),
                    image_name
                ));
                ui.colored_label(
                    egui::Color32::RED,
                    self.state.i18n.t("dialog.delete_sample_warning"),
                );
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(self.state.i18n.t("dialog.delete_sample_confirm")).clicked() {
                        confirm_clicked = true;
                    }
                    if ui.button(self.state.i18n.t("dialog.delete_sample_cancel")).clicked() {
                        cancel_clicked = true;
                    }
                });
            });

        if confirm_clicked {
            self.state.show_delete_confirm = false;
            if let Err(e) = self.state.delete_current_sample() {
                log::error!("Failed to delete sample: {}", e);
                let title = self.state.i18n.t("error.delete_sample");
                let _ = rfd::MessageDialog::new()
                    .set_title(&title)
                    .set_description(e.to_string())
                    .set_buttons(rfd::MessageButtons::Ok)
                    .set_level(rfd::MessageLevel::Error)
                    .show();
            }
        } else if cancel_clicked || !open {
            self.state.show_delete_confirm = false;
        }
    }
}
