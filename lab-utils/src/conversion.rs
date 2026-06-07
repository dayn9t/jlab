use lab_core::export::{coco::CocoExporter, voc::VocExporter, yolo::YoloExporter, Exporter};
use lab_core::{Label, LabelMeta, Result};
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
    annotation: &Label,
    meta: &LabelMeta,
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
    annotations: &[(String, Label, u32, u32)],
    meta: &LabelMeta,
) -> Result<()> {
    let exporter = CocoExporter;
    let content = exporter.export_batch(annotations, meta)?;
    fs::write(output_path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lab_core::{new_label, new_object, CatDef, Point, Polygon, RoiConfig, ShapeConfig};

    #[test]
    fn test_export_yolo() {
        let mut label = new_label("test");
        let obj = new_object(
            0,
            0,
            Polygon::from(vec![
                Point { x: 0.1, y: 0.1 },
                Point { x: 0.5, y: 0.1 },
                Point { x: 0.5, y: 0.5 },
                Point { x: 0.1, y: 0.5 },
            ]),
        );
        lab_core::add_object(&mut label, obj);

        let meta = LabelMeta {
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
            categories: vec![CatDef {
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
            &label,
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
