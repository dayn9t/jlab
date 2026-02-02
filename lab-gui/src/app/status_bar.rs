use super::LabApp;
use egui::{Context, TopBottomPanel};

impl LabApp {
    pub(super) fn show_status_bar(
        &mut self,
        ctx: &Context,
        status_hint: &Option<String>,
        cursor_pixel_pos: Option<(i32, i32)>,
    ) {
        // Bottom status bar
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut first = true;

                if let Some(project) = &self.state.project {
                    if let Ok(progress) = project.get_progress() {
                        Self::status_add_label(
                            ui,
                            &mut first,
                            format!(
                                "{} {}/{} ({:.1}%)",
                                self.state.i18n.t("sidebar.progress"),
                                progress.annotated,
                                progress.total,
                                progress.percentage()
                            ),
                        );
                    }
                }

                if let Some(image) = &self.state.current_image {
                    if let Some(filename) = image.path.file_name() {
                        Self::status_add_label(
                            ui,
                            &mut first,
                            format!(
                                "{} {}",
                                self.state.i18n.t("sidebar.file"),
                                filename.to_string_lossy()
                            ),
                        );
                    }
                    Self::status_add_label(
                        ui,
                        &mut first,
                        format!(
                            "{} {}x{}",
                            self.state.i18n.t("sidebar.size"),
                            image.width,
                            image.height
                        ),
                    );
                }

                if let Some(project) = &self.state.project {
                    let auto_save = project.meta.shape.auto_save;
                    let auto_save_text = if auto_save {
                        format!(
                            "{} {}",
                            self.state.i18n.t("sidebar.auto_save"),
                            self.state.i18n.t("sidebar.auto_save_on")
                        )
                    } else {
                        format!(
                            "{} {}",
                            self.state.i18n.t("sidebar.auto_save"),
                            self.state.i18n.t("sidebar.auto_save_off")
                        )
                    };
                    let auto_save_color = if auto_save {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::GRAY
                    };
                    Self::status_add_colored(ui, &mut first, auto_save_color, auto_save_text);
                }

                if self.state.has_unsaved_changes {
                    Self::status_add_colored(
                        ui,
                        &mut first,
                        egui::Color32::RED,
                        self.state.i18n.t("sidebar.unsaved_changes"),
                    );
                }

                if let Some((x, y)) = cursor_pixel_pos {
                    Self::status_add_label(
                        ui,
                        &mut first,
                        format!("{}: ({}, {})", self.state.i18n.t("status.cursor"), x, y),
                    );
                }

                let hint_text = status_hint.clone().unwrap_or_default();
                if hint_text.is_empty() {
                    Self::status_add_label(
                        ui,
                        &mut first,
                        format!("{}:", self.state.i18n.t("status.hint")),
                    );
                } else {
                    Self::status_add_label(
                        ui,
                        &mut first,
                        format!("{}: {}", self.state.i18n.t("status.hint"), hint_text),
                    );
                }
            });
        });
    }
}
