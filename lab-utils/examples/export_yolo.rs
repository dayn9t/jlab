//! Export all JLab project labels (`<project>/labels/*.yaml`) to YOLO format
//! (`<out>/labels/*.txt`). Images are not copied; link or copy them separately.
//!
//! Usage: `cargo run -p lab-utils --example export_yolo -- <project_dir> <out_dir>`
//!
//! Class ids are written as-is (they must exist in the project's `meta.yaml`).

use anyhow::{Context, Result};
use lab_utils::conversion::{export_annotation, ExportFormat};
use lab_utils::project::Project;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let project_dir =
        PathBuf::from(args.next().context("usage: export_yolo <project_dir> <out_dir>")?);
    let out_dir = PathBuf::from(args.next().context("usage: export_yolo <project_dir> <out_dir>")?);

    let project = Project::open(&project_dir)?;
    let out_labels = out_dir.join("labels");
    fs::create_dir_all(&out_labels)?;

    let mut total_files = 0usize;
    let mut total_boxes = 0usize;

    for image in project.list_images()? {
        let name =
            image.file_name().and_then(|s| s.to_str()).context("invalid image name")?.to_string();
        let Some(annotation) = project.load_annotation(&name)? else {
            continue;
        };
        let stem = name
            .strip_suffix(".jpg")
            .or_else(|| name.strip_suffix(".jpeg"))
            .or_else(|| name.strip_suffix(".png"))
            .unwrap_or(&name);
        export_annotation(
            out_labels.join(format!("{stem}.txt")),
            &annotation,
            &project.meta,
            &image.to_string_lossy(),
            0,
            0,
            ExportFormat::Yolo,
        )?;
        total_files += 1;
        total_boxes += annotation.objects.len();
    }

    println!("exported {total_files} labels, {total_boxes} boxes -> {}", out_labels.display());
    Ok(())
}
