use lab_core::{Annotation, Meta, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Represents an annotation project with its directory structure
pub struct Project {
    /// Root directory of the project
    pub root: PathBuf,

    /// Project metadata
    pub meta: Meta,
}

impl Project {
    /// Open an existing project from a root directory
    pub fn open<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let meta_path = root.join("meta.yaml");

        let meta = lab_core::io::load_meta(&meta_path)?;

        Ok(Self { root, meta })
    }

    /// Get the path to the images directory
    pub fn images_dir(&self) -> PathBuf {
        self.root.join("images")
    }

    /// Get the path to the labels directory
    pub fn labels_dir(&self) -> PathBuf {
        self.root.join("labels")
    }

    /// Get the path to a specific annotation file
    pub fn annotation_path(&self, image_name: &str) -> PathBuf {
        let stem = Path::new(image_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(image_name);

        self.labels_dir().join(format!("{}.yaml", stem))
    }

    /// Load an annotation for a specific image
    pub fn load_annotation(&self, image_name: &str) -> Result<Option<Annotation>> {
        let path = self.annotation_path(image_name);

        if !path.exists() {
            return Ok(None);
        }

        let annotation = lab_core::io::load_annotation(&path)?;
        Ok(Some(annotation))
    }

    /// Save an annotation for a specific image
    pub fn save_annotation(&self, image_name: &str, annotation: &Annotation) -> Result<()> {
        let path = self.annotation_path(image_name);

        // Ensure the labels directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        lab_core::io::save_annotation(&path, annotation)?;
        Ok(())
    }

    /// List all image files in the images directory
    pub fn list_images(&self) -> Result<Vec<PathBuf>> {
        let images_dir = self.images_dir();

        if !images_dir.exists() {
            return Ok(Vec::new());
        }

        let mut images = Vec::new();

        for entry in fs::read_dir(&images_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    if ext == "jpg" || ext == "jpeg" || ext == "png" {
                        images.push(path);
                    }
                }
            }
        }

        images.sort();
        Ok(images)
    }

    /// Check if an image has been annotated
    pub fn is_annotated(&self, image_name: &str) -> bool {
        self.annotation_path(image_name).exists()
    }

    /// Get annotation progress statistics
    pub fn get_progress(&self) -> Result<ProgressStats> {
        let images = self.list_images()?;
        let total = images.len();
        let mut annotated = 0;

        for image in &images {
            if let Some(name) = image.file_name().and_then(|s| s.to_str()) {
                if self.is_annotated(name) {
                    annotated += 1;
                }
            }
        }

        Ok(ProgressStats {
            total,
            annotated,
            remaining: total - annotated,
        })
    }
}

/// Annotation progress statistics
#[derive(Debug, Clone)]
pub struct ProgressStats {
    pub total: usize,
    pub annotated: usize,
    pub remaining: usize,
}

impl ProgressStats {
    pub fn percentage(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.annotated as f32 / self.total as f32) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_project_path() -> PathBuf {
        // Try to find label_root from the workspace root
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop(); // Go up to workspace root
        path.push("label_root");
        path
    }

    #[test]
    fn test_open_project() {
        let project_path = get_test_project_path();
        if !project_path.exists() {
            println!("Skipping test: label_root not found at {:?}", project_path);
            return;
        }

        let project = Project::open(&project_path);
        assert!(project.is_ok());

        let project = project.unwrap();
        assert_eq!(project.meta.id, 201);
        assert_eq!(project.meta.name, "FDOD");
    }

    #[test]
    fn test_list_images() {
        let project_path = get_test_project_path();
        if !project_path.exists() {
            println!("Skipping test: label_root not found");
            return;
        }

        let project = Project::open(&project_path).unwrap();
        let images = project.list_images().unwrap();
        println!("Found {} images", images.len());
    }

    #[test]
    fn test_load_annotation() {
        let project_path = get_test_project_path();
        if !project_path.exists() {
            println!("Skipping test: label_root not found");
            return;
        }

        let project = Project::open(&project_path).unwrap();
        let annotation = project.load_annotation("0001.jpg");

        if let Ok(Some(ann)) = annotation {
            assert_eq!(ann.version, "2.0");
            assert!(!ann.objects.is_empty());
            assert!(!ann.rois.is_empty());
        }
    }
}
