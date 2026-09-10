//! meta.json5 one-shot migration: per-entity `name` -> `names: {en}`.
//!
//! Bilingual metadata names (2026-09): `CatDef`/`PropDef`/`ValueDef`/
//! `CatProperty` replaced `name: String` with `names: LocalizedNames`, so
//! existing meta.json5 files no longer parse as `LabelMeta`. Per the
//! one-shot-migration ruling (2026-08-31: data products migrate via
//! one-shot scripts, no old-format compatibility layer), this driver
//! rewrites the file as dynamic JSON without going through `LabelMeta`.
//!
//! Top-level `meta.name` (the project name, not an enumeration entity)
//! is left untouched. Output is written through `save_meta`, so the
//! file always comes back in struct-declaration key order, pretty
//! JSON5 — identical to what the GUI writes. Comments in the
//! hand-edited file are lost on rewrite — same as any GUI `save_meta`.

use anyhow::{bail, Context};
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

/// Outcome of a `localize_names` run.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct LocalizeStats {
    /// Entities rewritten (`name` -> `names`).
    pub migrated: usize,
    /// Entities already in the new format (left untouched).
    pub already: usize,
}

/// Entity arrays of `LabelMeta` that carry per-entity names, plus the
/// nested arrays inside `property_types` entries.
const ENTITY_ARRAYS: [&str; 3] = ["categories", "property_types", "property_special_values"];

/// Migrate `<project_dir>/meta.json5` to the bilingual-names format.
pub fn localize_names(project_dir: &Path) -> anyhow::Result<LocalizeStats> {
    let meta_path = project_dir.join("meta.json5");
    let content = fs::read_to_string(&meta_path)
        .with_context(|| format!("failed to read {:?}", meta_path))?;
    let mut root: Value =
        json5::from_str(&content).with_context(|| format!("failed to parse {:?}", meta_path))?;
    let Some(obj) = root.as_object_mut() else {
        bail!("{:?} is not a JSON object", meta_path);
    };

    let mut stats = LocalizeStats::default();
    for key in ENTITY_ARRAYS {
        if let Some(Value::Array(entities)) = obj.get_mut(key) {
            for entity in entities.iter_mut() {
                let Some(entity) = entity.as_object_mut() else {
                    bail!("{key} contains a non-object entry");
                };
                localize_entity(entity, &mut stats)?;
                // Nested entity arrays: categories carry property
                // references (CatProperty), property types carry values.
                let nested_key = match key {
                    "categories" => Some("properties"),
                    "property_types" => Some("values"),
                    _ => None,
                };
                if let Some(nested_key) = nested_key {
                    if let Some(Value::Array(children)) = entity.get_mut(nested_key) {
                        for child in children.iter_mut() {
                            let Some(child) = child.as_object_mut() else {
                                bail!("{key}.{nested_key} contains a non-object entry");
                            };
                            localize_entity(child, &mut stats)?;
                        }
                    }
                }
            }
        }
    }

    // Typed round-trip: validate the migrated value parses as LabelMeta
    // *before* anything is written, then write via `save_meta` so the file
    // comes out in struct-declaration key order with the same pretty-JSON5
    // formatting the GUI produces (going through serde_json::Value directly
    // would alphabetize every key - BTreeMap - which is how this tool used
    // to scramble hand-written metas).
    let meta: vlabel_core::LabelMeta = serde_json::from_value(root)
        .with_context(|| format!("{:?} does not parse as LabelMeta after migration", meta_path))?;
    vlabel_core::io::save_meta(&meta_path, &meta)?;
    Ok(stats)
}

/// Rewrite one entity in place: `name` (string) -> `names: {en: name}`.
/// Already-migrated entities are counted and skipped; carrying both keys
/// or neither is bad data and fails (strict schema, no lenient parsing).
fn localize_entity(
    entity: &mut Map<String, Value>,
    stats: &mut LocalizeStats,
) -> anyhow::Result<()> {
    match (entity.remove("name"), entity.get("names").is_some()) {
        (Some(_), true) => {
            bail!("entity carries both `name` and `names` — resolve the conflict manually")
        }
        (None, true) => stats.already += 1,
        (None, false) => bail!("entity is missing both `name` and `names`"),
        (Some(Value::String(name)), false) => {
            entity.insert(
                "names".to_string(),
                Value::Object(Map::from_iter([("en".to_string(), Value::String(name))])),
            );
            stats.migrated += 1;
        }
        (Some(other), false) => bail!("entity `name` is not a string: {:?}", other),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vlabel_core::io::load_meta;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vlabel-meta-migrate-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Pre-bilingual meta.json5 covering every entity kind, with a
    /// top-level project name that must survive untouched.
    const OLD_META: &str = r##"
    {
        id: 1,
        name: "shtm-detect",
        description: "project",
        shape: { title_style: 0, thickness: 2 },
        roi: { color: "#0000FF" },
        categories: [
            {
                id: 0, name: "trash", description: "", hotkey: "1", color: "#FF0000",
                properties: [ { id: 3, name: "wet", type: "single_select" } ]
            }
        ],
        property_types: [
            {
                id: 3, name: "wet", description: "",
                values: [ { id: 0, name: "dry", description: "", hotkey: "q", color: "#00FF00", sign: "D" } ]
            }
        ],
        property_special_values: [
            { id: -1, name: "occluded", description: "", hotkey: "x", color: "#888888", sign: "O" }
        ]
    }
    "##;

    #[test]
    fn migrates_every_entity_and_parses_as_new_label_meta() {
        let dir = temp_dir("full");
        fs::write(dir.join("meta.json5"), OLD_META).unwrap();

        let stats = localize_names(&dir).unwrap();

        // 1 category + 1 cat-property + 1 property type + 1 value + 1 special
        assert_eq!(stats, LocalizeStats { migrated: 5, already: 0 });
        let meta = load_meta(dir.join("meta.json5")).unwrap();
        assert_eq!(meta.name, "shtm-detect", "project name must survive");
        assert_eq!(meta.categories[0].names.en, "trash");
        assert_eq!(meta.categories[0].properties[0].names.en, "wet");
        assert_eq!(meta.property_types[0].names.en, "wet");
        assert_eq!(meta.property_types[0].values[0].names.en, "dry");
        assert_eq!(meta.property_special_values[0].names.en, "occluded");
        assert_eq!(meta.categories[0].names.zh, None);

        // Output must be in struct-declaration order (not alphabetical):
        // top level id..categories..property_types, entity id..names..description.
        let out = fs::read_to_string(dir.join("meta.json5")).unwrap();
        let top = [
            "id:",
            "name:",
            "description:",
            "shape:",
            "roi:",
            "categories:",
            "property_types:",
            "property_special_values:",
        ];
        let mut last = 0usize;
        for key in top {
            let at = out.find(key).unwrap_or_else(|| panic!("{key} missing in output:\n{out}"));
            assert!(at > last, "{key} out of declaration order in:\n{out}");
            last = at;
        }
        let entity = ["id:", "names:", "description:", "hotkey:", "color:"];
        let cat_at = out.find("categories:").unwrap();
        let scope = &out[cat_at..];
        let mut last = 0usize;
        for key in entity {
            let at = scope
                .find(key)
                .unwrap_or_else(|| panic!("{key} missing in first category:\n{scope}"));
            assert!(at > last, "{key} out of declaration order in first category:\n{scope}");
            last = at;
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn second_run_is_idempotent() {
        let dir = temp_dir("idem");
        fs::write(dir.join("meta.json5"), OLD_META).unwrap();
        localize_names(&dir).unwrap();

        let stats = localize_names(&dir).unwrap();

        assert_eq!(stats, LocalizeStats { migrated: 0, already: 5 });
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn conflicting_name_and_names_fails_without_touching_file() {
        let dir = temp_dir("conflict");
        fs::write(
            dir.join("meta.json5"),
            r##"{ categories: [ { id: 0, name: "a", names: {en: "a"}, description: "", hotkey: "", color: "" } ] }"##,
        )
        .unwrap();
        let before = fs::read_to_string(dir.join("meta.json5")).unwrap();

        let err = localize_names(&dir).unwrap_err();

        assert!(format!("{err:#}").contains("both"), "message was: {err:#}");
        assert_eq!(fs::read_to_string(dir.join("meta.json5")).unwrap(), before);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn entity_without_any_name_fails() {
        let dir = temp_dir("noname");
        fs::write(dir.join("meta.json5"), r##"{ categories: [ { id: 0, description: "" } ] }"##)
            .unwrap();

        let err = localize_names(&dir).unwrap_err();

        assert!(format!("{err:#}").contains("missing"), "message was: {err:#}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_meta_fails() {
        let dir = temp_dir("nometa");
        let err = localize_names(&dir).unwrap_err();
        assert!(format!("{err:#}").contains("failed to read"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn preserves_existing_zh_names() {
        let dir = temp_dir("keepzh");
        fs::write(
            dir.join("meta.json5"),
            r##"{
                id: 1, name: "p", description: "",
                shape: { title_style: 0, thickness: 2 },
                roi: { color: "#0000FF" },
                categories: [ { id: 0, names: {en: "trash", zh: "垃圾桶"}, description: "", hotkey: "", color: "" } ],
                property_types: [], property_special_values: []
            }"##,
        )
        .unwrap();

        let stats = localize_names(&dir).unwrap();

        assert_eq!(stats, LocalizeStats { migrated: 0, already: 1 });
        let meta = load_meta(dir.join("meta.json5")).unwrap();
        assert_eq!(meta.categories[0].names.zh.as_deref(), Some("垃圾桶"));
        let _ = fs::remove_dir_all(&dir);
    }
}
