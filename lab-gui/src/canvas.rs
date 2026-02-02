use egui::{Color32, ColorImage, Pos2, Rect, Sense, Stroke, TextureHandle, Ui, Vec2};
use lab_core::{Annotation, Meta, Point};
use std::collections::HashMap;

/// Pending click for double-click detection
struct PendingClick {
    pos: Pos2,
    time: f64,
}

/// Canvas for displaying and annotating images
pub struct Canvas {
    /// Zoom level (1.0 = 100%)
    pub zoom: f32,

    /// Pan offset in screen coordinates
    pub pan_offset: Vec2,

    /// Texture cache for loaded images
    texture_cache: HashMap<String, TextureHandle>,

    /// Currently dragging vertex (object_id, vertex_index)
    dragging_vertex: Option<(i32, usize)>,

    /// Currently dragging object (object_id, drag_start_pos)
    dragging_object: Option<(i32, Point)>,

    /// Pending click for double-click detection in editing mode
    pending_click: Option<PendingClick>,
}

/// Double-click detection timeout in seconds
const DOUBLE_CLICK_TIMEOUT: f64 = 0.2;

impl Canvas {
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            pan_offset: Vec2::ZERO,
            texture_cache: HashMap::new(),
            dragging_vertex: None,
            dragging_object: None,
            pending_click: None,
        }
    }

    /// Reset view to fit image
    pub fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.pan_offset = Vec2::ZERO;
    }

    /// Set zoom level
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.1, 10.0);
    }

    /// Fit image to canvas
    pub fn fit_to_canvas(&mut self, canvas_size: Vec2, image_size: Vec2) {
        if image_size.x == 0.0 || image_size.y == 0.0 {
            return;
        }

        // Calculate zoom to fit
        let zoom_x = canvas_size.x / image_size.x;
        let zoom_y = canvas_size.y / image_size.y;
        self.zoom = zoom_x.min(zoom_y) * 0.95; // 95% to leave some margin

        // Center the image
        self.pan_offset = Vec2::ZERO;
    }

    /// Show the canvas
    pub fn show(
        &mut self,
        ui: &mut Ui,
        image_data: Option<&crate::state::ImageData>,
        annotation: Option<&Annotation>,
        meta: Option<&Meta>,
        selected_object_id: Option<i32>,
        selected_vertex: Option<(i32, usize)>,
        temp_points: &[Point],
        edit_mode: crate::state::EditMode,
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

            // Get or create texture for the image
            let texture_id = format!("{:?}", image_data.path);
            let texture = self
                .texture_cache
                .entry(texture_id.clone())
                .or_insert_with(|| {
                    // Convert pixels to ColorImage
                    let color_image = ColorImage::from_rgba_unmultiplied(
                        [image_data.width as usize, image_data.height as usize],
                        &image_data.pixels,
                    );
                    // Load texture
                    ui.ctx()
                        .load_texture(&texture_id, color_image, Default::default())
                });

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
                        annotation,
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
            if let Some(annotation) = annotation {
                // Find hovered vertex
                let hovered_vertex = if let Some(pointer_pos) = canvas_response.hover_pos() {
                    self.find_vertex_at_pos(
                        pointer_pos,
                        annotation,
                        selected_object_id,
                        image_rect,
                        image_size,
                    )
                } else {
                    None
                };

                self.draw_annotations(
                    &painter,
                    annotation,
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
            if let Some(annotation) = annotation {
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
                                    annotation,
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
                                        annotation,
                                        selected_object_id,
                                        image_rect,
                                        image_size,
                                    );

                                    if self.dragging_vertex.is_none() {
                                        if let Some(obj_id) = self.find_object_at_pos(
                                            pointer_pos,
                                            annotation,
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
                            let offset = Point::new(
                                current_pos.x - drag_start.x,
                                current_pos.y - drag_start.y,
                            );
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
                    } else if let Some(annotation) = annotation {
                        // First check if double-clicking on a vertex
                        if let Some((obj_id, vertex_idx)) = self.find_vertex_at_pos_any_object(
                            pos, annotation, image_rect, image_size, meta,
                        ) {
                            // Double-clicked on a vertex - select the shape
                            response.vertex_double_clicked = Some((obj_id, vertex_idx));
                        } else {
                            // Not on a vertex, check if double-clicking on an object
                            if let Some(obj_id) =
                                self.find_object_at_pos(pos, annotation, image_rect, image_size)
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
                    if let Some(_annotation) = annotation {
                        // In Drawing mode, place points for new object (no delay needed)
                        if edit_mode == crate::state::EditMode::Drawing {
                            let normalized_pos =
                                self.screen_to_normalized(pos, image_rect, image_size);
                            response.clicked_pos = Some(normalized_pos);
                        } else {
                            // In Editing mode and Browse mode, delay click to detect double-click
                            self.pending_click = Some(PendingClick {
                                pos,
                                time: current_time,
                            });
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

                    if let Some(annotation) = annotation {
                        if edit_mode == crate::state::EditMode::Editing {
                            // Editing mode: vertex or object selection
                            if let Some((obj_id, vertex_idx)) = self.find_vertex_at_pos(
                                pos,
                                annotation,
                                selected_object_id,
                                image_rect,
                                image_size,
                            ) {
                                response.vertex_clicked = Some((obj_id, vertex_idx));
                            } else if let Some(clicked_obj_id) = self.find_smallest_object_at_pos(
                                pos, annotation, image_rect, image_size,
                            ) {
                                response.object_clicked = Some(clicked_obj_id);
                            } else {
                                response.clicked_empty = true;
                            }
                        } else {
                            // Browse mode: object selection only
                            if let Some(clicked_obj_id) =
                                self.find_object_at_pos(pos, annotation, image_rect, image_size)
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
                        if let Some(annotation) = annotation {
                            // Check if right-clicking on vertex to delete
                            if let Some((obj_id, vertex_idx)) = self.find_vertex_at_pos(
                                pos,
                                annotation,
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
                "No image loaded\nOpen a project to start annotating",
                egui::FontId::proportional(16.0),
                Color32::GRAY,
            );
        }

        response
    }

    /// Draw all annotations
    fn draw_annotations(
        &self,
        painter: &egui::Painter,
        annotation: &Annotation,
        meta: Option<&Meta>,
        selected_object_id: Option<i32>,
        hovered_vertex: Option<(i32, usize)>,
        selected_vertex: Option<(i32, usize)>,
        image_rect: Rect,
        image_size: Vec2,
        edit_mode: crate::state::EditMode,
    ) {
        if !annotation.rois.is_empty() {
            let roi_color = meta
                .and_then(|m| parse_color(&m.roi.color))
                .unwrap_or(Color32::from_rgb(128, 0, 128));

            for (idx, roi_points) in annotation.rois.iter().enumerate() {
                let roi_id = crate::state::roi_id_from_index(idx);
                let is_selected = selected_object_id == Some(roi_id);
                let draw_vertices = edit_mode == crate::state::EditMode::Editing && is_selected;

                let hovered_vertex_idx = if let Some((hovered_obj_id, vertex_idx)) = hovered_vertex
                {
                    if hovered_obj_id == roi_id {
                        Some(vertex_idx)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let selected_vertex_idx = if let Some((sel_obj_id, vertex_idx)) = selected_vertex {
                    if sel_obj_id == roi_id {
                        Some(vertex_idx)
                    } else {
                        None
                    }
                } else {
                    None
                };

                self.draw_polygon(
                    painter,
                    roi_points,
                    image_rect,
                    image_size,
                    roi_color,
                    is_selected,
                    draw_vertices,
                    hovered_vertex_idx,
                    selected_vertex_idx,
                    roi_id,
                );
            }
        }

        // Draw objects
        for obj in &annotation.objects {
            let is_selected = selected_object_id == Some(obj.id);
            let draw_vertices = edit_mode == crate::state::EditMode::Editing && is_selected;

            // Check if this object has a hovered vertex
            let hovered_vertex_idx = if let Some((hovered_obj_id, vertex_idx)) = hovered_vertex {
                if hovered_obj_id == obj.id {
                    Some(vertex_idx)
                } else {
                    None
                }
            } else {
                None
            };

            // Check if this object has a selected vertex
            let selected_vertex_idx = if let Some((sel_obj_id, vertex_idx)) = selected_vertex {
                if sel_obj_id == obj.id {
                    Some(vertex_idx)
                } else {
                    None
                }
            } else {
                None
            };

            // Get category color
            let color = meta
                .and_then(|m| m.find_category(obj.category))
                .and_then(|c| parse_color(&c.color))
                .unwrap_or(Color32::RED);

            self.draw_polygon(
                painter,
                &obj.polygon,
                image_rect,
                image_size,
                color,
                is_selected,
                draw_vertices,
                hovered_vertex_idx,
                selected_vertex_idx,
                obj.id,
            );

            // Draw label
            if !obj.polygon.is_empty() {
                let category_name = meta
                    .and_then(|m| m.find_category(obj.category))
                    .map(|c| c.name.as_str())
                    .unwrap_or("Unknown");

                let first_point = &obj.polygon[0];
                let screen_pos = self.normalized_to_screen(first_point, image_rect, image_size);

                let label_text = format!("#{} {}", obj.id, category_name);
                painter.text(
                    screen_pos + Vec2::new(5.0, -5.0),
                    egui::Align2::LEFT_BOTTOM,
                    label_text,
                    egui::FontId::proportional(12.0),
                    Color32::WHITE,
                );
            }
        }
    }

    /// Draw a polygon
    fn draw_polygon(
        &self,
        painter: &egui::Painter,
        points: &[Point],
        image_rect: Rect,
        image_size: Vec2,
        color: Color32,
        is_selected: bool,
        draw_vertices: bool,
        hovered_vertex_idx: Option<usize>,
        selected_vertex_idx: Option<usize>,
        _obj_id: i32,
    ) {
        if points.is_empty() {
            return;
        }

        // Convert normalized coordinates to screen coordinates
        let screen_points: Vec<Pos2> = points
            .iter()
            .map(|p| self.normalized_to_screen(p, image_rect, image_size))
            .collect();

        // Draw polygon edges
        let stroke_width = if is_selected { 3.0 } else { 2.0 };
        let stroke = Stroke::new(stroke_width, color);

        for i in 0..screen_points.len() {
            let start = screen_points[i];
            let end = screen_points[(i + 1) % screen_points.len()];
            painter.line_segment([start, end], stroke);
        }

        // Draw vertices
        if draw_vertices {
            for (i, point) in screen_points.iter().enumerate() {
                let is_hovered = hovered_vertex_idx == Some(i);
                let is_selected_vertex = selected_vertex_idx == Some(i);

                // Determine radius and color
                let (radius, vertex_color, is_filled) = if is_selected_vertex {
                    // Selected vertex: yellow filled circle, larger
                    (8.0, Color32::YELLOW, true)
                } else if is_hovered {
                    // Hovered vertex: yellow filled circle
                    (8.0, Color32::YELLOW, true)
                } else if is_selected {
                    // Shape is selected: all vertices filled with shape color
                    (6.0, color, true)
                } else {
                    // Normal vertex: hollow circle
                    (4.0, color, false)
                };

                if is_filled {
                    painter.circle_filled(*point, radius, vertex_color);
                }
                painter.circle_stroke(*point, radius, Stroke::new(2.0, Color32::WHITE));
            }
        }
    }

    /// Draw temporary points (while drawing)
    fn draw_temp_points(
        &self,
        painter: &egui::Painter,
        points: &[Point],
        image_rect: Rect,
        image_size: Vec2,
    ) {
        if points.is_empty() {
            return;
        }

        let color = Color32::YELLOW;
        let screen_points: Vec<Pos2> = points
            .iter()
            .map(|p| self.normalized_to_screen(p, image_rect, image_size))
            .collect();

        // Draw lines between points
        for i in 0..screen_points.len().saturating_sub(1) {
            painter.line_segment(
                [screen_points[i], screen_points[i + 1]],
                Stroke::new(2.0, color),
            );
        }

        // Draw vertices
        for point in &screen_points {
            painter.circle_filled(*point, 3.0, color);
            painter.circle_stroke(*point, 3.0, Stroke::new(1.0, Color32::WHITE));
        }

        // Draw line from last point to first (for preview)
        if screen_points.len() > 2 {
            painter.line_segment(
                [*screen_points.last().unwrap(), screen_points[0]],
                Stroke::new(1.0, color.linear_multiply(0.5)),
            );
        }
    }

    /// Convert normalized coordinates (0.0-1.0) to screen coordinates
    fn normalized_to_screen(&self, point: &Point, image_rect: Rect, _image_size: Vec2) -> Pos2 {
        let x = image_rect.left() + point.x * image_rect.width();
        let y = image_rect.top() + point.y * image_rect.height();
        Pos2::new(x, y)
    }

    /// Convert screen coordinates to normalized coordinates (0.0-1.0)
    fn screen_to_normalized(&self, pos: Pos2, image_rect: Rect, _image_size: Vec2) -> Point {
        let x = ((pos.x - image_rect.left()) / image_rect.width()).clamp(0.0, 1.0);
        let y = ((pos.y - image_rect.top()) / image_rect.height()).clamp(0.0, 1.0);
        Point::new(x, y)
    }

    fn polygon_area(points: &[Point]) -> f32 {
        if points.len() < 3 {
            return 0.0;
        }

        let mut sum = 0.0;
        for i in 0..points.len() {
            let j = (i + 1) % points.len();
            sum += points[i].x * points[j].y;
            sum -= points[j].x * points[i].y;
        }

        (sum / 2.0).abs()
    }

    /// Find vertex near mouse position
    /// Returns (object_id, vertex_index) if found
    fn find_vertex_at_pos(
        &self,
        pos: Pos2,
        annotation: &Annotation,
        selected_object_id: Option<i32>,
        image_rect: Rect,
        image_size: Vec2,
    ) -> Option<(i32, usize)> {
        let threshold = 14.0; // pixels (expanded for easier selection)

        // Only check selected object
        if let Some(selected_id) = selected_object_id {
            if let Some(roi_index) = crate::state::roi_index_from_id(selected_id) {
                if let Some(roi_points) = annotation.rois.get(roi_index) {
                    for (i, point) in roi_points.iter().enumerate() {
                        let screen_pos = self.normalized_to_screen(point, image_rect, image_size);
                        let distance = pos.distance(screen_pos);
                        if distance < threshold {
                            return Some((selected_id, i));
                        }
                    }
                }
            } else if let Some(obj) = annotation.objects.iter().find(|o| o.id == selected_id) {
                for (i, point) in obj.polygon.iter().enumerate() {
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
        annotation: &Annotation,
        image_rect: Rect,
        image_size: Vec2,
        meta: Option<&Meta>,
    ) -> Option<(i32, usize)> {
        let threshold = meta.map(|m| m.shape.vertex_radius).unwrap_or(10.0); // pixels

        // Check all objects (in reverse order for top-to-bottom priority)
        for obj in annotation.objects.iter().rev() {
            for (i, point) in obj.polygon.iter().enumerate() {
                let screen_pos = self.normalized_to_screen(point, image_rect, image_size);
                let distance = pos.distance(screen_pos);
                if distance < threshold {
                    return Some((obj.id, i));
                }
            }
        }

        for (idx, roi_points) in annotation.rois.iter().enumerate().rev() {
            for (i, point) in roi_points.iter().enumerate() {
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
        annotation: &Annotation,
        selected_object_id: Option<i32>,
        image_rect: Rect,
        image_size: Vec2,
    ) -> Option<(i32, usize)> {
        let threshold = 0.015; // normalized coordinates threshold (expanded)
        let normalized_pos = self.screen_to_normalized(pos, image_rect, image_size);

        // Only check selected object
        if let Some(selected_id) = selected_object_id {
            if let Some(roi_index) = crate::state::roi_index_from_id(selected_id) {
                if let Some(roi_points) = annotation.rois.get(roi_index) {
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
            } else if let Some(obj) = annotation.objects.iter().find(|o| o.id == selected_id) {
                for i in 0..obj.polygon.len() {
                    let p1 = &obj.polygon[i];
                    let p2 = &obj.polygon[(i + 1) % obj.polygon.len()];

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
        annotation: &Annotation,
        image_rect: Rect,
        image_size: Vec2,
    ) -> Option<i32> {
        let normalized_pos = self.screen_to_normalized(pos, image_rect, image_size);

        // Check objects in reverse order (top to bottom)
        for obj in annotation.objects.iter().rev() {
            if crate::geometry::point_in_polygon(&normalized_pos, &obj.polygon) {
                return Some(obj.id);
            }
        }

        for (idx, roi_points) in annotation.rois.iter().enumerate().rev() {
            if crate::geometry::point_in_polygon(&normalized_pos, roi_points) {
                return Some(crate::state::roi_id_from_index(idx));
            }
        }

        None
    }

    /// Find the smallest-area object at mouse position
    fn find_smallest_object_at_pos(
        &self,
        pos: Pos2,
        annotation: &Annotation,
        image_rect: Rect,
        image_size: Vec2,
    ) -> Option<i32> {
        let normalized_pos = self.screen_to_normalized(pos, image_rect, image_size);
        let mut best: Option<(i32, f32)> = None;
        let eps = 1e-6;

        for obj in annotation.objects.iter().rev() {
            if crate::geometry::point_in_polygon(&normalized_pos, &obj.polygon) {
                let area = Self::polygon_area(&obj.polygon);
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

        for (idx, roi_points) in annotation.rois.iter().enumerate().rev() {
            if crate::geometry::point_in_polygon(&normalized_pos, roi_points) {
                let area = Self::polygon_area(roi_points);
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
        annotation: Option<&Annotation>,
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
            if let Some(annotation) = annotation {
                // Check for edge (to add vertex)
                if self
                    .find_edge_at_pos(pos, annotation, selected_object_id, image_rect, image_size)
                    .is_some()
                {
                    return egui::CursorIcon::Crosshair;
                }

                // Check for vertex
                if self
                    .find_vertex_at_pos(pos, annotation, selected_object_id, image_rect, image_size)
                    .is_some()
                {
                    return egui::CursorIcon::Grab;
                }

                // Check for object (selected only)
                if let Some(obj_id) =
                    self.find_object_at_pos(pos, annotation, image_rect, image_size)
                {
                    if Some(obj_id) == selected_object_id {
                        return egui::CursorIcon::Grab;
                    }
                }
            }
        }

        // Default cursor
        egui::CursorIcon::Default
    }

    fn segment_parameter(a: &Point, b: &Point, p: &Point) -> f32 {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let denom = dx * dx + dy * dy;
        if denom.abs() <= f32::EPSILON {
            return 0.0;
        }

        ((p.x - a.x) * dx + (p.y - a.y) * dy) / denom
    }

    /// Draw dashed line (helper function for crosshair)
    fn draw_dashed_line(&self, painter: &egui::Painter, start: Pos2, end: Pos2, color: Color32) {
        let dash_length = 5.0;
        let gap_length = 3.0;
        let total_length = start.distance(end);

        if total_length == 0.0 {
            return;
        }

        let direction = (end - start) / total_length;
        let mut current_pos = start;
        let mut distance = 0.0;
        let mut is_dash = true;

        while distance < total_length {
            let segment_length = if is_dash { dash_length } else { gap_length };
            let next_distance = (distance + segment_length).min(total_length);
            let next_pos = start + direction * next_distance;

            if is_dash {
                painter.line_segment([current_pos, next_pos], Stroke::new(1.0, color));
            }

            current_pos = next_pos;
            distance = next_distance;
            is_dash = !is_dash;
        }
    }

    /// Draw crosshair at mouse position (for drawing mode)
    fn draw_crosshair(&self, painter: &egui::Painter, pos: Pos2, rect: Rect) {
        let color = Color32::from_rgba_unmultiplied(255, 255, 255, 128);

        // Vertical dashed line
        self.draw_dashed_line(
            painter,
            Pos2::new(pos.x, rect.top()),
            Pos2::new(pos.x, rect.bottom()),
            color,
        );

        // Horizontal dashed line
        self.draw_dashed_line(
            painter,
            Pos2::new(rect.left(), pos.y),
            Pos2::new(rect.right(), pos.y),
            color,
        );
    }
}

/// Response from canvas interaction
#[derive(Default)]
pub struct CanvasResponse {
    /// Clicked position in normalized coordinates
    pub clicked_pos: Option<Point>,

    /// Right click occurred
    pub right_clicked: bool,

    /// Canvas bounds in screen coordinates
    pub canvas_rect: Option<Rect>,

    /// Vertex dragged (object_id, vertex_index, new_position)
    pub vertex_dragged: Option<(i32, usize, Point)>,

    /// Object dragged (object_id, offset)
    pub object_dragged: Option<(i32, Point)>,

    /// Vertex added (object_id, edge_index, new_position)
    pub vertex_added: Option<(i32, usize, Point)>,

    /// Vertex deleted (object_id, vertex_index)
    pub vertex_deleted: Option<(i32, usize)>,

    /// Object clicked (for selection)
    pub object_clicked: Option<i32>,

    /// Clicked on empty area
    pub clicked_empty: bool,

    /// Vertex clicked (for selection)
    pub vertex_clicked: Option<(i32, usize)>,

    /// Vertex double-clicked (select shape)
    pub vertex_double_clicked: Option<(i32, usize)>,

    /// Object double-clicked (enter edit mode)
    pub object_double_clicked: Option<i32>,

    /// Double-clicked in Drawing mode to finish drawing
    pub finish_drawing: bool,

    /// Double-clicked position in Drawing mode (normalized)
    pub finish_drawing_pos: Option<Point>,
}

/// Parse color string (hex format like "#FF0000")
fn parse_color(color_str: &str) -> Option<Color32> {
    if !color_str.starts_with('#') || color_str.len() != 7 {
        return None;
    }

    let r = u8::from_str_radix(&color_str[1..3], 16).ok()?;
    let g = u8::from_str_radix(&color_str[3..5], 16).ok()?;
    let b = u8::from_str_radix(&color_str[5..7], 16).ok()?;

    Some(Color32::from_rgb(r, g, b))
}
