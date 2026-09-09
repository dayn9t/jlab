//! 单条标注的格式转换（YOLO / VOC / COCO 行内容）与导出前置项：
//! classes.txt、目录安全守卫、ExportItem 收集。完整数据集目录布局的
//! 驱动在 `dataset_export`。

use crate::Project;
use anyhow::Context;
use image::GenericImageView;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::tests::test_meta;
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
