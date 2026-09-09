// Canvas pointer interaction: the `show` event routine plus hit-testing
// (vertex/edge/object picking), cursor selection, and the screen<->normalized
// helpers used only by picking. Child module of `canvas` declared via `#[path]`
// (same pattern as `shortcut_types.rs`) because `main.rs` maps modules at `src/`
// level; all methods remain inherent to Canvas so call sites are unchanged.

use super::{Canvas, CanvasResponse, PendingClick, DOUBLE_CLICK_TIMEOUT};
use egui::{Color32, ColorImage, Pos2, Rect, Sense, Ui, Vec2};
use vlabel_core::{Label, LabelMeta, Point};

impl Canvas {
    /// Show the canvas
    pub fn show(
        &mut self,
        ui: &mut Ui,
        image_data: Option<&crate::state::ImageData>,
        label: Option<&Label>,
        meta: Option<&LabelMeta>,
        selected_object_id: Option<i32>,
        selected_vertex: Option<(i32, usize)>,
        temp_points: &[Point<f32>],
        edit_mode: crate::state::EditMode,
        no_image_text: &str,
    ) -> CanvasResponse {
        let mut response = CanvasResponse::default();

        let (rect, canvas_response) =
            ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());

        // Handle panning with middle mouse button or Ctrl+drag
        if canvas_response.dragged_by(egui::PointerButton::Middle)
            || (canvas_response.dragged() && ui.input(|i| i.modifiers.ctrl))
        {
            self.pan_offset += canvas_response.drag_delta();
        }

        // Handle zoom with mouse wheel
        if canvas_response.hovered() {
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 {
                let zoom_delta = scroll_delta * 0.002;
                self.zoom = (self.zoom * (1.0 + zoom_delta)).clamp(0.1, 10.0);
            }
        }

        if let Some(image_data) = image_data {
            let painter = ui.painter_at(rect);

            // Calculate image display bounds
            let image_size = Vec2::new(image_data.width as f32, image_data.height as f32);
            let scaled_size = image_size * self.zoom;

            // Center the image in the canvas
            let center = rect.center();
            let image_rect = Rect::from_center_size(center + self.pan_offset, scaled_size);

            // Draw background - use theme-based color
            let bg_color = ui.ctx().style().visuals.panel_fill;
            painter.rect_filled(rect, 0.0, bg_color);

            // Get or create the display texture: the ROI film is baked into
            // the pixels, so the cache key includes the ROI content — any ROI
            // edit (add/remove/move vertex/paste/lock propagation) produces a
            // new key and re-bakes the film.
            let rois: Vec<vlabel_core::Polygon<f32>> =
                label.map(|l| l.rois.clone()).unwrap_or_default();
            let texture_key = format!("{:?}|{:?}", image_data.path, rois);
            if self.texture_cache.as_ref().is_none_or(|(k, _)| *k != texture_key) {
                let mut pixels = image_data.pixels.clone();
                crate::roi_film::apply_roi_film(
                    &mut pixels,
                    image_data.width as usize,
                    image_data.height as usize,
                    &rois,
                );
                let color_image = ColorImage::from_rgba_unmultiplied(
                    [image_data.width as usize, image_data.height as usize],
                    &pixels,
                );
                let texture = ui.ctx().load_texture(&texture_key, color_image, Default::default());
                self.texture_cache = Some((texture_key, texture));
            }
            let texture = &self.texture_cache.as_ref().expect("texture just ensured").1;

            // Draw the actual image texture
            painter.image(
                texture.id(),
                image_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );

            // Set cursor based on context
            if let Some(pointer_pos) = canvas_response.hover_pos() {
                if image_rect.contains(pointer_pos) {
                    let cursor = self.determine_cursor(
                        pointer_pos,
                        label,
                        selected_object_id,
                        image_rect,
                        image_size,
                        edit_mode,
                        ui.input(|i| i.modifiers.ctrl),
                    );
                    ui.ctx().set_cursor_icon(cursor);
                }
            }

            // Draw annotations
            if let Some(label) = label {
                // Find hovered vertex
                let hovered_vertex = if let Some(pointer_pos) = canvas_response.hover_pos() {
                    self.find_vertex_at_pos(
                        pointer_pos,
                        label,
                        selected_object_id,
                        image_rect,
                        image_size,
                    )
                } else {
                    None
                };

                self.draw_annotations(
                    &painter,
                    label,
                    meta,
                    selected_object_id,
                    hovered_vertex,
                    selected_vertex,
                    image_rect,
                    image_size,
                    edit_mode,
                );
            }

            // Draw temporary points (for drawing new shapes)
            if !temp_points.is_empty() {
                self.draw_temp_points(&painter, temp_points, image_rect, image_size);
            }

            // Draw crosshair in Drawing mode
            if edit_mode == crate::state::EditMode::Drawing {
                if let Some(pointer_pos) = canvas_response.hover_pos() {
                    if image_rect.contains(pointer_pos) {
                        self.draw_crosshair(&painter, pointer_pos, image_rect);
                    }
                }
            }

            // Handle vertex dragging
            if let Some(label) = label {
                if let Some(pointer_pos) = canvas_response.interact_pointer_pos() {
                    if canvas_response.drag_started()
                        && self.dragging_vertex.is_none()
                        && self.dragging_object.is_none()
                    {
                        match edit_mode {
                            crate::state::EditMode::Editing => {
                                // Check edge first (higher priority than vertex)
                                if let Some((obj_id, edge_idx)) = self.find_edge_at_pos(
                                    pointer_pos,
                                    label,
                                    selected_object_id,
                                    image_rect,
                                    image_size,
                                ) {
                                    let normalized_pos = self.screen_to_normalized(
                                        pointer_pos,
                                        image_rect,
                                        image_size,
                                    );
                                    response.vertex_added =
                                        Some((obj_id, edge_idx, normalized_pos));
                                    self.dragging_vertex = Some((obj_id, edge_idx + 1));
                                } else {
                                    self.dragging_vertex = self.find_vertex_at_pos(
                                        pointer_pos,
                                        label,
                                        selected_object_id,
                                        image_rect,
                                        image_size,
                                    );

                                    if self.dragging_vertex.is_none() {
                                        if let Some(obj_id) = self.find_object_at_pos(
                                            pointer_pos,
                                            label,
                                            image_rect,
                                            image_size,
                                        ) {
                                            if Some(obj_id) == selected_object_id {
                                                let normalized_pos = self.screen_to_normalized(
                                                    pointer_pos,
                                                    image_rect,
                                                    image_size,
                                                );
                                                self.dragging_object =
                                                    Some((obj_id, normalized_pos));
                                            }
                                        }
                                    }
                                }
                            }
                            crate::state::EditMode::Browse => {}
                            crate::state::EditMode::Drawing => {}
                        }
                    }

                    // Continue dragging vertex
                    if edit_mode == crate::state::EditMode::Editing
                        && canvas_response.dragged()
                        && self.dragging_vertex.is_some()
                    {
                        if let Some((obj_id, vertex_idx)) = self.dragging_vertex {
                            let new_pos =
                                self.screen_to_normalized(pointer_pos, image_rect, image_size);
                            response.vertex_dragged = Some((obj_id, vertex_idx, new_pos));
                        }
                    }

                    // Continue dragging object
                    if edit_mode == crate::state::EditMode::Editing
                        && canvas_response.dragged()
                        && self.dragging_object.is_some()
                    {
                        if let Some((obj_id, drag_start)) = self.dragging_object {
                            let current_pos =
                                self.screen_to_normalized(pointer_pos, image_rect, image_size);
                            let offset = Point {
                                x: current_pos.x - drag_start.x,
                                y: current_pos.y - drag_start.y,
                            };
                            response.object_dragged = Some((obj_id, offset));
                            // Update drag start for next frame
                            self.dragging_object = Some((obj_id, current_pos));
                        }
                    }

                    // Stop dragging
                    if canvas_response.drag_stopped() {
                        self.dragging_vertex = None;
                        self.dragging_object = None;
                    }
                }
            }

            // Get current time for double-click detection
            let current_time = ui.input(|i| i.time);

            // Handle double click first (before single click to avoid triggering both)
            let is_double_click = canvas_response.double_clicked();

            if is_double_click {
                // Clear pending click since this is a double-click
                self.pending_click = None;

                if let Some(pos) = canvas_response.interact_pointer_pos() {
                    if edit_mode == crate::state::EditMode::Drawing && !temp_points.is_empty() {
                        if image_rect.contains(pos) {
                            response.finish_drawing = true;
                            response.finish_drawing_pos =
                                Some(self.screen_to_normalized(pos, image_rect, image_size));
                        }
                    } else if let Some(label) = label {
                        // First check if double-clicking on a vertex
                        if let Some((obj_id, vertex_idx)) = self
                            .find_vertex_at_pos_any_object(pos, label, image_rect, image_size, meta)
                        {
                            // Double-clicked on a vertex - select the shape
                            response.vertex_double_clicked = Some((obj_id, vertex_idx));
                        } else {
                            // Not on a vertex, check if double-clicking on an object
                            if let Some(obj_id) =
                                self.find_object_at_pos(pos, label, image_rect, image_size)
                            {
                                // Double-clicked on an object - enter edit mode
                                response.object_double_clicked = Some(obj_id);
                            }
                        }
                    }
                }
            }

            // Handle mouse clicks for annotation (only if not dragging and not double-click)
            if canvas_response.clicked()
                && !is_double_click
                && self.dragging_vertex.is_none()
                && self.dragging_object.is_none()
                && image_rect.contains(canvas_response.interact_pointer_pos().unwrap_or_default())
            {
                if let Some(pos) = canvas_response.interact_pointer_pos() {
                    if let Some(_label) = label {
                        // In Drawing mode, place points for new object (no delay needed)
                        if edit_mode == crate::state::EditMode::Drawing {
                            let normalized_pos =
                                self.screen_to_normalized(pos, image_rect, image_size);
                            response.clicked_pos = Some(normalized_pos);
                        } else {
                            // In Editing mode and Browse mode, delay click to detect double-click
                            self.pending_click = Some(PendingClick { pos, time: current_time });
                        }
                    } else {
                        // No annotation
                        if edit_mode == crate::state::EditMode::Drawing {
                            let normalized_pos =
                                self.screen_to_normalized(pos, image_rect, image_size);
                            response.clicked_pos = Some(normalized_pos);
                        } else {
                            response.clicked_empty = true;
                        }
                    }
                }
            }

            // Process pending click after timeout (editing and browse mode)
            if let Some(pending) = &self.pending_click {
                if current_time - pending.time > DOUBLE_CLICK_TIMEOUT {
                    let pos = pending.pos;
                    self.pending_click = None;

                    if let Some(label) = label {
                        if edit_mode == crate::state::EditMode::Editing {
                            // Editing mode: vertex or object selection
                            if let Some((obj_id, vertex_idx)) = self.find_vertex_at_pos(
                                pos,
                                label,
                                selected_object_id,
                                image_rect,
                                image_size,
                            ) {
                                response.vertex_clicked = Some((obj_id, vertex_idx));
                            } else if let Some(clicked_obj_id) =
                                self.find_smallest_object_at_pos(pos, label, image_rect, image_size)
                            {
                                response.object_clicked = Some(clicked_obj_id);
                            } else {
                                response.clicked_empty = true;
                            }
                        } else {
                            // Browse mode: object selection only
                            if let Some(clicked_obj_id) =
                                self.find_object_at_pos(pos, label, image_rect, image_size)
                            {
                                response.object_clicked = Some(clicked_obj_id);
                            } else {
                                response.clicked_empty = true;
                            }
                        }
                    }
                }
            }

            // Handle right click
            if canvas_response.secondary_clicked() {
                if let Some(pos) = canvas_response.interact_pointer_pos() {
                    let mut handled = false;
                    if edit_mode == crate::state::EditMode::Editing {
                        if let Some(label) = label {
                            // Check if right-clicking on vertex to delete
                            if let Some((obj_id, vertex_idx)) = self.find_vertex_at_pos(
                                pos,
                                label,
                                selected_object_id,
                                image_rect,
                                image_size,
                            ) {
                                response.vertex_deleted = Some((obj_id, vertex_idx));
                                handled = true;
                            }
                        }
                    }

                    if !handled {
                        response.right_clicked = true;
                    }
                } else {
                    response.right_clicked = true;
                }
            }

            response.canvas_rect = Some(image_rect);
        } else {
            // No image loaded
            let painter = ui.painter_at(rect);
            let bg_color = ui.ctx().style().visuals.panel_fill;
            painter.rect_filled(rect, 0.0, bg_color);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                no_image_text,
                egui::FontId::proportional(16.0),
                Color32::GRAY,
            );
        }

        response
    }

    /// Convert screen coordinates to normalized coordinates (0.0-1.0)
    fn screen_to_normalized(&self, pos: Pos2, image_rect: Rect, _image_size: Vec2) -> Point<f32> {
        let x = ((pos.x - image_rect.left()) / image_rect.width()).clamp(0.0, 1.0);
        let y = ((pos.y - image_rect.top()) / image_rect.height()).clamp(0.0, 1.0);
        Point { x, y }
    }

    fn polygon_area(points: &[Point<f32>]) -> f32 {
        if points.len() < 3 {
            return 0.0;
        }

        let mut sum = 0.0;
        for i in 0..points.len() {
            let j = (i + 1) % points.len();
            sum += points[i].x * points[j].y;
            sum -= points[j].x * points[i].y;
        }

        (sum / 2.0_f32).abs()
    }

    /// Find vertex near mouse position
    /// Returns (object_id, vertex_index) if found
    fn find_vertex_at_pos(
        &self,
        pos: Pos2,
        label: &Label,
        selected_object_id: Option<i32>,
        image_rect: Rect,
        image_size: Vec2,
    ) -> Option<(i32, usize)> {
        let threshold = 14.0; // pixels (expanded for easier selection)

        // Only check selected object
        if let Some(selected_id) = selected_object_id {
            if let Some(roi_index) = crate::state::roi_index_from_id(selected_id) {
                if let Some(roi) = label.rois.get(roi_index) {
                    for (i, point) in roi.0.iter().enumerate() {
                        let screen_pos = self.normalized_to_screen(point, image_rect, image_size);
                        let distance = pos.distance(screen_pos);
                        if distance < threshold {
                            return Some((selected_id, i));
                        }
                    }
                }
            } else if let Some(obj) = label.objects.iter().find(|o| o.id == selected_id) {
                for (i, point) in obj.polygon.0.iter().enumerate() {
                    let screen_pos = self.normalized_to_screen(point, image_rect, image_size);
                    let distance = pos.distance(screen_pos);
                    if distance < threshold {
                        return Some((obj.id, i));
                    }
                }
            }
        }

        None
    }

    /// Find vertex near mouse position in any object
    /// Returns (object_id, vertex_index) if found
    /// Uses vertex_radius from meta configuration
    fn find_vertex_at_pos_any_object(
        &self,
        pos: Pos2,
        label: &Label,
        image_rect: Rect,
        image_size: Vec2,
        meta: Option<&LabelMeta>,
    ) -> Option<(i32, usize)> {
        let threshold = meta.map(|m| m.shape.vertex_radius).unwrap_or(10.0); // pixels

        // Check all objects (in reverse order for top-to-bottom priority)
        for obj in label.objects.iter().rev() {
            for (i, point) in obj.polygon.0.iter().enumerate() {
                let screen_pos = self.normalized_to_screen(point, image_rect, image_size);
                let distance = pos.distance(screen_pos);
                if distance < threshold {
                    return Some((obj.id, i));
                }
            }
        }

        for (idx, roi) in label.rois.iter().enumerate().rev() {
            for (i, point) in roi.0.iter().enumerate() {
                let screen_pos = self.normalized_to_screen(point, image_rect, image_size);
                let distance = pos.distance(screen_pos);
                if distance < threshold {
                    return Some((crate::state::roi_id_from_index(idx), i));
                }
            }
        }

        None
    }

    /// Find edge near mouse position
    /// Returns (object_id, edge_index) if found
    /// edge_index is the index of the first vertex of the edge
    fn find_edge_at_pos(
        &self,
        pos: Pos2,
        label: &Label,
        selected_object_id: Option<i32>,
        image_rect: Rect,
        image_size: Vec2,
    ) -> Option<(i32, usize)> {
        let threshold = 0.015; // normalized coordinates threshold (expanded)
        let normalized_pos = self.screen_to_normalized(pos, image_rect, image_size);

        // Only check selected object
        if let Some(selected_id) = selected_object_id {
            if let Some(roi_index) = crate::state::roi_index_from_id(selected_id) {
                if let Some(roi) = label.rois.get(roi_index) {
                    let roi_points = &roi.0;
                    for i in 0..roi_points.len() {
                        let p1 = &roi_points[i];
                        let p2 = &roi_points[(i + 1) % roi_points.len()];

                        let t = Self::segment_parameter(p1, p2, &normalized_pos);
                        if !(0.1..=0.9).contains(&t) {
                            continue;
                        }

                        let distance =
                            crate::geometry::point_to_segment_distance(&normalized_pos, p1, p2);
                        if distance < threshold {
                            return Some((selected_id, i));
                        }
                    }
                }
            } else if let Some(obj) = label.objects.iter().find(|o| o.id == selected_id) {
                let obj_points = &obj.polygon.0;
                for i in 0..obj_points.len() {
                    let p1 = &obj_points[i];
                    let p2 = &obj_points[(i + 1) % obj_points.len()];

                    let t = Self::segment_parameter(p1, p2, &normalized_pos);
                    if !(0.1..=0.9).contains(&t) {
                        continue;
                    }

                    let distance =
                        crate::geometry::point_to_segment_distance(&normalized_pos, p1, p2);
                    if distance < threshold {
                        return Some((obj.id, i));
                    }
                }
            }
        }

        None
    }

    /// Find object at mouse position
    /// Returns object_id if found
    fn find_object_at_pos(
        &self,
        pos: Pos2,
        label: &Label,
        image_rect: Rect,
        image_size: Vec2,
    ) -> Option<i32> {
        let normalized_pos = self.screen_to_normalized(pos, image_rect, image_size);

        // Check objects in reverse order (top to bottom)
        for obj in label.objects.iter().rev() {
            if crate::geometry::point_in_polygon(&normalized_pos, &obj.polygon.0) {
                return Some(obj.id);
            }
        }

        for (idx, roi) in label.rois.iter().enumerate().rev() {
            if crate::geometry::point_in_polygon(&normalized_pos, &roi.0) {
                return Some(crate::state::roi_id_from_index(idx));
            }
        }

        None
    }

    /// Find the smallest-area object at mouse position
    fn find_smallest_object_at_pos(
        &self,
        pos: Pos2,
        label: &Label,
        image_rect: Rect,
        image_size: Vec2,
    ) -> Option<i32> {
        let normalized_pos = self.screen_to_normalized(pos, image_rect, image_size);
        let mut best: Option<(i32, f32)> = None;
        let eps = 1e-6;

        for obj in label.objects.iter().rev() {
            if crate::geometry::point_in_polygon(&normalized_pos, &obj.polygon.0) {
                let area = Self::polygon_area(&obj.polygon.0);
                match best {
                    None => best = Some((obj.id, area)),
                    Some((_, best_area)) => {
                        if area + eps < best_area {
                            best = Some((obj.id, area));
                        }
                    }
                }
            }
        }

        for (idx, roi) in label.rois.iter().enumerate().rev() {
            if crate::geometry::point_in_polygon(&normalized_pos, &roi.0) {
                let area = Self::polygon_area(&roi.0);
                match best {
                    None => best = Some((crate::state::roi_id_from_index(idx), area)),
                    Some((_, best_area)) => {
                        if area + eps < best_area {
                            best = Some((crate::state::roi_id_from_index(idx), area));
                        }
                    }
                }
            }
        }

        best.map(|(id, _)| id)
    }

    /// Determine cursor icon based on context
    fn determine_cursor(
        &self,
        pos: Pos2,
        label: Option<&Label>,
        selected_object_id: Option<i32>,
        image_rect: Rect,
        image_size: Vec2,
        edit_mode: crate::state::EditMode,
        ctrl_held: bool,
    ) -> egui::CursorIcon {
        // If dragging, show grabbing cursor
        if self.dragging_vertex.is_some() || self.dragging_object.is_some() {
            return egui::CursorIcon::Grabbing;
        }

        // Ctrl held for panning
        if ctrl_held {
            return egui::CursorIcon::Move;
        }

        if edit_mode == crate::state::EditMode::Drawing {
            return egui::CursorIcon::Crosshair;
        }

        // Check what's under the cursor
        if edit_mode == crate::state::EditMode::Editing {
            if let Some(label) = label {
                // Check for edge (to add vertex)
                if self
                    .find_edge_at_pos(pos, label, selected_object_id, image_rect, image_size)
                    .is_some()
                {
                    return egui::CursorIcon::Crosshair;
                }

                // Check for vertex
                if self
                    .find_vertex_at_pos(pos, label, selected_object_id, image_rect, image_size)
                    .is_some()
                {
                    return egui::CursorIcon::PointingHand;
                }

                // Check for object (selected only)
                if let Some(obj_id) = self.find_object_at_pos(pos, label, image_rect, image_size) {
                    if Some(obj_id) == selected_object_id {
                        return egui::CursorIcon::Grab;
                    }
                }
            }
        }

        // Default cursor
        egui::CursorIcon::Default
    }

    fn segment_parameter(a: &Point<f32>, b: &Point<f32>, p: &Point<f32>) -> f32 {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let denom = dx * dx + dy * dy;
        if denom.abs() <= f32::EPSILON {
            return 0.0;
        }

        ((p.x - a.x) * dx + (p.y - a.y) * dy) / denom
    }
}
