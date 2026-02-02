//! lab-core: Core data structures for image annotation
//!
//! This crate provides the fundamental data structures for image annotation,
//! including metadata, annotation results, and geometry primitives.

pub mod annotation;
pub mod error;
pub mod export;
pub mod geometry;
pub mod io;
pub mod meta;

pub use annotation::{Annotation, Object, PropertyValueWithConfidence};
pub use error::{Error, Result};
pub use geometry::{Point, Polygon};
pub use meta::{
    Category, Meta, PropertySpecialValue, PropertyType, PropertyValue, RoiConfig, ShapeConfig,
};
