//! Non-interactive YOLO dataset import: convert `<src>/labels/*.txt` into
//! JLab project labels (`<dst>/labels/*.yaml`). Images are not copied.
//!
//! Usage: `cargo run -p lab-core --example import_yolo -- <src> <dst>`
//!
//! Each YOLO line `<class> <x_center> <y_center> <w> <h>` (normalized)
//! becomes a rectangular polygon object with the class id preserved.

use anyhow::{Context, Result};
use lab_core::{add_object, new_label, new_object, next_object_id, Point, Polygon};
use std::fs;
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let src = PathBuf::from(args.next().context("usage: import_yolo <src> <dst>")?);
    let dst = PathBuf::from(args.next().context("usage: import_yolo <src> <dst>")?);

    let labels_dir = src.join("labels");
    let out_dir = dst.join("labels");
    fs::create_dir_all(&out_dir)?;

    let mut total_files = 0usize;
    let mut total_objects = 0usize;

    for entry in fs::read_dir(&labels_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).context("invalid stem")?.to_string();
        let content = fs::read_to_string(&path)?;

        let mut label = new_label("yolo-import");
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() != 5 {
                continue;
            }
            let (Ok(class_id), Ok(xc), Ok(yc), Ok(w), Ok(h)) = (
                parts[0].parse::<i32>(),
                parts[1].parse::<f32>(),
                parts[2].parse::<f32>(),
                parts[3].parse::<f32>(),
                parts[4].parse::<f32>(),
            ) else {
                continue;
            };
            let (xmin, ymin) = (xc - w / 2.0, yc - h / 2.0);
            let (xmax, ymax) = (xc + w / 2.0, yc + h / 2.0);
            let polygon = Polygon::from(vec![
                Point { x: xmin, y: ymin },
                Point { x: xmax, y: ymin },
                Point { x: xmax, y: ymax },
                Point { x: xmin, y: ymax },
            ]);
            let id = next_object_id(&label);
            add_object(&mut label, new_object(id, class_id, polygon));
        }

        lab_core::io::save_annotation(out_dir.join(format!("{stem}.yaml")), &label)?;
        total_files += 1;
        total_objects += label.objects.len();
    }

    println!("imported {total_files} labels, {total_objects} objects -> {}", out_dir.display());
    Ok(())
}
