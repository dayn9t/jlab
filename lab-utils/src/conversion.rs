use crate::Project;
use anyhow::Context;
use image::GenericImageView;
use lab_core::export::{coco::CocoExporter, voc::VocExporter, yolo::YoloExporter, Exporter};
use lab_core::{Label, LabelMeta, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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
            return Err(lab_core::Error::Export("COCO format requires batch export".to_string()));
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

pub struct ExportItem {
    pub image_path: PathBuf,
    pub file_name: String,
    pub stem: String,
    pub width: u32,
    pub height: u32,
    pub annotation: Label,
}

pub fn collect_export_items(project: &Project) -> anyhow::Result<Vec<ExportItem>> {
    let mut items = Vec::new();
    for image_path in project.list_images()? {
        let file_name = image_path
            .file_name()
            .and_then(|s| s.to_str())
            .context("Invalid image name")?
            .to_string();
        let stem = Path::new(&file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&file_name)
            .to_string();

        let (width, height) = image::open(&image_path)
            .with_context(|| format!("Failed to read image {:?}", image_path))?
            .dimensions();

        let annotation =
            project.load_annotation(&file_name)?.unwrap_or_else(|| lab_core::new_label("export"));

        items.push(ExportItem { image_path, file_name, stem, width, height, annotation });
    }
    Ok(items)
}

pub fn export_labelme_annotation(
    output_path: &Path,
    item: &ExportItem,
    meta: &LabelMeta,
) -> anyhow::Result<()> {
    let mut shapes = Vec::new();
    for obj in &item.annotation.objects {
        if obj.polygon.0.len() < 3 {
            continue;
        }
        let label = lab_core::find_category(meta, obj.category)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let points = obj
            .polygon
            .0
            .iter()
            .map(|point| {
                vec![(point.x * item.width as f32) as f64, (point.y * item.height as f32) as f64]
            })
            .collect();

        shapes.push(LabelMeShapeOut {
            label,
            points,
            group_id: None,
            shape_type: "polygon".to_string(),
            flags: HashMap::new(),
        });
    }

    let labelme = LabelMeOut {
        version: "5.0.1".to_string(),
        flags: HashMap::new(),
        shapes,
        image_path: item.file_name.clone(),
        image_data: None,
        image_height: item.height,
        image_width: item.width,
    };

    let content = serde_json::to_string_pretty(&labelme)?;
    fs::write(output_path, content)?;
    Ok(())
}

#[derive(Serialize)]
struct LabelMeOut {
    version: String,
    flags: HashMap<String, serde_json::Value>,
    shapes: Vec<LabelMeShapeOut>,
    #[serde(rename = "imagePath")]
    image_path: String,
    #[serde(rename = "imageData")]
    image_data: Option<String>,
    #[serde(rename = "imageHeight")]
    image_height: u32,
    #[serde(rename = "imageWidth")]
    image_width: u32,
}

#[derive(Serialize)]
struct LabelMeShapeOut {
    label: String,
    points: Vec<Vec<f64>>,
    #[serde(rename = "group_id")]
    group_id: Option<i32>,
    #[serde(rename = "shape_type")]
    shape_type: String,
    flags: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::tests::test_meta;
    use crate::Project;
    use lab_core::io::save_meta;
    use lab_core::{new_label, new_object, CatDef, Point, Polygon, RoiConfig, ShapeConfig};
    use std::fs;

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
            roi: RoiConfig { color: "#800080".to_string() },
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

    #[test]
    fn collect_export_items_reads_dimensions_and_default_label() {
        let root = std::env::temp_dir().join(format!("lab-utils-cv-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("images")).unwrap();
        fs::create_dir_all(root.join("labels")).unwrap();
        save_meta(root.join("meta.yaml"), &test_meta()).unwrap();
        // real 4x2 png so image::open works
        image::RgbImage::from_pixel(4, 2, image::Rgb([0, 0, 0]))
            .save(root.join("images/p.png"))
            .unwrap();

        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!((items[0].width, items[0].height), (4, 2));
        assert_eq!(items[0].annotation.objects.len(), 0); // no yaml -> empty label
        let _ = fs::remove_dir_all(&root);
    }
}
