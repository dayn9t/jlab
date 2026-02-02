use crate::export::Exporter;
use crate::{Annotation, Meta, Result};

/// YOLO format exporter
///
/// YOLO format: <class_id> <x_center> <y_center> <width> <height>
/// All coordinates are normalized (0.0 to 1.0)
pub struct YoloExporter;

impl Exporter for YoloExporter {
    fn export_annotation(
        &self,
        annotation: &Annotation,
        _meta: &Meta,
        _image_path: &str,
        _image_width: u32,
        _image_height: u32,
    ) -> Result<String> {
        let mut lines = Vec::new();

        for obj in &annotation.objects {
            if obj.polygon.is_empty() {
                continue;
            }

            // Calculate bounding box from polygon
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

            let width = max_x - min_x;
            let height = max_y - min_y;
            let x_center = min_x + width / 2.0;
            let y_center = min_y + height / 2.0;

            // YOLO format: class_id x_center y_center width height
            lines.push(format!(
                "{} {:.6} {:.6} {:.6} {:.6}",
                obj.category, x_center, y_center, width, height
            ));
        }

        Ok(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::Object;
    use crate::geometry::Point;

    #[test]
    fn test_yolo_export() {
        let mut annotation = Annotation::new("test");

        // Create a simple polygon object
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
            shape: crate::meta::ShapeConfig {
                title_style: 1,
                thickness: 2,
                auto_save: true,
                vertex_radius: 10.0,
            },
            roi: crate::meta::RoiConfig {
                color: "#800080".to_string(),
            },
            categories: vec![],
            property_types: vec![],
            property_special_values: vec![],
        };

        let exporter = YoloExporter;
        let result = exporter
            .export_annotation(&annotation, &meta, "test.jpg", 1920, 1080)
            .unwrap();

        // Expected: class_id=0, x_center=0.3, y_center=0.3, width=0.4, height=0.4
        assert!(result.contains("0 0.300000 0.300000 0.400000 0.400000"));
    }
}
