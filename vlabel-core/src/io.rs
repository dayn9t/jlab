use crate::{Error, Label, LabelMeta, Result};
use std::fs;
use std::path::Path;

/// Load metadata from a JSON5 file
pub fn load_meta<P: AsRef<Path>>(path: P) -> Result<LabelMeta> {
    let content = fs::read_to_string(path)?;
    let meta: LabelMeta = json5::from_str(&content).map_err(|err| {
        // Pre-bilingual meta.json5 (`name` per entity) fails with a bare
        // "missing field `names`"; point at the one-shot migration instead
        // of leaving the user to guess.
        if err.to_string().contains("missing field `names`") {
            Error::InvalidData(format!(
                "{err} — meta.json5 predates bilingual names; run \
                 `vlabel-convert localize-names <project_dir>` to migrate"
            ))
        } else {
            Error::Json5(err)
        }
    })?;
    Ok(meta)
}

/// Save metadata to a JSON5 file
pub fn save_meta<P: AsRef<Path>>(path: P, meta: &LabelMeta) -> Result<()> {
    let json5 = json5::to_string(meta)?;
    fs::write(path, json5)?;
    Ok(())
}

/// Load annotation from a JSON5 file
pub fn load_annotation<P: AsRef<Path>>(path: P) -> Result<Label> {
    let content = fs::read_to_string(path)?;
    let annotation: Label = json5::from_str(&content)?;
    Ok(annotation)
}

/// Save annotation to a JSON5 file
pub fn save_annotation<P: AsRef<Path>>(path: P, annotation: &Label) -> Result<()> {
    let json5 = json5::to_string(annotation)?;
    fs::write(path, json5)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::{add_object, new_label, new_object};
    use crate::{Point, Polygon};
    use std::fs;

    #[test]
    fn load_meta_pre_bilingual_error_names_the_migration() {
        let dir = std::env::temp_dir().join("vlabel-io-oldmeta-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("meta.json5"),
            r##"{ id: 1, name: "p", description: "", shape: { title_style: 0, thickness: 2 },
                 roi: { color: "#0000FF" },
                 categories: [ { id: 0, name: "trash", description: "", hotkey: "1", color: "#FF0000" } ] }"##,
        )
        .unwrap();

        let err = load_meta(dir.join("meta.json5")).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("missing field `names`"), "message was: {msg}");
        assert!(msg.contains("localize-names"), "must name the fix: {msg}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_and_load_annotation() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_annotation.json5");

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

    #[test]
    fn test_load_annotation_with_hand_edits() {
        // 手改文件：注释 + 尾逗号 + 无引号 key 必须可解析（JSON5 宽容读）
        let content = "{
  // hand-edited comment
  version: '2.0',
  user_agent: 'test-tool',
  created_at: '2025-08-22T13:51:35.005806093Z',
  last_modified: '2025-08-22T13:51:35.005806093Z',
  rois: [],
  objects: [
    {
      id: 1,
      category: 0,
      confidence: 1.0,
      polygon: [{ x: 0.1, y: 0.1 }, { x: 0.5, y: 0.5 }],
      properties: [],
    },
  ],
}";
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_annotation_hand_edit.json5");
        fs::write(&test_file, content).unwrap();

        let loaded = load_annotation(&test_file).unwrap();

        assert_eq!(loaded.version, "2.0");
        assert_eq!(loaded.objects.len(), 1);
        assert_eq!(loaded.objects[0].polygon.0.len(), 2);

        let _ = fs::remove_file(test_file);
    }
}
