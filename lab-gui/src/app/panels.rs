use super::LabApp;
use egui::{Context, RichText, SidePanel, TextStyle};

impl LabApp {
    pub(super) fn show_left_panel(&mut self, ctx: &Context) {
        // Left sidebar - image list and info
        if self.state.show_left_panel {
            SidePanel::left("left_panel")
                .default_width(250.0)
                .resizable(true)
                .show(ctx, |ui| {
                    let heading_size = ui
                        .style()
                        .text_styles
                        .get(&TextStyle::Heading)
                        .map(|font| font.size * 0.9)
                        .unwrap_or(18.0 * 0.9);
                    if self.state.images.is_empty() {
                        ui.label(self.state.i18n.t("sidebar.no_images"));
                        ui.label(self.state.i18n.t("sidebar.open_project_hint"));
                        ui.separator();
                    }

                    if let Some(annotation) = &self.state.current_annotation {
                        ui.label(
                            RichText::new(format!(
                                "{}({})",
                                self.state.i18n.t("sidebar.objects"),
                                annotation.objects.len()
                            ))
                            .size(heading_size)
                            .strong(),
                        );
                    } else {
                        ui.label(
                            RichText::new(self.state.i18n.t("sidebar.objects"))
                                .size(heading_size)
                                .strong(),
                        );
                    }

                    if let Some(annotation) = &self.state.current_annotation {
                        // Collect object info first to avoid borrow issues
                        let objects_info: Vec<_> = annotation
                            .objects
                            .iter()
                            .map(|obj| {
                                let category_name = self
                                    .state
                                    .get_meta()
                                    .and_then(|m| m.find_category(obj.category))
                                    .map(|c| c.name.clone())
                                    .unwrap_or_else(|| "Unknown".to_string());

                                (obj.id, category_name)
                            })
                            .collect();

                        let selected_id = self.state.selected_object_id;

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for (obj_id, category_name) in objects_info {
                                let is_selected = selected_id == Some(obj_id);
                                let text = format!("#{} - {}", obj_id, category_name);

                                if ui.selectable_label(is_selected, text).clicked() {
                                    self.state.selected_object_id = Some(obj_id);
                                    self.state.editing_state.selected_vertex = None;
                                }
                            }
                        });

                        ui.separator();
                        ui.label(
                            RichText::new(format!(
                                "{}({})",
                                self.state.i18n.t("sidebar.rois"),
                                annotation.rois.len()
                            ))
                            .size(heading_size)
                            .strong(),
                        );

                        let selected_id = self.state.selected_object_id;
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for (idx, _roi_points) in annotation.rois.iter().enumerate() {
                                let roi_id = crate::state::roi_id_from_index(idx);
                                let is_selected = selected_id == Some(roi_id);
                                let text = format!("#{} - ROI", idx);

                                if ui.selectable_label(is_selected, text).clicked() {
                                    self.state.selected_object_id = Some(roi_id);
                                    self.state.editing_state.selected_vertex = None;
                                }
                            }
                        });
                    }
                });
        }
    }

    pub(super) fn show_right_panel(&mut self, ctx: &Context) {
        // Right sidebar - property editing panel
        if self.state.show_right_panel {
            SidePanel::right("right_panel")
                .default_width(300.0)
                .resizable(true)
                .show(ctx, |ui| {
                    let heading_size = ui
                        .style()
                        .text_styles
                        .get(&TextStyle::Heading)
                        .map(|font| font.size * 0.9)
                        .unwrap_or(18.0 * 0.9);
                    ui.label(
                        RichText::new(self.state.i18n.t("sidebar.properties"))
                            .size(heading_size)
                            .strong(),
                    );

                // Collect data first to avoid borrow checker issues
                let selected_id = self.state.selected_object_id;
                let selected_object_id = selected_id.filter(|id| *id >= 0);
                let roi_selected = selected_id.and_then(crate::state::roi_index_from_id);
                let meta_clone = self.state.get_meta().cloned();
                let can_edit_properties =
                    self.state.editing_state.mode == crate::state::EditMode::Editing;

                if !can_edit_properties {
                    ui.colored_label(
                        egui::Color32::GRAY,
                        self.state.i18n.t("sidebar.properties_read_only"),
                    );
                    ui.separator();
                }

                if let (Some(annotation), Some(selected_id), Some(meta)) = (
                    &mut self.state.current_annotation,
                    selected_object_id,
                    meta_clone.as_ref(),
                ) {
                    // Single object editing mode
                    if let Some(obj) = annotation.objects.iter_mut().find(|o| o.id == selected_id) {
                        ui.label(format!("Object #{}", obj.id));
                        ui.separator();
                        ui.add_enabled_ui(can_edit_properties, |ui| {
                            // Category selector
                            ui.label(format!("{}:", self.state.i18n.t("sidebar.category")));
                            egui::ScrollArea::vertical()
                                .max_height(160.0)
                                .show(ui, |ui| {
                                    for category in &meta.categories {
                                        let hotkey_suffix = category
                                            .hotkey
                                            .parse::<u32>()
                                            .ok()
                                            .filter(|d| (1..=9).contains(d))
                                            .map(|d| format!(" ({})", d))
                                            .unwrap_or_default();
                                        let item_text =
                                            format!("{}{}", category.name, hotkey_suffix);
                                        if ui
                                            .selectable_label(
                                                obj.category == category.id,
                                                item_text,
                                            )
                                            .clicked()
                                        {
                                            obj.category = category.id;
                                            // Clear properties when category changes
                                            obj.properties.clear();
                                            self.state.has_unsaved_changes = true;
                                        }
                                    }
                                });

                            ui.separator();

                            // Get category to find its properties
                            if let Some(category) = meta.find_category(obj.category) {
                                if category.properties.is_empty() {
                                    ui.label(self.state.i18n.t("sidebar.no_properties"));
                                } else {
                                    egui::ScrollArea::vertical().show(ui, |ui| {
                                        for prop_def in &category.properties {
                                            // Find property type definition
                                            if let Some(prop_type) = meta
                                                .property_types
                                                .iter()
                                                .find(|pt| pt.name == prop_def.property_type)
                                            {
                                                ui.label(format!("{}:", prop_type.name));

                                                // Get current value
                                                let current_value = obj
                                                    .properties
                                                    .get(&prop_def.id.to_string())
                                                    .and_then(|values| values.first())
                                                    .map(|v| v.value);

                                                // Render property values as list
                                                let selected_value = current_value.unwrap_or(-1);

                                                if ui
                                                    .selectable_label(
                                                        selected_value == -1,
                                                        self.state.i18n.t("sidebar.not_set"),
                                                    )
                                                    .clicked()
                                                {
                                                    obj.properties
                                                        .remove(&prop_def.id.to_string());
                                                    self.state.has_unsaved_changes = true;
                                                }

                                                for value in &prop_type.values {
                                                    if ui
                                                        .selectable_label(
                                                            selected_value == value.id,
                                                            &value.name,
                                                        )
                                                        .clicked()
                                                    {
                                                        // Update property value
                                                        obj.properties.insert(
                                                            prop_def.id.to_string(),
                                                            vec![lab_core::PropertyValueWithConfidence {
                                                                value: value.id,
                                                                confidence: 1.0,
                                                            }],
                                                        );
                                                        self.state.has_unsaved_changes = true;
                                                    }
                                                }

                                                if !meta.property_special_values.is_empty() {
                                                    ui.separator();
                                                    ui.label(self.state.i18n.t(
                                                        "sidebar.special_values",
                                                    ));
                                                    for special in &meta.property_special_values {
                                                        if ui
                                                            .selectable_label(
                                                                selected_value == special.id,
                                                                &special.name,
                                                            )
                                                            .clicked()
                                                        {
                                                            // Update property value
                                                            obj.properties.insert(
                                                                prop_def.id.to_string(),
                                                                vec![lab_core::PropertyValueWithConfidence {
                                                                    value: special.id,
                                                                    confidence: 1.0,
                                                                }],
                                                            );
                                                            self.state.has_unsaved_changes = true;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    });
                                }
                            }
                        });
                    } else {
                        ui.label(self.state.i18n.t("sidebar.no_object_selected"));
                    }
                } else if roi_selected.is_some() {
                    ui.label(self.state.i18n.t("sidebar.roi_no_properties"));
                } else {
                    ui.label(self.state.i18n.t("sidebar.select_to_edit"));
                }
            });
        }
    }
}
