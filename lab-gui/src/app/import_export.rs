use super::LabApp;
use anyhow::Context;
use lab_utils::conversion::{
    collect_export_items, ensure_safe_export_dir, export_dataset_coco, export_dataset_labelme,
    export_dataset_voc, export_dataset_yolo,
};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy)]
pub(super) enum DatasetFormat {
    Yolo,
    Voc,
    Coco,
    LabelMe,
}

impl LabApp {
    pub(super) fn import_dataset(&mut self, format: DatasetFormat) {
        let result = self.try_import_dataset(format);
        if let Err(err) = result {
            self.show_io_error(self.state.i18n.t("error.import_failed"), err.to_string());
        }
    }

    pub(super) fn export_dataset(&mut self, format: DatasetFormat) {
        let result = self.try_export_dataset(format);
        if let Err(err) = result {
            self.show_io_error(self.state.i18n.t("error.export_failed"), err.to_string());
        }
    }
}

impl LabApp {
    fn show_io_error(&self, title: String, message: String) {
        log::error!("{}: {}", title, message);
        let _ = rfd::MessageDialog::new()
            .set_title(&title)
            .set_description(&message)
            .set_buttons(rfd::MessageButtons::Ok)
            .set_level(rfd::MessageLevel::Error)
            .show();
    }

    fn try_import_dataset(&mut self, format: DatasetFormat) -> anyhow::Result<()> {
        let project = self.state.project.as_ref().context(self.state.i18n.t("error.no_project"))?;
        let meta = project.meta.clone();
        let existing_names = self
            .state
            .images
            .iter()
            .filter_map(|p| p.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()))
            .collect::<HashSet<String>>();
        let duplicate_template = self.state.i18n.t("error.import_duplicate_image");

        let imported = match format {
            DatasetFormat::Yolo => {
                let Some(root) =
                    rfd::FileDialog::new().set_title("Select YOLO dataset root").pick_folder()
                else {
                    return Ok(());
                };
                lab_utils::import::import_from_yolo(&root, &meta)?
            }
            DatasetFormat::Voc => {
                let Some(root) =
                    rfd::FileDialog::new().set_title("Select VOC dataset root").pick_folder()
                else {
                    return Ok(());
                };
                lab_utils::import::import_from_voc(&root, &meta)?
            }
            DatasetFormat::Coco => {
                let Some(json_path) = rfd::FileDialog::new()
                    .add_filter("COCO", &["json"])
                    .set_title("Select COCO annotation JSON")
                    .pick_file()
                else {
                    return Ok(());
                };
                let Some(images_dir) =
                    rfd::FileDialog::new().set_title("Select COCO images folder").pick_folder()
                else {
                    return Ok(());
                };
                lab_utils::import::import_from_coco(&json_path, &images_dir, &meta)?
            }
            DatasetFormat::LabelMe => {
                let Some(root) =
                    rfd::FileDialog::new().set_title("Select LabelMe folder").pick_folder()
                else {
                    return Ok(());
                };
                lab_utils::import::import_from_labelme(&root, &meta)?
            }
        };

        lab_utils::import::merge_imported_images(
            imported,
            project,
            &existing_names,
            &duplicate_template,
        )?;
        self.refresh_project_images()?;
        Ok(())
    }

    fn try_export_dataset(&mut self, format: DatasetFormat) -> anyhow::Result<()> {
        let project = self.state.project.as_ref().context(self.state.i18n.t("error.no_project"))?;

        let Some(output_root) =
            rfd::FileDialog::new().set_title("Select export folder").pick_folder()
        else {
            return Ok(());
        };

        // Fail fast before reading any image when the output dir would
        // overwrite the project itself (the drivers re-check before writing).
        ensure_safe_export_dir(&project.root, &output_root)?;

        let meta = project.meta.clone();
        let export_items = collect_export_items(project)?;

        match format {
            DatasetFormat::Yolo => {
                export_dataset_yolo(project, &export_items, &meta, &output_root)?
            }
            DatasetFormat::Voc => export_dataset_voc(project, &export_items, &meta, &output_root)?,
            DatasetFormat::Coco => {
                export_dataset_coco(project, &export_items, &meta, &output_root)?
            }
            DatasetFormat::LabelMe => {
                export_dataset_labelme(project, &export_items, &meta, &output_root)?
            }
        }

        Ok(())
    }
}

impl LabApp {
    fn refresh_project_images(&mut self) -> anyhow::Result<()> {
        if let Some(project) = &self.state.project {
            let current_path = self.state.current_image.as_ref().map(|img| img.path.clone());
            self.state.images = project.list_images()?;
            if let Some(current_path) = current_path {
                if let Some(idx) = self.state.images.iter().position(|p| p == &current_path) {
                    self.state.current_image_index = idx;
                }
            }
        }
        Ok(())
    }
}
