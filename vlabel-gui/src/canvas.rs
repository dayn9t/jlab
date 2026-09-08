use egui::{Color32, Pos2, Rect, Stroke, TextureHandle, Vec2};
use std::collections::HashMap;
use vlabel_core::{Label, LabelMeta, Point};

// Pointer-event handling and hit-testing live in `canvas_interaction.rs`,
// declared here with `#[path]` (same pattern as `shortcut_types.rs`) because
// `main.rs` maps modules at `src/` level; methods stay inherent to Canvas so
// `crate::canvas` paths are unchanged.
#[path = "canvas_interaction.rs"]
mod canvas_interaction;

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
    dragging_object: Option<(i32, Point<f32>)>,

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

    /// Draw all annotations
    fn draw_annotations(
        &self,
        painter: &egui::Painter,
        label: &Label,
        meta: Option<&LabelMeta>,
        selected_object_id: Option<i32>,
        hovered_vertex: Option<(i32, usize)>,
        selected_vertex: Option<(i32, usize)>,
        image_rect: Rect,
        image_size: Vec2,
        edit_mode: crate::state::EditMode,
    ) {
        if !label.rois.is_empty() {
            let roi_color = meta
                .and_then(|m| parse_color(&m.roi.color))
                .unwrap_or(Color32::from_rgb(128, 0, 128));

            for (idx, roi) in label.rois.iter().enumerate() {
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
                    &roi.0,
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
        for obj in &label.objects {
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
                .and_then(|m| vlabel_core::find_category(m, obj.category))
                .and_then(|c| parse_color(&c.color))
                .unwrap_or(Color32::RED);

            self.draw_polygon(
                painter,
                &obj.polygon.0,
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
            if !obj.polygon.0.is_empty() {
                let category_name = meta
                    .and_then(|m| vlabel_core::find_category(m, obj.category))
                    .map(|c| c.name.as_str())
                    .unwrap_or("Unknown");

                let first_point = &obj.polygon.0[0];
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
        points: &[Point<f32>],
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
        let screen_points: Vec<Pos2> =
            points.iter().map(|p| self.normalized_to_screen(p, image_rect, image_size)).collect();

        // Draw polygon edges
        let stroke_width = if is_selected { 3.0_f32 } else { 2.0_f32 };
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
                painter.circle_stroke(*point, radius, Stroke::new(2.0_f32, Color32::WHITE));
            }
        }
    }

    /// Draw temporary points (while drawing)
    fn draw_temp_points(
        &self,
        painter: &egui::Painter,
        points: &[Point<f32>],
        image_rect: Rect,
        image_size: Vec2,
    ) {
        if points.is_empty() {
            return;
        }

        let color = Color32::YELLOW;
        let screen_points: Vec<Pos2> =
            points.iter().map(|p| self.normalized_to_screen(p, image_rect, image_size)).collect();

        // Draw lines between points
        for i in 0..screen_points.len().saturating_sub(1) {
            painter.line_segment(
                [screen_points[i], screen_points[i + 1]],
                Stroke::new(2.0_f32, color),
            );
        }

        // Draw vertices
        for point in &screen_points {
            painter.circle_filled(*point, 3.0, color);
            painter.circle_stroke(*point, 3.0, Stroke::new(1.0_f32, Color32::WHITE));
        }

        // Draw line from last point to first (for preview)
        if screen_points.len() > 2 {
            painter.line_segment(
                [*screen_points.last().unwrap(), screen_points[0]],
                Stroke::new(1.0_f32, color.linear_multiply(0.5)),
            );
        }
    }

    /// Convert normalized coordinates (0.0-1.0) to screen coordinates
    fn normalized_to_screen(
        &self,
        point: &Point<f32>,
        image_rect: Rect,
        _image_size: Vec2,
    ) -> Pos2 {
        let x = image_rect.left() + point.x * image_rect.width();
        let y = image_rect.top() + point.y * image_rect.height();
        Pos2::new(x, y)
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
                painter.line_segment([current_pos, next_pos], Stroke::new(1.0_f32, color));
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
    pub clicked_pos: Option<Point<f32>>,

    /// Right click occurred
    pub right_clicked: bool,

    /// Canvas bounds in screen coordinates
    pub canvas_rect: Option<Rect>,

    /// Vertex dragged (object_id, vertex_index, new_position)
    pub vertex_dragged: Option<(i32, usize, Point<f32>)>,

    /// Object dragged (object_id, offset)
    pub object_dragged: Option<(i32, Point<f32>)>,

    /// Vertex added (object_id, edge_index, new_position)
    pub vertex_added: Option<(i32, usize, Point<f32>)>,

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
    pub finish_drawing_pos: Option<Point<f32>>,
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
