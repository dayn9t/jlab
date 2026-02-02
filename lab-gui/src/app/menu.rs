use super::import_export::DatasetFormat;
use super::LabApp;
use egui::{Context, TopBottomPanel, Vec2};

impl LabApp {
    pub(super) fn show_top_menu(&mut self, ctx: &Context, status_hint: &mut Option<String>) {
        // Top menu bar
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // File menu
                let file_menu_label = self.state.i18n.t("menu.file");
                let file_menu_hint = self.state.i18n.t("hint.menu_file");
                let file_menu_response = ui.menu_button(file_menu_label.clone(), |ui| {
                    let open_label = self.state.i18n.t("menu.file_open");
                    let open_hint = self.hint_with_shortcut(
                        self.state.i18n.t("hint.file_open"),
                        crate::shortcuts::ShortcutAction::OpenProject,
                    );
                    let open_response = ui.button(self.menu_text(
                        open_label.clone(),
                        crate::shortcuts::ShortcutAction::OpenProject,
                    ));
                    Self::update_status_hint(status_hint, &open_response, open_hint);
                    if open_response.clicked() {
                        ui.close_menu();
                        self.open_project_dialog();
                    }

                    // Recent projects
                    if !self.state.recent_projects.is_empty() {
                        let recent_label = self.state.i18n.t("menu.file_recent");
                        let recent_hint_prefix = self.state.i18n.t("hint.file_recent_open");
                        ui.separator();
                        let recent_menu_response = ui.menu_button(recent_label.clone(), |ui| {
                            for project_path in self.state.recent_projects.clone() {
                                let path = project_path
                                    .to_str()
                                    .unwrap_or("Unknown");
                                let recent_response = ui.button(path);
                                Self::update_status_hint(
                                    status_hint,
                                    &recent_response,
                                    format!("{} {}", recent_hint_prefix, path),
                                );
                                if recent_response.clicked() {
                                    if let Err(e) = self.state.load_project(project_path) {
                                        log::error!("Failed to load project: {}", e);
                                    }
                                    ui.close_menu();
                                }
                            }
                        });
                        Self::update_status_hint(
                            status_hint,
                            &recent_menu_response.response,
                            recent_hint_prefix,
                        );
                    }

                    ui.separator();

                    let can_import_export = self.state.project.is_some();
                    let import_label = self.state.i18n.t("menu.file_import");
                    let import_hint = self.state.i18n.t("hint.file_import");
                    let import_menu_response = ui.menu_button(import_label.clone(), |ui| {
                        let yolo_label = self.state.i18n.t("menu.file_import_yolo");
                        let yolo_hint = self.state.i18n.t("hint.file_import_yolo");
                        let yolo_response = ui
                            .add_enabled(can_import_export, egui::Button::new(yolo_label.clone()));
                        Self::update_status_hint(status_hint, &yolo_response, yolo_hint);
                        if yolo_response.clicked() {
                            ui.close_menu();
                            self.import_dataset(DatasetFormat::Yolo);
                        }

                        let voc_label = self.state.i18n.t("menu.file_import_voc");
                        let voc_hint = self.state.i18n.t("hint.file_import_voc");
                        let voc_response =
                            ui.add_enabled(can_import_export, egui::Button::new(voc_label.clone()));
                        Self::update_status_hint(status_hint, &voc_response, voc_hint);
                        if voc_response.clicked() {
                            ui.close_menu();
                            self.import_dataset(DatasetFormat::Voc);
                        }

                        let coco_label = self.state.i18n.t("menu.file_import_coco");
                        let coco_hint = self.state.i18n.t("hint.file_import_coco");
                        let coco_response = ui
                            .add_enabled(can_import_export, egui::Button::new(coco_label.clone()));
                        Self::update_status_hint(status_hint, &coco_response, coco_hint);
                        if coco_response.clicked() {
                            ui.close_menu();
                            self.import_dataset(DatasetFormat::Coco);
                        }

                        let labelme_label = self.state.i18n.t("menu.file_import_labelme");
                        let labelme_hint = self.state.i18n.t("hint.file_import_labelme");
                        let labelme_response = ui.add_enabled(
                            can_import_export,
                            egui::Button::new(labelme_label.clone()),
                        );
                        Self::update_status_hint(status_hint, &labelme_response, labelme_hint);
                        if labelme_response.clicked() {
                            ui.close_menu();
                            self.import_dataset(DatasetFormat::LabelMe);
                        }
                    });
                    Self::update_status_hint(
                        status_hint,
                        &import_menu_response.response,
                        import_hint,
                    );

                    let export_label = self.state.i18n.t("menu.file_export");
                    let export_hint = self.state.i18n.t("hint.file_export");
                    let export_menu_response = ui.menu_button(export_label.clone(), |ui| {
                        let yolo_label = self.state.i18n.t("menu.file_export_yolo");
                        let yolo_hint = self.state.i18n.t("hint.file_export_yolo");
                        let yolo_response = ui
                            .add_enabled(can_import_export, egui::Button::new(yolo_label.clone()));
                        Self::update_status_hint(status_hint, &yolo_response, yolo_hint);
                        if yolo_response.clicked() {
                            ui.close_menu();
                            self.export_dataset(DatasetFormat::Yolo);
                        }

                        let voc_label = self.state.i18n.t("menu.file_export_voc");
                        let voc_hint = self.state.i18n.t("hint.file_export_voc");
                        let voc_response =
                            ui.add_enabled(can_import_export, egui::Button::new(voc_label.clone()));
                        Self::update_status_hint(status_hint, &voc_response, voc_hint);
                        if voc_response.clicked() {
                            ui.close_menu();
                            self.export_dataset(DatasetFormat::Voc);
                        }

                        let coco_label = self.state.i18n.t("menu.file_export_coco");
                        let coco_hint = self.state.i18n.t("hint.file_export_coco");
                        let coco_response = ui
                            .add_enabled(can_import_export, egui::Button::new(coco_label.clone()));
                        Self::update_status_hint(status_hint, &coco_response, coco_hint);
                        if coco_response.clicked() {
                            ui.close_menu();
                            self.export_dataset(DatasetFormat::Coco);
                        }

                        let labelme_label = self.state.i18n.t("menu.file_export_labelme");
                        let labelme_hint = self.state.i18n.t("hint.file_export_labelme");
                        let labelme_response = ui.add_enabled(
                            can_import_export,
                            egui::Button::new(labelme_label.clone()),
                        );
                        Self::update_status_hint(status_hint, &labelme_response, labelme_hint);
                        if labelme_response.clicked() {
                            ui.close_menu();
                            self.export_dataset(DatasetFormat::LabelMe);
                        }
                    });
                    Self::update_status_hint(
                        status_hint,
                        &export_menu_response.response,
                        export_hint,
                    );

                    ui.separator();

                    let save_label = self.state.i18n.t("menu.file_save");
                    let save_hint = self.hint_with_shortcut(
                        self.state.i18n.t("hint.file_save"),
                        crate::shortcuts::ShortcutAction::Save,
                    );
                    let save_response = ui.button(
                        self.menu_text(save_label.clone(), crate::shortcuts::ShortcutAction::Save),
                    );
                    Self::update_status_hint(status_hint, &save_response, save_hint);
                    if save_response.clicked() {
                        if self.state.editing_state.mode == crate::state::EditMode::Drawing {
                            self.finish_drawing();
                        }
                        if let Err(e) = self.state.save_annotation() {
                            log::error!("Failed to save annotation: {}", e);
                        }
                        ui.close_menu();
                    }

                    let close_label = self.state.i18n.t("menu.file_close");
                    let close_hint = self.hint_with_shortcut(
                        self.state.i18n.t("hint.file_close"),
                        crate::shortcuts::ShortcutAction::CloseProject,
                    );
                    let close_response = ui.button(self.menu_text(
                        close_label.clone(),
                        crate::shortcuts::ShortcutAction::CloseProject,
                    ));
                    Self::update_status_hint(status_hint, &close_response, close_hint);
                    if close_response.clicked() {
                        if let Err(e) = self.state.close_project() {
                            log::error!("Failed to close project: {}", e);
                        }
                        ui.close_menu();
                    }

                    ui.separator();

                    // Options menu item
                    let options_label = self.state.i18n.t("menu.file_options");
                    let options_hint = self.state.i18n.t("hint.file_options");
                    let options_response = ui.button(options_label.clone());
                    Self::update_status_hint(status_hint, &options_response, options_hint);
                    if options_response.clicked() {
                        self.state.show_options_dialog = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    let exit_label = self.state.i18n.t("menu.file_exit");
                    let exit_hint = self.hint_with_shortcut(
                        self.state.i18n.t("hint.file_exit"),
                        crate::shortcuts::ShortcutAction::Quit,
                    );
                    let exit_response = ui.button(
                        self.menu_text(exit_label.clone(), crate::shortcuts::ShortcutAction::Quit),
                    );
                    Self::update_status_hint(status_hint, &exit_response, exit_hint);
                    if exit_response.clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                Self::update_status_hint(status_hint, &file_menu_response.response, file_menu_hint);

                // Edit menu
                let edit_menu_label = self.state.i18n.t("menu.edit");
                let edit_menu_hint = self.state.i18n.t("hint.menu_edit");
                let edit_menu_response = ui.menu_button(edit_menu_label.clone(), |ui| {
                    let read_only = self.state.editing_state.mode == crate::state::EditMode::Browse;
                    let editing_mode =
                        self.state.editing_state.mode == crate::state::EditMode::Editing;

                    // Mode submenu
                    let mode_label = self.state.i18n.t("menu.edit_mode");
                    let mode_hint = self.state.i18n.t("hint.menu_edit_mode");
                    let mode_response = ui.menu_button(mode_label.clone(), |ui| {
                        let normal_label = self.state.i18n.t("menu.edit_mode_normal");
                        let normal_hint = self.hint_with_shortcut(
                            self.state.i18n.t("hint.edit_mode_browse"),
                            crate::shortcuts::ShortcutAction::SwitchToNormalMode,
                        );
                        let normal_response = ui.button(self.menu_text(
                            normal_label.clone(),
                            crate::shortcuts::ShortcutAction::SwitchToNormalMode,
                        ));
                        Self::update_status_hint(status_hint, &normal_response, normal_hint);
                        if normal_response.clicked() {
                            self.state.editing_state.mode = crate::state::EditMode::Browse;
                            ui.close_menu();
                        }

                        let drawing_label = self.state.i18n.t("menu.edit_mode_drawing");
                        let drawing_hint = self.hint_with_shortcut(
                            self.state.i18n.t("hint.edit_mode_drawing"),
                            crate::shortcuts::ShortcutAction::SwitchToDrawingMode,
                        );
                        let drawing_response = ui.button(self.menu_text(
                            drawing_label.clone(),
                            crate::shortcuts::ShortcutAction::SwitchToDrawingMode,
                        ));
                        Self::update_status_hint(status_hint, &drawing_response, drawing_hint);
                        if drawing_response.clicked() {
                            self.state.editing_state.mode = crate::state::EditMode::Drawing;
                            self.state.clear_drawing_state();
                            ui.close_menu();
                        }

                        let editing_label = self.state.i18n.t("menu.edit_mode_editing");
                        let editing_hint = self.hint_with_shortcut(
                            self.state.i18n.t("hint.edit_mode_editing"),
                            crate::shortcuts::ShortcutAction::SwitchToEditingMode,
                        );
                        let editing_response = ui.button(self.menu_text(
                            editing_label.clone(),
                            crate::shortcuts::ShortcutAction::SwitchToEditingMode,
                        ));
                        Self::update_status_hint(status_hint, &editing_response, editing_hint);
                        if editing_response.clicked() {
                            // Only allow entering Editing mode if an object is selected
                            if self.state.selected_object_id.is_some() {
                                self.state.editing_state.mode = crate::state::EditMode::Editing;
                                self.state.editing_state.selected_vertex = None;
                            }
                            ui.close_menu();
                        }
                    });
                    Self::update_status_hint(status_hint, &mode_response.response, mode_hint);

                    ui.separator();

                    let copy_label = self.state.i18n.t("menu.edit_copy");
                    let copy_hint = self.hint_with_shortcut(
                        self.state.i18n.t("hint.edit_copy"),
                        crate::shortcuts::ShortcutAction::Copy,
                    );
                    let copy_response =
                        ui.add_enabled(
                            editing_mode,
                            egui::Button::new(self.menu_text(
                                copy_label.clone(),
                                crate::shortcuts::ShortcutAction::Copy,
                            )),
                        );
                    Self::update_status_hint(status_hint, &copy_response, copy_hint);
                    if copy_response.clicked() {
                        self.state.copy_selected();
                        ui.close_menu();
                    }

                    let paste_label = self.state.i18n.t("menu.edit_paste");
                    let paste_hint = self.hint_with_shortcut(
                        self.state.i18n.t("hint.edit_paste"),
                        crate::shortcuts::ShortcutAction::Paste,
                    );
                    let paste_response = ui.add_enabled(
                        editing_mode,
                        egui::Button::new(self.menu_text(
                            paste_label.clone(),
                            crate::shortcuts::ShortcutAction::Paste,
                        )),
                    );
                    Self::update_status_hint(status_hint, &paste_response, paste_hint);
                    if paste_response.clicked() {
                        self.state.paste_from_clipboard();
                        ui.close_menu();
                    }

                    let delete_label = self.state.i18n.t("menu.edit_delete");
                    let delete_hint = self.hint_with_shortcut(
                        self.state.i18n.t("hint.edit_delete"),
                        crate::shortcuts::ShortcutAction::Delete,
                    );
                    let delete_response = ui.add_enabled(
                        !read_only,
                        egui::Button::new(self.menu_text(
                            delete_label.clone(),
                            crate::shortcuts::ShortcutAction::Delete,
                        )),
                    );
                    Self::update_status_hint(status_hint, &delete_response, delete_hint);
                    if delete_response.clicked() {
                        self.state.delete_selected();
                        ui.close_menu();
                    }

                    ui.separator();

                    let can_finish =
                        self.state.editing_state.mode == crate::state::EditMode::Drawing;
                    let finish_label = self.state.i18n.t("menu.edit_finish_object");
                    let finish_hint = self.hint_with_shortcut(
                        self.state.i18n.t("hint.edit_finish"),
                        crate::shortcuts::ShortcutAction::FinishDrawing,
                    );
                    let finish_response = ui.add_enabled(
                        can_finish,
                        egui::Button::new(self.menu_text(
                            finish_label.clone(),
                            crate::shortcuts::ShortcutAction::FinishDrawing,
                        )),
                    );
                    Self::update_status_hint(status_hint, &finish_response, finish_hint);
                    if finish_response.clicked() {
                        self.finish_drawing();
                        ui.close_menu();
                    }

                    let edit_obj_label = self.state.i18n.t("menu.edit_edit_object");
                    let edit_obj_hint = self.hint_with_shortcut(
                        self.state.i18n.t("hint.edit_edit_object"),
                        crate::shortcuts::ShortcutAction::SwitchToEditingMode,
                    );
                    let edit_obj_response = ui.button(edit_obj_label.clone());
                    Self::update_status_hint(status_hint, &edit_obj_response, edit_obj_hint);
                    if edit_obj_response.clicked() {
                        // Enter Editing mode for selected object
                        if self.state.selected_object_id.is_some() {
                            self.state.editing_state.mode = crate::state::EditMode::Editing;
                            self.state.editing_state.selected_vertex = None;
                            self.state.clear_drawing_state(); // Ensure no temp points
                        }
                        ui.close_menu();
                    }

                    ui.separator();

                    let can_modify_shape = self.state.editing_state.mode
                        == crate::state::EditMode::Editing
                        && self.state.selected_object_id.is_some();
                    let shape_label = self.state.i18n.t("menu.edit_shape");
                    let shape_hint = self.state.i18n.t("hint.menu_edit_shape");
                    let shape_response = ui.menu_button(shape_label.clone(), |ui| {
                        let rect_label = self.state.i18n.t("menu.edit_shape_rect");
                        let rect_hint = self.hint_with_shortcut(
                            self.state.i18n.t("hint.edit_shape_rect"),
                            crate::shortcuts::ShortcutAction::ConvertToRectangle,
                        );
                        let rect_response = ui.add_enabled(
                            can_modify_shape,
                            egui::Button::new(self.menu_text(
                                rect_label.clone(),
                                crate::shortcuts::ShortcutAction::ConvertToRectangle,
                            )),
                        );
                        Self::update_status_hint(status_hint, &rect_response, rect_hint);
                        if rect_response.clicked() {
                            self.convert_selected_to_rectangle();
                            ui.close_menu();
                        }

                        let fix_label = self.state.i18n.t("menu.edit_shape_fix_edges");
                        let fix_hint = self.hint_with_shortcut(
                            self.state.i18n.t("hint.edit_shape_fix"),
                            crate::shortcuts::ShortcutAction::FixSelfIntersection,
                        );
                        let fix_response = ui.add_enabled(
                            can_modify_shape,
                            egui::Button::new(self.menu_text(
                                fix_label.clone(),
                                crate::shortcuts::ShortcutAction::FixSelfIntersection,
                            )),
                        );
                        Self::update_status_hint(status_hint, &fix_response, fix_hint);
                        if fix_response.clicked() {
                            self.fix_selected_self_intersections();
                            ui.close_menu();
                        }
                    });
                    Self::update_status_hint(status_hint, &shape_response.response, shape_hint);

                    ui.separator();

                    let deselect_label = self.state.i18n.t("menu.edit_deselect");
                    let deselect_hint = self.hint_with_shortcut(
                        self.state.i18n.t("hint.edit_deselect"),
                        crate::shortcuts::ShortcutAction::Deselect,
                    );
                    let deselect_response = ui.button(self.menu_text(
                        deselect_label.clone(),
                        crate::shortcuts::ShortcutAction::Deselect,
                    ));
                    Self::update_status_hint(status_hint, &deselect_response, deselect_hint);
                    if deselect_response.clicked() {
                        self.state.selected_object_id = None;
                        self.state.editing_state.selected_vertex = None;
                        ui.close_menu();
                    }

                    let target_label = self.state.i18n.t("menu.edit_draw_target");
                    let target_hint = self.state.i18n.t("hint.menu_edit_draw_target");
                    let target_response = ui.menu_button(target_label.clone(), |ui| {
                        let target_object_label = self.state.i18n.t("menu.edit_draw_target_object");
                        let target_object_hint = self.state.i18n.t("hint.draw_target_object");
                        let target_object_response = ui.selectable_label(
                            self.state.draw_target == crate::state::DrawTarget::Object,
                            target_object_label.clone(),
                        );
                        Self::update_status_hint(
                            status_hint,
                            &target_object_response,
                            target_object_hint,
                        );
                        if target_object_response.clicked() {
                            self.state.draw_target = crate::state::DrawTarget::Object;
                            ui.close_menu();
                        }

                        let target_roi_label = self.state.i18n.t("menu.edit_draw_target_roi");
                        let target_roi_hint = self.state.i18n.t("hint.draw_target_roi");
                        let target_roi_response = ui.selectable_label(
                            self.state.draw_target == crate::state::DrawTarget::Roi,
                            target_roi_label.clone(),
                        );
                        Self::update_status_hint(
                            status_hint,
                            &target_roi_response,
                            target_roi_hint,
                        );
                        if target_roi_response.clicked() {
                            self.state.draw_target = crate::state::DrawTarget::Roi;
                            ui.close_menu();
                        }
                    });
                    Self::update_status_hint(status_hint, &target_response.response, target_hint);
                });
                Self::update_status_hint(status_hint, &edit_menu_response.response, edit_menu_hint);

                // View menu
                let view_menu_label = self.state.i18n.t("menu.view");
                let view_menu_hint = self.state.i18n.t("hint.menu_view");
                let view_menu_response = ui.menu_button(view_menu_label.clone(), |ui| {
                    let fit_label = self.state.i18n.t("menu.view_fit");
                    let fit_hint = self.hint_with_shortcut(
                        self.state.i18n.t("hint.view_fit"),
                        crate::shortcuts::ShortcutAction::FitToCanvas,
                    );
                    let fit_response = ui.button(self.menu_text(
                        fit_label.clone(),
                        crate::shortcuts::ShortcutAction::FitToCanvas,
                    ));
                    Self::update_status_hint(status_hint, &fit_response, fit_hint);
                    if fit_response.clicked() {
                        if let Some(image) = &self.state.current_image {
                            let canvas_size = ctx.screen_rect().size();
                            let image_size = Vec2::new(image.width as f32, image.height as f32);
                            self.canvas.fit_to_canvas(canvas_size, image_size);
                        }
                        ui.close_menu();
                    }

                    let reset_zoom_label = self.state.i18n.t("menu.view_reset_zoom");
                    let reset_zoom_hint = self.hint_with_shortcut(
                        self.state.i18n.t("hint.view_reset_zoom"),
                        crate::shortcuts::ShortcutAction::ResetZoom,
                    );
                    let reset_zoom_response = ui.button(self.menu_text(
                        reset_zoom_label.clone(),
                        crate::shortcuts::ShortcutAction::ResetZoom,
                    ));
                    Self::update_status_hint(status_hint, &reset_zoom_response, reset_zoom_hint);
                    if reset_zoom_response.clicked() {
                        self.canvas.reset_view();
                        ui.close_menu();
                    }

                    let zoom_label = self.state.i18n.t("menu.view_zoom");
                    let zoom_hint = self.state.i18n.t("hint.menu_view_zoom");
                    let zoom_hint_prefix = self.state.i18n.t("hint.zoom_to");
                    let zoom_response = ui.menu_button(zoom_label.clone(), |ui| {
                        let current_zoom = (self.canvas.zoom * 100.0).round();
                        for zoom in super::ZOOM_LEVELS {
                            let label = format!("{:.0}%", zoom);
                            let zoom_item_response =
                                ui.selectable_label((current_zoom - zoom).abs() < 0.5, label);
                            Self::update_status_hint(
                                status_hint,
                                &zoom_item_response,
                                format!("{} {:.0}%", zoom_hint_prefix, zoom),
                            );
                            if zoom_item_response.clicked() {
                                self.set_zoom(zoom);
                                ui.close_menu();
                            }
                        }
                    });
                    Self::update_status_hint(status_hint, &zoom_response.response, zoom_hint);

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
                Self::update_status_hint(status_hint, &view_menu_response.response, view_menu_hint);

                // Navigate menu
                let navigate_menu_label = self.state.i18n.t("menu.navigate");
                let navigate_menu_hint = self.state.i18n.t("hint.menu_navigate");
                let navigate_menu_response = ui.menu_button(navigate_menu_label.clone(), |ui| {
                    let prev_label = self.state.i18n.t("menu.navigate_prev");
                    let prev_hint = self.hint_with_shortcut(
                        self.state.i18n.t("hint.navigate_prev"),
                        crate::shortcuts::ShortcutAction::PreviousImage,
                    );
                    let prev_response = ui.button(self.menu_text(
                        prev_label.clone(),
                        crate::shortcuts::ShortcutAction::PreviousImage,
                    ));
                    Self::update_status_hint(status_hint, &prev_response, prev_hint);
                    if prev_response.clicked() {
                        if self.state.editing_state.mode == crate::state::EditMode::Drawing {
                            self.finish_drawing();
                        }
                        let _ = self.state.prev_image();
                        self.canvas.reset_view();
                        ui.close_menu();
                    }

                    let next_label = self.state.i18n.t("menu.navigate_next");
                    let next_hint = self.hint_with_shortcut(
                        self.state.i18n.t("hint.navigate_next"),
                        crate::shortcuts::ShortcutAction::NextImage,
                    );
                    let next_response = ui.button(self.menu_text(
                        next_label.clone(),
                        crate::shortcuts::ShortcutAction::NextImage,
                    ));
                    Self::update_status_hint(status_hint, &next_response, next_hint);
                    if next_response.clicked() {
                        if self.state.editing_state.mode == crate::state::EditMode::Drawing {
                            self.finish_drawing();
                        }
                        let _ = self.state.next_image();
                        self.canvas.reset_view();
                        ui.close_menu();
                    }

                    ui.separator();

                    let backward_label = self.state.i18n.t("menu.navigate_backward");
                    let backward_hint = self.hint_with_shortcut(
                        self.state.i18n.t("hint.navigate_backward"),
                        crate::shortcuts::ShortcutAction::JumpBackward10,
                    );
                    let backward_response = ui.button(self.menu_text(
                        backward_label.clone(),
                        crate::shortcuts::ShortcutAction::JumpBackward10,
                    ));
                    Self::update_status_hint(status_hint, &backward_response, backward_hint);
                    if backward_response.clicked() {
                        if self.state.editing_state.mode == crate::state::EditMode::Drawing {
                            self.finish_drawing();
                        }
                        let _ = self.state.jump_backward(10);
                        self.canvas.reset_view();
                        ui.close_menu();
                    }

                    let forward_label = self.state.i18n.t("menu.navigate_forward");
                    let forward_hint = self.hint_with_shortcut(
                        self.state.i18n.t("hint.navigate_forward"),
                        crate::shortcuts::ShortcutAction::JumpForward10,
                    );
                    let forward_response = ui.button(self.menu_text(
                        forward_label.clone(),
                        crate::shortcuts::ShortcutAction::JumpForward10,
                    ));
                    Self::update_status_hint(status_hint, &forward_response, forward_hint);
                    if forward_response.clicked() {
                        if self.state.editing_state.mode == crate::state::EditMode::Drawing {
                            self.finish_drawing();
                        }
                        let _ = self.state.jump_forward(10);
                        self.canvas.reset_view();
                        ui.close_menu();
                    }
                });
                Self::update_status_hint(
                    status_hint,
                    &navigate_menu_response.response,
                    navigate_menu_hint,
                );

                // Help menu
                let help_menu_label = self.state.i18n.t("menu.help");
                let help_menu_hint = self.state.i18n.t("hint.menu_help");
                let help_menu_response = ui.menu_button(help_menu_label.clone(), |ui| {
                    // Help - user manual
                    let help_label = self.state.i18n.t("menu.help_help");
                    let help_hint = self.state.i18n.t("hint.help_help");
                    let help_response = ui.button(help_label.clone());
                    Self::update_status_hint(status_hint, &help_response, help_hint);
                    if help_response.clicked() {
                        // Open project documentation on GitHub
                        let _ = open::that("https://github.com/dayn9t/jlab");
                        ui.close_menu();
                    }

                    ui.separator();

                    // Check for updates
                    let updates_label = self.state.i18n.t("menu.help_check_updates");
                    let updates_hint = self.state.i18n.t("hint.help_check_updates");
                    let updates_response = ui.button(updates_label.clone());
                    Self::update_status_hint(status_hint, &updates_response, updates_hint);
                    if updates_response.clicked() {
                        // TODO: Implement update check
                        ui.close_menu();
                    }

                    ui.separator();

                    // About
                    let about_label = self.state.i18n.t("menu.help_about");
                    let about_hint = self.state.i18n.t("hint.help_about");
                    let about_response = ui.button(about_label.clone());
                    Self::update_status_hint(status_hint, &about_response, about_hint);
                    if about_response.clicked() {
                        self.state.show_about_dialog = true;
                        ui.close_menu();
                    }
                });
                Self::update_status_hint(status_hint, &help_menu_response.response, help_menu_hint);
            });
        });
    }
}
