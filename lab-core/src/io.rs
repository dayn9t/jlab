use crate::{Label, LabelMeta, Result};
use std::fs;
use std::path::Path;

/// Load metadata from a YAML file
pub fn load_meta<P: AsRef<Path>>(path: P) -> Result<LabelMeta> {
    let content = fs::read_to_string(path)?;
    let meta: LabelMeta = serde_yaml::from_str(&content)?;
    Ok(meta)
}

/// Save metadata to a YAML file
pub fn save_meta<P: AsRef<Path>>(path: P, meta: &LabelMeta) -> Result<()> {
    let yaml = serde_yaml::to_string(meta)?;
    fs::write(path, yaml)?;
    Ok(())
}

/// Load annotation from a YAML file
pub fn load_annotation<P: AsRef<Path>>(path: P) -> Result<Label> {
    let content = fs::read_to_string(path)?;
    let annotation: Label = serde_yaml::from_str(&content)?;
    Ok(annotation)
}

/// Save annotation to a YAML file
pub fn save_annotation<P: AsRef<Path>>(path: P, annotation: &Label) -> Result<()> {
    let yaml = serde_yaml::to_string(annotation)?;
    fs::write(path, yaml)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::{add_object, new_label, new_object};
    use crate::{Point, Polygon};
    use std::fs;

    #[test]
    fn test_save_and_load_annotation() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_annotation.yaml");

        let mut label = new_label("test-tool");
        let obj = new_object(
            0,
            1,
            Polygon::from(vec![
                Point { x: 0.1, y: 0.1 },
                Point { x: 0.5, y: 0.1 },
                Point { x: 0.5, y: 0.5 },
                Point { x: 0.1, y: 0.5 },
            ]),
        );
        add_object(&mut label, obj);

        // Save
        save_annotation(&test_file, &label).unwrap();

        // Load
        let loaded = load_annotation(&test_file).unwrap();

        assert_eq!(loaded.version, label.version);
        assert_eq!(loaded.objects.len(), label.objects.len());

        // Cleanup
        let _ = fs::remove_file(test_file);
    }
}
