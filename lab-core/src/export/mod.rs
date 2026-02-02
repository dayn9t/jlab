use crate::{Annotation, Error, Meta, Result};

pub mod coco;
pub mod voc;
pub mod yolo;

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Pascal VOC XML format
    Voc,
    /// YOLO TXT format
    Yolo,
    /// COCO JSON format
    Coco,
}

/// Trait for exporting annotations to different formats
pub trait Exporter {
    /// Export a single annotation
    fn export_annotation(
        &self,
        annotation: &Annotation,
        meta: &Meta,
        image_path: &str,
        image_width: u32,
        image_height: u32,
    ) -> Result<String>;

    /// Export multiple annotations (for formats like COCO that support batch export)
    fn export_batch(
        &self,
        _annotations: &[(String, Annotation, u32, u32)], // (image_path, annotation, width, height)
        _meta: &Meta,
    ) -> Result<String> {
        // Default implementation: not supported
        Err(Error::Export(
            "Batch export not supported for this format".to_string(),
        ))
    }
}
