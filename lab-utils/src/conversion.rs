use lab_core::export::{coco::CocoExporter, voc::VocExporter, yolo::YoloExporter, Exporter};
use lab_core::{Annotation, Meta, Result};
use std::fs;
use std::path::Path;

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Voc,
    Yolo,
    Coco,
}

/// Export a single annotation to a file
pub fn export_annotation<P: AsRef<Path>>(
    output_path: P,
    annotation: &Annotation,
    meta: &Meta,
    image_path: &str,
    image_width: u32,
    image_height: u32,
    format: ExportFormat,
) -> Result<()> {
    let content = match format {
        ExportFormat::Voc => {
            let exporter = VocExporter;
            exporter.export_annotation(annotation, meta, image_path, image_width, image_height)?
        }
        ExportFormat::Yolo => {
            let exporter = YoloExporter;
            exporter.export_annotation(annotation, meta, image_path, image_width, image_height)?
        }
        ExportFormat::Coco => {
            return Err(lab_core::Error::Export(
                "COCO format requires batch export".to_string(),
            ));
        }
    };

    fs::write(output_path, content)?;
    Ok(())
}

/// Export multiple annotations in COCO format
pub fn export_coco_batch<P: AsRef<Path>>(
    output_path: P,
    annotations: &[(String, Annotation, u32, u32)],
    meta: &Meta,
) -> Result<()> {
    let exporter = CocoExporter;
    let content = exporter.export_batch(annotations, meta)?;
    fs::write(output_path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lab_core::annotation::Object;
    use lab_core::geometry::Point;
    use lab_core::meta::{Category, RoiConfig, ShapeConfig};

    #[test]
    fn test_export_yolo() {
        let mut annotation = Annotation::new("test");
        let obj = Object::new(
            0,
            0,
            vec![
                Point::new(0.1, 0.1),
                Point::new(0.5, 0.1),
                Point::new(0.5, 0.5),
                Point::new(0.1, 0.5),
            ],
        );
        annotation.add_object(obj);

        let meta = Meta {
            id: 1,
            name: "test".to_string(),
            description: "test".to_string(),
            shape: ShapeConfig {
                title_style: 1,
                thickness: 2,
                auto_save: true,
                vertex_radius: 10.0,
            },
            roi: RoiConfig {
                color: "#800080".to_string(),
            },
            categories: vec![Category {
                id: 0,
                name: "person".to_string(),
                description: "Person".to_string(),
                hotkey: "1".to_string(),
                color: "#FF0000".to_string(),
                properties: vec![],
            }],
            property_types: vec![],
            property_special_values: vec![],
        };

        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_export.txt");

        let result = export_annotation(
            &output_path,
            &annotation,
            &meta,
            "test.jpg",
            1000,
            1000,
            ExportFormat::Yolo,
        );

        assert!(result.is_ok());
        assert!(output_path.exists());

        // Cleanup
        let _ = fs::remove_file(output_path);
    }
}
