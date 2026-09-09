//! Non-interactive annotation format converter for VLabel projects.
//!
//! Usage:
//!   vlabel-convert import --format <yolo|voc|coco|label-me> [--roi <file>] <src> <project_dir>
//!       (coco: <src> is the annotation json; `--images <dir>` is required)
//!   vlabel-convert export --format <yolo|voc|coco|label-me> [--no-mask] [--symlink]
//!       <project_dir> <out_dir>  (yolo-only image options)
//!   vlabel-convert export --format image-folder --property <name> [--crop <margin>]
//!       <project_dir> <out_dir>  (classification set: crops per property value)
//!   vlabel-convert migrate-yaml [--global] <project_dir>
//!   vlabel-convert verify-roundtrip [--iou-tolerance 0.01] [--report <file.jsonl>] <yolo_src>

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashSet;
use std::path::PathBuf;
use vlabel_utils::conversion::{collect_export_items, ensure_safe_export_dir};
use vlabel_utils::dataset_export::{
    export_dataset_coco, export_dataset_labelme, export_dataset_voc, export_dataset_yolo,
    YoloExportOptions,
};
use vlabel_utils::image_folder_export::{
    export_dataset_image_folder, ImageFolderExportOptions, DEFAULT_CROP_MARGIN,
};
use vlabel_utils::import::{
    import_from_coco, import_from_labelme, import_from_voc, import_from_yolo,
    merge_imported_images, ImportedImage,
};
use vlabel_utils::migrate;
use vlabel_utils::roi_inject;
use vlabel_utils::Project;

#[derive(Parser)]
#[command(name = "vlabel-convert", version, about = "VLabel annotation format converter")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Import an external dataset into a VLabel project
    Import {
        /// Dataset format
        #[arg(short, long)]
        format: Format,
        /// Source root (coco: path to annotation json)
        src: PathBuf,
        /// Images directory (required when --format coco)
        #[arg(long)]
        images: Option<PathBuf>,
        /// JSON5 map of file-stem prefix → ROI polygons to inject into the
        /// imported frames, e.g. {"1": [[[0,0],[1,0],[1,1],[0,1]]]}
        #[arg(long, value_name = "FILE")]
        roi: Option<PathBuf>,
        /// Target VLabel project directory (must contain meta.json5)
        project: PathBuf,
    },
    /// Export a VLabel project to an external dataset
    Export {
        /// Dataset format
        #[arg(short, long)]
        format: Format,
        /// YOLO only: skip letterbox-gray masking outside ROIs (coordinates-only
        /// export; images are written verbatim)
        #[arg(long)]
        no_mask: bool,
        /// YOLO only: symlink images into the export instead of copying
        /// (masked images are still written as real files)
        #[arg(long)]
        symlink: bool,
        /// image-folder only: property name (from meta.json5 property_types)
        /// whose values become the class directories
        #[arg(long, value_name = "NAME")]
        property: Option<String>,
        /// image-folder only: extra crop margin around each object's bbox, as
        /// a fraction of the bbox's larger side (default 0.05)
        #[arg(long, value_name = "MARGIN")]
        crop: Option<f32>,
        /// VLabel project directory
        project: PathBuf,
        /// Output directory
        out: PathBuf,
    },
    /// Migrate a legacy YAML project (meta.yaml + vlabels/*.yaml + shortcuts.yaml) to JSON5
    MigrateYaml {
        /// VLabel project directory
        project: PathBuf,
        /// Also migrate the global shortcuts config (~/.config/vlabel/shortcuts.yaml)
        #[arg(long)]
        global: bool,
    },
    /// Verify YOLO round-trip fidelity: import the source into a throwaway
    /// project, re-export it (no masking), and reconcile every box against
    /// the source. Exits non-zero on any mismatch.
    VerifyRoundtrip {
        /// YOLO source root (images/ + labels/; classes.txt optional)
        src: PathBuf,
        /// Boxes match when classes are equal and IoU >= 1 - tolerance.
        ///     Default 0.01 (IoU >= 0.99) absorbs 6-decimal quantization
        ///     noise on tiny boxes; pass 0.001 for strict IoU >= 0.999.
        #[arg(long, default_value_t = vlabel_utils::roundtrip::DEFAULT_IOU_TOLERANCE)]
        iou_tolerance: f32,
        /// Write one JSON line per difference to this file
        #[arg(long)]
        report: Option<PathBuf>,
    },
}

#[derive(ValueEnum, Clone, Copy, PartialEq, Eq)]
enum Format {
    Yolo,
    Voc,
    Coco,
    LabelMe,
    ImageFolder,
}

fn main() {
    // Skipped-row warnings from import must be visible (spec §5: no silent
    // degradation), so default to `warn` even without RUST_LOG.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Import { format, src, images, roi, project } => {
            run_import(format, src, images, roi, project)
        }
        Command::Export { format, project, out, no_mask, symlink, property, crop } => {
            run_export(format, no_mask, symlink, property, crop, project, out)
        }
        Command::MigrateYaml { project, global } => {
            let global_path = if global {
                Some(
                    migrate::global_shortcuts_yaml_path()
                        .context("could not resolve global config directory")?,
                )
            } else {
                None
            };
            let report = migrate::migrate_project(&project, global_path.as_deref())
                .with_context(|| format!("migration failed in {}", project.display()))?;
            println!(
                "migrated: meta={}, labels={}, shortcuts={}",
                report.meta, report.labels, report.shortcuts
            );
            Ok(())
        }
        Command::VerifyRoundtrip { src, iou_tolerance, report } => {
            run_verify_roundtrip(src, iou_tolerance, report)
        }
    }
}

fn run_verify_roundtrip(
    src: PathBuf,
    iou_tolerance: f32,
    report_path: Option<PathBuf>,
) -> Result<()> {
    let report =
        vlabel_utils::roundtrip::verify_yolo_roundtrip(&src, iou_tolerance, report_path.as_deref())
            .with_context(|| {
                format!("round-trip verification failed to run in {}", src.display())
            })?;

    println!(
        "round-trip: {} frames | src {} boxes / out {} | matched {} missing {} extra {} | max coord delta {:.6}",
        report.frames,
        report.boxes_src,
        report.boxes_out,
        report.matched,
        report.missing,
        report.extra,
        report.max_delta
    );
    if report.is_pass() {
        println!("round-trip PASS");
        Ok(())
    } else {
        let where_line = match &report_path {
            Some(p) => format!("; differences written to {}", p.display()),
            None => "; re-run with --report <file.jsonl> for the difference list".to_string(),
        };
        anyhow::bail!(
            "round-trip FAIL: {} missing, {} extra box(es), {} frame(s) lost{where_line}",
            report.missing,
            report.extra,
            report.frame_missing
        )
    }
}

fn run_import(
    format: Format,
    src: PathBuf,
    images: Option<PathBuf>,
    roi: Option<PathBuf>,
    project_dir: PathBuf,
) -> Result<()> {
    let project = Project::open(&project_dir).context(
        "failed to open project (meta.json5 required; legacy YAML projects: run \
         `vlabel-convert migrate-yaml <project_dir>`)",
    )?;
    let meta = project.meta.clone();
    let existing_names = project
        .list_images()?
        .iter()
        .filter_map(|p| p.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()))
        .collect::<HashSet<String>>();

    let mut imported: Vec<ImportedImage> = match format {
        Format::Yolo => import_from_yolo(&src, &meta)?,
        Format::Voc => import_from_voc(&src, &meta)?,
        Format::Coco => {
            let images = images.context("--images <dir> is required for --format coco")?;
            import_from_coco(&src, &images, &meta)?
        }
        Format::LabelMe => import_from_labelme(&src, &meta)?,
        // Classification sets have no box annotations to import back; the
        // detection formats keep that direction.
        Format::ImageFolder => {
            anyhow::bail!("--format image-folder is export-only: classification sets are produced by export and cannot be imported")
        }
    };

    if let Some(roi_path) = &roi {
        let rules = roi_inject::RoiRules::parse_file(roi_path)?;
        let mut matched = 0usize;
        for item in &mut imported {
            if rules.apply_to_label(&mut item.annotation, &item.file_name)? {
                matched += 1;
            }
        }
        let unmatched = imported.len() - matched;
        println!("roi injection: {matched} frame(s) matched, {unmatched} without a rule");
        if unmatched > 0 {
            // Visible, not silent: frames without a rule keep no ROI, which on
            // export means no letterbox masking for them.
            log::warn!(
                "{unmatched} imported frame(s) matched no ROI rule in {}",
                roi_path.display()
            );
        }
    }

    let total_boxes = imported.iter().map(|item| item.annotation.objects.len()).sum::<usize>();
    let count = imported.len();

    merge_imported_images(imported, &project, &existing_names, "duplicate image name: {name}")?;
    println!("imported {count} labels, {total_boxes} boxes");
    Ok(())
}

fn run_export(
    format: Format,
    no_mask: bool,
    symlink: bool,
    property: Option<String>,
    crop: Option<f32>,
    project_dir: PathBuf,
    out_dir: PathBuf,
) -> Result<()> {
    // Both image options are YOLO-driver features; a silent no-op for the
    // verbatim-copy formats would hide the mistake.
    if format != Format::Yolo && (no_mask || symlink) {
        anyhow::bail!("--no-mask and --symlink only apply to --format yolo");
    }
    // The classification-set options are image-folder-driver features.
    if format != Format::ImageFolder && (property.is_some() || crop.is_some()) {
        anyhow::bail!("--property and --crop only apply to --format image-folder");
    }
    let folder_options = if format == Format::ImageFolder {
        let property =
            property.context("--property <name> is required for --format image-folder")?;
        let crop_margin = crop.unwrap_or(DEFAULT_CROP_MARGIN);
        if crop_margin < 0.0 {
            anyhow::bail!("--crop margin must be >= 0, got {crop_margin}");
        }
        Some(ImageFolderExportOptions { property, crop_margin })
    } else {
        None
    };
    let yolo_options = YoloExportOptions { mask_outside_rois: !no_mask, link_images: symlink };
    let project = Project::open(&project_dir).context(
        "failed to open project (meta.json5 required; legacy YAML projects: run \
         `vlabel-convert migrate-yaml <project_dir>`)",
    )?;
    // Fail fast before reading any image when the output dir would overwrite
    // the project itself (the per-format drivers re-check before writing).
    ensure_safe_export_dir(&project.root, &out_dir)?;
    let meta = project.meta.clone();
    let items = collect_export_items(&project)?;
    let total_boxes = items.iter().map(|item| item.annotation.objects.len()).sum::<usize>();

    if let Some(options) = &folder_options {
        let summary = export_dataset_image_folder(&project, &items, &meta, &out_dir, options)?;
        println!(
            "exported {} crop(s) -> {} ({} class dir(s), {} unlabeled, {} skipped)",
            summary.crops,
            out_dir.display(),
            summary.classes,
            summary.unlabeled,
            summary.skipped
        );
        return Ok(());
    }

    match format {
        Format::Yolo => export_dataset_yolo(&project, &items, &meta, &out_dir, &yolo_options)?,
        Format::Voc => export_dataset_voc(&project, &items, &meta, &out_dir)?,
        Format::Coco => export_dataset_coco(&project, &items, &meta, &out_dir)?,
        Format::LabelMe => export_dataset_labelme(&project, &items, &meta, &out_dir)?,
        Format::ImageFolder => unreachable!("handled above with folder_options"),
    }

    println!("exported {} labels, {total_boxes} boxes -> {}", items.len(), out_dir.display());
    Ok(())
}
