# lab-convert CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move all format-conversion logic from lab-gui into lab-utils and add a non-interactive CLI binary `lab-convert` (4 formats × import/export), leaving lab-gui as a thin dialog shell.

**Architecture:** lab-utils becomes the tool layer owning conversion (new `src/import.rs`, extended `src/conversion.rs`, new `src/bin/lab-convert.rs`). Dependency direction stays: lab-utils → lab-core; lab-gui → lab-utils + lab-core. lab-core is untouched except deleting one interim example.

**Tech Stack:** Rust workspace, clap 4.5 (derive), serde/serde_json/serde_yaml, image 0.25, anyhow.

**Spec:** `docs/superpowers/specs/2026-09-06-lab-convert-cli-design.md`

## Global Constraints

- 完成标准：`cargo test --workspace`、`cargo clippy --workspace`、`cargo fmt --check` 全绿。
- lab-core 除删除 `examples/import_yolo.rs` 外一行不动。
- 迁移函数保持原逻辑，仅做：去 `&self`、`pub` 可见性、use 路径适配。**禁止**顺手重构/改行为。
- 下沉函数返回 `anyhow::Result`；畸形数据行跳过必须打 `log::warn!`（无静默降级）。
- 函数 <50 行、文件 <800 行（j-coding-style）；commit 用 conventional commits（英文）。
- 每任务结束必须 `cargo test -p lab-utils`（或对应 crate）通过后再 commit。

## Source Map（迁移源，全部在 `lab-gui/src/app/import_export.rs`）

| 源位置 | 内容 | 去向 |
|--------|------|------|
| L280-357 | `import_from_yolo`（`&self` 方法） | import.rs |
| L359-421 | `import_from_voc` | import.rs |
| L423-497 | `import_from_coco` | import.rs |
| L499-555 | `import_from_labelme` | import.rs |
| L247-277 | `merge_imported_images`（已是自由函数） | import.rs |
| L187-191 | `struct ImportedImage` | import.rs（字段 pub） |
| L610-625 | `list_images_in_dir` | import.rs（pub） |
| L627-651 | `rect_polygon` + `clamp01` | import.rs（rect_polygon pub） |
| L653-663 | `build_label` | import.rs（pub） |
| L665-667 | `find_category_id_by_name` | import.rs（pub） |
| L669-723 | `parse_voc_size` / `parse_voc_objects` / `extract_tag_value` | import.rs |
| L725-772 | `bbox_to_polygon` / `coco_segmentation_to_polygon` | import.rs |
| L774-823 | `labelme_shape_to_polygon` / `find_image_by_stem` | import.rs |
| L825-883 | `VocObject` / `Coco*` / `LabelMeFile` / `LabelMeShape` DTO | import.rs |
| L216-244 | `collect_export_items`（`&self` 方法） | conversion.rs |
| L193-200 | `struct ExportItem` | conversion.rs（字段 pub） |
| L559-607 | `export_labelme_annotation`（`&self` 方法） | conversion.rs |
| L885-909 | `LabelMeOut` / `LabelMeShapeOut` DTO | conversion.rs |
| L1-17, 19-60 | use 块、`DatasetFormat`、对话框/错误展示 | 留在 lab-gui |

**通用变换规则**（适用所有迁移函数）：删除 `impl LabApp` 包装与 `&self` 参数；函数与 struct 加 `pub`；`use super::LabApp;` 删除；`lab_core::` 路径保持；`anyhow::Context` 等 use 移入新文件头。

---

### Task 1: lab-utils 基础设施 + YOLO 导入迁移

**Files:**
- Modify: `Cargo.toml`（workspace.dependencies）
- Modify: `lab-utils/Cargo.toml`
- Modify: `lab-utils/src/lib.rs`
- Create: `lab-utils/src/import.rs`

**Interfaces (Produces):**
```rust
// lab_utils::import
pub struct ImportedImage { pub source_path: PathBuf, pub file_name: String, pub annotation: Option<Label> }
pub fn import_from_yolo(root: &Path, meta: &LabelMeta) -> anyhow::Result<Vec<ImportedImage>>;
pub fn build_label(objects: Vec<Object>, user_agent: &str) -> Option<Label>;
pub fn list_images_in_dir(dir: &Path) -> anyhow::Result<Vec<PathBuf>>;
fn rect_polygon(xmin: f32, ymin: f32, xmax: f32, ymax: f32) -> Polygon<f32>;  // pub(crate) 供同文件测试
fn clamp01(value: f32) -> f32;
fn find_category_id_by_name(meta: &LabelMeta, name: &str) -> Option<i32>;
```

- [ ] **Step 1: 依赖与模块骨架**

`Cargo.toml` workspace.dependencies 追加两行（放在 chrono 之后）：
```toml
clap = { version = "4.5", features = ["derive"] }
image = "0.25"
```

`lab-utils/Cargo.toml` [dependencies] 追加：
```toml
serde.workspace = true
serde_json.workspace = true
log = "0.4"
image.workspace = true
clap.workspace = true
```

`lab-utils/src/lib.rs` 加 `pub mod import;`。创建 `lab-utils/src/import.rs` **骨架**：文件头 use + 从源 L187-191 迁入 `struct ImportedImage`（三字段加 `pub`）。**本步不迁 `import_from_yolo` 等函数**（留给 Step 3，保证 Step 3 测试先失败）。文件头：
```rust
//! 外部数据集格式导入（YOLO / VOC / COCO / LabelMe → JLab 项目）。

use anyhow::Context;
use lab_core::{Label, LabelMeta, Object, Point, Polygon};
use std::fs;
use std::path::{Path, PathBuf};
```

- [ ] **Step 2: 写失败测试**

在 `import.rs` 末尾添加 tests 模块（后续任务共用其中的 helpers）：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use lab_core::{CatDef, RoiConfig, ShapeConfig};

    pub(crate) fn test_meta() -> LabelMeta {
        LabelMeta {
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
                name: "person".to_string(),
                description: String::new(),
                hotkey: "1".to_string(),
                color: "#FF0000".to_string(),
                properties: vec![],
            }],
            property_types: vec![],
            property_special_values: vec![],
        }
    }

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lab-utils-test-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("images")).unwrap();
        fs::create_dir_all(dir.join("labels")).unwrap();
        dir
    }

    #[test]
    fn import_from_yolo_parses_boxes_and_skips_unknown_category() {
        let root = temp_root("yolo");
        fs::write(root.join("images/a.jpg"), b"fake").unwrap();
        fs::write(
            root.join("labels/a.txt"),
            "0 0.5 0.5 0.2 0.2\n9 0.1 0.1 0.1 0.1\nbad line\n",
        )
        .unwrap();

        let imported = import_from_yolo(&root, &test_meta()).unwrap();

        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].file_name, "a.jpg");
        let label = imported[0].annotation.as_ref().unwrap();
        assert_eq!(label.objects.len(), 1); // category 9 unknown -> skipped
        assert_eq!(label.objects[0].category, 0);
        assert_eq!(label.objects[0].polygon.0.len(), 4);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn import_from_yolo_without_labels_gives_empty_annotation() {
        let root = temp_root("yolo-empty");
        fs::write(root.join("images/b.png"), b"fake").unwrap();

        let imported = import_from_yolo(&root, &test_meta()).unwrap();

        assert_eq!(imported.len(), 1);
        assert!(imported[0].annotation.is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rect_polygon_clamps_and_swaps() {
        let p = rect_polygon(-0.1, 0.8, 0.5, 0.2);
        assert_eq!(p.0.len(), 4);
        assert_eq!(p.0[0], Point { x: 0.0, y: 0.2 });
        assert!(rect_polygon(0.5, 0.5, 0.5, 0.8).0.is_empty());
    }
}
```

- [ ] **Step 3: 跑测试确认失败**

Run: `cargo test -p lab-utils`
Expected: 编译失败（`import_from_yolo` 未定义）。

- [ ] **Step 4: 补全实现使测试通过**

从源 L280-357 迁入 `import_from_yolo`，L610-667 迁入 `list_images_in_dir`/`rect_polygon`/`clamp01`/`build_label`/`find_category_id_by_name`（按通用变换规则）。Run: `cargo test -p lab-utils`
Expected: PASS（新 3 测试 + 既有 6 测试）。

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml lab-utils/Cargo.toml lab-utils/src/lib.rs lab-utils/src/import.rs
git commit -m "feat: move YOLO import from lab-gui into lab-utils"
```

---

### Task 2: VOC 导入迁移

**Files:**
- Modify: `lab-utils/src/import.rs`

**Interfaces (Produces):**
```rust
pub fn import_from_voc(root: &Path, meta: &LabelMeta) -> anyhow::Result<Vec<ImportedImage>>;
```

- [ ] **Step 1: 写失败测试**（tests 模块内追加）

```rust
    #[test]
    fn import_from_voc_parses_objects_by_name() {
        let root = temp_root("voc");
        let voc_root = root.join("voc");
        fs::create_dir_all(voc_root.join("JPEGImages")).unwrap();
        fs::create_dir_all(voc_root.join("Annotations")).unwrap();
        fs::write(voc_root.join("JPEGImages/a.jpg"), b"fake").unwrap();
        fs::write(
            voc_root.join("Annotations/a.xml"),
            r#"<annotation><size><width>100</width><height>200</height></size>
               <object><name>person</name><xmin>10</xmin><ymin>20</ymin><xmax>30</xmax><ymax>60</ymax></object>
               <object><name>dog</name><xmin>1</xmin><ymin>1</ymin><xmax>9</xmax><ymax>9</ymax></object>
               </annotation>"#,
        )
        .unwrap();

        let imported = import_from_voc(&voc_root, &test_meta()).unwrap();

        let label = imported[0].annotation.as_ref().unwrap();
        assert_eq!(label.objects.len(), 1); // "dog" unknown -> skipped
        assert_eq!(label.objects[0].category, 0);
        // pixel (10,20)-(30,60) on 100x200 -> normalized (0.1,0.1)-(0.3,0.3)
        assert_eq!(label.objects[0].polygon.0[0], Point { x: 0.1, y: 0.1 });
        assert_eq!(label.objects[0].polygon.0[2], Point { x: 0.3, y: 0.3 });
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn import_from_voc_requires_dirs() {
        let err = import_from_voc(&temp_root("voc-bad"), &test_meta());
        assert!(err.is_err());
    }
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test -p lab-utils import_from_voc`
Expected: 编译失败（函数未定义）。

- [ ] **Step 3: 迁移实现**

迁入源 L359-421（`import_from_voc`）、L669-723（`parse_voc_size`/`parse_voc_objects`/`extract_tag_value`）、L825-832（`VocObject`），按通用变换规则。

- [ ] **Step 4: 跑测试通过**

Run: `cargo test -p lab-utils`
Expected: PASS。

- [ ] **Step 5: Commit**

```bash
git add lab-utils/src/import.rs
git commit -m "feat: move VOC import from lab-gui into lab-utils"
```

---

### Task 3: COCO 导入迁移

**Files:**
- Modify: `lab-utils/src/import.rs`

**Interfaces (Produces):**
```rust
pub fn import_from_coco(json_path: &Path, images_dir: &Path, meta: &LabelMeta) -> anyhow::Result<Vec<ImportedImage>>;
```

- [ ] **Step 1: 写失败测试**（tests 模块内追加）

```rust
    #[test]
    fn import_from_coco_maps_categories_and_segmentation() {
        let root = temp_root("coco");
        fs::write(root.join("a.jpg"), b"fake").unwrap();
        fs::write(
            root.join("ann.json"),
            r#"{"images":[{"id":1,"file_name":"a.jpg","width":100,"height":100}],
                "annotations":[
                  {"image_id":1,"category_id":7,"bbox":[10,10,20,20]},
                  {"image_id":1,"category_id":8,"bbox":[0,0,0,0],
                   "segmentation":[[10.0,10.0, 30.0,10.0, 30.0,30.0]]}],
                "categories":[{"id":7,"name":"person"},{"id":8,"name":"person"}]}"#,
        )
        .unwrap();

        let imported = import_from_coco(&root.join("ann.json"), &root, &test_meta()).unwrap();

        let label = imported[0].annotation.as_ref().unwrap();
        assert_eq!(label.objects.len(), 2);
        // category 7 named "person" -> mapped to meta id 0
        assert_eq!(label.objects[0].category, 0);
        // segmentation polygon (not bbox)
        assert_eq!(label.objects[1].polygon.0.len(), 3);
        let _ = fs::remove_dir_all(&root);
    }
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test -p lab-utils import_from_coco`
Expected: 编译失败。

- [ ] **Step 3: 迁移实现**

迁入源 L423-497（`import_from_coco`）、L725-772（`bbox_to_polygon`/`coco_segmentation_to_polygon`）、L834-863（`CocoDataset`/`CocoImage`/`CocoCategory`/`CocoAnnotation`）。文件头 use 增加 `use std::collections::HashMap;`。

- [ ] **Step 4: 跑测试通过**

Run: `cargo test -p lab-utils` — Expected: PASS。

- [ ] **Step 5: Commit**

```bash
git add lab-utils/src/import.rs
git commit -m "feat: move COCO import from lab-gui into lab-utils"
```

---

### Task 4: LabelMe 导入迁移

**Files:**
- Modify: `lab-utils/src/import.rs`

**Interfaces (Produces):**
```rust
pub fn import_from_labelme(root: &Path, meta: &LabelMeta) -> anyhow::Result<Vec<ImportedImage>>;
```

- [ ] **Step 1: 写失败测试**（tests 模块内追加）

```rust
    #[test]
    fn import_from_labelme_parses_polygon_shapes() {
        let root = temp_root("labelme");
        fs::write(root.join("a.jpg"), b"fake").unwrap();
        fs::write(
            root.join("a.json"),
            r#"{"imagePath":"a.jpg","imageHeight":100,"imageWidth":100,
                "shapes":[{"label":"person","shape_type":"polygon",
                           "points":[[10.0,10.0],[30.0,10.0],[30.0,30.0]]}]}"#,
        )
        .unwrap();

        let imported = import_from_labelme(&root, &test_meta()).unwrap();

        let label = imported[0].annotation.as_ref().unwrap();
        assert_eq!(label.objects.len(), 1);
        assert_eq!(label.objects[0].category, 0);
        assert_eq!(label.objects[0].polygon.0.len(), 3);
        assert_eq!(label.objects[0].polygon.0[0], Point { x: 0.1, y: 0.1 });
        let _ = fs::remove_dir_all(&root);
    }
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test -p lab-utils import_from_labelme` — Expected: 编译失败。

- [ ] **Step 3: 迁移实现**

迁入源 L499-555（`import_from_labelme`）、L774-823（`labelme_shape_to_polygon`/`find_image_by_stem`）、L865-883（`LabelMeFile`/`LabelMeShape`）。文件头 use 增加 `use serde::Deserialize;`。

- [ ] **Step 4: 跑测试通过**

Run: `cargo test -p lab-utils` — Expected: PASS。

- [ ] **Step 5: Commit**

```bash
git add lab-utils/src/import.rs
git commit -m "feat: move LabelMe import from lab-gui into lab-utils"
```

---

### Task 5: merge_imported_images 迁移

**Files:**
- Modify: `lab-utils/src/import.rs`

**Interfaces (Produces):**
```rust
pub fn merge_imported_images(
    imported: Vec<ImportedImage>,
    project: &lab_utils::Project,   // 同 crate: &crate::Project
    existing_names: &HashSet<String>,
    duplicate_template: &str,       // 含 "{name}" 占位符
) -> anyhow::Result<()>;
```

- [ ] **Step 1: 写失败测试**（tests 模块内追加；需 `use crate::Project;`）

```rust
    fn make_project(tag: &str) -> (PathBuf, Project) {
        let root = temp_root(tag);
        let meta = test_meta();
        lab_core::io::save_meta(root.join("meta.yaml"), &meta).unwrap();
        let project = Project::open(&root).unwrap();
        (root, project)
    }

    fn image(path: &Path, name: &str) -> ImportedImage {
        ImportedImage {
            source_path: path.to_path_buf(),
            file_name: name.to_string(),
            annotation: None,
        }
    }

    #[test]
    fn merge_copies_image_and_writes_annotation() {
        let (root, project) = make_project("merge-ok");
        let src = root.join("src.jpg");
        fs::write(&src, b"fake").unwrap();
        let ann = lab_core::new_label("test");

        merge_imported_images(
            vec![ImportedImage {
                source_path: src.clone(),
                file_name: "src.jpg".to_string(),
                annotation: Some(ann),
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

        let err = merge_imported_images(
            vec![image(&src, "dup.jpg"), image(&src, "dup.jpg")],
            &project,
            &Default::default(),
            "duplicate: {name}",
        )
        .unwrap_err();
        assert!(err.to_string().contains("duplicate: dup.jpg"));
        let _ = fs::remove_dir_all(&root);
    }
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test -p lab-utils merge_` — Expected: 编译失败。

- [ ] **Step 3: 迁移实现**

迁入源 L247-277（`merge_imported_images`，自由函数加 `pub`），其中 `ImportedImage` 字段读取处不变（同文件可见）。文件头 use 追加 `use std::collections::HashSet;`。

- [ ] **Step 4: 跑测试通过**

Run: `cargo test -p lab-utils` — Expected: PASS。

- [ ] **Step 5: Commit**

```bash
git add lab-utils/src/import.rs
git commit -m "feat: move image merge logic from lab-gui into lab-utils"
```

---

### Task 6: 导出侧迁移（collect_export_items / LabelMe 导出）

**Files:**
- Modify: `lab-utils/src/conversion.rs`

**Interfaces (Produces):**
```rust
// lab_utils::conversion
pub struct ExportItem {
    pub image_path: PathBuf,
    pub file_name: String,
    pub stem: String,
    pub width: u32,
    pub height: u32,
    pub annotation: Label,
}
pub fn collect_export_items(project: &crate::Project) -> anyhow::Result<Vec<ExportItem>>;
pub fn export_labelme_annotation(
    output_path: &Path,
    item: &ExportItem,
    meta: &LabelMeta,
) -> anyhow::Result<()>;
```

- [ ] **Step 1: 写失败测试**（conversion.rs 末尾新增 tests 模块）

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::tests::test_meta; // 若不可见则复制 Task 1 的 test_meta 到本模块
    use crate::Project;
    use lab_core::io::save_meta;
    use std::fs;

    #[test]
    fn collect_export_items_reads_dimensions_and_default_label() {
        let root = std::env::temp_dir().join(format!("lab-utils-cv-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("images")).unwrap();
        fs::create_dir_all(root.join("labels")).unwrap();
        save_meta(root.join("meta.yaml"), &test_meta()).unwrap();
        // real 4x2 png so image::open works
        image::RgbImage::from_pixel(4, 2, image::Rgb([0, 0, 0]))
            .save(root.join("images/p.png"))
            .unwrap();

        let project = Project::open(&root).unwrap();
        let items = collect_export_items(&project).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!((items[0].width, items[0].height), (4, 2));
        assert_eq!(items[0].annotation.objects.len(), 0); // no yaml -> empty label
        let _ = fs::remove_dir_all(&root);
    }
}
```

注：若 `crate::import::tests::test_meta` 跨模块不可见，把 `test_meta` 提为 `import.rs` 的 `#[cfg(test)] pub(crate) fn test_meta()`（放 tests 模块内声明 `pub(crate)`）。

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test -p lab-utils collect_export` — Expected: 编译失败。

- [ ] **Step 3: 迁移实现**

迁入源 L216-244（去 `&self`，`LabApp` 前缀删）、L193-200（`ExportItem` 字段 pub）、L559-607（同）、L885-909（`LabelMeOut`/`LabelMeShapeOut`）。conversion.rs 文件头 use 增加：
```rust
use crate::Project;
use anyhow::Context;
use image::GenericImageView;
use lab_core::{Label, LabelMeta};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
```
（与现有 use 合并去重，保留现有条目。）

- [ ] **Step 4: 跑测试通过**

Run: `cargo test -p lab-utils` — Expected: PASS。

- [ ] **Step 5: Commit**

```bash
git add lab-utils/src/conversion.rs
git commit -m "feat: move export collection and LabelMe export into lab-utils"
```

---

### Task 7: lab-gui 瘦身（删除本地实现，改调 lab-utils）

**Files:**
- Modify: `lab-gui/src/app/import_export.rs`（909 → ~250 行）
- Modify: `lab-gui/Cargo.toml`（image/clap 改 workspace 引用）

**Interfaces (Consumes):** Task 1-6 的全部 `lab_utils::import::*` 与 `lab_utils::conversion::*`。

- [ ] **Step 1: 重写 import_export.rs**

保留：L11-17 `DatasetFormat`、L19-60 对话框层（`import_dataset`/`export_dataset`/`show_io_error`/`try_import_dataset`/`try_export_dataset` 的对话框部分）、L202-214 `refresh_project_images`。
删除：Source Map 列出的全部迁移项。
`try_import_dataset` 中 `self.import_from_yolo(&root, &meta)?` 等改为：
```rust
lab_utils::import::import_from_yolo(&root, &meta)?
```
`merge_imported_images(imported, project, &existing_names, &duplicate_template)?` 改为：
```rust
lab_utils::import::merge_imported_images(imported, project, &existing_names, &duplicate_template)?;
```
`try_export_dataset` 中 `self.collect_export_items(project)?` → `lab_utils::conversion::collect_export_items(project)?`；`self.export_labelme_annotation(...)` → `lab_utils::conversion::export_labelme_annotation(...)`。
文件头 use 精简为实际所需（`use super::LabApp; use anyhow::Context; use std::collections::HashSet; use std::fs; use std::path::PathBuf;` 及 rfd 调用处按现状保留）。
`lab-gui/Cargo.toml`：`image = "0.25"` → `image.workspace = true`；`clap = { version = "4.5", features = ["derive"] }` → `clap.workspace = true`。

- [ ] **Step 2: 全 workspace 回归**

Run: `cargo test --workspace && cargo clippy --workspace && cargo fmt --check`
Expected: 全部通过，无未使用 import 警告。`wc -l lab-gui/src/app/import_export.rs` 应 < 400。

- [ ] **Step 3: 手动冒烟（可选但推荐）**

Run: `cargo run -p lab-gui`，打开菜单 文件→导入/导出 确认对话框可弹出（无项目时应报"未打开项目"而非 panic）。

- [ ] **Step 4: Commit**

```bash
git add lab-gui/src/app/import_export.rs lab-gui/Cargo.toml
git commit -m "refactor: lab-gui delegates conversion to lab-utils"
```

---

### Task 8: CLI bin `lab-convert` + 清理 examples

**Files:**
- Create: `lab-utils/src/bin/lab-convert.rs`
- Delete: `lab-core/examples/import_yolo.rs`
- Delete: `lab-utils/examples/export_yolo.rs`
- Modify: `README.md`（构建/运行小节补 lab-convert 一句）

**Interfaces (Consumes):** Task 1-6 全部 API + `Project`。

- [ ] **Step 1: 写 CLI 主程序**

```rust
//! Non-interactive annotation format converter for JLab projects.
//!
//! Usage:
//!   lab-convert import --format <yolo|voc|coco|labelme> <src> <project_dir>
//!       (coco: <src> is the annotation json; `--images <dir>` is required)
//!   lab-convert export --format <yolo|voc|coco|labelme> <project_dir> <out_dir>

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use lab_utils::conversion::{
    collect_export_items, export_annotation, export_coco_batch, export_labelme_annotation,
    ExportFormat,
};
use lab_utils::import::{
    import_from_coco, import_from_labelme, import_from_voc, import_from_yolo,
    merge_imported_images, ImportedImage,
};
use lab_utils::Project;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "lab-convert", version, about = "JLab annotation format converter")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Import an external dataset into a JLab project
    Import {
        /// Dataset format
        #[arg(short, long)]
        format: Format,
        /// Source root (coco: path to annotation json)
        src: PathBuf,
        /// Images directory (required when --format coco)
        #[arg(long)]
        images: Option<PathBuf>,
        /// Target JLab project directory (must contain meta.yaml)
        project: PathBuf,
    },
    /// Export a JLab project to an external dataset
    Export {
        /// Dataset format
        #[arg(short, long)]
        format: Format,
        /// JLab project directory
        project: PathBuf,
        /// Output directory
        out: PathBuf,
    },
}

#[derive(ValueEnum, Clone, Copy)]
enum Format {
    Yolo,
    Voc,
    Coco,
    LabelMe,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Import { format, src, images, project } => run_import(format, src, images, project),
        Command::Export { format, project, out } => run_export(format, project, out),
    }
}

fn run_import(format: Format, src: PathBuf, images: Option<PathBuf>, project_dir: PathBuf) -> Result<()> {
    let project = Project::open(&project_dir).context("failed to open project (meta.yaml required)")?;
    let meta = project.meta.clone();
    let existing_names = project
        .list_images()?
        .iter()
        .filter_map(|p| p.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()))
        .collect::<HashSet<String>>();

    let imported: Vec<ImportedImage> = match format {
        Format::Yolo => import_from_yolo(&src, &meta)?,
        Format::Voc => import_from_voc(&src, &meta)?,
        Format::Coco => {
            let images = images.context("--images <dir> is required for --format coco")?;
            import_from_coco(&src, &images, &meta)?
        }
        Format::LabelMe => import_from_labelme(&src, &meta)?,
    };

    let total_boxes = imported
        .iter()
        .map(|item| item.annotation.as_ref().map(|l| l.objects.len()).unwrap_or(0))
        .sum::<usize>();
    let count = imported.len();

    merge_imported_images(imported, &project, &existing_names, "duplicate image name: {name}")?;
    println!("imported {count} labels, {total_boxes} boxes");
    Ok(())
}

fn run_export(format: Format, project_dir: PathBuf, out_dir: PathBuf) -> Result<()> {
    let project = Project::open(&project_dir).context("failed to open project (meta.yaml required)")?;
    let meta = project.meta.clone();
    let items = collect_export_items(&project)?;
    let total_boxes = items.iter().map(|item| item.annotation.objects.len()).sum::<usize>();

    match format {
        Format::Yolo => {
            let images_dir = out_dir.join("images");
            let labels_dir = out_dir.join("labels");
            fs::create_dir_all(&images_dir)?;
            fs::create_dir_all(&labels_dir)?;
            for item in &items {
                export_annotation(
                    labels_dir.join(format!("{}.txt", item.stem)),
                    &item.annotation,
                    &meta,
                    &item.image_path.to_string_lossy(),
                    item.width,
                    item.height,
                    ExportFormat::Yolo,
                )?;
                fs::copy(&item.image_path, images_dir.join(&item.file_name))?;
            }
        }
        Format::Voc => {
            let images_dir = out_dir.join("JPEGImages");
            let annotations_dir = out_dir.join("Annotations");
            fs::create_dir_all(&images_dir)?;
            fs::create_dir_all(&annotations_dir)?;
            for item in &items {
                export_annotation(
                    annotations_dir.join(format!("{}.xml", item.stem)),
                    &item.annotation,
                    &meta,
                    &item.image_path.to_string_lossy(),
                    item.width,
                    item.height,
                    ExportFormat::Voc,
                )?;
                fs::copy(&item.image_path, images_dir.join(&item.file_name))?;
            }
        }
        Format::Coco => {
            let images_dir = out_dir.join("images");
            fs::create_dir_all(&images_dir)?;
            for item in &items {
                fs::copy(&item.image_path, images_dir.join(&item.file_name))?;
            }
            let coco_items = items
                .iter()
                .map(|item| (item.file_name.clone(), item.annotation.clone(), item.width, item.height))
                .collect::<Vec<_>>();
            export_coco_batch(&out_dir.join("annotations.json"), &coco_items, &meta)?;
        }
        Format::LabelMe => {
            fs::create_dir_all(&out_dir)?;
            for item in &items {
                fs::copy(&item.image_path, out_dir.join(&item.file_name))?;
                export_labelme_annotation(&out_dir.join(format!("{}.json", item.stem)), item, &meta)?;
            }
        }
    }

    println!("exported {} labels, {total_boxes} boxes -> {}", items.len(), out_dir.display());
    Ok(())
}
```

注：`run_export` 的 yolo/voc 分支与 GUI `try_export_dataset`（源 L117-181）行为逐行对齐（目录名、复制、文件名后缀）；若函数超过 50 行上限，把 yolo/voc/coco/labelme 四分支提成 `fn export_yolo(...)` 等四个私有函数（每个 <20 行），主函数只做 match 分发。

- [ ] **Step 2: 编译并跑单测**

Run: `cargo build --release -p lab-utils && cargo test --workspace`
Expected: 产出 `target/release/lab-convert`；测试全绿。

- [ ] **Step 3: 真实数据集成验证**

```bash
B=/mnt/data/jiang/ws/sgcc/person/datasets/sgcc-n001/crop640_persons
D=$B/cli_verify_project
rm -rf "$D" && mkdir -p "$D/images" "$D/labels"
cp "$B/manual_annotate_2/meta.yaml" "$D/"
./target/release/lab-convert import --format yolo "$B/manual_annotate" "$D"
./target/release/lab-convert export --format yolo "$D" "$D/../cli_verify_yolo"
```
Expected: import 输出 `imported 354 labels, 5232 boxes`；export 输出 `exported 354 labels, 5232 boxes`。
验证一致性：
```bash
python3 - <<'EOF'
import glob, collections
def dist(root):
    c = collections.Counter()
    for f in glob.glob(root + '/labels/*.txt'):
        for l in open(f):
            if l.strip(): c[l.split()[0]] += 1
    return dict(sorted(c.items()))
a = dist('/mnt/data/jiang/ws/sgcc/person/datasets/sgcc-n001/crop640_persons/manual_annotate')
b = dist('/mnt/data/jiang/ws/sgcc/person/datasets/sgcc-n001/crop640_persons/cli_verify_yolo')
assert a == b, (a, b)
print('distribution identical:', b)
EOF
rm -rf "$D" "$B/cli_verify_yolo"
```
Expected: `distribution identical: {'1': 558, ...}`。

- [ ] **Step 4: 删除 interim examples + README**

```bash
git rm lab-core/examples/import_yolo.rs lab-utils/examples/export_yolo.rs
```
README.md 构建/运行小节追加：
```markdown
### 格式转换 CLI（非交互）

```bash
cargo run --release -p lab-utils --bin lab-convert -- import --format yolo <src_root> <project_dir>
cargo run --release -p lab-utils --bin lab-convert -- export --format coco <project_dir> <out_dir>
```
```

- [ ] **Step 5: 最终全量检查**

Run: `cargo test --workspace && cargo clippy --workspace && cargo fmt --check`
Expected: 全绿。

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: add lab-convert CLI; remove interim examples"
```
