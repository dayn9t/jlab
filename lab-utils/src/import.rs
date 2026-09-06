//! 外部数据集格式导入（YOLO / VOC / COCO / LabelMe → JLab 项目）。

use anyhow::Context;
use lab_core::{Label, LabelMeta, Object, Point, Polygon};
use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ImportedImage {
    pub source_path: PathBuf,
    pub file_name: String,
    pub annotation: Label,
}

pub fn import_from_yolo(root: &Path, meta: &LabelMeta) -> anyhow::Result<Vec<ImportedImage>> {
    let images_dir = root.join("images");
    let labels_dir = root.join("labels");
    if !images_dir.exists() || !labels_dir.exists() {
        return Err(anyhow::anyhow!("YOLO root must contain images/ and labels/ directories"));
    }

    let image_paths = list_images_in_dir(&images_dir)?;
    let mut imported = Vec::new();

    for image_path in image_paths {
        let file_name = image_path
            .file_name()
            .and_then(|s| s.to_str())
            .context("Invalid image name")?
            .to_string();
        let stem = Path::new(&file_name).file_stem().and_then(|s| s.to_str()).unwrap_or(&file_name);
        let label_path = labels_dir.join(format!("{}.txt", stem));

        let mut objects = Vec::new();
        if label_path.exists() {
            let content = fs::read_to_string(&label_path)?;
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() != 5 {
                    continue;
                }
                let class_id: i32 = match parts[0].parse() {
                    Ok(val) => val,
                    Err(_) => continue,
                };
                if lab_core::find_category(meta, class_id).is_none() {
                    log::warn!("Unknown category id {} in {:?}", class_id, label_path);
                    continue;
                }
                let x_center: f32 = match parts[1].parse() {
                    Ok(val) => val,
                    Err(_) => continue,
                };
                let y_center: f32 = match parts[2].parse() {
                    Ok(val) => val,
                    Err(_) => continue,
                };
                let width: f32 = match parts[3].parse() {
                    Ok(val) => val,
                    Err(_) => continue,
                };
                let height: f32 = match parts[4].parse() {
                    Ok(val) => val,
                    Err(_) => continue,
                };

                let xmin = x_center - width / 2.0;
                let ymin = y_center - height / 2.0;
                let xmax = x_center + width / 2.0;
                let ymax = y_center + height / 2.0;
                let polygon = rect_polygon(xmin, ymin, xmax, ymax);
                if !polygon.is_valid() {
                    continue;
                }
                objects.push(lab_core::new_object(0, class_id, polygon));
            }
        }

        let annotation = build_label(objects, "import");
        imported.push(ImportedImage { source_path: image_path, file_name, annotation });
    }

    Ok(imported)
}

pub fn import_from_voc(root: &Path, meta: &LabelMeta) -> anyhow::Result<Vec<ImportedImage>> {
    let images_dir = root.join("JPEGImages");
    let labels_dir = root.join("Annotations");
    if !images_dir.exists() || !labels_dir.exists() {
        return Err(anyhow::anyhow!(
            "VOC root must contain JPEGImages/ and Annotations/ directories"
        ));
    }

    let image_paths = list_images_in_dir(&images_dir)?;
    let mut imported = Vec::new();

    for image_path in image_paths {
        let file_name = image_path
            .file_name()
            .and_then(|s| s.to_str())
            .context("Invalid image name")?
            .to_string();
        let stem = Path::new(&file_name).file_stem().and_then(|s| s.to_str()).unwrap_or(&file_name);
        let label_path = labels_dir.join(format!("{}.xml", stem));

        let mut objects = Vec::new();
        if label_path.exists() {
            let xml = fs::read_to_string(&label_path)?;
            if let Some((width, height)) = parse_voc_size(&xml) {
                let voc_objects = parse_voc_objects(&xml);
                for voc_obj in voc_objects {
                    let category_id = match find_category_id_by_name(meta, &voc_obj.label) {
                        Some(id) => id,
                        None => {
                            log::warn!(
                                "Unknown category name {} in {:?}",
                                voc_obj.label,
                                label_path
                            );
                            continue;
                        }
                    };

                    let xmin = voc_obj.xmin / width;
                    let ymin = voc_obj.ymin / height;
                    let xmax = voc_obj.xmax / width;
                    let ymax = voc_obj.ymax / height;
                    let polygon = rect_polygon(xmin, ymin, xmax, ymax);
                    if !polygon.is_valid() {
                        continue;
                    }
                    objects.push(lab_core::new_object(0, category_id, polygon));
                }
            }
        }

        let annotation = build_label(objects, "import");
        imported.push(ImportedImage { source_path: image_path, file_name, annotation });
    }

    Ok(imported)
}

pub fn import_from_coco(
    json_path: &Path,
    images_dir: &Path,
    meta: &LabelMeta,
) -> anyhow::Result<Vec<ImportedImage>> {
    let content = fs::read_to_string(json_path)?;
    let dataset: CocoDataset = serde_json::from_str(&content)?;

    let mut category_map = HashMap::new();
    for cat in &dataset.categories {
        if lab_core::find_category(meta, cat.id).is_some() {
            category_map.insert(cat.id, cat.id);
        } else if let Some(id) = find_category_id_by_name(meta, &cat.name) {
            category_map.insert(cat.id, id);
        }
    }

    let mut annotations_by_image: HashMap<i32, Vec<CocoAnnotation>> = HashMap::new();
    for ann in dataset.annotations {
        annotations_by_image.entry(ann.image_id).or_default().push(ann);
    }

    let mut imported = Vec::new();
    for image in dataset.images {
        let file_name = Path::new(&image.file_name)
            .file_name()
            .and_then(|s| s.to_str())
            .context("Invalid image name")?
            .to_string();
        let source_path = images_dir.join(&image.file_name);
        if !source_path.exists() {
            return Err(anyhow::anyhow!(format!("Missing image file: {:?}", source_path)));
        }

        let mut objects = Vec::new();
        if let Some(anns) = annotations_by_image.get(&image.id) {
            for ann in anns {
                let category_id = match category_map.get(&ann.category_id) {
                    Some(id) => *id,
                    None => {
                        log::warn!(
                            "Unknown category id {} for image {}",
                            ann.category_id,
                            image.file_name
                        );
                        continue;
                    }
                };

                let polygon = if let Some(segmentation) = ann.segmentation.as_ref() {
                    if let Some(points) =
                        coco_segmentation_to_polygon(segmentation, image.width, image.height)
                    {
                        points
                    } else {
                        bbox_to_polygon(&ann.bbox, image.width, image.height)
                    }
                } else {
                    bbox_to_polygon(&ann.bbox, image.width, image.height)
                };

                if !polygon.is_valid() {
                    continue;
                }
                objects.push(lab_core::new_object(0, category_id, polygon));
            }
        }

        let annotation = build_label(objects, "import");
        imported.push(ImportedImage { source_path, file_name, annotation });
    }

    Ok(imported)
}

pub fn import_from_labelme(root: &Path, meta: &LabelMeta) -> anyhow::Result<Vec<ImportedImage>> {
    let mut imported = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let content = fs::read_to_string(&path)?;
        let labelme: LabelMeFile = serde_json::from_str(&content)?;

        let file_name = if let Some(image_path) = labelme.image_path.as_ref() {
            Path::new(image_path)
                .file_name()
                .and_then(|s| s.to_str())
                .context("Invalid image name")?
                .to_string()
        } else {
            let stem =
                path.file_stem().and_then(|s| s.to_str()).context("Invalid label file name")?;
            find_image_by_stem(root, stem)?
        };

        let source_path = root.join(&file_name);
        if !source_path.exists() {
            return Err(anyhow::anyhow!(format!("Missing image file: {:?}", source_path)));
        }

        let mut objects = Vec::new();
        for shape in labelme.shapes {
            let category_id = match find_category_id_by_name(meta, &shape.label) {
                Some(id) => id,
                None => {
                    log::warn!("Unknown category name {} in {:?}", shape.label, path);
                    continue;
                }
            };

            let polygon =
                labelme_shape_to_polygon(&shape, labelme.image_width, labelme.image_height);
            if polygon.0.len() < 3 {
                continue;
            }
            objects.push(lab_core::new_object(0, category_id, polygon));
        }

        let annotation = build_label(objects, "import");
        imported.push(ImportedImage { source_path, file_name, annotation });
    }

    Ok(imported)
}

pub fn merge_imported_images(
    imported: Vec<ImportedImage>,
    project: &crate::Project,
    existing_names: &HashSet<String>,
    duplicate_template: &str,
) -> anyhow::Result<()> {
    let mut incoming_names = HashSet::new();
    for item in &imported {
        if existing_names.contains(&item.file_name) {
            return Err(anyhow::anyhow!(duplicate_template.replace("{name}", &item.file_name)));
        }
        if !incoming_names.insert(item.file_name.clone()) {
            return Err(anyhow::anyhow!(duplicate_template.replace("{name}", &item.file_name)));
        }
    }

    fs::create_dir_all(project.images_dir())?;
    fs::create_dir_all(project.labels_dir())?;

    for item in imported {
        let dest_image = project.images_dir().join(&item.file_name);
        fs::copy(&item.source_path, &dest_image)?;

        let label_path = project.annotation_path(&item.file_name);
        lab_core::io::save_annotation(&label_path, &item.annotation)?;
    }

    Ok(())
}

pub fn list_images_in_dir(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut images = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        if ext == "jpg" || ext == "jpeg" || ext == "png" {
            images.push(path);
        }
    }
    images.sort();
    Ok(images)
}

pub(crate) fn rect_polygon(xmin: f32, ymin: f32, xmax: f32, ymax: f32) -> Polygon<f32> {
    let mut xmin = clamp01(xmin);
    let mut ymin = clamp01(ymin);
    let mut xmax = clamp01(xmax);
    let mut ymax = clamp01(ymax);
    if xmax < xmin {
        std::mem::swap(&mut xmin, &mut xmax);
    }
    if ymax < ymin {
        std::mem::swap(&mut ymin, &mut ymax);
    }
    if xmax <= xmin || ymax <= ymin {
        return Polygon::empty();
    }
    Polygon::from(vec![
        Point { x: xmin, y: ymin },
        Point { x: xmax, y: ymin },
        Point { x: xmax, y: ymax },
        Point { x: xmin, y: ymax },
    ])
}

fn clamp01(value: f32) -> f32 {
    value.max(0.0).min(1.0)
}

pub fn build_label(objects: Vec<Object>, user_agent: &str) -> Label {
    let mut label = lab_core::new_label(user_agent);
    for (idx, mut obj) in objects.into_iter().enumerate() {
        obj.id = idx as i32;
        lab_core::add_object(&mut label, obj);
    }
    label
}

fn find_category_id_by_name(meta: &LabelMeta, name: &str) -> Option<i32> {
    meta.categories.iter().find(|cat| cat.name == name).map(|cat| cat.id)
}

fn parse_voc_size(xml: &str) -> Option<(f32, f32)> {
    let width = extract_tag_value(xml, "width")?.parse::<f32>().ok()?;
    let height = extract_tag_value(xml, "height")?.parse::<f32>().ok()?;
    if width <= 0.0 || height <= 0.0 {
        None
    } else {
        Some((width, height))
    }
}

fn parse_voc_objects(xml: &str) -> Vec<VocObject> {
    let mut objects = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<object>") {
        rest = &rest[start + "<object>".len()..];
        let end = match rest.find("</object>") {
            Some(end) => end,
            None => break,
        };
        let block = &rest[..end];
        rest = &rest[end + "</object>".len()..];

        let label = match extract_tag_value(block, "name") {
            Some(val) => val,
            None => continue,
        };
        let xmin = match extract_tag_value(block, "xmin").and_then(|v| v.parse().ok()) {
            Some(val) => val,
            None => continue,
        };
        let ymin = match extract_tag_value(block, "ymin").and_then(|v| v.parse().ok()) {
            Some(val) => val,
            None => continue,
        };
        let xmax = match extract_tag_value(block, "xmax").and_then(|v| v.parse().ok()) {
            Some(val) => val,
            None => continue,
        };
        let ymax = match extract_tag_value(block, "ymax").and_then(|v| v.parse().ok()) {
            Some(val) => val,
            None => continue,
        };

        objects.push(VocObject { label, xmin, ymin, xmax, ymax });
    }
    objects
}

fn extract_tag_value(content: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);
    let start = content.find(&start_tag)? + start_tag.len();
    let end = content[start..].find(&end_tag)? + start;
    Some(content[start..end].trim().to_string())
}

fn bbox_to_polygon(bbox: &[f32], width: u32, height: u32) -> Polygon<f32> {
    if bbox.len() < 4 || width == 0 || height == 0 {
        return Polygon::empty();
    }
    let x = bbox[0] / width as f32;
    let y = bbox[1] / height as f32;
    let w = bbox[2] / width as f32;
    let h = bbox[3] / height as f32;
    rect_polygon(x, y, x + w, y + h)
}

fn coco_segmentation_to_polygon(
    segmentation: &serde_json::Value,
    width: u32,
    height: u32,
) -> Option<Polygon<f32>> {
    let coords = match segmentation {
        serde_json::Value::Array(items) => {
            if items.is_empty() {
                return None;
            }
            if items[0].is_array() {
                items[0].as_array()?.clone()
            } else {
                items.clone()
            }
        }
        _ => return None,
    };

    let mut points = Vec::new();
    let mut iter = coords.iter().filter_map(|v| v.as_f64());
    while let (Some(x), Some(y)) = (iter.next(), iter.next()) {
        if width == 0 || height == 0 {
            break;
        }
        points.push(Point {
            x: clamp01(x as f32 / width as f32),
            y: clamp01(y as f32 / height as f32),
        });
    }

    if points.len() >= 3 {
        Some(Polygon::from(points))
    } else {
        None
    }
}

fn labelme_shape_to_polygon(shape: &LabelMeShape, width: u32, height: u32) -> Polygon<f32> {
    if width == 0 || height == 0 {
        return Polygon::empty();
    }
    let shape_type = shape.shape_type.as_deref().unwrap_or("polygon").to_lowercase();

    let polygon = match shape_type.as_str() {
        "rectangle" if shape.points.len() >= 2 => {
            let p1 = &shape.points[0];
            let p2 = &shape.points[1];
            if p1.len() < 2 || p2.len() < 2 {
                return Polygon::empty();
            }
            let x1 = p1[0] as f32 / width as f32;
            let y1 = p1[1] as f32 / height as f32;
            let x2 = p2[0] as f32 / width as f32;
            let y2 = p2[1] as f32 / height as f32;
            rect_polygon(x1, y1, x2, y2)
        }
        _ => Polygon::from(
            shape
                .points
                .iter()
                .filter_map(|p| {
                    if p.len() < 2 {
                        None
                    } else {
                        Some(Point {
                            x: clamp01(p[0] as f32 / width as f32),
                            y: clamp01(p[1] as f32 / height as f32),
                        })
                    }
                })
                .collect::<Vec<_>>(),
        ),
    };

    polygon
}

fn find_image_by_stem(root: &Path, stem: &str) -> anyhow::Result<String> {
    let candidates = ["jpg", "jpeg", "png"];
    for ext in candidates {
        let path = root.join(format!("{}.{}", stem, ext));
        if path.exists() {
            return Ok(format!("{}.{}", stem, ext));
        }
    }
    Err(anyhow::anyhow!("Cannot find image for {}", stem))
}

#[derive(Debug)]
struct VocObject {
    label: String,
    xmin: f32,
    ymin: f32,
    xmax: f32,
    ymax: f32,
}

#[derive(Debug, Deserialize)]
struct CocoDataset {
    images: Vec<CocoImage>,
    annotations: Vec<CocoAnnotation>,
    categories: Vec<CocoCategory>,
}

#[derive(Debug, Deserialize)]
struct CocoImage {
    id: i32,
    file_name: String,
    width: u32,
    height: u32,
}

#[derive(Debug, Deserialize)]
struct CocoCategory {
    id: i32,
    name: String,
}

#[derive(Debug, Deserialize)]
struct CocoAnnotation {
    image_id: i32,
    category_id: i32,
    #[serde(default)]
    bbox: Vec<f32>,
    #[serde(default)]
    segmentation: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct LabelMeFile {
    #[serde(rename = "imagePath")]
    image_path: Option<String>,
    #[serde(rename = "imageHeight")]
    image_height: u32,
    #[serde(rename = "imageWidth")]
    image_width: u32,
    #[serde(default)]
    shapes: Vec<LabelMeShape>,
}

#[derive(Debug, Deserialize)]
struct LabelMeShape {
    label: String,
    points: Vec<Vec<f64>>,
    #[serde(rename = "shape_type")]
    shape_type: Option<String>,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::Project;
    use lab_core::{CatDef, RoiConfig, ShapeConfig};

    pub(crate) fn test_meta() -> LabelMeta {
        LabelMeta {
            id: 1,
            name: "test".to_string(),
            description: String::new(),
            shape: ShapeConfig {
                title_style: 0,
                thickness: 2,
                auto_save: true,
                vertex_radius: 10.0,
            },
            roi: RoiConfig { color: "#800080".to_string() },
            categories: vec![CatDef {
                id: 0,
                name: "person".to_string(),
                description: String::new(),
                hotkey: "1".to_string(),
                color: "#FF0000".to_string(),
                properties: vec![],
            }],
            property_types: vec![],
            property_special_values: vec![],
        }
    }

    fn temp_root(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("lab-utils-test-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("images")).unwrap();
        fs::create_dir_all(dir.join("labels")).unwrap();
        dir
    }

    #[test]
    fn import_from_yolo_parses_boxes_and_skips_unknown_category() {
        let root = temp_root("yolo");
        fs::write(root.join("images/a.jpg"), b"fake").unwrap();
        fs::write(root.join("labels/a.txt"), "0 0.5 0.5 0.2 0.2\n9 0.1 0.1 0.1 0.1\nbad line\n")
            .unwrap();

        let imported = import_from_yolo(&root, &test_meta()).unwrap();

        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].file_name, "a.jpg");
        let label = &imported[0].annotation;
        assert_eq!(label.objects.len(), 1); // category 9 unknown -> skipped
        assert_eq!(label.objects[0].category, 0);
        assert_eq!(label.objects[0].polygon.0.len(), 4);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn import_from_yolo_without_labels_gives_empty_annotation() {
        let root = temp_root("yolo-empty");
        fs::write(root.join("images/b.png"), b"fake").unwrap();

        let imported = import_from_yolo(&root, &test_meta()).unwrap();

        assert_eq!(imported.len(), 1);
        assert!(imported[0].annotation.objects.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rect_polygon_clamps_and_swaps() {
        let p = rect_polygon(-0.1, 0.8, 0.5, 0.2);
        assert_eq!(p.0.len(), 4);
        assert_eq!(p.0[0], Point { x: 0.0, y: 0.2 });
        assert!(rect_polygon(0.5, 0.5, 0.5, 0.8).0.is_empty());
    }

    #[test]
    fn import_from_voc_parses_objects_by_name() {
        let root = temp_root("voc");
        let voc_root = root.join("voc");
        fs::create_dir_all(voc_root.join("JPEGImages")).unwrap();
        fs::create_dir_all(voc_root.join("Annotations")).unwrap();
        fs::write(voc_root.join("JPEGImages/a.jpg"), b"fake").unwrap();
        fs::write(
            voc_root.join("Annotations/a.xml"),
            r#"<annotation><size><width>100</width><height>200</height></size>
               <object><name>person</name><xmin>10</xmin><ymin>20</ymin><xmax>30</xmax><ymax>60</ymax></object>
               <object><name>dog</name><xmin>1</xmin><ymin>1</ymin><xmax>9</xmax><ymax>9</ymax></object>
               </annotation>"#,
        )
        .unwrap();

        let imported = import_from_voc(&voc_root, &test_meta()).unwrap();

        let label = &imported[0].annotation;
        assert_eq!(label.objects.len(), 1); // "dog" unknown -> skipped
        assert_eq!(label.objects[0].category, 0);
        // pixel (10,20)-(30,60) on 100x200 -> normalized (0.1,0.1)-(0.3,0.3)
        assert_eq!(label.objects[0].polygon.0[0], Point { x: 0.1, y: 0.1 });
        assert_eq!(label.objects[0].polygon.0[2], Point { x: 0.3, y: 0.3 });
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn import_from_voc_requires_dirs() {
        let err = import_from_voc(&temp_root("voc-bad"), &test_meta());
        assert!(err.is_err());
    }

    #[test]
    fn import_from_coco_maps_categories_and_segmentation() {
        let root = temp_root("coco");
        fs::write(root.join("a.jpg"), b"fake").unwrap();
        fs::write(
            root.join("ann.json"),
            r#"{"images":[{"id":1,"file_name":"a.jpg","width":100,"height":100}],
                "annotations":[
                  {"image_id":1,"category_id":7,"bbox":[10,10,20,20]},
                  {"image_id":1,"category_id":8,"bbox":[0,0,0,0],
                   "segmentation":[[10.0,10.0, 30.0,10.0, 30.0,30.0]]}],
                "categories":[{"id":7,"name":"person"},{"id":8,"name":"person"}]}"#,
        )
        .unwrap();

        let imported = import_from_coco(&root.join("ann.json"), &root, &test_meta()).unwrap();

        let label = &imported[0].annotation;
        assert_eq!(label.objects.len(), 2);
        // category 7 named "person" -> mapped to meta id 0
        assert_eq!(label.objects[0].category, 0);
        // segmentation polygon (not bbox)
        assert_eq!(label.objects[1].polygon.0.len(), 3);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn import_from_labelme_parses_polygon_shapes() {
        let root = temp_root("labelme");
        fs::write(root.join("a.jpg"), b"fake").unwrap();
        fs::write(
            root.join("a.json"),
            r#"{"imagePath":"a.jpg","imageHeight":100,"imageWidth":100,
                "shapes":[{"label":"person","shape_type":"polygon",
                           "points":[[10.0,10.0],[30.0,10.0],[30.0,30.0]]}]}"#,
        )
        .unwrap();

        let imported = import_from_labelme(&root, &test_meta()).unwrap();

        let label = &imported[0].annotation;
        assert_eq!(label.objects.len(), 1);
        assert_eq!(label.objects[0].category, 0);
        assert_eq!(label.objects[0].polygon.0.len(), 3);
        assert_eq!(label.objects[0].polygon.0[0], Point { x: 0.1, y: 0.1 });
        let _ = fs::remove_dir_all(&root);
    }

    fn make_project(tag: &str) -> (PathBuf, Project) {
        let root = temp_root(tag);
        let meta = test_meta();
        lab_core::io::save_meta(root.join("meta.yaml"), &meta).unwrap();
        let project = Project::open(&root).unwrap();
        (root, project)
    }

    fn image(path: &Path, name: &str) -> ImportedImage {
        ImportedImage {
            source_path: path.to_path_buf(),
            file_name: name.to_string(),
            annotation: lab_core::new_label("test"),
        }
    }

    #[test]
    fn merge_copies_image_and_writes_annotation() {
        let (root, project) = make_project("merge-ok");
        let src = root.join("src.jpg");
        fs::write(&src, b"fake").unwrap();
        let ann = lab_core::new_label("test");

        merge_imported_images(
            vec![ImportedImage {
                source_path: src.clone(),
                file_name: "src.jpg".to_string(),
                annotation: ann,
            }],
            &project,
            &Default::default(),
            "duplicate: {name}",
        )
        .unwrap();

        assert!(project.images_dir().join("src.jpg").exists());
        assert!(project.annotation_path("src.jpg").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn merge_rejects_duplicate_names() {
        let (root, project) = make_project("merge-dup");
        let src = root.join("x.jpg");
        fs::write(&src, b"fake").unwrap();

        let err = merge_imported_images(
            vec![image(&src, "dup.jpg"), image(&src, "dup.jpg")],
            &project,
            &Default::default(),
            "duplicate: {name}",
        )
        .unwrap_err();
        assert!(err.to_string().contains("duplicate: dup.jpg"));
        let _ = fs::remove_dir_all(&root);
    }
}
