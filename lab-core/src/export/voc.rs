use crate::export::Exporter;
use crate::{Annotation, Meta, Result};
use std::path::Path;

/// Pascal VOC XML format exporter
pub struct VocExporter;

impl Exporter for VocExporter {
    fn export_annotation(
        &self,
        annotation: &Annotation,
        meta: &Meta,
        image_path: &str,
        image_width: u32,
        image_height: u32,
    ) -> Result<String> {
        let filename = Path::new(image_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.jpg");

        let folder = Path::new(image_path)
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("images");

        let mut xml = String::new();
        xml.push_str("<annotation>\n");
        xml.push_str(&format!("  <folder>{}</folder>\n", folder));
        xml.push_str(&format!("  <filename>{}</filename>\n", filename));
        xml.push_str(&format!("  <path>{}</path>\n", image_path));
        xml.push_str("  <source>\n");
        xml.push_str(&format!("    <database>{}</database>\n", meta.name));
        xml.push_str("  </source>\n");
        xml.push_str("  <size>\n");
        xml.push_str(&format!("    <width>{}</width>\n", image_width));
        xml.push_str(&format!("    <height>{}</height>\n", image_height));
        xml.push_str("    <depth>3</depth>\n");
        xml.push_str("  </size>\n");
        xml.push_str("  <segmented>0</segmented>\n");

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
            let xmin = (min_x * image_width as f32) as i32;
            let ymin = (min_y * image_height as f32) as i32;
            let xmax = (max_x * image_width as f32) as i32;
            let ymax = (max_y * image_height as f32) as i32;

            // Get category name
            let category_name = meta
                .find_category(obj.category)
                .map(|c| c.name.as_str())
                .unwrap_or("unknown");

            xml.push_str("  <object>\n");
            xml.push_str(&format!("    <name>{}</name>\n", category_name));
            xml.push_str("    <pose>Unspecified</pose>\n");
            xml.push_str("    <truncated>0</truncated>\n");
            xml.push_str("    <difficult>0</difficult>\n");
            xml.push_str("    <bndbox>\n");
            xml.push_str(&format!("      <xmin>{}</xmin>\n", xmin));
            xml.push_str(&format!("      <ymin>{}</ymin>\n", ymin));
            xml.push_str(&format!("      <xmax>{}</xmax>\n", xmax));
            xml.push_str(&format!("      <ymax>{}</ymax>\n", ymax));
            xml.push_str("    </bndbox>\n");
            xml.push_str("  </object>\n");
        }

        xml.push_str("</annotation>\n");

        Ok(xml)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::Object;
    use crate::geometry::Point;
    use crate::meta::{Category, RoiConfig, ShapeConfig};

    #[test]
    fn test_voc_export() {
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

        let exporter = VocExporter;
        let result = exporter
            .export_annotation(&annotation, &meta, "/path/to/test.jpg", 1000, 1000)
            .unwrap();

        assert!(result.contains("<annotation>"));
        assert!(result.contains("<name>person</name>"));
        assert!(result.contains("<width>1000</width>"));
        assert!(result.contains("<xmin>100</xmin>"));
    }
}
