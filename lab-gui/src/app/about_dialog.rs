// About dialog for JLab
use egui::Context;

/// About dialog state
pub struct AboutDialogState {
    pub show: bool,
}

impl AboutDialogState {
    pub fn new() -> Self {
        Self { show: false }
    }

    /// Show the about dialog
    pub fn show_dialog(&mut self) {
        self.show = true;
    }

    /// Render the about dialog
    pub fn show(&mut self, ctx: &Context, i18n: &crate::i18n::I18n) -> bool {
        if !self.show {
            return false;
        }

        let mut open = true;
        let mut button_clicked = false;

        egui::Window::new(i18n.t("about.title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);

                    // App name and title
                    ui.heading(i18n.t("app.title"));
                    ui.label(format!("{}: {}", i18n.t("about.version"), env!("CARGO_PKG_VERSION")));

                    ui.add_space(20.0);

                    // Description
                    ui.label(i18n.t("about.description"));
                    ui.label(i18n.t("about.features"));

                    ui.add_space(20.0);

                    // License
                    ui.label(i18n.t("about.license"));

                    ui.add_space(20.0);

                    // Website
                    if ui.link("🌐 GitHub").clicked() {
                        let _ = open::that("https://github.com/dayn9t/jlab");
                    }

                    ui.add_space(20.0);

                    // Close button (with Esc hint)
                    if ui.button(format!("{} (Esc)", i18n.t("about.close"))).clicked() {
                        button_clicked = true;
                    }
                });
            });

        // Check for Escape key to close dialog
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                button_clicked = true;
            }
        });

        // Close if button clicked, escape pressed, or X button clicked
        if button_clicked || !open {
            self.show = false;
        }

        self.show
    }
}
