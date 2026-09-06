//! 外部数据集格式导入（YOLO / VOC / COCO / LabelMe → JLab 项目）。

use anyhow::Context;
use lab_core::{Label, LabelMeta, Object, Point, Polygon};
use std::fs;
use std::path::{Path, PathBuf};

pub struct ImportedImage {
    pub source_path: PathBuf,
    pub file_name: String,
    pub annotation: Option<Label>,
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

pub fn build_label(objects: Vec<Object>, user_agent: &str) -> Option<Label> {
    if objects.is_empty() {
        return None;
    }
    let mut label = lab_core::new_label(user_agent);
    for (idx, mut obj) in objects.into_iter().enumerate() {
        obj.id = idx as i32;
        lab_core::add_object(&mut label, obj);
    }
    Some(label)
}

fn find_category_id_by_name(meta: &LabelMeta, name: &str) -> Option<i32> {
    meta.categories.iter().find(|cat| cat.name == name).map(|cat| cat.id)
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let label = imported[0].annotation.as_ref().unwrap();
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
        assert!(imported[0].annotation.is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rect_polygon_clamps_and_swaps() {
        let p = rect_polygon(-0.1, 0.8, 0.5, 0.2);
        assert_eq!(p.0.len(), 4);
        assert_eq!(p.0[0], Point { x: 0.0, y: 0.2 });
        assert!(rect_polygon(0.5, 0.5, 0.5, 0.8).0.is_empty());
    }
}
