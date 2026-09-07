use std::fs;
use std::path::{Path, PathBuf};
use vlabel_core::{Error, Label, LabelMeta, Result};

/// Extract the file stem used to key annotation files (labels are keyed by stem,
/// so a.jpg and a.png share the same labels/a.yaml).
fn file_stem(file_name: &str) -> &str {
    Path::new(file_name).file_stem().and_then(|s| s.to_str()).unwrap_or(file_name)
}

/// Represents an annotation project with its directory structure
pub struct Project {
    /// Root directory of the project
    pub root: PathBuf,

    /// Project metadata
    pub meta: LabelMeta,
}

impl Project {
    /// Open an existing project from a root directory
    pub fn open<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let meta_path = root.join("meta.yaml");

        let meta = vlabel_core::io::load_meta(&meta_path)?;

        Ok(Self { root, meta })
    }

    /// Get the path to the images directory
    pub fn images_dir(&self) -> PathBuf {
        self.root.join("images")
    }

    /// Get the path to the labels directory
    pub fn vlabels_dir(&self) -> PathBuf {
        self.root.join("vlabels")
    }

    /// Get the path to a specific annotation file
    pub fn annotation_path(&self, image_name: &str) -> PathBuf {
        self.vlabels_dir().join(format!("{}.yaml", file_stem(image_name)))
    }

    /// Load an annotation for a specific image
    pub fn load_annotation(&self, image_name: &str) -> Result<Option<Label>> {
        let path = self.annotation_path(image_name);

        if !path.exists() {
            return Ok(None);
        }

        let annotation = vlabel_core::io::load_annotation(&path)?;
        Ok(Some(annotation))
    }

    /// Save an annotation for a specific image
    pub fn save_annotation(&self, image_name: &str, annotation: &Label) -> Result<()> {
        let path = self.annotation_path(image_name);

        // Ensure the labels directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        vlabel_core::io::save_annotation(&path, annotation)?;
        Ok(())
    }

    /// Delete an image file and its annotation file. The image must exist;
    /// the annotation file is deleted only if present. Fails without deleting
    /// anything when another image with the same stem (e.g. a.png for a.jpg)
    /// still shares the annotation slot.
    pub fn delete_sample(&self, image_path: &Path) -> Result<()> {
        if let Some(name) = image_path.file_name().and_then(|s| s.to_str()) {
            if let Some(sibling) = self.find_stem_sibling(name)? {
                return Err(Error::InvalidData(format!(
                    "cannot delete {}: sibling image {} shares the annotation {}; delete or rename one of them first",
                    name,
                    sibling.display(),
                    self.annotation_path(name).display()
                )));
            }
        }

        fs::remove_file(image_path)?;

        if let Some(name) = image_path.file_name().and_then(|s| s.to_str()) {
            let annotation = self.annotation_path(name);
            if annotation.exists() {
                fs::remove_file(&annotation)?;
            }
        }
        Ok(())
    }

    /// Find another image in images_dir with the same stem but a different
    /// file name — it maps to the same annotation file.
    fn find_stem_sibling(&self, image_name: &str) -> Result<Option<PathBuf>> {
        let stem = file_stem(image_name);
        for path in self.list_images()? {
            if let Some(other) = path.file_name().and_then(|s| s.to_str()) {
                if other != image_name && file_stem(other) == stem {
                    return Ok(Some(path));
                }
            }
        }
        Ok(None)
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

        Ok(ProgressStats { total, annotated, remaining: total - annotated })
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
    use std::collections::HashSet;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vlabel-utils-project-test-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("images")).unwrap();
        fs::create_dir_all(dir.join("vlabels")).unwrap();
        dir
    }

    #[test]
    fn delete_sample_removes_image_and_annotation() {
        let root = temp_root("del1");
        fs::write(root.join("images/a.jpg"), b"fake").unwrap();
        let project = Project { root: root.clone(), meta: crate::import::tests::test_meta() };
        project.save_annotation("a.jpg", &vlabel_core::new_label("test")).unwrap();
        assert!(project.annotation_path("a.jpg").exists());

        project.delete_sample(&root.join("images/a.jpg")).unwrap();

        assert!(!root.join("images/a.jpg").exists());
        assert!(!project.annotation_path("a.jpg").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn delete_sample_without_annotation_still_removes_image() {
        let root = temp_root("del2");
        fs::write(root.join("images/b.png"), b"fake").unwrap();
        let project = Project { root: root.clone(), meta: crate::import::tests::test_meta() };

        project.delete_sample(&root.join("images/b.png")).unwrap();

        assert!(!root.join("images/b.png").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn delete_sample_with_same_stem_sibling_errors_and_keeps_files() {
        let root = temp_root("del4");
        fs::write(root.join("images/a.jpg"), b"fake").unwrap();
        fs::write(root.join("images/a.png"), b"fake").unwrap();
        let project = Project { root: root.clone(), meta: crate::import::tests::test_meta() };
        project.save_annotation("a.jpg", &vlabel_core::new_label("test")).unwrap();
        assert!(project.annotation_path("a.jpg").exists());

        let err = project.delete_sample(&root.join("images/a.jpg")).unwrap_err();

        assert!(err.to_string().contains("a.png"), "error should name the sibling: {err}");
        // Annotation slot is shared by a.png — nothing may be deleted.
        assert!(project.annotation_path("a.jpg").exists());
        assert!(root.join("images/a.jpg").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn delete_sample_missing_image_errors() {
        let root = temp_root("del3");
        let project = Project { root, meta: crate::import::tests::test_meta() };
        assert!(project.delete_sample(Path::new("/nonexistent/x.jpg")).is_err());
    }

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

    // --- merge_imported_images (import.rs): Project-merge behavior ---
    // Lives here (not in import.rs tests) because merging is Project-centric;
    // import.rs sits at the file-length cap.

    fn make_project(tag: &str) -> (PathBuf, Project) {
        let root = temp_root(tag);
        let meta = crate::import::tests::test_meta();
        vlabel_core::io::save_meta(root.join("meta.yaml"), &meta).unwrap();
        let project = Project::open(&root).unwrap();
        (root, project)
    }

    fn image(path: &Path, name: &str) -> crate::import::ImportedImage {
        crate::import::ImportedImage {
            source_path: path.to_path_buf(),
            file_name: name.to_string(),
            annotation: vlabel_core::new_label("test"),
        }
    }

    #[test]
    fn merge_copies_image_and_writes_annotation() {
        let (root, project) = make_project("merge-ok");
        let src = root.join("src.jpg");
        fs::write(&src, b"fake").unwrap();
        let ann = vlabel_core::new_label("test");

        crate::import::merge_imported_images(
            vec![crate::import::ImportedImage {
                source_path: src.clone(),
                file_name: "src.jpg".to_string(),
                annotation: ann,
            }],
            &project,
            &Default::default(),
            "duplicate: {name}",
        )
        .unwrap();

        assert!(project.images_dir().join("src.jpg").exists());
        assert!(project.annotation_path("src.jpg").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn merge_rejects_duplicate_names() {
        let (root, project) = make_project("merge-dup");
        let src = root.join("x.jpg");
        fs::write(&src, b"fake").unwrap();

        let err = crate::import::merge_imported_images(
            vec![image(&src, "dup.jpg"), image(&src, "dup.jpg")],
            &project,
            &Default::default(),
            "duplicate: {name}",
        )
        .unwrap_err();
        assert!(err.to_string().contains("duplicate: dup.jpg"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn merge_rejects_same_stem_with_different_extension() {
        let (root, project) = make_project("merge-stem");
        let src = root.join("y.png");
        fs::write(&src, b"fake").unwrap();
        fs::write(project.images_dir().join("b.jpg"), b"fake").unwrap();
        project.save_annotation("b.jpg", &vlabel_core::new_label("existing")).unwrap();
        let existing: HashSet<String> = ["b.jpg".to_string()].into_iter().collect();

        let err = crate::import::merge_imported_images(
            vec![image(&src, "b.png")],
            &project,
            &existing,
            "duplicate: {name}",
        )
        .unwrap_err();

        assert!(err.to_string().contains("duplicate: b.png"));
        assert!(!project.images_dir().join("b.png").exists());
        // The existing b.jpg annotation must not be overwritten.
        let label = project.load_annotation("b.jpg").unwrap().unwrap();
        assert_eq!(label.user_agent, "existing");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn merge_rejects_incoming_same_stem_different_extension() {
        let (root, project) = make_project("merge-stem-in");
        let src = root.join("y.jpg");
        fs::write(&src, b"fake").unwrap();

        let err = crate::import::merge_imported_images(
            vec![image(&src, "b.png"), image(&src, "b.jpg")],
            &project,
            &Default::default(),
            "duplicate: {name}",
        )
        .unwrap_err();

        assert!(err.to_string().contains("duplicate:"));
        assert!(!project.images_dir().join("b.png").exists());
        assert!(!project.images_dir().join("b.jpg").exists());
        let _ = fs::remove_dir_all(&root);
    }
}
