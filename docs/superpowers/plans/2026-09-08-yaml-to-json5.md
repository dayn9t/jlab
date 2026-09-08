# 标注格式 YAML → JSON5 迁移 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 磁盘标注格式从 YAML 全家（meta + vlabels + shortcuts）迁到 JSON5，主读写路径移除 serde_yaml，并提供长期保留的 `vlabel-convert migrate-yaml` 迁移子命令。

**Architecture:** 序列化收口在 `vlabel_core::io`（json5 双向）+ `Project::annotation_path`（扩展名推导）+ `shortcuts.rs`（GUI 配置）。serde_yaml 降级为 vlabel-utils 私有依赖，仅供新模块 `migrate.rs` 读旧文件——显式兼容桥，非主路径宽容层。

**Tech Stack:** Rust workspace（vlabel-core / vlabel-utils / vlabel-gui）、`json5` crate 1.x（serde 双向）、serde_yaml 0.9（仅迁移）、clap 4。

**Spec:** `docs/superpowers/specs/2026-09-08-yaml-to-json5-design.md`

## Global Constraints

- 扩展名一律 `.json5`（meta.json5 / vlabels/<stem>.json5 / shortcuts.json5）
- 读 `json5::from_str`（宽容：注释/尾逗号/无引号 key），写 `json5::to_string`（真 JSON5 输出）
- `version` 字段保持 `"2.0"`——schema 不变，纯序列化替换
- 主代码路径（vlabel-core、vlabel-gui）禁止出现 serde_yaml；仅 `vlabel-utils/src/migrate.rs`（及其测试）允许
- 不写宽容解析/兼容层：`properties: {}` 坏 fixture 用手工编辑修，不给迁移工具加 dict→Vec 容错（已实证 serde_yaml 报 `invalid type: map, expected a sequence`，见 Task 5）
- 每个 commit 后 `cargo test --workspace` 必须全绿
- Commit 信息 conventional 格式，无 attribution 脚注
- 已实证：`Error::Yaml` 变体无模式匹配消费者（仅 `#[from]` 自动转换），重命名零波及

---

### Task 1: vlabel-core 读写层切换 JSON5

**Files:**
- Modify: `vlabel-core/src/io.rs`（全文件 32 行，serde_yaml → json5）
- Modify: `vlabel-core/src/error.rs:8-9`（错误变体）
- Modify: `Cargo.toml`（workspace.dependencies 加 json5）
- Modify: `vlabel-core/Cargo.toml`（serde_yaml → json5）

**Interfaces:**
- Consumes: 无（首发任务）
- Produces: `vlabel_core::io::{load_meta, save_meta, load_annotation, save_annotation}` 签名不变（后续任务依赖路径推导而非扩展名感知）；`Error::Json5(#[from] json5::Error)`

- [ ] **Step 1: 写失败测试**

替换 `vlabel-core/src/io.rs` 底部 tests 模块中 `test_annotation.yaml` → `test_annotation.json5`，并新增手改宽容用例：

```rust
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
        assert_eq!(loaded.objects[0].polygon.len(), 2);

        let _ = fs::remove_file(test_file);
    }
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p vlabel-core`
Expected: FAIL——当前 io.rs 用 serde_yaml 解析含注释的 JSON5 文本报错

- [ ] **Step 3: 最小实现**

`Cargo.toml` [workspace.dependencies] 中 `serde_yaml = "0.9"` 之后加一行（若 crates.io 版本解析失败，用 `cargo add json5 --package vlabel-core` 取实际最新版）：

```toml
json5 = "1"
```

`vlabel-core/Cargo.toml` 把 `serde_yaml.workspace = true` 替换为：

```toml
json5.workspace = true
```

`vlabel-core/src/error.rs:8-9` 变体替换（若 `#[from] json5::Error` 编译不过——json5::Error 未实现 std::error::Error 的情形——降级为 `Json5(String)` 并在 io.rs 用 `.map_err(|e| Error::Json5(e.to_string()))`）：

```rust
    #[error("JSON5 serialization error: {0}")]
    Json5(#[from] json5::Error),
```

`vlabel-core/src/io.rs` 四个函数体内的 `serde_yaml::` 全部替换为 `json5::`，doc 注释中 "YAML file" 改 "JSON5 file"：

```rust
/// Load metadata from a JSON5 file
pub fn load_meta<P: AsRef<Path>>(path: P) -> Result<LabelMeta> {
    let content = fs::read_to_string(path)?;
    let meta: LabelMeta = json5::from_str(&content)?;
    Ok(meta)
}
```

（save_meta / load_annotation / save_annotation 同型替换：`json5::to_string(meta)?`、`json5::from_str(&content)?`、`json5::to_string(annotation)?`）

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p vlabel-core`
Expected: PASS（round-trip + 手改宽容两用例）

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock vlabel-core/Cargo.toml vlabel-core/src/io.rs vlabel-core/src/error.rs
git commit -m "feat: switch core annotation serialization from YAML to JSON5"
```

---

### Task 2: 项目路径推导切 .json5 + 全 workspace 测试 fixture 同步

**Files:**
- Modify: `vlabel-utils/src/project.rs:6,24,43`（路径推导 + 注释）
- Modify: `vlabel-utils/src/project.rs`（tests 中 `meta.yaml` 引用，:312 附近）
- Modify: `vlabel-utils/src/conversion.rs:688,782`（tests）
- Modify: `vlabel-utils/src/import.rs:285-286`（注释）
- Modify: `vlabel-utils/src/bin/vlabel-convert.rs:39`（CLI help 文案 "must contain meta.yaml"）
- Modify: `vlabel-gui/src/state_persistence.rs:229,258,322,345,380`（tests + 注释，:380 的 `labels/a.yaml` 一并修正为 `vlabels/a.json5`）

**Interfaces:**
- Consumes: Task 1 的 io 层（对扩展名无感知，路径由本任务决定）
- Produces: `Project::open` 读 `meta.json5`；`Project::annotation_path` 返回 `vlabels/<stem>.json5`——Task 4 迁移产物的消费契约

- [ ] **Step 1: 改路径推导（源代码三处）**

`vlabel-utils/src/project.rs`：

```rust
/// Extract the file stem used to key annotation files (labels are keyed by stem,
/// so a.jpg and a.png share the same vlabels/a.json5).
```

```rust
        let meta_path = root.join("meta.json5");
```

```rust
        self.vlabels_dir().join(format!("{}.json5", file_stem(image_name)))
```

- [ ] **Step 2: 同步全部测试与注释中的路径**

上述 Files 列表中所有 `"meta.yaml"` 字面量 → `"meta.json5"`、`labels/*.yaml` 注释 → `vlabels/*.json5`。逐文件改完后验证无残留：

Run: `grep -rn "yaml" vlabel-utils/src vlabel-gui/src vlabel-core/src --include="*.rs"`
Expected: 仅剩本任务不改的语义性引用（如错误消息文案），无 `"meta.yaml"` / `".yaml"` 路径构造残留

- [ ] **Step 3: 全 workspace 测试**

Run: `cargo test --workspace`
Expected: PASS——凡构造 `meta.yaml` fixture 再 `Project::open` 的测试若漏改会在此暴露

- [ ] **Step 4: Commit**

```bash
git add vlabel-utils/src vlabel-gui/src
git commit -m "feat: derive project paths as .json5 and update test fixtures"
```

---

### Task 3: GUI 快捷键配置切 JSON5

**Files:**
- Modify: `vlabel-gui/src/shortcuts.rs:505-533,826-832,848-875`（读写 + 路径 + doc 注释）
- Modify: `vlabel-gui/Cargo.toml`（`serde_yaml = "0.9"` → `json5.workspace = true`）

**Interfaces:**
- Consumes: 无前置依赖（与 Task 1/2 并行安全，但按序执行）
- Produces: `ShortcutManager::{load_from_file, save_to_file}` 走 json5；`user_config_path` 返回 `shortcuts.json5`——Task 4 迁移 `--global` 的目标格式

- [ ] **Step 1: 写失败测试**

`vlabel-gui/src/shortcuts.rs` 无 tests 模块，文件末尾新增：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortcuts_roundtrip_and_hand_edit_tolerance() {
        let dir = std::env::temp_dir().join("vlabel_shortcuts_json5_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("shortcuts.json5");

        let manager = ShortcutManager::new();
        manager.save_to_file(&path).unwrap();
        let loaded = ShortcutManager::load_from_file(&path).unwrap();
        assert_eq!(
            loaded.get_config().shortcuts.len(),
            manager.get_config().shortcuts.len()
        );

        // 手改：文件头加注释（JSON5 宽容读）
        let hand_edited = format!("// edited by hand\n{}", std::fs::read_to_string(&path).unwrap());
        std::fs::write(&path, hand_edited).unwrap();
        ShortcutManager::load_from_file(&path).unwrap();

        let _ = std::fs::remove_dir_all(&dir);
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p vlabel-gui test_shortcuts_roundtrip`
Expected: FAIL——serde_yaml 无法解析含注释内容（写出的 YAML 头加 `//` 注释行后解析报错）

- [ ] **Step 3: 最小实现**

`vlabel-gui/Cargo.toml` 把 `serde_yaml = "0.9"` 替换为 `json5.workspace = true`。

`vlabel-gui/src/shortcuts.rs` 四处替换：

- `:510` `serde_yaml::from_str(&content)` → `json5::from_str(&content)`
- `:527` `serde_yaml::to_string(&self.config)` → `json5::to_string(&self.config)`
- `:829` `path.push("shortcuts.yaml")` → `path.push("shortcuts.json5")`
- `:854` `project_dir.join("shortcuts.yaml")` → `project_dir.join("shortcuts.json5")`
- `:864` `serde_yaml::from_str(&content)` → `json5::from_str(&content)`（project config 分支，同一处代码块）

doc 注释 "Load/Save configuration from/to YAML file" → "JSON5 file"。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p vlabel-gui test_shortcuts_roundtrip && cargo build -p vlabel-gui`
Expected: PASS + 编译通过（确认无 serde_yaml 残留引用）

- [ ] **Step 5: Commit**

```bash
git add vlabel-gui/Cargo.toml vlabel-gui/src/shortcuts.rs Cargo.lock
git commit -m "feat: switch shortcuts config to JSON5"
```

---

### Task 4: vlabel-convert migrate-yaml 迁移子命令

**Files:**
- Create: `vlabel-utils/src/migrate.rs`
- Modify: `vlabel-utils/src/lib.rs`（挂 `pub mod migrate;`——按现有模块声明风格）
- Modify: `vlabel-utils/src/bin/vlabel-convert.rs`（新子命令）
- Modify: `vlabel-utils/Cargo.toml`（加 serde_yaml / json5 / dirs）

**Interfaces:**
- Consumes: `vlabel_core::{Label, LabelMeta}`（serde 类型）；Task 2 的 `Project::open`（测试里验证迁移产物可被主路径加载）
- Produces: `migrate::migrate_project(&Path, Option<&Path>) -> anyhow::Result<MigrateReport>`；`migrate::global_shortcuts_yaml_path() -> Option<PathBuf>`；`MigrateReport { meta: usize, labels: usize, shortcuts: usize }`（derive PartialEq 供断言）

**设计要点（实现须知）：** meta/labels 走**类型化**迁移（serde_yaml 严格解析 → json5 写出 → 类型化回读验证 → 删旧文件）；shortcuts 配置类型 `ShortcutConfig` 定义在 vlabel-gui（依赖 egui，不可下沉），走**非类型化** `serde_json::Value` 直通迁移（字段全是标量，风险可忽略），回读验证为 Value 级解析。

- [ ] **Step 1: 写失败测试**

`vlabel-utils/src/migrate.rs` 创建时即含完整测试模块。注意 `test_meta()` 从 `vlabel-utils/src/conversion.rs` 的 tests 模块**原样复制**（LabelMeta 构造较长，仓库已有权威副本）：

```rust
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
    fs::write(&dst, json5::to_string(&value)?)?;
    let _verify: T = json5::from_str(&fs::read_to_string(&dst)?)
        .with_context(|| format!("verification failed for {}", dst.display()))?;
    fs::remove_file(src).with_context(|| format!("failed to remove {}", src.display()))?;
    Ok(())
}

/// 非类型化迁移单文件（shortcuts 专用，类型在 vlabel-gui 不可达）。
fn migrate_untyped(src: &Path) -> Result<()> {
    let dst = src.with_extension("json5");
    if dst.exists() {
        bail!("target already exists, refusing to overwrite: {}", dst.display());
    }
    let content =
        fs::read_to_string(src).with_context(|| format!("failed to read {}", src.display()))?;
    let value: serde_json::Value = serde_yaml::from_str(&content)
        .with_context(|| format!("failed to parse YAML {}", src.display()))?;
    fs::write(&dst, json5::to_string(&value)?)?;
    let _verify: serde_json::Value = json5::from_str(&fs::read_to_string(&dst)?)
        .with_context(|| format!("verification failed for {}", dst.display()))?;
    fs::remove_file(src).with_context(|| format!("failed to remove {}", src.display()))?;
    Ok(())
}

/// 迁移项目目录下所有旧 YAML 文件；`global_shortcuts` 为 --global 传入的
/// 全局快捷键旧配置路径。无可迁移文件时报错（不静默成功）。
pub fn migrate_project(root: &Path, global_shortcuts: Option<&Path>) -> Result<MigrateReport> {
    let mut report = MigrateReport::default();

    let meta_path = root.join("meta.yaml");
    if meta_path.exists() {
        migrate_one::<vlabel_core::LabelMeta>(&meta_path)?;
        report.meta += 1;
    }

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
    use vlabel_core::{Point, Polygon};

    // 从 conversion.rs tests 原样复制的 LabelMeta 构造器（仓库权威副本）。
    // 复制而非共享：两处均为 #[cfg(test)] 私有，抽公共 test-util 模块收益不足。
    fn test_meta() -> vlabel_core::LabelMeta { /* copy from conversion.rs */ }

    #[test]
    fn test_migrate_project() {
        let root = std::env::temp_dir().join("vlabel_migrate_test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("vlabels")).unwrap();

        let label = new_label("test-tool");
        let obj = new_object(
            0,
            1,
            Polygon::from(vec![
                Point { x: 0.1, y: 0.1 },
                Point { x: 0.5, y: 0.5 },
            ]),
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
    fn test_migrate_nothing_to_migrate() {
        let root = std::env::temp_dir().join("vlabel_migrate_empty_test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let err = migrate_project(&root, None).unwrap_err();
        assert!(err.to_string().contains("nothing to migrate"));

        let _ = fs::remove_dir_all(&root);
    }
}
```

`vlabel-utils/src/lib.rs` 按现有风格声明 `pub mod migrate;`。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p vlabel-utils migrate`
Expected: FAIL——模块尚未被 lib.rs 声明 / 或编译错误（dirs、json5 依赖未加）

- [ ] **Step 3: 最小实现（依赖 + CLI）**

`vlabel-utils/Cargo.toml` [dependencies] 追加：

```toml
serde_yaml.workspace = true
json5.workspace = true
dirs = "5.0"
```

（migrate.rs 主体已在 Step 1 给出全量代码，此处补 `test_meta()` 实体——从 conversion.rs tests 复制。）

`vlabel-utils/src/bin/vlabel-convert.rs` 的 `Command` 枚举追加：

```rust
    /// Migrate a legacy YAML project (meta.yaml + vlabels/*.yaml + shortcuts.yaml) to JSON5
    MigrateYaml {
        /// VLabel project directory
        project: PathBuf,
        /// Also migrate the global shortcuts config (~/.config/vlabel/shortcuts.yaml)
        #[arg(long)]
        global: bool,
    },
```

文件头 usage 注释追加一行：

```rust
//!   vlabel-convert migrate-yaml [--global] <project_dir>
```

`match cli.command` 追加分支（顶部 use 加 `vlabel_utils::migrate`）：

```rust
        Command::MigrateYaml { project, global } => {
            let global_path = if global {
                Some(
                    migrate::global_shortcuts_yaml_path()
                        .context("could not resolve global config directory")?,
                )
            } else {
                None
            };
            let report = migrate::migrate_project(&project, global_path.as_deref())
                .with_context(|| format!("migration failed in {}", project.display()))?;
            println!(
                "migrated: meta={}, labels={}, shortcuts={}",
                report.meta, report.labels, report.shortcuts
            );
        }
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p vlabel-utils && cargo test --workspace`
Expected: PASS（迁移三用例 + 全 workspace 回归）

- [ ] **Step 5: Commit**

```bash
git add vlabel-utils/src vlabel-utils/Cargo.toml Cargo.lock
git commit -m "feat: add vlabel-convert migrate-yaml subcommand for YAML-to-JSON5 migration"
```

---

### Task 5: 仓库 fixture 迁移 + 文档同步

**Files:**
- Modify: `assets/meta.yaml` → `assets/meta.json5`（经迁移工具产出）
- Modify: `assets/labels/0001.yaml` → `assets/vlabels/0001.json5`（先修坏数据，再经迁移工具）
- Modify: `README.md:43,99`、`CLAUDE.md:9,12`、`vlabel-utils/CLAUDE.md:8`、`docs/图像标注工具设计.md`（grep 定位）、`docs/标注格式对比-Rust与Python.md`（顶部注记）

**Interfaces:**
- Consumes: Task 4 的 `vlabel-convert migrate-yaml`（dogfood 自用）
- Produces: 仓库内无 YAML 标注 fixture；文档与新格式一致

**前置事实（已实证）：** `assets/labels/0001.yaml` 各 object 的 `properties: {}` 是 Python 时代 dict 残留，serde_yaml 严格解析必报错——先手工修为 `[]`，不给迁移工具加宽容层。

- [ ] **Step 1: 修坏 fixture 并重排目录名**

```bash
git mv assets/labels assets/vlabels
```

编辑 `assets/vlabels/0001.yaml`：三处 `properties: {}` → `properties: []`（对象 id 1/3/5/9 共四处，grep 确认无 `{}` 残留）。

- [ ] **Step 2: 迁移工具 dogfood**

```bash
cargo run -p vlabel-utils --bin vlabel-convert -- migrate-yaml assets
```

Expected: 输出 `migrated: meta=1, labels=1, shortcuts=0`；`ls assets` 只剩 `meta.json5 images vlabels`；抽查 `head assets/vlabels/0001.json5` 为无引号 key 的 JSON5。

- [ ] **Step 3: 文档同步**

- `README.md:43` `meta.yaml` → `meta.json5`；`:99` `shortcuts.yaml` → `shortcuts.json5`；grep README 其余 `yaml` 一并改
- `CLAUDE.md:9` "存储为 YAML" → "存储为 JSON5"；`:12` `meta.yaml` → `meta.json5`
- `vlabel-utils/CLAUDE.md:8` 同步
- `docs/图像标注工具设计.md`：`grep -n yaml` 定位后同步
- `docs/标注格式对比-Rust与Python.md` 顶部引用块后加一行：`> 2026-09-08 注：磁盘格式已由 YAML 迁至 JSON5（见 specs/2026-09-08-yaml-to-json5-design.md），本文其余 YAML 表述为历史记录。`
- 终检：`grep -rn "yaml\|YAML" README.md CLAUDE.md */CLAUDE.md docs/图像标注工具设计.md`——除归档文档的历史记录外无残留

- [ ] **Step 4: 全量回归**

Run: `cargo test --workspace && cargo build --workspace`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add assets README.md CLAUDE.md vlabel-utils/CLAUDE.md docs
git commit -m "chore: migrate repo fixtures to JSON5 and sync docs"
```

---

## 完成后（非本计划任务）

用户存量数据集执行 `vlabel-convert migrate-yaml <project_dir> [--global]` 一次性迁移；迁移前建议整目录备份。
