use crate::state::AppState;
use lab_core::Point;

/// Handle drawing tool interactions
pub struct DrawingTools;

impl DrawingTools {
    /// Handle canvas click for drawing
    pub fn handle_click(state: &mut AppState, clicked_pos: Point<f32>) {
        state.temp_points.push(clicked_pos);
    }

    /// Change category of selected object
    pub fn change_selected_category(state: &mut AppState, category_id: i32) {
        if let (Some(label), Some(obj_id)) =
            (&mut state.current_annotation, state.selected_object_id)
        {
            if let Some(obj) = lab_core::find_object_mut(label, obj_id) {
                obj.category = category_id;
                lab_core::touch(label);
                state.has_unsaved_changes = true;
                log::info!("Changed object #{} category to {}", obj_id, category_id);
            }
        }
    }

    /// Cancel current drawing
    pub fn cancel_drawing(state: &mut AppState) {
        if !state.temp_points.is_empty() || !state.pending_draw_clicks.is_empty() {
            state.clear_drawing_state();
            log::info!("Cancelled drawing");
        }
    }
}
