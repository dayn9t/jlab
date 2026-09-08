//! YOLO round-trip 对账：import → export 全链路后与源逐框核对。
//!
//! 工具化此前手工做过的对账（如 purge_review 的 1,508/1,508）：把 YOLO 源
//! 导入一次性临时项目（图片 symlink、vlabels 走真实 save/load JSON5），
//! 再以 no-mask 方式导出，最后与源逐框比对（IoU 容差级）。
//!
//! 结构：`reconcile` 是纯函数（functional core，直接单测）；
//! `verify_yolo_roundtrip` 是带 IO 的外壳。

use crate::conversion::collect_export_items;
use crate::dataset_export::{export_dataset_yolo, link_or_copy, YoloExportOptions};
use crate::import::{import_from_yolo, list_images_in_dir};
use crate::Project;
use anyhow::Context;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::Path;
use vlabel_core::{CatDef, LabelMeta, RoiConfig, ShapeConfig};

/// One YOLO label line: `<class> <xc> <yc> <w> <h>` (normalized).
#[derive(Debug, Clone, Copy, PartialEq)]
struct YoloBox {
    class: i32,
    coords: [f32; 4],
}

/// Summary of a round-trip reconciliation.
#[derive(Debug, Clone, Default)]
pub struct RoundtripReport {
    pub frames: usize,
    pub boxes_src: usize,
    pub boxes_out: usize,
    pub matched: usize,
    /// In the source but not matched in the export.
    pub missing: usize,
    /// In the export but not matched in the source.
    pub extra: usize,
    /// Frames present on one side only (whole-file loss, not per-box).
    pub frame_missing: usize,
    pub frame_extra: usize,
    /// Max absolute coordinate delta among matched pairs (xc/yc/w/h).
    pub max_delta: f32,
    pub diffs: Vec<DiffEntry>,
}

impl RoundtripReport {
    pub fn is_pass(&self) -> bool {
        self.missing == 0 && self.extra == 0 && self.frame_missing == 0 && self.frame_extra == 0
    }
}

/// One reconciliation difference, one JSON line per entry in the report file.
#[derive(Debug, Clone, Serialize)]
pub struct DiffEntry {
    pub stem: String,
    pub kind: DiffKind,
    pub class: Option<i32>,
    pub src: Option<[f32; 4]>,
    pub out: Option<[f32; 4]>,
    /// IoU of a below-tolerance near miss (diagnostic), otherwise absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_iou: Option<f32>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffKind {
    MissingBox,
    ExtraBox,
    FrameMissing,
    FrameExtra,
}

/// Boxes match when classes are equal and IoU >= `1 - iou_tolerance`.
pub fn verify_yolo_roundtrip(
    src_root: &Path,
    iou_tolerance: f32,
    report_path: Option<&Path>,
) -> anyhow::Result<RoundtripReport> {
    if !(0.0..1.0).contains(&iou_tolerance) {
        anyhow::bail!("iou tolerance must be in (0, 1), got {iou_tolerance}");
    }
    let temp = std::env::temp_dir().join(format!(
        "vlabel-verify-roundtrip-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before UNIX epoch")
            .as_nanos()
    ));
    let result = run_roundtrip(src_root, &temp, iou_tolerance);
    match result {
        Ok(report) => {
            if let Some(path) = report_path {
                write_diff_jsonl(path, &report)?;
            }
            fs::remove_dir_all(&temp)
                .with_context(|| format!("failed to clean up {}", temp.display()))?;
            Ok(report)
        }
        Err(err) => {
            // Keep the intermediate project: it is the debugging evidence.
            eprintln!("round-trip failed; intermediate project kept at {}", temp.display());
            Err(err)
        }
    }
}

fn run_roundtrip(
    src_root: &Path,
    temp_project: &Path,
    iou_tolerance: f32,
) -> anyhow::Result<RoundtripReport> {
    // -- functional core inputs: source labels and image stems --
    let src_labels = parse_labels_dir(&src_root.join("labels"))?;
    let images = list_images_in_dir(&src_root.join("images"))
        .context("failed to list source images; is this a YOLO root (images/ + labels/)?")?;
    let image_stems = images
        .iter()
        .map(|p| Path::new(p).file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string())
        .collect::<Vec<_>>();

    let label_stems: HashSet<&String> = src_labels.keys().collect();
    let orphan_labels = label_stems.iter().filter(|stem| !image_stems.contains(stem)).count();
    if orphan_labels > 0 {
        log::warn!("{orphan_labels} source label file(s) have no image; import ignores them");
    }

    // -- throwaway project: real import, real JSON5 save/load --
    let meta = build_meta(src_root, &src_labels)?;
    fs::create_dir_all(temp_project)?;
    vlabel_core::io::save_meta(temp_project.join("meta.json5"), &meta)?;
    fs::create_dir_all(temp_project.join("images"))?;
    fs::create_dir_all(temp_project.join("vlabels"))?;
    let project = Project::open(temp_project)?;
    for item in import_from_yolo(src_root, &meta)? {
        // Symlinked, not copied: verification reads dimensions only, and a
        // 13k-frame copy would be pure waste.
        link_or_copy(&item.source_path, &project.images_dir().join(&item.file_name), true)?;
        vlabel_core::io::save_annotation(
            project.annotation_path(&item.file_name),
            &item.annotation,
        )?;
    }

    // -- export (no-mask: coordinate fidelity only) --
    let out_dir = temp_project.join("export");
    let items = collect_export_items(&project)?;
    export_dataset_yolo(
        &project,
        &items,
        &meta,
        &out_dir,
        &YoloExportOptions { mask_outside_rois: false, link_images: true },
    )?;

    // -- reconcile --
    let out_labels = parse_labels_dir(&out_dir.join("labels"))?;
    Ok(reconcile(&image_stems, &src_labels, &out_labels, iou_tolerance))
}

/// Pure reconciliation over per-stem box lists. The frame set is driven by
/// `image_stems` (a source label file may legitimately be absent for a
/// negative frame; ultralytics treats missing txt as empty).
fn reconcile(
    image_stems: &[String],
    src: &BTreeMap<String, Vec<YoloBox>>,
    out: &BTreeMap<String, Vec<YoloBox>>,
    iou_tolerance: f32,
) -> RoundtripReport {
    let min_iou = 1.0 - iou_tolerance;
    let mut report = RoundtripReport { frames: image_stems.len(), ..Default::default() };

    for stem in image_stems {
        // clone-free borrow: absent keys read as empty
        let empty = Vec::new();
        let src_boxes = src.get(stem).unwrap_or(&empty);
        let out_boxes = out.get(stem).unwrap_or(&empty);
        report.boxes_src += src_boxes.len();
        report.boxes_out += out_boxes.len();
        if out.get(stem).is_none() {
            // the export lost this frame entirely (it always writes one txt
            // per image, so this is a driver-level failure safety net)
            report.frame_missing += 1;
            report.diffs.push(DiffEntry {
                stem: stem.clone(),
                kind: DiffKind::FrameMissing,
                class: None,
                src: None,
                out: None,
                best_iou: None,
            });
        }
        reconcile_frame(stem, src_boxes, out_boxes, min_iou, &mut report);
    }
    report
}

/// Greedy IoU matching for one frame: pairs sorted by IoU descending, each
/// box consumed once. Unmatched source boxes are missing, unmatched export
/// boxes are extra.
fn reconcile_frame(
    stem: &str,
    src_boxes: &[YoloBox],
    out_boxes: &[YoloBox],
    min_iou: f32,
    report: &mut RoundtripReport,
) {
    let mut pairs: Vec<(f32, usize, usize)> = Vec::new();
    for (i, s) in src_boxes.iter().enumerate() {
        for (j, o) in out_boxes.iter().enumerate() {
            let iou = iou(s, o);
            if s.class == o.class && iou >= min_iou {
                pairs.push((iou, i, j));
            }
        }
    }
    pairs.sort_by(|a, b| b.0.total_cmp(&a.0));

    let mut used_src = vec![false; src_boxes.len()];
    let mut used_out = vec![false; out_boxes.len()];
    for &(_, i, j) in &pairs {
        if used_src[i] || used_out[j] {
            continue;
        }
        used_src[i] = true;
        used_out[j] = true;
        report.matched += 1;
        report.max_delta =
            report.max_delta.max(coord_delta(&src_boxes[i].coords, &out_boxes[j].coords));
    }

    for (i, s) in src_boxes.iter().enumerate() {
        if !used_src[i] {
            report.missing += 1;
            report.diffs.push(DiffEntry {
                stem: stem.to_string(),
                kind: DiffKind::MissingBox,
                class: Some(s.class),
                src: Some(s.coords),
                out: None,
                best_iou: best_iou_for(s, out_boxes),
            });
        }
    }
    for (j, o) in out_boxes.iter().enumerate() {
        if !used_out[j] {
            report.extra += 1;
            report.diffs.push(DiffEntry {
                stem: stem.to_string(),
                kind: DiffKind::ExtraBox,
                class: Some(o.class),
                src: None,
                out: Some(o.coords),
                best_iou: None,
            });
        }
    }
}

/// Best IoU against the opposite side regardless of class/tolerance — helps
/// tell "box shifted a lot" from "box vanished".
fn best_iou_for(box_: &YoloBox, others: &[YoloBox]) -> Option<f32> {
    others.iter().map(|o| iou(box_, o)).fold(None, |acc, v| Some(acc.map_or(v, |a| a.max(v))))
}

fn coord_delta(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0_f32, f32::max)
}

fn iou(a: &YoloBox, b: &YoloBox) -> f32 {
    let to_rect =
        |c: &[f32; 4]| (c[0] - c[2] / 2.0, c[1] - c[3] / 2.0, c[0] + c[2] / 2.0, c[1] + c[3] / 2.0);
    let (ax1, ay1, ax2, ay2) = to_rect(&a.coords);
    let (bx1, by1, bx2, by2) = to_rect(&b.coords);
    let iw = (ax2.min(bx2) - ax1.max(bx1)).max(0.0);
    let ih = (ay2.min(by2) - ay1.max(by1)).max(0.0);
    if iw <= 0.0 || ih <= 0.0 {
        return 0.0;
    }
    let inter = iw * ih;
    let union = a.coords[2] * a.coords[3] + b.coords[2] * b.coords[3] - inter;
    if union <= 0.0 {
        0.0
    } else {
        inter / union
    }
}

/// Parse every `*.txt` in `dir` into stem → boxes. Strict: a malformed line
/// fails the whole verification naming file and line (silent skipping here
/// would hide exactly the corruption the tool exists to catch).
fn parse_labels_dir(dir: &Path) -> anyhow::Result<BTreeMap<String, Vec<YoloBox>>> {
    let mut labels = BTreeMap::new();
    if !dir.is_dir() {
        anyhow::bail!("labels directory not found: {}", dir.display());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("txt") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .context("invalid label file name")?
            .to_string();
        let content = fs::read_to_string(&path)?;
        let mut boxes = Vec::new();
        for (line_no, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            let parsed = if parts.len() == 5 {
                match (
                    parts[0].parse::<i32>(),
                    parts[1].parse::<f32>(),
                    parts[2].parse::<f32>(),
                    parts[3].parse::<f32>(),
                    parts[4].parse::<f32>(),
                ) {
                    (Ok(class), Ok(a), Ok(b), Ok(c), Ok(d)) => {
                        Some(YoloBox { class, coords: [a, b, c, d] })
                    }
                    _ => None,
                }
            } else {
                None
            };
            let Some(box_) = parsed else {
                anyhow::bail!(
                    "{}:{}: malformed YOLO line {:?} (expected: <class> <xc> <yc> <w> <h>)",
                    path.display(),
                    line_no + 1,
                    line
                );
            };
            boxes.push(box_);
        }
        labels.insert(stem, boxes);
    }
    Ok(labels)
}

/// Synthesize the throwaway project meta: categories from `classes.txt` when
/// the source ships one, else `class_<id>` names over the observed ids.
/// Ids must be contiguous 0-based (YOLO export writes line index = class id).
fn build_meta(
    src_root: &Path,
    labels: &BTreeMap<String, Vec<YoloBox>>,
) -> anyhow::Result<LabelMeta> {
    let classes_txt = src_root.join("classes.txt");
    let names: Vec<Option<String>> = if classes_txt.exists() {
        fs::read_to_string(&classes_txt)?
            .lines()
            .map(|line| {
                let name = line.trim();
                if name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let mut ids: BTreeSet<i32> = labels.values().flatten().map(|b| b.class).collect();
    for line in 0..names.len() {
        ids.insert(line as i32);
    }
    let ids: Vec<i32> = ids.into_iter().collect();
    if ids.iter().enumerate().any(|(line, &id)| id != line as i32) {
        anyhow::bail!(
            "class ids must be contiguous 0-based for YOLO export (line index = class id), got {ids:?}"
        );
    }

    let categories = ids
        .into_iter()
        .map(|id| CatDef {
            id,
            name: names
                .get(id as usize)
                .and_then(|n| n.clone())
                .unwrap_or_else(|| format!("class_{id}")),
            description: String::new(),
            hotkey: String::new(),
            color: "#FF0000".to_string(),
            properties: vec![],
        })
        .collect();

    Ok(LabelMeta {
        id: 1,
        name: "verify-roundtrip".to_string(),
        description: "throwaway project for YOLO round-trip verification".to_string(),
        shape: ShapeConfig { title_style: 0, thickness: 2, auto_save: true, vertex_radius: 10.0 },
        roi: RoiConfig { color: "#800080".to_string() },
        categories,
        property_types: vec![],
        property_special_values: vec![],
    })
}

fn write_diff_jsonl(path: &Path, report: &RoundtripReport) -> anyhow::Result<()> {
    use std::io::Write;
    let file = fs::File::create(path)
        .with_context(|| format!("failed to create diff report {}", path.display()))?;
    let mut writer = std::io::BufWriter::new(file);
    for diff in &report.diffs {
        serde_json::to_writer(&mut writer, diff)?;
        writeln!(writer)?;
    }
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_(class: i32, coords: [f32; 4]) -> YoloBox {
        YoloBox { class, coords }
    }

    fn labels(entries: &[(&str, Vec<YoloBox>)]) -> BTreeMap<String, Vec<YoloBox>> {
        entries.iter().map(|(stem, boxes)| (stem.to_string(), boxes.clone())).collect()
    }

    #[test]
    fn reconcile_full_match_with_tiny_delta() {
        let src = labels(&[("a", vec![box_(0, [0.5, 0.5, 0.2, 0.2])])]);
        let out = labels(&[("a", vec![box_(0, [0.5, 0.5000001, 0.2, 0.2])])]);

        let report = reconcile(&["a".to_string()], &src, &out, 1e-3);

        assert!(report.is_pass());
        assert_eq!(report.matched, 1);
        assert!(report.max_delta < 1e-6);
        assert!(report.diffs.is_empty());
    }

    #[test]
    fn reconcile_missing_and_extra_boxes() {
        let src =
            labels(&[("a", vec![box_(0, [0.3, 0.3, 0.1, 0.1]), box_(1, [0.7, 0.7, 0.1, 0.1])])]);
        let out =
            labels(&[("a", vec![box_(0, [0.3, 0.3, 0.1, 0.1]), box_(1, [0.9, 0.9, 0.1, 0.1])])]);

        let report = reconcile(&["a".to_string()], &src, &out, 1e-3);

        assert!(!report.is_pass());
        assert_eq!(report.matched, 1);
        assert_eq!(report.missing, 1);
        assert_eq!(report.extra, 1);
        assert_eq!(report.diffs.len(), 2);
    }

    #[test]
    fn reconcile_class_mismatch_never_matches() {
        // identical geometry, different class -> missing + extra, never a match
        let src = labels(&[("a", vec![box_(0, [0.5, 0.5, 0.2, 0.2])])]);
        let out = labels(&[("a", vec![box_(1, [0.5, 0.5, 0.2, 0.2])])]);

        let report = reconcile(&["a".to_string()], &src, &out, 1e-3);

        assert_eq!(report.matched, 0);
        assert_eq!(report.missing, 1);
        assert_eq!(report.extra, 1);
    }

    #[test]
    fn reconcile_absent_src_label_counts_as_negative_frame() {
        // ultralytics semantics: no txt = empty; must not be a frame diff
        let src = labels(&[]);
        let out = labels(&[("neg", vec![])]);

        let report = reconcile(&["neg".to_string()], &src, &out, 1e-3);

        assert!(report.is_pass(), "missing negative-frame txt is not a diff");
    }

    #[test]
    fn reconcile_export_missing_frame_file_fails() {
        let src = labels(&[("a", vec![box_(0, [0.5, 0.5, 0.2, 0.2])])]);
        let out = labels(&[]);

        let report = reconcile(&["a".to_string()], &src, &out, 1e-3);

        assert!(!report.is_pass());
        assert_eq!(report.missing, 1, "the frame's box is missing");
    }

    #[test]
    fn iou_identical_overlapping_disjoint() {
        let a = box_(0, [0.5, 0.5, 0.2, 0.2]);
        assert!((iou(&a, &a) - 1.0).abs() < 1e-6);
        let half = box_(0, [0.5, 0.5, 0.1, 0.2]);
        assert!((iou(&a, &half) - 0.5).abs() < 1e-6);
        let disjoint = box_(0, [0.9, 0.9, 0.1, 0.1]);
        assert_eq!(iou(&a, &disjoint), 0.0);
    }

    #[test]
    fn parse_labels_dir_is_strict_about_malformed_lines() {
        let dir = std::env::temp_dir().join(format!("vlabel-rt-parse-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.txt"), "0 0.5 0.5\n").unwrap();

        let err = parse_labels_dir(&dir).unwrap_err();
        assert!(format!("{err:#}").contains("malformed YOLO line"), "message was: {err:#}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn verify_yolo_roundtrip_end_to_end_passes() {
        let root = std::env::temp_dir().join(format!("vlabel-rt-e2e-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src/images")).unwrap();
        fs::create_dir_all(root.join("src/labels")).unwrap();
        // positive frame, 6-decimal coords exactly like the real pipeline
        image::RgbImage::from_pixel(8, 8, image::Rgb([120, 120, 120]))
            .save(root.join("src/images/1_pos.jpg"))
            .unwrap();
        fs::write(
            root.join("src/labels/1_pos.txt"),
            "0 0.972822 0.549871 0.054356 0.197841\n0 0.221356 0.408914 0.071921 0.269547\n",
        )
        .unwrap();
        // negative frame with a blank label file
        image::RgbImage::from_pixel(8, 8, image::Rgb([10, 10, 10]))
            .save(root.join("src/images/2_neg.jpg"))
            .unwrap();
        fs::write(root.join("src/labels/2_neg.txt"), "").unwrap();
        fs::write(root.join("src/classes.txt"), "person\n").unwrap();

        let report = verify_yolo_roundtrip(&root.join("src"), 1e-3, None).unwrap();

        assert!(report.is_pass(), "diffs: {:?}", report.diffs);
        assert_eq!(report.frames, 2);
        assert_eq!(report.boxes_src, 2);
        assert_eq!(report.matched, 2);
        assert!(report.max_delta <= 1e-3, "max delta {}", report.max_delta);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn verify_yolo_roundtrip_reports_dropped_box() {
        // a source box that import must skip: class id absent from meta is
        // impossible here (meta is derived), so drop a box via a class not in
        // classes.txt + non-contiguous ids are rejected instead — use a
        // tolerance too tight for the 6-decimal formatting instead.
        let root = std::env::temp_dir().join(format!("vlabel-rt-tol-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src/images")).unwrap();
        fs::create_dir_all(root.join("src/labels")).unwrap();
        image::RgbImage::from_pixel(8, 8, image::Rgb([5, 5, 5]))
            .save(root.join("src/images/a.jpg"))
            .unwrap();
        fs::write(root.join("src/labels/a.txt"), "0 0.123456 0.654321 0.111111 0.222222\n")
            .unwrap();

        // 1e-9 tolerance: formatting at 6 decimals cannot round-trip below 1e-6
        let report = verify_yolo_roundtrip(&root.join("src"), 1e-9, None).unwrap();

        assert!(!report.is_pass());
        assert_eq!(report.missing, 1);
        assert_eq!(report.extra, 1);
        assert!(report.diffs[0].best_iou.is_some(), "near miss should carry a diagnostic IoU");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn verify_yolo_roundtrip_rejects_noncontiguous_class_ids() {
        let root = std::env::temp_dir().join(format!("vlabel-rt-ids-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src/images")).unwrap();
        fs::create_dir_all(root.join("src/labels")).unwrap();
        image::RgbImage::from_pixel(8, 8, image::Rgb([5, 5, 5]))
            .save(root.join("src/images/a.jpg"))
            .unwrap();
        fs::write(root.join("src/labels/a.txt"), "0 0.5 0.5 0.2 0.2\n2 0.3 0.3 0.1 0.1\n").unwrap();

        let err = verify_yolo_roundtrip(&root.join("src"), 1e-3, None).unwrap_err();
        assert!(format!("{err:#}").contains("contiguous"), "message was: {err:#}");
        let _ = fs::remove_dir_all(&root);
    }
}
