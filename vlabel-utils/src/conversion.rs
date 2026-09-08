use crate::Project;
use anyhow::Context;
use image::GenericImageView;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use vlabel_core::export::{coco::CocoExporter, voc::VocExporter, yolo::YoloExporter, Exporter};
use vlabel_core::{Label, LabelMeta, Result};

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
            return Err(vlabel_core::Error::Export(
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

pub struct ExportItem {
    pub image_path: PathBuf,
    pub file_name: String,
    pub stem: String,
    pub width: u32,
    pub height: u32,
    pub annotation: Label,
}

/// Write `classes.txt` for a YOLO export: one class name per line, ordered by
/// category id (line index = class id). Same convention as labelImg /
/// X-AnyLabeling.
///
/// YOLO label files store the raw `obj.category` id, so the line index equals
/// the class id only when category ids are contiguous 0-based. Any other id
/// set would silently mis-map class names, so it is rejected up front.
pub fn export_yolo_classes_txt(meta: &LabelMeta, out_path: &Path) -> anyhow::Result<()> {
    let mut categories = meta.categories.clone();
    categories.sort_by_key(|c| c.id);
    let ids: Vec<i32> = categories.iter().map(|c| c.id).collect();
    if ids.iter().enumerate().any(|(line, &id)| id != line as i32) {
        anyhow::bail!(
            "classes.txt requires contiguous 0-based category ids (line index = class id), got ids {:?}",
            ids
        );
    }
    let content = categories.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join("\n");
    fs::write(out_path, content)?;
    Ok(())
}

/// Guard against exporting a dataset on top of the project itself: exporting
/// into the project root, its `images/` dir, or its `vlabels/` dir would
/// overwrite the project's own images in place with masked/lossy copies (YOLO
/// export writes `<out>/images` and `<out>/labels`, so the project root is
/// equally dangerous). Both paths are resolved (symlinks + `..`) before
/// comparison, so non-canonical spellings are caught too.
pub fn ensure_safe_export_dir(project_root: &Path, output_root: &Path) -> anyhow::Result<()> {
    let root = resolve_for_compare(project_root)?;
    let out = resolve_for_compare(output_root)?;

    for (what, dir) in [
        ("the project root", root.clone()),
        ("the project images directory", root.join("images")),
        ("the project vlabels directory", root.join("vlabels")),
    ] {
        if out == dir {
            anyhow::bail!(
                "export directory {:?} is {}: exporting would overwrite the project's own data in place; choose an output directory outside the project",
                output_root,
                what
            );
        }
    }
    Ok(())
}

/// Resolve a path for equality comparison: canonicalize it when it exists;
/// otherwise canonicalize the longest existing ancestor and re-attach the
/// remaining components lexically (normalizing `..`/`.`, which is safe
/// because the canonicalized prefix contains no symlinks).
fn resolve_for_compare(path: &Path) -> anyhow::Result<PathBuf> {
    if let Ok(resolved) = path.canonicalize() {
        return Ok(resolved);
    }
    let absolute = std::path::absolute(path)?;
    let components: Vec<std::path::Component> = absolute.components().collect();
    for split in (1..=components.len()).rev() {
        let prefix: PathBuf = components[..split].iter().collect();
        if let Ok(resolved) = prefix.canonicalize() {
            return Ok(attach_normalized(resolved, &components[split..]));
        }
    }
    anyhow::bail!("cannot resolve path {:?}", path);
}

fn attach_normalized(mut full: PathBuf, tail: &[std::path::Component]) -> PathBuf {
    use std::path::Component;
    for component in tail {
        match component {
            Component::ParentDir => {
                full.pop();
            }
            Component::Normal(name) => full.push(name),
            // `CurDir` is normalized away by `components()`; `RootDir`/`Prefix`
            // cannot appear after the first component of an absolute path.
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    full
}

/// Write one export image for the YOLO driver: when the annotation has ROIs,
/// the area outside every ROI is painted `mask::MASK_GRAY`; without ROIs the
/// image is copied byte-identically. Returns whether the image was actually
/// masked (masked images must be re-encoded; verbatim copies are not).
pub fn export_image_roi_masked(item: &ExportItem, images_dir: &Path) -> anyhow::Result<bool> {
    let out_path = images_dir.join(&item.file_name);
    if item.annotation.rois.is_empty() {
        fs::copy(&item.image_path, &out_path)?;
        return Ok(false);
    }

    let img = image::open(&item.image_path)
        .with_context(|| format!("Failed to read image {:?}", item.image_path))?;
    let masked = crate::mask::mask_outside_rois(&img, &item.annotation.rois);
    if masked.as_bytes() == img.to_rgba8().as_raw().as_slice() {
        // The ROIs cover every pixel, so masking changed nothing: keep the
        // original bytes instead of taking a lossy re-encode.
        fs::copy(&item.image_path, &out_path)?;
        return Ok(false);
    }

    let ext = out_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        // Re-encode JPEG at high quality; the default encoder quality would
        // needlessly degrade training data.
        "jpg" | "jpeg" => {
            let file = fs::File::create(&out_path)?;
            let mut writer = std::io::BufWriter::new(file);
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, 95)
                .encode_image(&masked)?;
        }
        _ => masked.save(&out_path)?,
    }
    Ok(true)
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

        let annotation = project
            .load_annotation(&file_name)?
            .unwrap_or_else(|| vlabel_core::new_label("export"));

        items.push(ExportItem { image_path, file_name, stem, width, height, annotation });
    }
    Ok(items)
}

/// YOLO dataset export driver: writes `<out_dir>/images`, `<out_dir>/labels`
/// (one `<stem>.txt` per image with raw category ids) and `<out_dir>/classes.txt`.
/// Refuses output directories that would overwrite the project itself.
///
/// Unlike the VOC/COCO/LabelMe drivers, images whose annotation has ROIs are
/// written with the area outside the ROIs painted `mask::MASK_GRAY`: an ROI
/// means "only this region is training signal", and letterbox gray matches
/// what YOLO sees as padding. Images without ROIs — and ROIs covering every
/// pixel — are copied byte-identically (see `export_image_roi_masked`).
pub fn export_dataset_yolo(
    project: &Project,
    items: &[ExportItem],
    meta: &LabelMeta,
    out_dir: &Path,
) -> anyhow::Result<()> {
    ensure_safe_export_dir(&project.root, out_dir)?;
    let images_dir = out_dir.join("images");
    let labels_dir = out_dir.join("labels");
    fs::create_dir_all(&images_dir)?;
    fs::create_dir_all(&labels_dir)?;
    // Written before the per-image loop so non-contiguous category ids fail
    // before anything else is written.
    export_yolo_classes_txt(meta, &out_dir.join("classes.txt"))?;
    let mut masked_count = 0;
    for item in items {
        export_annotation(
            labels_dir.join(format!("{}.txt", item.stem)),
            &item.annotation,
            meta,
            &item.image_path.to_string_lossy(),
            item.width,
            item.height,
            ExportFormat::Yolo,
        )?;
        if export_image_roi_masked(item, &images_dir)? {
            masked_count += 1;
        }
    }
    if masked_count > 0 {
        // Not silent: masked images differ from the project's originals, and
        // only this driver transforms them (the others copy verbatim).
        log::warn!(
            "YOLO export: painted the area outside the ROIs of {masked_count}/{} image(s) with letterbox gray; VOC/COCO/LabelMe exports copy images verbatim",
            items.len()
        );
    }
    Ok(())
}

/// VOC dataset export driver: writes `<out_dir>/JPEGImages` and
/// `<out_dir>/Annotations` (one `<stem>.xml` per image).
///
/// Images are copied byte-identically even when annotated with ROIs: VOC is
/// an annotation-exchange format whose consumers must receive the original
/// image bytes; ROI masking is a YOLO-letterbox-specific convention (see
/// `export_dataset_yolo`).
pub fn export_dataset_voc(
    project: &Project,
    items: &[ExportItem],
    meta: &LabelMeta,
    out_dir: &Path,
) -> anyhow::Result<()> {
    ensure_safe_export_dir(&project.root, out_dir)?;
    let images_dir = out_dir.join("JPEGImages");
    let annotations_dir = out_dir.join("Annotations");
    fs::create_dir_all(&images_dir)?;
    fs::create_dir_all(&annotations_dir)?;
    for item in items {
        export_annotation(
            annotations_dir.join(format!("{}.xml", item.stem)),
            &item.annotation,
            meta,
            &item.image_path.to_string_lossy(),
            item.width,
            item.height,
            ExportFormat::Voc,
        )?;
        fs::copy(&item.image_path, images_dir.join(&item.file_name))?;
    }
    Ok(())
}

/// COCO dataset export driver: writes `<out_dir>/images` and
/// `<out_dir>/annotations.json`.
///
/// Images are copied byte-identically even when annotated with ROIs: COCO is
/// an annotation-exchange format whose consumers must receive the original
/// image bytes; ROI masking is a YOLO-letterbox-specific convention (see
/// `export_dataset_yolo`).
pub fn export_dataset_coco(
    project: &Project,
    items: &[ExportItem],
    meta: &LabelMeta,
    out_dir: &Path,
) -> anyhow::Result<()> {
    ensure_safe_export_dir(&project.root, out_dir)?;
    let images_dir = out_dir.join("images");
    fs::create_dir_all(&images_dir)?;
    for item in items {
        fs::copy(&item.image_path, images_dir.join(&item.file_name))?;
    }
    let coco_items = items
        .iter()
        .map(|item| (item.file_name.clone(), item.annotation.clone(), item.width, item.height))
        .collect::<Vec<_>>();
    export_coco_batch(out_dir.join("annotations.json"), &coco_items, meta)?;
    Ok(())
}

/// LabelMe export driver: writes each image and a sibling `<stem>.json` into
/// `out_dir` itself (flat layout, as LabelMe expects).
///
/// Images are copied byte-identically even when annotated with ROIs: LabelMe
/// is an editing/exchange format whose consumers must receive the original
/// image bytes; ROI masking is a YOLO-letterbox-specific convention (see
/// `export_dataset_yolo`).
pub fn export_dataset_labelme(
    project: &Project,
    items: &[ExportItem],
    meta: &LabelMeta,
    out_dir: &Path,
) -> anyhow::Result<()> {
    ensure_safe_export_dir(&project.root, out_dir)?;
    fs::create_dir_all(out_dir)?;
    for item in items {
        fs::copy(&item.image_path, out_dir.join(&item.file_name))?;
        export_labelme_annotation(&out_dir.join(format!("{}.json", item.stem)), item, meta)?;
    }
    Ok(())
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
        let label = vlabel_core::find_category(meta, obj.category)
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
    use std::fs;
    use vlabel_core::io::save_meta;
    use vlabel_core::{new_label, new_object, CatDef, Point, Polygon, RoiConfig, ShapeConfig};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vlabel-utils-conv-test-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn export_item(dir: &Path, rois: Vec<Polygon<f32>>) -> (ExportItem, PathBuf) {
        let src_path = dir.join("a.png");
        let red = image::Rgba([200, 10, 10, 255]);
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(4, 4, red))
            .save(&src_path)
            .unwrap();

        let mut annotation = new_label("test");
        annotation.rois = rois; // left half: pixel centers x=.125/.375 inside, .625/.875 outside
        let item = ExportItem {
            image_path: src_path.clone(),
            file_name: "a.png".to_string(),
            stem: "a".to_string(),
            width: 4,
            height: 4,
            annotation,
        };
        (item, src_path)
    }

    fn meta_with_ids(ids: &[i32]) -> LabelMeta {
        let mut meta = test_meta();
        meta.categories = ids
            .iter()
            .enumerate()
            .map(|(i, &id)| CatDef {
                id,
                name: format!("c{i}"),
                description: String::new(),
                hotkey: format!("{i}"),
                color: "#FF0000".to_string(),
                properties: vec![],
            })
            .collect();
        meta
    }

    #[test]
    fn export_yolo_classes_txt_rejects_noncontiguous_ids() {
        let dir = temp_dir("classes-bad");
        let meta = meta_with_ids(&[1, 2]);
        let out = dir.join("classes.txt");

        let err = export_yolo_classes_txt(&meta, &out).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("contiguous 0-based"), "message was: {msg}");
        assert!(msg.contains("[1, 2]"), "message was: {msg}");
        assert!(!out.exists(), "nothing must be written on failure");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_yolo_classes_txt_rejects_negative_ids() {
        let dir = temp_dir("classes-neg");
        let meta = meta_with_ids(&[-1, 0]);
        assert!(export_yolo_classes_txt(&meta, &dir.join("classes.txt")).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_yolo_classes_txt_accepts_contiguous_ids() {
        let dir = temp_dir("classes-ok");
        let meta = meta_with_ids(&[0, 1]);
        let out = dir.join("classes.txt");

        export_yolo_classes_txt(&meta, &out).unwrap();
        assert_eq!(fs::read_to_string(&out).unwrap(), "c0\nc1");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_yolo_classes_txt_orders_by_id() {
        let dir = temp_dir("classes");
        let mut meta = test_meta(); // has id 0 "person"
        meta.categories.push(CatDef {
            id: 2,
            name: "car".to_string(),
            description: String::new(),
            hotkey: "2".to_string(),
            color: "#0000FF".to_string(),
            properties: vec![],
        });
        meta.categories.push(CatDef {
            id: 1,
            name: "dog".to_string(),
            description: String::new(),
            hotkey: "1".to_string(),
            color: "#FF0000".to_string(),
            properties: vec![],
        });

        let out = dir.join("classes.txt");
        export_yolo_classes_txt(&meta, &out).unwrap();

        assert_eq!(fs::read_to_string(&out).unwrap(), "person\ndog\ncar");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_image_roi_masked_paints_outside_roi() {
        let dir = temp_dir("mask1");
        let roi = Polygon::from(vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 0.5, y: 0.0 },
            Point { x: 0.5, y: 1.0 },
            Point { x: 0.0, y: 1.0 },
        ]);
        let (item, _) = export_item(&dir, vec![roi]);

        let masked = export_image_roi_masked(&item, &dir).unwrap();
        assert!(masked, "left-half ROI must trigger masking");
        let img = image::open(dir.join("a.png")).unwrap();
        assert_eq!(img.get_pixel(0, 0), image::Rgba([200, 10, 10, 255])); // inside ROI
        assert_eq!(
            img.get_pixel(2, 0),
            image::Rgba([
                crate::mask::MASK_GRAY,
                crate::mask::MASK_GRAY,
                crate::mask::MASK_GRAY,
                255
            ])
        ); // outside ROI
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_image_without_roi_is_copied_verbatim() {
        let dir = temp_dir("mask2");
        let (item, src_path) = export_item(&dir, vec![]);

        export_image_roi_masked(&item, &dir).unwrap();
        assert_eq!(fs::read(dir.join("a.png")).unwrap(), fs::read(&src_path).unwrap());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_image_roi_covering_everything_is_copied_verbatim() {
        let dir = temp_dir("mask3");
        // A JPEG with gradient content: any re-encode changes the bytes, so a
        // verbatim copy is observable as byte equality with the source.
        let src_path = dir.join("a.jpg");
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(16, 16, |x, y| {
            image::Rgb([(x * 16) as u8, (y * 16) as u8, ((x + y) * 8) as u8])
        }));
        {
            let file = fs::File::create(&src_path).unwrap();
            image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut std::io::BufWriter::new(file),
                50,
            )
            .encode_image(&img)
            .unwrap();
        }

        // ROI covering the whole frame: masking must not touch any pixel.
        let roi = Polygon::from(vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
            Point { x: 0.0, y: 1.0 },
        ]);
        let mut annotation = new_label("test");
        annotation.rois = vec![roi];
        let item = ExportItem {
            image_path: src_path.clone(),
            file_name: "a.jpg".to_string(),
            stem: "a".to_string(),
            width: 16,
            height: 16,
            annotation,
        };

        let out_dir = dir.join("out");
        fs::create_dir_all(&out_dir).unwrap();
        export_image_roi_masked(&item, &out_dir).unwrap();
        assert_eq!(
            fs::read(out_dir.join("a.jpg")).unwrap(),
            fs::read(&src_path).unwrap(),
            "full-frame ROI masks nothing: original bytes must be kept, not re-encoded"
        );
        let _ = fs::remove_dir_all(&dir);
    }

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
        vlabel_core::add_object(&mut label, obj);

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

    fn project_with_image(tag: &str) -> PathBuf {
        let root = temp_dir(tag);
        fs::create_dir_all(root.join("images")).unwrap();
        fs::create_dir_all(root.join("vlabels")).unwrap();
        save_meta(root.join("meta.json5"), &test_meta()).unwrap();
        image::RgbImage::from_pixel(4, 2, image::Rgb([0, 0, 0]))
            .save(root.join("images/p.png"))
            .unwrap();
        root
    }

    #[test]
    fn ensure_safe_export_dir_rejects_project_dirs() {
        let root = temp_dir("guard1"); // images/ and vlabels/ intentionally absent
        assert!(ensure_safe_export_dir(&root, &root).is_err());
        assert!(ensure_safe_export_dir(&root, &root.join("images")).is_err());
        assert!(ensure_safe_export_dir(&root, &root.join("vlabels")).is_err());
        // non-canonical spelling of the project root must still be caught
        let dotted = root.join("sub").join("..");
        assert!(ensure_safe_export_dir(&root, &dotted).is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn ensure_safe_export_dir_allows_disjoint_dir() {
        let root = temp_dir("guard2a");
        let out = temp_dir("guard2b");
        assert!(ensure_safe_export_dir(&root, &out).is_ok());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn export_dataset_yolo_writes_layout_and_guards_project() {
        let root = project_with_image("drv-yolo");
        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();
        let out = temp_dir("drv-yolo-out");

        export_dataset_yolo(&project, &items, &project.meta, &out).unwrap();
        assert!(out.join("images/p.png").exists());
        assert!(out.join("labels/p.txt").exists());
        assert!(out.join("classes.txt").exists());

        // exporting onto the project itself must fail without touching it
        let before = fs::read(root.join("images/p.png")).unwrap();
        let err = export_dataset_yolo(&project, &items, &project.meta, &root).unwrap_err();
        assert!(format!("{err:#}").contains("project root"), "message was: {err:#}");
        assert_eq!(fs::read(root.join("images/p.png")).unwrap(), before);
        assert!(!root.join("classes.txt").exists());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn export_dataset_yolo_fails_before_writing_on_noncontiguous_ids() {
        let root = project_with_image("drv-yolo-bad");
        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();
        let mut meta = test_meta();
        meta.categories[0].id = 1; // not 0-based
        let out = temp_dir("drv-yolo-bad-out");

        assert!(export_dataset_yolo(&project, &items, &meta, &out).is_err());
        assert!(!out.join("classes.txt").exists());
        assert!(out.join("images").read_dir().map(|mut d| d.next().is_none()).unwrap_or(true));
        assert!(out.join("labels").read_dir().map(|mut d| d.next().is_none()).unwrap_or(true));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn export_dataset_labelme_writes_layout_and_guards_images_dir() {
        let root = project_with_image("drv-lm");
        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();
        let out = temp_dir("drv-lm-out");

        export_dataset_labelme(&project, &items, &project.meta, &out).unwrap();
        assert!(out.join("p.png").exists());
        assert!(out.join("p.json").exists());

        // exporting onto the project's own images must fail without touching it
        let before = fs::read(root.join("images/p.png")).unwrap();
        let err = export_dataset_labelme(&project, &items, &project.meta, &project.images_dir())
            .unwrap_err();
        assert!(format!("{err:#}").contains("images"), "message was: {err:#}");
        assert_eq!(fs::read(root.join("images/p.png")).unwrap(), before);
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn collect_export_items_reads_dimensions_and_default_label() {
        let root = std::env::temp_dir().join(format!("vlabel-utils-cv-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("images")).unwrap();
        fs::create_dir_all(root.join("vlabels")).unwrap();
        save_meta(root.join("meta.json5"), &test_meta()).unwrap();
        // real 4x2 png so image::open works
        image::RgbImage::from_pixel(4, 2, image::Rgb([0, 0, 0]))
            .save(root.join("images/p.png"))
            .unwrap();

        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!((items[0].width, items[0].height), (4, 2));
        assert_eq!(items[0].annotation.objects.len(), 0); // no vlabels/p.json5 -> empty label
        let _ = fs::remove_dir_all(&root);
    }
}
