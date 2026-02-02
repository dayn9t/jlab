mod app;
mod canvas;
mod geometry;
mod i18n;
mod shortcuts;
mod state;
mod tools;

use app::LabApp;
use clap::Parser;
use std::path::PathBuf;

/// JLab - Image Annotation Tool
#[derive(Parser, Debug)]
#[command(name = "lab-gui")]
#[command(author = "JLab Contributors")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A powerful image annotation tool for computer vision tasks", long_about = None)]
struct Args {
    /// Project directory to open on startup
    #[arg(value_name = "PROJECT_DIR")]
    project_dir: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<(), eframe::Error> {
    let args = Args::parse();

    // Initialize logger with appropriate level
    let log_level = if args.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    log::info!("Starting JLab v{}", env!("CARGO_PKG_VERSION"));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("JLab - Image Annotation Tool"),
        ..Default::default()
    };

    eframe::run_native(
        "JLab",
        options,
        Box::new(move |cc| {
            let mut app = LabApp::new(cc);

            // Auto-open project if specified
            if let Some(project_dir) = args.project_dir {
                log::info!("Auto-opening project: {:?}", project_dir);
                if let Err(e) = app.load_project_from_path(project_dir) {
                    log::error!("Failed to auto-open project: {}", e);
                }
            }

            Ok(Box::new(app))
        }),
    )
}
