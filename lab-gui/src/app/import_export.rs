use super::LabApp;
use anyhow::Context;
use lab_core::Label;
use lab_utils::conversion::{export_annotation, export_coco_batch, ExportFormat};
use std::collections::HashSet;
use std::fs;

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
        let meta = project.meta.clone();

        let Some(output_root) =
            rfd::FileDialog::new().set_title("Select export folder").pick_folder()
        else {
            return Ok(());
        };

        let export_items = lab_utils::conversion::collect_export_items(project)?;

        match format {
            DatasetFormat::Yolo => {
                let images_dir = output_root.join("images");
                let labels_dir = output_root.join("labels");
                fs::create_dir_all(&images_dir)?;
                fs::create_dir_all(&labels_dir)?;

                for item in &export_items {
                    let label_path = labels_dir.join(format!("{}.txt", item.stem));
                    export_annotation(
                        &label_path,
                        &item.annotation,
                        &meta,
                        item.image_path.to_string_lossy().as_ref(),
                        item.width,
                        item.height,
                        ExportFormat::Yolo,
                    )?;
                    fs::copy(&item.image_path, images_dir.join(&item.file_name))?;
                }
            }
            DatasetFormat::Voc => {
                let images_dir = output_root.join("JPEGImages");
                let annotations_dir = output_root.join("Annotations");
                fs::create_dir_all(&images_dir)?;
                fs::create_dir_all(&annotations_dir)?;

                for item in &export_items {
                    let label_path = annotations_dir.join(format!("{}.xml", item.stem));
                    export_annotation(
                        &label_path,
                        &item.annotation,
                        &meta,
                        item.image_path.to_string_lossy().as_ref(),
                        item.width,
                        item.height,
                        ExportFormat::Voc,
                    )?;
                    fs::copy(&item.image_path, images_dir.join(&item.file_name))?;
                }
            }
            DatasetFormat::Coco => {
                let images_dir = output_root.join("images");
                fs::create_dir_all(&images_dir)?;
                for item in &export_items {
                    fs::copy(&item.image_path, images_dir.join(&item.file_name))?;
                }

                let coco_path = output_root.join("annotations.json");
                let coco_items: Vec<(String, Label, u32, u32)> = export_items
                    .iter()
                    .map(|item| {
                        (item.file_name.clone(), item.annotation.clone(), item.width, item.height)
                    })
                    .collect();
                export_coco_batch(&coco_path, &coco_items, &meta)?;
            }
            DatasetFormat::LabelMe => {
                fs::create_dir_all(&output_root)?;
                for item in &export_items {
                    fs::copy(&item.image_path, output_root.join(&item.file_name))?;
                    let label_path = output_root.join(format!("{}.json", item.stem));
                    lab_utils::conversion::export_labelme_annotation(&label_path, item, &meta)?;
                }
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
