// Shortcut dispatch and editing operations on the current annotation
use super::LabApp;
use egui::{Context, Vec2};

impl LabApp {
    pub(super) fn with_polygon_mut<F>(
        label: &mut vlabel_core::Label,
        shape_id: i32,
        mutator: F,
    ) -> bool
    where
        F: FnOnce(&mut Vec<vlabel_core::Point<f32>>),
    {
        if let Some(roi_index) = crate::state::roi_index_from_id(shape_id) {
            if let Some(roi) = label.rois.get_mut(roi_index) {
                mutator(&mut roi.0);
                return true;
            }
        } else if let Some(obj) = label.objects.iter_mut().find(|o| o.id == shape_id) {
            mutator(&mut obj.polygon.0);
            return true;
        }

        false
    }

    pub(super) fn handle_shortcuts(&mut self, ctx: &Context) {
        // Handle shortcuts through ShortcutManager
        if let Some(action) =
            self.state.shortcut_manager.handle_input(ctx, self.state.editing_state.mode)
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
                    crate::tools::DrawingTools::change_selected_category(
                        &mut self.state,
                        category_id,
                    );
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
                if let Some(label) = &self.state.current_annotation {
                    if !label.objects.is_empty() {
                        let current_idx = self
                            .state
                            .selected_object_id
                            .and_then(|id| label.objects.iter().position(|o| o.id == id))
                            .unwrap_or(0);

                        let next_idx = (current_idx + 1) % label.objects.len();
                        self.state.selected_object_id = Some(label.objects[next_idx].id);
                    }
                }
            }
            ShortcutAction::CyclePreviousObject => {
                if let Some(label) = &self.state.current_annotation {
                    if !label.objects.is_empty() {
                        let current_idx = self
                            .state
                            .selected_object_id
                            .and_then(|id| label.objects.iter().position(|o| o.id == id))
                            .unwrap_or(0);

                        let prev_idx = if current_idx == 0 {
                            label.objects.len() - 1
                        } else {
                            current_idx - 1
                        };
                        self.state.selected_object_id = Some(label.objects[prev_idx].id);
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
                    crate::tools::DrawingTools::cancel_drawing(&mut self.state);
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

    pub(super) fn move_selected(&mut self, direction: (f32, f32), ctx: &Context) {
        if self.state.editing_state.mode != crate::state::EditMode::Editing {
            return;
        }

        if let Some(image) = &self.state.current_image {
            let shift_held = ctx.input(|i| i.modifiers.shift);
            let move_distance = if shift_held { 10.0 } else { 1.0 };

            // Convert pixel distance to normalized coordinates
            let dx_norm = (move_distance * direction.0) / image.width as f32;
            let dy_norm = (move_distance * direction.1) / image.height as f32;

            if let Some(label) = &mut self.state.current_annotation {
                // Priority 1: Move selected vertex if any
                if let Some((obj_id, vertex_idx)) = self.state.editing_state.selected_vertex {
                    let mut updated = false;
                    Self::with_polygon_mut(label, obj_id, |polygon| {
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
                    Self::with_polygon_mut(label, obj_id, |polygon| {
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

    pub(super) fn apply_pending_draw_clicks(&mut self, now: f64, double_click_delay: f64) {
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
            crate::tools::DrawingTools::handle_click(&mut self.state, position);
        }
    }

    pub(super) fn flush_pending_draw_clicks(&mut self) {
        if self.state.pending_draw_clicks.is_empty() {
            return;
        }

        let pending: Vec<_> =
            self.state.pending_draw_clicks.drain(..).map(|item| item.position).collect();
        for position in pending {
            crate::tools::DrawingTools::handle_click(&mut self.state, position);
        }
    }

    pub(super) fn drop_recent_pending_draw_click(&mut self, now: f64, double_click_delay: f64) {
        if let Some(last) = self.state.pending_draw_clicks.last() {
            if now - last.time <= double_click_delay {
                self.state.pending_draw_clicks.pop();
            }
        }
    }

    /// Finish drawing a new object
    pub(super) fn finish_drawing(&mut self) -> bool {
        self.flush_pending_draw_clicks();

        if self.state.temp_points.len() < 3 {
            log::warn!("Need at least 3 points to create a shape");
            return false;
        }

        if self.state.draw_target == crate::state::DrawTarget::Roi {
            if let Some(label) = &mut self.state.current_annotation {
                label.rois.push(vlabel_core::Polygon::from(self.state.temp_points.clone()));
                vlabel_core::touch(label);
                self.state.has_unsaved_changes = true;
                let roi_id = crate::state::roi_id_from_index(label.rois.len() - 1);
                self.state.selected_object_id = Some(roi_id);
            }

            self.state.clear_drawing_state();
            log::info!("Updated ROI");
            return true;
        }

        let polygon = vlabel_core::Polygon::from(self.state.temp_points.clone());

        // Get next object ID
        let new_id = if let Some(label) = &self.state.current_annotation {
            label.objects.iter().map(|o| o.id).max().unwrap_or(-1) + 1
        } else {
            0
        };

        // Get default category
        let default_category =
            self.state.get_meta().and_then(|m| m.categories.first()).map(|c| c.id).unwrap_or(0);

        // Create new object
        let new_object = vlabel_core::Object {
            id: new_id,
            category: default_category,
            confidence: 1.0,
            polygon,
            properties: Vec::new(),
        };

        // Add to annotation
        if let Some(label) = &mut self.state.current_annotation {
            label.objects.push(new_object);
            self.state.has_unsaved_changes = true;

            // Select the new object
            self.state.selected_object_id = Some(new_id);
        }

        self.state.clear_drawing_state();

        log::info!("Created new object #{}", new_id);
        true
    }

    pub(super) fn open_project_dialog(&mut self) {
        // Use rfd to open a folder picker dialog
        if let Some(path) =
            rfd::FileDialog::new().set_title("Select Project Directory").pick_folder()
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

    pub(super) fn convert_selected_to_rectangle(&mut self) {
        if self.state.editing_state.mode != crate::state::EditMode::Editing {
            return;
        }

        if let (Some(label), Some(obj_id)) =
            (&mut self.state.current_annotation, self.state.selected_object_id)
        {
            let mut updated = false;
            Self::with_polygon_mut(label, obj_id, |polygon| {
                if let Some((min, max)) = crate::geometry::bounding_box(polygon) {
                    *polygon = vec![
                        vlabel_core::Point { x: min.x, y: min.y },
                        vlabel_core::Point { x: max.x, y: min.y },
                        vlabel_core::Point { x: max.x, y: max.y },
                        vlabel_core::Point { x: min.x, y: max.y },
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

    pub(super) fn fix_selected_self_intersections(&mut self) {
        if self.state.editing_state.mode != crate::state::EditMode::Editing {
            return;
        }

        if let (Some(label), Some(obj_id)) =
            (&mut self.state.current_annotation, self.state.selected_object_id)
        {
            let mut updated = false;
            Self::with_polygon_mut(label, obj_id, |polygon| {
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
    pub(super) fn scale_selected_object(&mut self, scale_factor: f32) {
        if self.state.editing_state.mode != crate::state::EditMode::Editing {
            return;
        }

        if let (Some(label), Some(obj_id)) =
            (&mut self.state.current_annotation, self.state.selected_object_id)
        {
            Self::with_polygon_mut(label, obj_id, |polygon| {
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
