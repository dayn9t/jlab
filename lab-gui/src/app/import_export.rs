use super::LabApp;
use anyhow::Context;
use image::GenericImageView;
use lab_core::{Annotation, Object, Point};
use lab_utils::conversion::{export_annotation, export_coco_batch, ExportFormat};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub(super) enum DatasetFormat {
    Yolo,
    Voc,
    Coco,
    LabelMe,
}

impl LabApp {
    pub(super) fn import_dataset(&mut self, format: DatasetFormat) {
        let result = self.try_import_dataset(format);
        if let Err(err) = result {
            self.show_io_error(self.state.i18n.t("error.import_failed"), err.to_string());
        }
    }

    pub(super) fn export_dataset(&mut self, format: DatasetFormat) {
        let result = self.try_export_dataset(format);
        if let Err(err) = result {
            self.show_io_error(self.state.i18n.t("error.export_failed"), err.to_string());
        }
    }
}

impl LabApp {
    fn show_io_error(&self, title: String, message: String) {
        log::error!("{}: {}", title, message);
        let _ = rfd::MessageDialog::new()
            .set_title(&title)
            .set_description(&message)
            .set_buttons(rfd::MessageButtons::Ok)
            .set_level(rfd::MessageLevel::Error)
            .show();
    }

    fn try_import_dataset(&mut self, format: DatasetFormat) -> anyhow::Result<()> {
        let project = self
            .state
            .project
            .as_ref()
            .context(self.state.i18n.t("error.no_project"))?;
        let meta = project.meta.clone();
        let existing_names = self
            .state
            .images
            .iter()
            .filter_map(|p| {
                p.file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            })
            .collect::<HashSet<String>>();
        let duplicate_template = self.state.i18n.t("error.import_duplicate_image");

        let imported = match format {
            DatasetFormat::Yolo => {
                let Some(root) = rfd::FileDialog::new()
                    .set_title("Select YOLO dataset root")
                    .pick_folder()
                else {
                    return Ok(());
                };
                self.import_from_yolo(&root, &meta)?
            }
            DatasetFormat::Voc => {
                let Some(root) = rfd::FileDialog::new()
                    .set_title("Select VOC dataset root")
                    .pick_folder()
                else {
                    return Ok(());
                };
                self.import_from_voc(&root, &meta)?
            }
            DatasetFormat::Coco => {
                let Some(json_path) = rfd::FileDialog::new()
                    .add_filter("COCO", &["json"])
                    .set_title("Select COCO annotation JSON")
                    .pick_file()
                else {
                    return Ok(());
                };
                let Some(images_dir) = rfd::FileDialog::new()
                    .set_title("Select COCO images folder")
                    .pick_folder()
                else {
                    return Ok(());
                };
                self.import_from_coco(&json_path, &images_dir, &meta)?
            }
            DatasetFormat::LabelMe => {
                let Some(root) = rfd::FileDialog::new()
                    .set_title("Select LabelMe folder")
                    .pick_folder()
                else {
                    return Ok(());
                };
                self.import_from_labelme(&root, &meta)?
            }
        };

        merge_imported_images(imported, project, &existing_names, &duplicate_template)?;
        self.refresh_project_images()?;
        Ok(())
    }

    fn try_export_dataset(&mut self, format: DatasetFormat) -> anyhow::Result<()> {
        let project = self
            .state
            .project
            .as_ref()
            .context(self.state.i18n.t("error.no_project"))?;
        let meta = project.meta.clone();

        let Some(output_root) = rfd::FileDialog::new()
            .set_title("Select export folder")
            .pick_folder()
        else {
            return Ok(());
        };

        let export_items = self.collect_export_items(project)?;

        match format {
            DatasetFormat::Yolo => {
                let images_dir = output_root.join("images");
                let labels_dir = output_root.join("labels");
                fs::create_dir_all(&images_dir)?;
                fs::create_dir_all(&labels_dir)?;

                for item in &export_items {
                    let label_path = labels_dir.join(format!("{}.txt", item.stem));
                    export_annotation(
                        &label_path,
                        &item.annotation,
                        &meta,
                        item.image_path.to_string_lossy().as_ref(),
                        item.width,
                        item.height,
                        ExportFormat::Yolo,
                    )?;
                    fs::copy(&item.image_path, images_dir.join(&item.file_name))?;
                }
            }
            DatasetFormat::Voc => {
                let images_dir = output_root.join("JPEGImages");
                let annotations_dir = output_root.join("Annotations");
                fs::create_dir_all(&images_dir)?;
                fs::create_dir_all(&annotations_dir)?;

                for item in &export_items {
                    let label_path = annotations_dir.join(format!("{}.xml", item.stem));
                    export_annotation(
                        &label_path,
                        &item.annotation,
                        &meta,
                        item.image_path.to_string_lossy().as_ref(),
                        item.width,
                        item.height,
                        ExportFormat::Voc,
                    )?;
                    fs::copy(&item.image_path, images_dir.join(&item.file_name))?;
                }
            }
            DatasetFormat::Coco => {
                let images_dir = output_root.join("images");
                fs::create_dir_all(&images_dir)?;
                for item in &export_items {
                    fs::copy(&item.image_path, images_dir.join(&item.file_name))?;
                }

                let coco_path = output_root.join("annotations.json");
                let coco_items: Vec<(String, Annotation, u32, u32)> = export_items
                    .iter()
                    .map(|item| {
                        (
                            item.file_name.clone(),
                            item.annotation.clone(),
                            item.width,
                            item.height,
                        )
                    })
                    .collect();
                export_coco_batch(&coco_path, &coco_items, &meta)?;
            }
            DatasetFormat::LabelMe => {
                fs::create_dir_all(&output_root)?;
                for item in &export_items {
                    fs::copy(&item.image_path, output_root.join(&item.file_name))?;
                    let label_path = output_root.join(format!("{}.json", item.stem));
                    self.export_labelme_annotation(&label_path, &item, &meta)?;
                }
            }
        }

        Ok(())
    }
}

struct ImportedImage {
    source_path: PathBuf,
    file_name: String,
    annotation: Option<Annotation>,
}

struct ExportItem {
    image_path: PathBuf,
    file_name: String,
    stem: String,
    width: u32,
    height: u32,
    annotation: Annotation,
}

impl LabApp {
    fn refresh_project_images(&mut self) -> anyhow::Result<()> {
        if let Some(project) = &self.state.project {
            let current_path = self
                .state
                .current_image
                .as_ref()
                .map(|img| img.path.clone());
            self.state.images = project.list_images()?;
            if let Some(current_path) = current_path {
                if let Some(idx) = self.state.images.iter().position(|p| p == &current_path) {
                    self.state.current_image_index = idx;
                }
            }
        }
        Ok(())
    }

    fn collect_export_items(
        &self,
        project: &lab_utils::Project,
    ) -> anyhow::Result<Vec<ExportItem>> {
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

            let annotation = project
                .load_annotation(&file_name)?
                .unwrap_or_else(|| Annotation::new("export"));

            items.push(ExportItem {
                image_path,
                file_name,
                stem,
                width,
                height,
                annotation,
            });
        }
        Ok(items)
    }
}

fn merge_imported_images(
    imported: Vec<ImportedImage>,
    project: &lab_utils::Project,
    existing_names: &HashSet<String>,
    duplicate_template: &str,
) -> anyhow::Result<()> {
    let mut incoming_names = HashSet::new();
    for item in &imported {
        if existing_names.contains(&item.file_name) {
            return Err(anyhow::anyhow!(
                duplicate_template.replace("{name}", &item.file_name)
            ));
        }
        if !incoming_names.insert(item.file_name.clone()) {
            return Err(anyhow::anyhow!(
                duplicate_template.replace("{name}", &item.file_name)
            ));
        }
    }

    fs::create_dir_all(project.images_dir())?;
    fs::create_dir_all(project.labels_dir())?;

    for item in imported {
        let dest_image = project.images_dir().join(&item.file_name);
        fs::copy(&item.source_path, &dest_image)?;

        if let Some(annotation) = item.annotation {
            let label_path = project.annotation_path(&item.file_name);
            lab_core::io::save_annotation(&label_path, &annotation)?;
        }
    }

    Ok(())
}

impl LabApp {
    fn import_from_yolo(
        &self,
        root: &Path,
        meta: &lab_core::Meta,
    ) -> anyhow::Result<Vec<ImportedImage>> {
        let images_dir = root.join("images");
        let labels_dir = root.join("labels");
        if !images_dir.exists() || !labels_dir.exists() {
            return Err(anyhow::anyhow!(
                "YOLO root must contain images/ and labels/ directories"
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
            let stem = Path::new(&file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&file_name);
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
                    if meta.find_category(class_id).is_none() {
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
                    if polygon.len() < 3 {
                        continue;
                    }
                    objects.push(Object::new(0, class_id, polygon));
                }
            }

            let annotation = build_annotation(objects, "import");
            imported.push(ImportedImage {
                source_path: image_path,
                file_name,
                annotation,
            });
        }

        Ok(imported)
    }

    fn import_from_voc(
        &self,
        root: &Path,
        meta: &lab_core::Meta,
    ) -> anyhow::Result<Vec<ImportedImage>> {
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
            let stem = Path::new(&file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&file_name);
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
                        if polygon.len() < 3 {
                            continue;
                        }
                        objects.push(Object::new(0, category_id, polygon));
                    }
                }
            }

            let annotation = build_annotation(objects, "import");
            imported.push(ImportedImage {
                source_path: image_path,
                file_name,
                annotation,
            });
        }

        Ok(imported)
    }

    fn import_from_coco(
        &self,
        json_path: &Path,
        images_dir: &Path,
        meta: &lab_core::Meta,
    ) -> anyhow::Result<Vec<ImportedImage>> {
        let content = fs::read_to_string(json_path)?;
        let dataset: CocoDataset = serde_json::from_str(&content)?;

        let mut category_map = HashMap::new();
        for cat in &dataset.categories {
            if meta.find_category(cat.id).is_some() {
                category_map.insert(cat.id, cat.id);
            } else if let Some(id) = find_category_id_by_name(meta, &cat.name) {
                category_map.insert(cat.id, id);
            }
        }

        let mut annotations_by_image: HashMap<i32, Vec<CocoAnnotation>> = HashMap::new();
        for ann in dataset.annotations {
            annotations_by_image
                .entry(ann.image_id)
                .or_default()
                .push(ann);
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
                return Err(anyhow::anyhow!(format!(
                    "Missing image file: {:?}",
                    source_path
                )));
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

                    if polygon.len() < 3 {
                        continue;
                    }
                    objects.push(Object::new(0, category_id, polygon));
                }
            }

            let annotation = build_annotation(objects, "import");
            imported.push(ImportedImage {
                source_path,
                file_name,
                annotation,
            });
        }

        Ok(imported)
    }

    fn import_from_labelme(
        &self,
        root: &Path,
        meta: &lab_core::Meta,
    ) -> anyhow::Result<Vec<ImportedImage>> {
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
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .context("Invalid label file name")?;
                find_image_by_stem(root, stem)?
            };

            let source_path = root.join(&file_name);
            if !source_path.exists() {
                return Err(anyhow::anyhow!(format!(
                    "Missing image file: {:?}",
                    source_path
                )));
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
                if polygon.len() < 3 {
                    continue;
                }
                objects.push(Object::new(0, category_id, polygon));
            }

            let annotation = build_annotation(objects, "import");
            imported.push(ImportedImage {
                source_path,
                file_name,
                annotation,
            });
        }

        Ok(imported)
    }
}

impl LabApp {
    fn export_labelme_annotation(
        &self,
        output_path: &Path,
        item: &ExportItem,
        meta: &lab_core::Meta,
    ) -> anyhow::Result<()> {
        let mut shapes = Vec::new();
        for obj in &item.annotation.objects {
            if obj.polygon.len() < 3 {
                continue;
            }
            let label = meta
                .find_category(obj.category)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "unknown".to_string());
            let points = obj
                .polygon
                .iter()
                .map(|point| {
                    vec![
                        (point.x * item.width as f32) as f64,
                        (point.y * item.height as f32) as f64,
                    ]
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
}

fn list_images_in_dir(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut images = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext == "jpg" || ext == "jpeg" || ext == "png" {
            images.push(path);
        }
    }
    images.sort();
    Ok(images)
}

fn rect_polygon(xmin: f32, ymin: f32, xmax: f32, ymax: f32) -> Vec<Point> {
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
        return Vec::new();
    }
    vec![
        Point::new(xmin, ymin),
        Point::new(xmax, ymin),
        Point::new(xmax, ymax),
        Point::new(xmin, ymax),
    ]
}

fn clamp01(value: f32) -> f32 {
    value.max(0.0).min(1.0)
}

fn build_annotation(objects: Vec<Object>, user_agent: &str) -> Option<Annotation> {
    if objects.is_empty() {
        return None;
    }
    let mut annotation = Annotation::new(user_agent);
    for (idx, mut obj) in objects.into_iter().enumerate() {
        obj.id = idx as i32;
        annotation.add_object(obj);
    }
    Some(annotation)
}

fn find_category_id_by_name(meta: &lab_core::Meta, name: &str) -> Option<i32> {
    meta.categories
        .iter()
        .find(|cat| cat.name == name)
        .map(|cat| cat.id)
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

        objects.push(VocObject {
            label,
            xmin,
            ymin,
            xmax,
            ymax,
        });
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

fn bbox_to_polygon(bbox: &[f32], width: u32, height: u32) -> Vec<Point> {
    if bbox.len() < 4 || width == 0 || height == 0 {
        return Vec::new();
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
) -> Option<Vec<Point>> {
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
        points.push(Point::new(
            clamp01(x as f32 / width as f32),
            clamp01(y as f32 / height as f32),
        ));
    }

    if points.len() >= 3 {
        Some(points)
    } else {
        None
    }
}

fn labelme_shape_to_polygon(shape: &LabelMeShape, width: u32, height: u32) -> Vec<Point> {
    if width == 0 || height == 0 {
        return Vec::new();
    }
    let shape_type = shape
        .shape_type
        .as_deref()
        .unwrap_or("polygon")
        .to_lowercase();

    let points = match shape_type.as_str() {
        "rectangle" if shape.points.len() >= 2 => {
            let p1 = &shape.points[0];
            let p2 = &shape.points[1];
            if p1.len() < 2 || p2.len() < 2 {
                return Vec::new();
            }
            let x1 = p1[0] as f32 / width as f32;
            let y1 = p1[1] as f32 / height as f32;
            let x2 = p2[0] as f32 / width as f32;
            let y2 = p2[1] as f32 / height as f32;
            rect_polygon(x1, y1, x2, y2)
        }
        _ => shape
            .points
            .iter()
            .filter_map(|p| {
                if p.len() < 2 {
                    None
                } else {
                    Some(Point::new(
                        clamp01(p[0] as f32 / width as f32),
                        clamp01(p[1] as f32 / height as f32),
                    ))
                }
            })
            .collect(),
    };

    points
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
