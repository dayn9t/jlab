//! lab-core: Core data structures for image annotation
//!
//! This crate provides the fundamental data structures for image annotation,
//! re-exporting types from v3-types and adding helper functions.

pub mod annotation;
pub mod error;
pub mod export;
pub mod io;

// Re-export annotation types (with helper functions)
pub use annotation::{
    add_object, find_object, find_object_mut, new_label, new_object, next_object_id, remove_object,
    remove_object_property, set_object_property, touch,
};

// Re-export v3-types types used throughout lab-core
pub use v3_types::{
    // Meta types
    CatDef,
    CatProperty,
    // Annotation types
    Label,
    LabelMeta,
    Object,
    // Geometry
    Point,
    Polygon,
    PropDef,
    PropertyEntry,
    RoiConfig,
    ShapeConfig,
    SpecialValue,
    ValueDef,
};

pub use error::{Error, Result};

// --- LabelMeta helper functions ---
// (Cannot impl on v3-types::LabelMeta due to orphan rule)

/// Find a category by ID in LabelMeta.
pub fn find_category(meta: &LabelMeta, id: i32) -> Option<&CatDef> {
    meta.categories.iter().find(|c| c.id == id)
}

/// Find a property type by ID in LabelMeta.
pub fn find_property_type(meta: &LabelMeta, id: i32) -> Option<&PropDef> {
    meta.property_types.iter().find(|pt| pt.id == id)
}

/// Find a special property value by ID in LabelMeta.
pub fn find_special_value(meta: &LabelMeta, id: i32) -> Option<&SpecialValue> {
    meta.property_special_values.iter().find(|sv| sv.id == id)
}

/// Find a property value by ID within a PropDef.
pub fn find_prop_value(prop_def: &PropDef, id: i32) -> Option<&ValueDef> {
    prop_def.values.iter().find(|v| v.id == id)
}
