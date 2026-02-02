use crate::export::Exporter;
use crate::{Annotation, Meta, Result};
use chrono::Datelike;
use serde::{Deserialize, Serialize};

/// COCO JSON format exporter
pub struct CocoExporter;

#[derive(Debug, Serialize, Deserialize)]
struct CocoDataset {
    info: CocoInfo,
    licenses: Vec<CocoLicense>,
    images: Vec<CocoImage>,
    annotations: Vec<CocoAnnotation>,
    categories: Vec<CocoCategory>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CocoInfo {
    description: String,
    version: String,
    year: i32,
    contributor: String,
    date_created: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CocoLicense {
    id: i32,
    name: String,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CocoImage {
    id: i32,
    width: u32,
    height: u32,
    file_name: String,
    license: i32,
    date_captured: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CocoAnnotation {
    id: i32,
    image_id: i32,
    category_id: i32,
    segmentation: Vec<Vec<f32>>,
    area: f32,
    bbox: Vec<f32>, // [x, y, width, height]
    iscrowd: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct CocoCategory {
    id: i32,
    name: String,
    supercategory: String,
}

impl Exporter for CocoExporter {
    fn export_annotation(
        &self,
        _annotation: &Annotation,
        _meta: &Meta,
        _image_path: &str,
        _image_width: u32,
        _image_height: u32,
    ) -> Result<String> {
        Err(crate::Error::Export(
            "COCO format requires batch export. Use export_batch instead.".to_string(),
        ))
    }

    fn export_batch(
        &self,
        annotations: &[(String, Annotation, u32, u32)],
        meta: &Meta,
    ) -> Result<String> {
        let now = chrono::Utc::now();

        let info = CocoInfo {
            description: meta.description.clone(),
            version: "1.0".to_string(),
            year: now.year(),
            contributor: meta.name.clone(),
            date_created: now.format("%Y-%m-%d").to_string(),
        };

        let licenses = vec![CocoLicense {
            id: 1,
            name: "Unknown".to_string(),
            url: "".to_string(),
        }];

        let mut images = Vec::new();
        let mut coco_annotations = Vec::new();
        let mut annotation_id = 0;

        for (image_id, (image_path, annotation, width, height)) in annotations.iter().enumerate() {
            let file_name = std::path::Path::new(image_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown.jpg")
                .to_string();

            images.push(CocoImage {
                id: image_id as i32,
                width: *width,
                height: *height,
                file_name,
                license: 1,
                date_captured: annotation
                    .created_at
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
            });

            for obj in &annotation.objects {
                if obj.polygon.is_empty() {
                    continue;
                }

                // Calculate bounding box
                let mut min_x = f32::MAX;
                let mut min_y = f32::MAX;
                let mut max_x = f32::MIN;
                let mut max_y = f32::MIN;

                for point in &obj.polygon {
                    min_x = min_x.min(point.x);
                    min_y = min_y.min(point.y);
                    max_x = max_x.max(point.x);
                    max_y = max_y.max(point.y);
                }

                // Convert to pixel coordinates
                let x = min_x * *width as f32;
                let y = min_y * *height as f32;
                let w = (max_x - min_x) * *width as f32;
                let h = (max_y - min_y) * *height as f32;

                // Convert polygon to COCO segmentation format
                let mut segmentation = Vec::new();
                for point in &obj.polygon {
                    segmentation.push(point.x * *width as f32);
                    segmentation.push(point.y * *height as f32);
                }

                coco_annotations.push(CocoAnnotation {
                    id: annotation_id,
                    image_id: image_id as i32,
                    category_id: obj.category,
                    segmentation: vec![segmentation],
                    area: w * h,
                    bbox: vec![x, y, w, h],
                    iscrowd: 0,
                });

                annotation_id += 1;
            }
        }

        let categories: Vec<CocoCategory> = meta
            .categories
            .iter()
            .map(|cat| CocoCategory {
                id: cat.id,
                name: cat.name.clone(),
                supercategory: "object".to_string(),
            })
            .collect();

        let dataset = CocoDataset {
            info,
            licenses,
            images,
            annotations: coco_annotations,
            categories,
        };

        let json = serde_json::to_string_pretty(&dataset)
            .map_err(|e| crate::Error::Export(format!("JSON serialization error: {}", e)))?;

        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::Object;
    use crate::geometry::Point;
    use crate::meta::{Category, RoiConfig, ShapeConfig};

    #[test]
    fn test_coco_batch_export() {
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
            name: "TestDataset".to_string(),
            description: "Test COCO export".to_string(),
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

        let batch = vec![("test.jpg".to_string(), annotation, 1000, 1000)];

        let exporter = CocoExporter;
        let result = exporter.export_batch(&batch, &meta).unwrap();

        assert!(result.contains("\"images\""));
        assert!(result.contains("\"annotations\""));
        assert!(result.contains("\"categories\""));
    }
}
