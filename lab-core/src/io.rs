use crate::{Annotation, Meta, Result};
use std::fs;
use std::path::Path;

/// Load metadata from a YAML file
pub fn load_meta<P: AsRef<Path>>(path: P) -> Result<Meta> {
    let content = fs::read_to_string(path)?;
    let meta: Meta = serde_yaml::from_str(&content)?;
    Ok(meta)
}

/// Save metadata to a YAML file
pub fn save_meta<P: AsRef<Path>>(path: P, meta: &Meta) -> Result<()> {
    let yaml = serde_yaml::to_string(meta)?;
    fs::write(path, yaml)?;
    Ok(())
}

/// Load annotation from a YAML file
pub fn load_annotation<P: AsRef<Path>>(path: P) -> Result<Annotation> {
    let content = fs::read_to_string(path)?;
    let annotation: Annotation = serde_yaml::from_str(&content)?;
    Ok(annotation)
}

/// Save annotation to a YAML file
pub fn save_annotation<P: AsRef<Path>>(path: P, annotation: &Annotation) -> Result<()> {
    let yaml = serde_yaml::to_string(annotation)?;
    fs::write(path, yaml)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::Object;
    use crate::geometry::Point;
    use std::fs;

    #[test]
    fn test_save_and_load_annotation() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_annotation.yaml");

        let mut annotation = Annotation::new("test-tool");
        let obj = Object::new(
            0,
            1,
            vec![
                Point::new(0.1, 0.1),
                Point::new(0.5, 0.1),
                Point::new(0.5, 0.5),
                Point::new(0.1, 0.5),
            ],
        );
        annotation.add_object(obj);

        // Save
        save_annotation(&test_file, &annotation).unwrap();

        // Load
        let loaded = load_annotation(&test_file).unwrap();

        assert_eq!(loaded.version, annotation.version);
        assert_eq!(loaded.objects.len(), annotation.objects.len());

        // Cleanup
        let _ = fs::remove_file(test_file);
    }
}
