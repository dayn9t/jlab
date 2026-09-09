//! Dataset 级导出驱动（YOLO / VOC / COCO / LabelMe）：每个驱动写一个完整
//! 数据集目录布局。单条标注的格式转换在 `conversion`；本模块只做布局、
//! 图片写入策略（copy / symlink / ROI 灰化）与 classes.txt。
//! ImageFolder 分类集导出（按属性值分目录的 crop）在 `image_folder_export`。

use crate::conversion::{
    ensure_safe_export_dir, export_annotation, export_coco_batch, export_yolo_classes_txt,
    ExportFormat, ExportItem,
};
use crate::Project;
use anyhow::Context;
use serde::Serialize;
use std::fs;
use std::path::Path;
use vlabel_core::LabelMeta;

/// Options for the YOLO dataset export driver.
#[derive(Debug, Clone)]
pub struct YoloExportOptions {
    /// Paint the area outside annotation ROIs with letterbox gray (default).
    /// `false` = coordinate-only export: images are never re-encoded.
    pub mask_outside_rois: bool,
    /// Link images into the export instead of copying them (default: copy).
    /// Masked images are still written as real files — a symlink can only
    /// reference the original, unmasked project image.
    pub link_images: bool,
}

impl Default for YoloExportOptions {
    fn default() -> Self {
        Self { mask_outside_rois: true, link_images: false }
    }
}

/// Write `src` to `dst` as a symlink (`link`) or a byte copy. Symlink targets
/// are canonicalized so the link stays valid from any working directory;
/// an existing `dst` (including a stale symlink) is replaced first, matching
/// the overwrite semantics of `fs::copy`.
pub(crate) fn link_or_copy(src: &Path, dst: &Path, link: bool) -> anyhow::Result<()> {
    if !link {
        fs::copy(src, dst)
            .with_context(|| format!("failed to copy {} -> {}", src.display(), dst.display()))?;
        return Ok(());
    }
    #[cfg(unix)]
    {
        let target =
            src.canonicalize().with_context(|| format!("failed to resolve {}", src.display()))?;
        if dst.symlink_metadata().is_ok() {
            fs::remove_file(dst)?;
        }
        std::os::unix::fs::symlink(&target, dst).with_context(|| {
            format!("failed to symlink {} -> {}", target.display(), dst.display())
        })?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        anyhow::bail!("linking images requires a Unix platform; use copying instead")
    }
}

/// Save `img` to `out_path`, re-encoding it: JPEG at high quality (the
/// default encoder quality would needlessly degrade training data), any other
/// extension via the image crate's default encoder for it.
pub(crate) fn save_image_high_quality(
    img: &image::DynamicImage,
    out_path: &Path,
) -> anyhow::Result<()> {
    let ext = out_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "jpg" | "jpeg" => {
            let file = fs::File::create(out_path)?;
            let mut writer = std::io::BufWriter::new(file);
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, 95)
                .encode_image(img)?;
        }
        _ => img.save(out_path)?,
    }
    Ok(())
}

/// Write one export image for the YOLO driver: when the annotation has ROIs
/// and `options.mask_outside_rois`, the area outside every ROI is painted
/// `mask::MASK_GRAY`; otherwise the image is written verbatim (linked when
/// `options.link_images`). Returns whether the image was actually masked
/// (masked images must be re-encoded; verbatim writes are not).
pub fn export_image_yolo(
    item: &ExportItem,
    images_dir: &Path,
    options: &YoloExportOptions,
) -> anyhow::Result<bool> {
    let out_path = images_dir.join(&item.file_name);
    if !options.mask_outside_rois || item.annotation.rois.is_empty() {
        link_or_copy(&item.image_path, &out_path, options.link_images)?;
        return Ok(false);
    }

    let img = image::open(&item.image_path)
        .with_context(|| format!("Failed to read image {:?}", item.image_path))?;
    let masked = crate::mask::mask_outside_rois(&img, &item.annotation.rois);
    if masked.as_bytes() == img.to_rgba8().as_raw().as_slice() {
        // The ROIs cover every pixel, so masking changed nothing: keep the
        // original bytes instead of taking a lossy re-encode.
        link_or_copy(&item.image_path, &out_path, options.link_images)?;
        return Ok(false);
    }

    save_image_high_quality(&masked, &out_path)?;
    Ok(true)
}

// The former design sketch here (properties → ImageFolder classification-set
// export) is implemented in `image_folder_export` (R5). Still open from the
// sketch: a `--split train:val` deterministic (stem-hash) split.

/// YOLO dataset export driver: writes `<out_dir>/images`, `<out_dir>/labels`
/// (one `<stem>.txt` per image with raw category ids) and `<out_dir>/classes.txt`.
/// Refuses output directories that would overwrite the project itself.
///
/// Unlike the VOC/COCO/LabelMe drivers, images whose annotation has ROIs are
/// written with the area outside the ROIs painted `mask::MASK_GRAY` (disable
/// with `YoloExportOptions::mask_outside_rois = false` for a coordinate-only
/// export): an ROI means "only this region is training signal", and letterbox
/// gray matches what YOLO sees as padding. Images without ROIs — and ROIs
/// covering every pixel — are copied byte-identically (see `export_image_yolo`).
pub fn export_dataset_yolo(
    project: &Project,
    items: &[ExportItem],
    meta: &LabelMeta,
    out_dir: &Path,
    options: &YoloExportOptions,
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
        if export_image_yolo(item, &images_dir, options)? {
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
        if options.link_images {
            log::warn!(
                "YOLO export: those {masked_count} masked image(s) were written as real files, not symlinks — masking changes their bytes"
            );
        }
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
            .map(|c| c.names.en.clone())
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
            flags: std::collections::HashMap::new(),
        });
    }

    let labelme = LabelMeOut {
        version: "5.0.1".to_string(),
        flags: std::collections::HashMap::new(),
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
    flags: std::collections::HashMap<String, serde_json::Value>,
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
    flags: std::collections::HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversion::collect_export_items;
    use crate::import::tests::test_meta;
    use image::GenericImageView;
    use std::path::PathBuf;
    use vlabel_core::io::save_meta;
    use vlabel_core::{new_label, Point, Polygon};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vlabel-utils-dsexp-test-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn export_item(dir: &Path, rois: Vec<Polygon<f32>>) -> ExportItem {
        let src_path = dir.join("a.png");
        let red = image::Rgba([200, 10, 10, 255]);
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(4, 4, red))
            .save(&src_path)
            .unwrap();

        let mut annotation = new_label("test");
        annotation.rois = rois; // left half: pixel centers x=.125/.375 inside, .625/.875 outside
        ExportItem {
            image_path: src_path.clone(),
            file_name: "a.png".to_string(),
            stem: "a".to_string(),
            width: 4,
            height: 4,
            annotation,
        }
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
    fn export_image_roi_masked_paints_outside_roi() {
        let dir = temp_dir("mask1");
        let roi = Polygon::from(vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 0.5, y: 0.0 },
            Point { x: 0.5, y: 1.0 },
            Point { x: 0.0, y: 1.0 },
        ]);
        let item = export_item(&dir, vec![roi]);

        let masked = export_image_yolo(&item, &dir, &YoloExportOptions::default()).unwrap();
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
    fn export_image_roi_mask_disabled_writes_original_bytes() {
        let dir = temp_dir("mask-off");
        let roi = Polygon::from(vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 0.5, y: 0.0 },
            Point { x: 0.5, y: 1.0 },
            Point { x: 0.0, y: 1.0 },
        ]);
        let item = export_item(&dir, vec![roi]);
        let src_path = item.image_path.clone();
        let options = YoloExportOptions { mask_outside_rois: false, ..Default::default() };

        let masked = export_image_yolo(&item, &dir, &options).unwrap();
        assert!(!masked, "--no-mask must never re-encode");
        assert_eq!(
            fs::read(dir.join("a.png")).unwrap(),
            fs::read(&src_path).unwrap(),
            "no-mask export keeps the original bytes despite the ROI"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_image_without_roi_is_copied_verbatim() {
        let dir = temp_dir("mask2");
        let item = export_item(&dir, vec![]);
        let src_path = item.image_path.clone();

        export_image_yolo(&item, &dir, &YoloExportOptions::default()).unwrap();
        assert_eq!(fs::read(dir.join("a.png")).unwrap(), fs::read(&src_path).unwrap());
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn export_image_link_mode_symlinks_unmasked_images() {
        let dir = temp_dir("link1");
        let out_dir = dir.join("out");
        fs::create_dir_all(&out_dir).unwrap();
        let item = export_item(&dir, vec![]);
        let options = YoloExportOptions { link_images: true, ..Default::default() };

        export_image_yolo(&item, &out_dir, &options).unwrap();
        let link = out_dir.join("a.png");
        assert!(
            link.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false),
            "unmasked image must be a symlink in link mode"
        );
        assert!(fs::metadata(&link).is_ok(), "symlink must resolve to the source image");

        // re-export over the existing symlink must not fail with "file exists"
        export_image_yolo(&item, &out_dir, &options).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn export_image_link_mode_masks_write_real_files() {
        let dir = temp_dir("link2");
        let out_dir = dir.join("out");
        fs::create_dir_all(&out_dir).unwrap();
        let roi = Polygon::from(vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 0.5, y: 0.0 },
            Point { x: 0.5, y: 1.0 },
            Point { x: 0.0, y: 1.0 },
        ]);
        let item = export_item(&dir, vec![roi]);
        let src_path = item.image_path.clone();
        let options = YoloExportOptions { link_images: true, ..Default::default() };

        let masked = export_image_yolo(&item, &out_dir, &options).unwrap();
        assert!(masked);
        let out = out_dir.join("a.png");
        assert!(
            out.symlink_metadata().map(|m| !m.file_type().is_symlink()).unwrap_or(false),
            "masked image must be a real file even in link mode"
        );
        assert_ne!(
            fs::read(&out).unwrap(),
            fs::read(&src_path).unwrap(),
            "masked bytes must differ from the original"
        );
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
        export_image_yolo(&item, &out_dir, &YoloExportOptions::default()).unwrap();
        assert_eq!(
            fs::read(out_dir.join("a.jpg")).unwrap(),
            fs::read(&src_path).unwrap(),
            "full-frame ROI masks nothing: original bytes must be kept, not re-encoded"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_dataset_yolo_writes_layout_and_guards_project() {
        let root = project_with_image("drv-yolo");
        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();
        let out = temp_dir("drv-yolo-out");

        export_dataset_yolo(&project, &items, &project.meta, &out, &YoloExportOptions::default())
            .unwrap();
        assert!(out.join("images/p.png").exists());
        assert!(out.join("labels/p.txt").exists());
        assert!(out.join("classes.txt").exists());

        // exporting onto the project itself must fail without touching it
        let before = fs::read(root.join("images/p.png")).unwrap();
        let err = export_dataset_yolo(&project, &items, &project.meta, &root, &Default::default())
            .unwrap_err();
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

        assert!(export_dataset_yolo(&project, &items, &meta, &out, &Default::default()).is_err());
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
}
