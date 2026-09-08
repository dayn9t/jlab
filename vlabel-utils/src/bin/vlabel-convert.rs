//! Non-interactive annotation format converter for VLabel projects.
//!
//! Usage:
//!   vlabel-convert import --format <yolo|voc|coco|labelme> <src> <project_dir>
//!       (coco: <src> is the annotation json; `--images <dir>` is required)
//!   vlabel-convert export --format <yolo|voc|coco|labelme> <project_dir> <out_dir>
//!   vlabel-convert migrate-yaml [--global] <project_dir>

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashSet;
use std::path::PathBuf;
use vlabel_utils::conversion::{
    collect_export_items, ensure_safe_export_dir, export_dataset_coco, export_dataset_labelme,
    export_dataset_voc, export_dataset_yolo,
};
use vlabel_utils::import::{
    import_from_coco, import_from_labelme, import_from_voc, import_from_yolo,
    merge_imported_images, ImportedImage,
};
use vlabel_utils::migrate;
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
        /// Target VLabel project directory (must contain meta.json5)
        project: PathBuf,
    },
    /// Export a VLabel project to an external dataset
    Export {
        /// Dataset format
        #[arg(short, long)]
        format: Format,
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
}

#[derive(ValueEnum, Clone, Copy)]
enum Format {
    Yolo,
    Voc,
    Coco,
    LabelMe,
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
        Command::Import { format, src, images, project } => {
            run_import(format, src, images, project)
        }
        Command::Export { format, project, out } => run_export(format, project, out),
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
    }
}

fn run_import(
    format: Format,
    src: PathBuf,
    images: Option<PathBuf>,
    project_dir: PathBuf,
) -> Result<()> {
    let project =
        Project::open(&project_dir).context("failed to open project (meta.yaml required)")?;
    let meta = project.meta.clone();
    let existing_names = project
        .list_images()?
        .iter()
        .filter_map(|p| p.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()))
        .collect::<HashSet<String>>();

    let imported: Vec<ImportedImage> = match format {
        Format::Yolo => import_from_yolo(&src, &meta)?,
        Format::Voc => import_from_voc(&src, &meta)?,
        Format::Coco => {
            let images = images.context("--images <dir> is required for --format coco")?;
            import_from_coco(&src, &images, &meta)?
        }
        Format::LabelMe => import_from_labelme(&src, &meta)?,
    };

    let total_boxes = imported.iter().map(|item| item.annotation.objects.len()).sum::<usize>();
    let count = imported.len();

    merge_imported_images(imported, &project, &existing_names, "duplicate image name: {name}")?;
    println!("imported {count} labels, {total_boxes} boxes");
    Ok(())
}

fn run_export(format: Format, project_dir: PathBuf, out_dir: PathBuf) -> Result<()> {
    let project =
        Project::open(&project_dir).context("failed to open project (meta.yaml required)")?;
    // Fail fast before reading any image when the output dir would overwrite
    // the project itself (the per-format drivers re-check before writing).
    ensure_safe_export_dir(&project.root, &out_dir)?;
    let meta = project.meta.clone();
    let items = collect_export_items(&project)?;
    let total_boxes = items.iter().map(|item| item.annotation.objects.len()).sum::<usize>();

    match format {
        Format::Yolo => export_dataset_yolo(&project, &items, &meta, &out_dir)?,
        Format::Voc => export_dataset_voc(&project, &items, &meta, &out_dir)?,
        Format::Coco => export_dataset_coco(&project, &items, &meta, &out_dir)?,
        Format::LabelMe => export_dataset_labelme(&project, &items, &meta, &out_dir)?,
    }

    println!("exported {} labels, {total_boxes} boxes -> {}", items.len(), out_dir.display());
    Ok(())
}
