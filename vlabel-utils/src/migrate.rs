//! 旧版 YAML 标注项目 → JSON5 一次性迁移。
//!
//! 显式兼容桥：只在本模块（vlabel-convert migrate-yaml）读取 YAML，
//! 主读写路径（vlabel-core::io、GUI shortcuts）只认 JSON5。

use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// 迁移结果计数。
#[derive(Debug, Default, PartialEq)]
pub struct MigrateReport {
    pub meta: usize,
    pub labels: usize,
    pub shortcuts: usize,
}

/// 全局快捷键旧配置路径。
/// 镜像 vlabel-gui/src/shortcuts.rs 的 user_config_dir（含扩展名差异是有意的：
/// 这里指向待迁移的旧文件），两处路径逻辑需同步修改。
pub fn global_shortcuts_yaml_path() -> Option<PathBuf> {
    dirs::config_dir().map(|mut p| {
        p.push("vlabel");
        p.push("shortcuts.yaml");
        p
    })
}

/// 类型化迁移单文件：严格解析 → 写出 .json5 → 回读验证 → 删旧文件。
/// 目标 .json5 已存在即拒跑（不静默覆盖）。
fn migrate_one<T: Serialize + DeserializeOwned>(src: &Path) -> Result<()> {
    let dst = src.with_extension("json5");
    if dst.exists() {
        bail!("target already exists, refusing to overwrite: {}", dst.display());
    }
    let content =
        fs::read_to_string(src).with_context(|| format!("failed to read {}", src.display()))?;
    let value: T = serde_yaml::from_str(&content)
        .with_context(|| format!("failed to parse YAML {}", src.display()))?;
    fs::write(&dst, json5::to_string(&value)?)
        .with_context(|| format!("failed to write {}", dst.display()))?;
    let written = fs::read_to_string(&dst)
        .with_context(|| format!("failed to read back {}", dst.display()))?;
    let _verify: T = json5::from_str(&written)
        .with_context(|| format!("verification failed for {}", dst.display()))?;
    fs::remove_file(src).with_context(|| format!("failed to remove {}", src.display()))?;
    Ok(())
}

/// 非类型化迁移单文件（shortcuts 专用，类型在 vlabel-gui 不可达）。
/// serde_json::Value 实现 Serialize + DeserializeOwned，直接复用 migrate_one。
fn migrate_untyped(src: &Path) -> Result<()> {
    migrate_one::<serde_json::Value>(src)
}

/// 迁移项目目录下所有旧 YAML 文件；`global_shortcuts` 为 --global 传入的
/// 全局快捷键旧配置路径。无可迁移文件时报错（不静默成功）。
///
/// meta 在项目内文件中最后迁移：labels/shortcuts 任一失败时 meta.yaml 仍在、
/// meta.json5 不存在，Project::open 直接失败——半迁移项目无法被 GUI 打开编辑
/// （防止自动保存的 .json5 遮蔽残留 .yaml）；已成功文件的 .yaml 已删，
/// 修复故障源后重跑即可续迁，不触发「目标已存在」守卫。
pub fn migrate_project(root: &Path, global_shortcuts: Option<&Path>) -> Result<MigrateReport> {
    let mut report = MigrateReport::default();

    let vlabels_dir = root.join("vlabels");
    if vlabels_dir.is_dir() {
        let mut entries: Vec<PathBuf> = fs::read_dir(&vlabels_dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yaml"))
            .collect();
        entries.sort();
        for path in entries {
            migrate_one::<vlabel_core::Label>(&path)?;
            report.labels += 1;
        }
    }

    let project_shortcuts = root.join("shortcuts.yaml");
    if project_shortcuts.exists() {
        migrate_untyped(&project_shortcuts)?;
        report.shortcuts += 1;
    }

    let meta_path = root.join("meta.yaml");
    if meta_path.exists() {
        migrate_one::<vlabel_core::LabelMeta>(&meta_path)?;
        report.meta += 1;
    }

    if let Some(global) = global_shortcuts {
        if !global.exists() {
            bail!("global shortcuts config not found at {}", global.display());
        }
        migrate_untyped(global)?;
        report.shortcuts += 1;
    }

    if report == MigrateReport::default() {
        bail!("nothing to migrate in {}", root.display());
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vlabel_core::annotation::{add_object, new_label, new_object};
    use vlabel_core::{CatDef, Point, Polygon, RoiConfig, ShapeConfig};

    // 从 conversion.rs tests 原样复制的 LabelMeta 构造器（仓库权威副本，
    // 权威定义在 import.rs tests——conversion.rs 亦从那里引入）。
    // 复制而非共享：两处均为 #[cfg(test)] 私有，抽公共 test-util 模块收益不足。
    fn test_meta() -> vlabel_core::LabelMeta {
        vlabel_core::LabelMeta {
            id: 1,
            name: "test".to_string(),
            description: String::new(),
            shape: ShapeConfig {
                title_style: 0,
                thickness: 2,
                auto_save: true,
                vertex_radius: 10.0,
            },
            roi: RoiConfig { color: "#800080".to_string() },
            categories: vec![CatDef {
                id: 0,
                names: vlabel_core::LocalizedNames::en("person"),
                description: String::new(),
                hotkey: "1".to_string(),
                color: "#FF0000".to_string(),
                properties: vec![],
            }],
            property_types: vec![],
            property_special_values: vec![],
        }
    }

    #[test]
    fn test_migrate_project() {
        let root = std::env::temp_dir().join("vlabel_migrate_test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("vlabels")).unwrap();

        let label = new_label("test-tool");
        let obj = new_object(
            0,
            1,
            Polygon::from(vec![Point { x: 0.1, y: 0.1 }, Point { x: 0.5, y: 0.5 }]),
        );
        let mut label = label;
        add_object(&mut label, obj);

        fs::write(root.join("meta.yaml"), serde_yaml::to_string(&test_meta()).unwrap()).unwrap();
        fs::write(root.join("vlabels/0001.yaml"), serde_yaml::to_string(&label).unwrap()).unwrap();
        fs::write(root.join("shortcuts.yaml"), "version: '1.0'\nshortcuts: []\n").unwrap();

        let report = migrate_project(&root, None).unwrap();
        assert_eq!(report, MigrateReport { meta: 1, labels: 1, shortcuts: 1 });

        assert!(root.join("meta.json5").exists());
        assert!(!root.join("meta.yaml").exists());
        assert!(root.join("vlabels/0001.json5").exists());
        assert!(!root.join("vlabels/0001.yaml").exists());

        // 迁移产物可被主路径加载
        let project = crate::Project::open(&root).unwrap();
        let loaded = project.load_annotation("0001.jpg").unwrap().unwrap();
        assert_eq!(loaded.objects.len(), 1);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_migrate_refuses_existing_target() {
        let root = std::env::temp_dir().join("vlabel_migrate_refuse_test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("vlabels")).unwrap();
        fs::write(root.join("meta.yaml"), serde_yaml::to_string(&test_meta()).unwrap()).unwrap();
        fs::write(root.join("meta.json5"), "{}").unwrap(); // 目标已存在

        let err = migrate_project(&root, None).unwrap_err();
        assert!(err.to_string().contains("refusing to overwrite"));

        // 旧文件未被删除（fail-fast，无半迁移状态）
        assert!(root.join("meta.yaml").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_migrate_failure_keeps_project_closed() {
        let root = std::env::temp_dir().join("vlabel_migrate_partial_test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("vlabels")).unwrap();
        fs::write(root.join("meta.yaml"), serde_yaml::to_string(&test_meta()).unwrap()).unwrap();
        fs::write(
            root.join("vlabels/0001.yaml"),
            serde_yaml::to_string(&new_label("0001.jpg")).unwrap(),
        )
        .unwrap();
        // 语法非法的残留文件：第二个 label 解析失败，迁移中途 abort
        fs::write(root.join("vlabels/0002.yaml"), "objects: [unclosed\n").unwrap();

        let err = migrate_project(&root, None).unwrap_err();
        assert!(err.to_string().contains("0002.yaml"));

        // 半迁移态不可打开：meta.yaml 仍在、meta.json5 不存在，Project::open 失败
        assert!(root.join("meta.yaml").exists());
        assert!(!root.join("meta.json5").exists());

        // 修复故障源后重跑续迁成功（已迁移的 0001 不触发「目标已存在」守卫）
        fs::write(
            root.join("vlabels/0002.yaml"),
            serde_yaml::to_string(&new_label("0002.jpg")).unwrap(),
        )
        .unwrap();
        let report = migrate_project(&root, None).unwrap();
        assert_eq!(report, MigrateReport { meta: 1, labels: 1, shortcuts: 0 });

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_migrate_nothing_to_migrate() {
        let root = std::env::temp_dir().join("vlabel_migrate_empty_test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let err = migrate_project(&root, None).unwrap_err();
        assert!(err.to_string().contains("nothing to migrate"));

        let _ = fs::remove_dir_all(&root);
    }
}
