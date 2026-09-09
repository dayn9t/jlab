//! ImageFolder classification-set export (convert-pipeline R5): one cropped
//! image per object, filed into `out/<property-value>/` class directories.
//!
//! Design (implemented from the dataset_export.rs design sketch):
//! - class dir = property *value name* from meta.json5 (not the raw id), so
//!   the exported tree is self-describing;
//! - objects whose property is missing — or set to a special ("uncertain")
//!   value such as `occluded` — land in `unlabeled/` instead of being
//!   silently dropped; downstream training decides whether to exclude them.
//!   Frames without objects contribute nothing (a classification set has no
//!   negative-sample concept; the detection pipeline keeps that role);
//! - crop = polygon bbox expanded by `crop_margin` (fraction of the bbox's
//!   larger pixel side, default 0.05), clamped to the image bounds. A crop is
//!   a new re-encoded file (JPEG at quality 95), so `--symlink` never applies;
//! - everything is validated (property exists and is unambiguous, value names
//!   sanitize to unique non-reserved dirs, every annotation resolves to a
//!   known value) *before* the first file is written;
//! - sketch item NOT implemented: `--split train:val` deterministic stem-hash
//!   split.

use crate::conversion::{ensure_safe_export_dir, ExportItem};
use crate::Project;
use anyhow::Context;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use vlabel_core::{find_special_value, LabelMeta, Object, Polygon, PropDef};

/// Directory for objects without a usable value for the exported property
/// (missing entry, or a special value like `occluded`): kept visible instead
/// of silently dropped.
pub const UNLABELED_DIR: &str = "unlabeled";

/// Default bbox expansion margin, as a fraction of the bbox's larger pixel
/// side.
pub const DEFAULT_CROP_MARGIN: f32 = 0.05;

/// Options for the ImageFolder classification-set export driver.
#[derive(Debug, Clone)]
pub struct ImageFolderExportOptions {
    /// Property name (`PropDef.names.en` in meta.json5 `property_types`)
    /// whose values become the class directories.
    pub property: String,
    /// Extra margin around each object's bbox, as a fraction of the bbox's
    /// larger pixel side.
    pub crop_margin: f32,
}

/// Counts reported by an ImageFolder export.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImageFolderExportSummary {
    /// Crop images written.
    pub crops: usize,
    /// Class directories created (excluding `unlabeled/`).
    pub classes: usize,
    /// Crops filed into `unlabeled/`.
    pub unlabeled: usize,
    /// Objects skipped: polygon with fewer than 3 vertices or zero extent.
    pub skipped: usize,
}

/// ImageFolder classification-set export driver: writes one cropped,
/// re-encoded image per annotated object into `out/<value-name>/`, plus
/// `out/unlabeled/` for objects without a usable value (see module docs).
/// Refuses output directories that would overwrite the project itself.
///
/// Two passes: every object is resolved to a class dir and a crop rect
/// *before* anything is written, so schema drift (unknown value ids, see
/// `class_dir_for`) fails with the output directory untouched.
pub fn export_dataset_image_folder(
    project: &Project,
    items: &[ExportItem],
    meta: &LabelMeta,
    out_dir: &Path,
    options: &ImageFolderExportOptions,
) -> anyhow::Result<ImageFolderExportSummary> {
    ensure_safe_export_dir(&project.root, out_dir)?;
    if options.crop_margin < 0.0 {
        anyhow::bail!(
            "crop margin must be >= 0 (fraction of the bbox's larger side), got {}",
            options.crop_margin
        );
    }
    let prop = find_property_def(meta, &options.property)?;
    let value_dirs = class_dir_map(prop)?;

    // Pass 1: plan every crop (and fail on schema drift) before writing.
    let mut skipped = 0usize;
    let mut plan: Vec<PlannedCrop> = Vec::new();
    for (item_index, item) in items.iter().enumerate() {
        for obj in &item.annotation.objects {
            let dir = class_dir_for(obj, prop, meta, &value_dirs, &item.file_name)?;
            match crop_rect(&obj.polygon, item.width, item.height, options.crop_margin) {
                Some(rect) => plan.push(PlannedCrop { item_index, obj, dir, rect }),
                None => {
                    // Visible, not silent (import skipped-row precedent): the
                    // object has no croppable region.
                    log::warn!(
                        "image-folder export: skipping object {} on {}: polygon has fewer than 3 vertices or zero extent",
                        obj.id,
                        item.file_name
                    );
                    skipped += 1;
                }
            }
        }
    }

    // Pass 2: write (each source image is opened once per crop group).
    fs::create_dir_all(out_dir)?;
    let mut written_dirs: BTreeSet<&str> = BTreeSet::new();
    let mut unlabeled = 0usize;
    let mut current_item = usize::MAX;
    let mut img: Option<image::DynamicImage> = None;
    for PlannedCrop { item_index, obj, dir, rect: (x, y, w, h) } in &plan {
        if *item_index != current_item {
            let item = &items[*item_index];
            img = Some(
                image::open(&item.image_path)
                    .with_context(|| format!("Failed to read image {:?}", item.image_path))?,
            );
            current_item = *item_index;
        }
        let item = &items[*item_index];
        let dir_path = out_dir.join(dir);
        fs::create_dir_all(&dir_path)?;
        let ext = Path::new(&item.file_name)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_else(|| "png".to_string());
        let out_path = dir_path.join(format!("{}_{}.{}", item.stem, obj.id, ext));
        let crop = img.as_ref().expect("image loaded for current item").crop_imm(*x, *y, *w, *h);
        crate::dataset_export::save_image_high_quality(&crop, &out_path)
            .with_context(|| format!("failed to write crop {:?}", out_path))?;
        if *dir == UNLABELED_DIR {
            unlabeled += 1;
        } else {
            written_dirs.insert(dir);
        }
    }

    Ok(ImageFolderExportSummary {
        crops: plan.len(),
        classes: written_dirs.len(),
        unlabeled,
        skipped,
    })
}

/// A crop resolved by pass 1: source item, object, class dir, pixel rect.
struct PlannedCrop<'a> {
    item_index: usize,
    obj: &'a Object,
    dir: &'a str,
    rect: (u32, u32, u32, u32),
}

/// Resolve an object's class dir: the sanitized value name for a normal
/// value; `UNLABELED_DIR` for a missing entry or a special ("uncertain")
/// value; a hard error for a value id that matches nothing (schema drift
/// between meta.json5 and the vlabels — never silently relabeled).
fn class_dir_for<'a>(
    obj: &Object,
    prop: &PropDef,
    meta: &LabelMeta,
    value_dirs: &'a BTreeMap<i32, String>,
    file_name: &str,
) -> anyhow::Result<&'a str> {
    let entry = match obj.properties.iter().find(|e| e.id == prop.id) {
        None => return Ok(UNLABELED_DIR),
        Some(entry) => entry,
    };
    if let Some(dir) = value_dirs.get(&entry.value) {
        return Ok(dir.as_str());
    }
    if find_special_value(meta, entry.value).is_some() {
        return Ok(UNLABELED_DIR);
    }
    anyhow::bail!(
        "object {} on {} has value id {} for property {:?} that matches no value in property_types and no special value: fix meta.json5 or the annotation",
        obj.id,
        file_name,
        entry.value,
        prop.names.en
    );
}

/// Find the property definition by name; fail on unknown or ambiguous names
/// (names are the CLI-facing key, so drift must not silently pick a wrong one).
fn find_property_def<'a>(meta: &'a LabelMeta, name: &str) -> anyhow::Result<&'a PropDef> {
    let matches: Vec<&PropDef> =
        meta.property_types.iter().filter(|p| p.names.en == name).collect();
    if matches.is_empty() {
        anyhow::bail!(
            "property {:?} is not defined in meta.json5 property_types (available: {:?})",
            name,
            meta.property_types.iter().map(|p| p.names.en.as_str()).collect::<Vec<_>>()
        );
    }
    if matches.len() > 1 {
        anyhow::bail!(
            "property name {:?} is ambiguous: {} definitions with ids {:?}; property names must be unique to export by name",
            name,
            matches.len(),
            matches.iter().map(|p| p.id).collect::<Vec<_>>()
        );
    }
    Ok(matches[0])
}

/// Map characters outside `[A-Za-z0-9_-]` (path separators, dots, spaces,
/// non-ASCII, ...) to `_` so any value name is a single safe dir component.
fn sanitize_class_dir_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if sanitized.is_empty() {
        "_".to_string()
    } else {
        sanitized
    }
}

/// Build value-id -> sanitized class dir name, rejecting collisions: two
/// values sanitizing to the same dir would silently merge two classes, and a
/// value named `unlabeled` would collide with the unlabeled bucket.
fn class_dir_map(prop: &PropDef) -> anyhow::Result<BTreeMap<i32, String>> {
    let mut dirs: BTreeMap<i32, String> = BTreeMap::new();
    let mut owners: BTreeMap<String, i32> = BTreeMap::new();
    for value in &prop.values {
        let dir = sanitize_class_dir_name(&value.names.en);
        if dir == UNLABELED_DIR {
            anyhow::bail!(
                "value {:?} (id {}) sanitizes to the reserved directory {:?}",
                value.names.en,
                value.id,
                UNLABELED_DIR
            );
        }
        if let Some(prev_id) = owners.insert(dir.clone(), value.id) {
            anyhow::bail!(
                "values {:?} (id {}) and {:?} (id {}) collide on directory {:?} after sanitizing; rename one in meta.json5",
                prop.values.iter().find(|v| v.id == prev_id).map(|v| v.names.en.as_str()).unwrap_or("?"),
                prev_id,
                value.names.en,
                value.id,
                dir
            );
        }
        dirs.insert(value.id, dir);
    }
    Ok(dirs)
}

/// Object bbox (polygon vertices) expanded by `margin` on every side and
/// clamped to the image, as an integer pixel rect `(x, y, w, h)` rounded
/// outward so the margin never shrinks the box. `None` for polygons with
/// fewer than 3 vertices or zero pixel extent (no region to crop) — matching
/// the validity bar of `validation::validate_annotation`.
fn crop_rect(
    polygon: &Polygon<f32>,
    width: u32,
    height: u32,
    margin: f32,
) -> Option<(u32, u32, u32, u32)> {
    if polygon.0.len() < 3 {
        return None;
    }
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for point in &polygon.0 {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    let (w, h) = (width as f32, height as f32);
    let (bx0, by0, bx1, by1) = (min_x * w, min_y * h, max_x * w, max_y * h);
    // Isotropic pixel margin: every side grows by margin x the bbox's larger
    // side, so thin boxes get the same absolute context on all sides.
    let m = margin * (bx1 - bx0).max(by1 - by0);
    let px0 = (bx0 - m).clamp(0.0, w).floor();
    let py0 = (by0 - m).clamp(0.0, h).floor();
    let px1 = (bx1 + m).clamp(0.0, w).ceil();
    let py1 = (by1 + m).clamp(0.0, h).ceil();
    if px1 <= px0 || py1 <= py0 {
        return None;
    }
    Some((px0 as u32, py0 as u32, (px1 - px0) as u32, (py1 - py0) as u32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversion::collect_export_items;
    use crate::import::tests::test_meta;
    use image::GenericImageView;
    use std::path::PathBuf;
    use vlabel_core::io::save_meta;
    use vlabel_core::{
        add_object, new_label, new_object, Point, PropertyEntry, SpecialValue, ValueDef,
    };

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vlabel-utils-ifexp-test-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn prop_entry(value: i32) -> PropertyEntry {
        PropertyEntry { id: 3, value, confidence: 1.0 }
    }

    fn quad(x0: f32, y0: f32, x1: f32, y1: f32) -> Polygon<f32> {
        Polygon::from(vec![
            Point { x: x0, y: y0 },
            Point { x: x1, y: y0 },
            Point { x: x1, y: y1 },
            Point { x: x0, y: y1 },
        ])
    }

    fn object(id: i32, x0: f32, y0: f32, x1: f32, y1: f32, props: Vec<PropertyEntry>) -> Object {
        let mut obj = new_object(id, 0, quad(x0, y0, x1, y1));
        obj.properties = props;
        obj
    }

    fn value_def(id: i32, name: &str) -> ValueDef {
        ValueDef {
            id,
            names: vlabel_core::LocalizedNames::en(name),
            description: String::new(),
            hotkey: String::new(),
            color: "#000000".to_string(),
            sign: String::new(),
        }
    }

    fn meta_with_wet_property() -> LabelMeta {
        let mut meta = test_meta();
        meta.property_types = vec![PropDef {
            id: 3,
            names: vlabel_core::LocalizedNames::en("wet"),
            description: String::new(),
            values: vec![value_def(0, "dry"), value_def(1, "wet floor")],
        }];
        meta.property_special_values = vec![SpecialValue {
            id: -1,
            names: vlabel_core::LocalizedNames::en("occluded"),
            ..value_def(-1, "")
        }];
        meta
    }

    /// 8x8 image (left half red, right half blue) + vlabels/p.json5 holding
    /// `objects`; returns the project root.
    fn project_with_objects(tag: &str, objects: Vec<Object>) -> PathBuf {
        let root = temp_dir(tag);
        fs::create_dir_all(root.join("images")).unwrap();
        fs::create_dir_all(root.join("vlabels")).unwrap();
        save_meta(root.join("meta.json5"), &meta_with_wet_property()).unwrap();
        image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(8, 8, |x, _| {
            if x < 4 {
                image::Rgb([200, 10, 10])
            } else {
                image::Rgb([10, 10, 200])
            }
        }))
        .save(root.join("images/p.png"))
        .unwrap();

        let mut label = new_label("test");
        for obj in objects {
            add_object(&mut label, obj);
        }
        let project = Project::open(&root).unwrap();
        project.save_annotation("p.png", &label).unwrap();
        root
    }

    fn options(property: &str, margin: f32) -> ImageFolderExportOptions {
        ImageFolderExportOptions { property: property.to_string(), crop_margin: margin }
    }

    #[test]
    fn layout_groups_objects_by_property_value() {
        let root = project_with_objects(
            "if-layout",
            vec![
                object(0, 0.0, 0.0, 0.5, 0.5, vec![prop_entry(0)]), // dry, top-left
                object(1, 0.5, 0.0, 1.0, 0.5, vec![prop_entry(1)]), // "wet floor"
                object(2, 0.5, 0.5, 1.0, 1.0, vec![]),              // missing property
                object(3, 0.0, 0.5, 0.5, 1.0, vec![prop_entry(-1)]), // occluded
            ],
        );
        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();
        let out = temp_dir("if-layout-out");

        let summary = export_dataset_image_folder(
            &project,
            &items,
            &project.meta,
            &out,
            &options("wet", 0.0),
        )
        .unwrap();

        assert_eq!(
            summary,
            ImageFolderExportSummary { crops: 4, classes: 2, unlabeled: 2, skipped: 0 }
        );
        let mut dirs: Vec<String> = fs::read_dir(&out)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        dirs.sort();
        assert_eq!(dirs, vec!["dry", "unlabeled", "wet_floor"]);
        // margin 0 -> exact bbox crop, named <stem>_<object-id>.png
        let dry = image::open(out.join("dry/p_0.png")).unwrap();
        assert_eq!(dry.dimensions(), (4, 4));
        assert_eq!(dry.get_pixel(1, 1), image::Rgba([200, 10, 10, 255])); // left half red
        let wet = image::open(out.join("wet_floor/p_1.png")).unwrap();
        assert_eq!(wet.dimensions(), (4, 4));
        assert_eq!(wet.get_pixel(1, 1), image::Rgba([10, 10, 200, 255])); // right half blue
        assert!(out.join("unlabeled/p_2.png").exists());
        assert!(out.join("unlabeled/p_3.png").exists());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn crop_margin_expands_by_larger_side_and_clamps() {
        let root = project_with_objects(
            "if-margin",
            vec![
                // bbox [2,6]x[2,6] px; margin 0.5 * 4px = 2px/side -> clamped to 8x8
                object(0, 0.25, 0.25, 0.75, 0.75, vec![prop_entry(0)]),
                // bbox [2,4]x[2,4] px; margin 0.5 * 2px = 1px/side -> (1,1)-(5,5) = 4x4
                object(1, 0.25, 0.25, 0.5, 0.5, vec![prop_entry(0)]),
            ],
        );
        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();
        let out = temp_dir("if-margin-out");

        export_dataset_image_folder(&project, &items, &project.meta, &out, &options("wet", 0.5))
            .unwrap();

        assert_eq!(image::open(out.join("dry/p_0.png")).unwrap().dimensions(), (8, 8));
        assert_eq!(image::open(out.join("dry/p_1.png")).unwrap().dimensions(), (4, 4));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn missing_property_definition_fails_listing_available() {
        let root = project_with_objects("if-noprop", vec![object(0, 0.0, 0.0, 0.5, 0.5, vec![])]);
        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();
        let out = temp_dir("if-noprop-out");

        let err = export_dataset_image_folder(
            &project,
            &items,
            &project.meta,
            &out,
            &options("nope", DEFAULT_CROP_MARGIN),
        )
        .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("not defined"), "message was: {msg}");
        assert!(msg.contains("wet"), "must list available property names: {msg}");
        assert!(
            out.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true),
            "nothing must be written when validation fails"
        );
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn ambiguous_property_name_fails() {
        let root = project_with_objects("if-amb", vec![object(0, 0.0, 0.0, 0.5, 0.5, vec![])]);
        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();
        let mut meta = project.meta.clone();
        meta.property_types.push(PropDef {
            id: 4,
            names: vlabel_core::LocalizedNames::en("wet"),
            description: String::new(),
            values: vec![],
        });
        let out = temp_dir("if-amb-out");

        let err = export_dataset_image_folder(&project, &items, &meta, &out, &options("wet", 0.0))
            .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("ambiguous"), "message was: {msg}");
        assert!(out.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn unknown_value_id_fails_before_any_file_is_written() {
        let root = project_with_objects(
            "if-badval",
            vec![
                object(0, 0.0, 0.0, 0.5, 0.5, vec![prop_entry(0)]), // valid, written first
                object(1, 0.5, 0.0, 1.0, 0.5, vec![prop_entry(99)]), // schema drift
            ],
        );
        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();
        let out = temp_dir("if-badval-out");

        let err = export_dataset_image_folder(
            &project,
            &items,
            &project.meta,
            &out,
            &options("wet", 0.0),
        )
        .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("99"), "message was: {msg}");
        assert!(
            out.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true),
            "validation pass must fail before the first crop is written"
        );
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn degenerate_polygons_are_skipped() {
        let mut empty = new_object(1, 0, Polygon::empty());
        empty.properties = vec![prop_entry(0)];
        let point = object(2, 0.25, 0.25, 0.25, 0.25, vec![prop_entry(0)]); // zero extent
        let root = project_with_objects(
            "if-degen",
            vec![object(0, 0.0, 0.0, 0.5, 0.5, vec![prop_entry(0)]), empty, point],
        );
        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();
        let out = temp_dir("if-degen-out");

        let summary = export_dataset_image_folder(
            &project,
            &items,
            &project.meta,
            &out,
            &options("wet", 0.0),
        )
        .unwrap();

        assert_eq!(
            summary,
            ImageFolderExportSummary { crops: 1, classes: 1, unlabeled: 0, skipped: 2 }
        );
        assert!(out.join("dry/p_0.png").exists());
        assert!(!out.join("dry/p_1.png").exists());
        assert!(!out.join("dry/p_2.png").exists());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn negative_margin_fails() {
        let root = project_with_objects("if-negm", vec![object(0, 0.0, 0.0, 0.5, 0.5, vec![])]);
        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();
        let out = temp_dir("if-negm-out");

        let err = export_dataset_image_folder(
            &project,
            &items,
            &project.meta,
            &out,
            &options("wet", -0.1),
        )
        .unwrap_err();
        assert!(format!("{err:#}").contains("margin"), "message was: {err:#}");
        assert!(out.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn sanitize_maps_illegal_characters_to_underscore() {
        assert_eq!(sanitize_class_dir_name("wet floor"), "wet_floor");
        assert_eq!(sanitize_class_dir_name("a/b"), "a_b");
        assert_eq!(sanitize_class_dir_name("a.b"), "a_b"); // no dot components: no hidden/`..` dirs
        assert_eq!(sanitize_class_dir_name(""), "_");
        assert_eq!(sanitize_class_dir_name("湿润"), "__");
        assert_eq!(sanitize_class_dir_name("A-1_x"), "A-1_x");
    }

    #[test]
    fn colliding_sanitized_value_names_fail() {
        let root = project_with_objects("if-coll", vec![object(0, 0.0, 0.0, 0.5, 0.5, vec![])]);
        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();
        let mut meta = project.meta.clone();
        meta.property_types[0].values = vec![value_def(0, "a/b"), value_def(1, "a_b")];
        let out = temp_dir("if-coll-out");

        let err = export_dataset_image_folder(&project, &items, &meta, &out, &options("wet", 0.0))
            .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("collide"), "message was: {msg}");
        assert!(out.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn reserved_unlabeled_value_name_fails() {
        let root = project_with_objects("if-resv", vec![object(0, 0.0, 0.0, 0.5, 0.5, vec![])]);
        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();
        let mut meta = project.meta.clone();
        meta.property_types[0].values = vec![value_def(0, "unlabeled"), value_def(1, "wet")];
        let out = temp_dir("if-resv-out");

        let err = export_dataset_image_folder(&project, &items, &meta, &out, &options("wet", 0.0))
            .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("unlabeled"), "message was: {msg}");
        assert!(msg.contains("reserved"), "message was: {msg}");
        assert!(out.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn export_refuses_output_inside_project() {
        let root = project_with_objects("if-guard", vec![object(0, 0.0, 0.0, 0.5, 0.5, vec![])]);
        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();

        let err = export_dataset_image_folder(
            &project,
            &items,
            &project.meta,
            &root,
            &options("wet", 0.0),
        )
        .unwrap_err();
        assert!(format!("{err:#}").contains("project root"), "message was: {err:#}");
        let _ = fs::remove_dir_all(&root);
    }
}
