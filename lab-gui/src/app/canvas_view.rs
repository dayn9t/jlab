use super::LabApp;
use egui::{CentralPanel, Context};

impl LabApp {
    pub(super) fn show_canvas(&mut self, ctx: &Context, cursor_pixel_pos: &mut Option<(i32, i32)>) {
        // Central canvas
        CentralPanel::default().show(ctx, |ui| {
            let now = ctx.input(|i| i.time);
            let double_click_delay = ctx.options(|o| o.input_options.max_double_click_delay);

            let canvas_response = self.canvas.show(
                ui,
                self.state.current_image.as_ref(),
                self.state.current_annotation.as_ref(),
                self.state.get_meta(),
                self.state.selected_object_id,
                self.state.editing_state.selected_vertex,
                &self.state.temp_points,
                self.state.editing_state.mode,
            );

            if let (Some(image), Some(image_rect), Some(pointer_pos)) = (
                self.state.current_image.as_ref(),
                canvas_response.canvas_rect,
                ctx.pointer_latest_pos(),
            ) {
                if image_rect.width() > 0.0
                    && image_rect.height() > 0.0
                    && image_rect.contains(pointer_pos)
                {
                    let x_ratio = (pointer_pos.x - image_rect.min.x) / image_rect.width();
                    let y_ratio = (pointer_pos.y - image_rect.min.y) / image_rect.height();
                    let mut x = (x_ratio * image.width as f32).floor() as i32;
                    let mut y = (y_ratio * image.height as f32).floor() as i32;
                    x = x.clamp(0, image.width.saturating_sub(1) as i32);
                    y = y.clamp(0, image.height.saturating_sub(1) as i32);
                    *cursor_pixel_pos = Some((x, y));
                }
            }

            // Handle vertex clicked (select vertex)
            if let Some((obj_id, vertex_idx)) = canvas_response.vertex_clicked {
                self.state.editing_state.selected_vertex = Some((obj_id, vertex_idx));
                self.state.selected_object_id = Some(obj_id);
            }

            // Handle vertex double-clicked (select shape)
            if let Some((obj_id, _vertex_idx)) = canvas_response.vertex_double_clicked {
                // Select the shape
                self.state.selected_object_id = Some(obj_id);
                // Deselect vertex
                self.state.editing_state.selected_vertex = None;
            }

            // Handle object double-clicked (enter Editing mode)
            if let Some(obj_id) = canvas_response.object_double_clicked {
                // Select the object
                self.state.selected_object_id = Some(obj_id);
                // Enter Editing mode
                self.state.editing_state.mode = crate::state::EditMode::Editing;
                // Deselect vertex (do not select any specific vertex)
                self.state.editing_state.selected_vertex = None;
                // Clear temp points to ensure vertex editing is enabled
                self.state.clear_drawing_state();
                log::info!("Double-clicked object {}, entering Editing mode", obj_id);
            }

            if canvas_response.right_clicked {
                // In Drawing mode, delete last point
                if self.state.editing_state.mode == crate::state::EditMode::Drawing {
                    if self.state.pending_draw_clicks.pop().is_none() {
                        self.state.temp_points.pop();
                    }
                    let remaining =
                        self.state.temp_points.len() + self.state.pending_draw_clicks.len();
                    log::info!("Removed last point, {} points remaining", remaining);
                }
            }

            if canvas_response.finish_drawing {
                if self.state.editing_state.mode == crate::state::EditMode::Drawing {
                    self.drop_recent_pending_draw_click(now, double_click_delay);
                    let finished = self.finish_drawing();
                    if finished && self.state.selected_object_id.is_some() {
                        self.state.editing_state.mode = crate::state::EditMode::Editing;
                        self.state.editing_state.selected_vertex = None;
                    }
                }
            }

            // Handle canvas interactions
            if let Some(clicked_pos) = canvas_response.clicked_pos {
                if !canvas_response.finish_drawing
                    && self.state.editing_state.mode == crate::state::EditMode::Drawing
                {
                    self.state
                        .pending_draw_clicks
                        .push(crate::state::PendingDrawClick {
                            position: clicked_pos,
                            time: now,
                        });
                }
            }

            self.apply_pending_draw_clicks(now, double_click_delay);

            // Handle vertex added
            if let Some((obj_id, edge_idx, new_pos)) = canvas_response.vertex_added {
                if let Some(annotation) = &mut self.state.current_annotation {
                    let mut updated = false;
                    Self::with_polygon_mut(annotation, obj_id, |polygon| {
                        if edge_idx < polygon.len() {
                            polygon.insert(edge_idx + 1, new_pos);
                            updated = true;
                        }
                    });
                    if updated {
                        self.state.has_unsaved_changes = true;
                        log::info!("Added vertex to shape #{} at edge {}", obj_id, edge_idx);
                    }
                }
            }

            // Handle vertex dragging
            if let Some((obj_id, vertex_idx, new_pos)) = canvas_response.vertex_dragged {
                if let Some(annotation) = &mut self.state.current_annotation {
                    let mut updated = false;
                    Self::with_polygon_mut(annotation, obj_id, |polygon| {
                        if vertex_idx < polygon.len() {
                            polygon[vertex_idx] = new_pos;
                            updated = true;
                        }
                    });
                    if updated {
                        self.state.has_unsaved_changes = true;
                    }
                }
            }

            // Handle object dragging
            if let Some((obj_id, offset)) = canvas_response.object_dragged {
                if let Some(annotation) = &mut self.state.current_annotation {
                    let mut updated = false;
                    Self::with_polygon_mut(annotation, obj_id, |polygon| {
                        for vertex in polygon {
                            vertex.x = (vertex.x + offset.x).clamp(0.0, 1.0);
                            vertex.y = (vertex.y + offset.y).clamp(0.0, 1.0);
                        }
                        updated = true;
                    });
                    if updated {
                        self.state.has_unsaved_changes = true;
                    }
                }
            }

            // Handle vertex deleted
            if let Some((obj_id, vertex_idx)) = canvas_response.vertex_deleted {
                if let Some(annotation) = &mut self.state.current_annotation {
                    let mut updated = false;
                    Self::with_polygon_mut(annotation, obj_id, |polygon| {
                        let min_vertices = 3;
                        if polygon.len() > min_vertices && vertex_idx < polygon.len() {
                            polygon.remove(vertex_idx);
                            updated = true;
                        } else {
                            log::warn!(
                                "Cannot delete vertex: minimum {} vertices required",
                                min_vertices
                            );
                        }
                    });
                    if updated {
                        self.state.has_unsaved_changes = true;
                        log::info!("Deleted vertex {} from shape #{}", vertex_idx, obj_id);
                    }
                }
            }

            // Handle object clicked (normal selection)
            if let Some(obj_id) = canvas_response.object_clicked {
                self.state.selected_object_id = Some(obj_id);
                self.state.editing_state.selected_vertex = None;
            }

            if canvas_response.clicked_empty
                && matches!(
                    self.state.editing_state.mode,
                    crate::state::EditMode::Browse | crate::state::EditMode::Editing
                )
            {
                self.state.selected_object_id = None;
                self.state.editing_state.selected_vertex = None;
            }
        });
    }
}
