# lab-convert CLI 设计

**日期**: 2026-09-06
**状态**: 已与用户逐节确认
**主题**: 为 JLab 新增非交互格式转换 CLI，与 lab-gui 形成两个平等的可执行程序

## 1. 背景与目标

JLab 目前只有一个二进制（lab-gui，交互式标注 GUI）。格式转换（YOLO/VOC/COCO/LabelMe
双向）寄居在 `lab-gui/src/app/import_export.rs`（909 行），只能通过 GUI 对话框使用，
无法脚本化；此前数据迁移靠临时 examples（`lab-core/examples/import_yolo.rs`、
`lab-utils/examples/export_yolo.rs`）完成。

目标：把格式转换逻辑归位到工具层（lab-utils），并提供 CLI 可执行程序
`lab-convert`，与 `lab-gui` 平行：

- `target/release/lab-gui` —— 交互式标注（不变）
- `target/release/lab-convert` —— 非交互格式转换（新增）

## 2. 设计决策（用户裁决记录）

| 决策点 | 裁决 | 否决的备选 |
|--------|------|-----------|
| CLI 功能范围 | 全格式对齐 GUI（4 格式 × 导入/导出双向） | 仅 YOLO |
| CLI 之家 | lab-utils（工具库，conversion API 已在此） | lab-gui 包（名不副实）、新 CLI crate（太重量）、lab-core（导入导出不是核心功能） |
| lab-core 现有 `src/export/` | 不动（lab-utils conversion 层继续调用它） | 迁出到 lab-utils |
| 依赖方向 | 工具依赖核心（lab-utils → lab-core）；GUI 依赖工具 + 核心（lab-gui → lab-utils + lab-core）——既有结构，本设计不改变 | — |

## 3. 架构

```
lab-core  （核心：数据模型 + 导出格式实现）—— 一行不动
   ↑              ↑
lab-utils        │
（工具：转换编排 + 导入 + CLI bin）          │
   ↑              │
   └──── lab-gui（GUI：对话框壳 + 调用 lab-utils API）
```

### 3.1 lab-utils 模块规划

```
lab-utils/src/
├── import.rs          # 新增：4 格式导入纯函数 + ImportedImage + merge + rect_polygon/build_label
├── conversion.rs      # 扩展：现有导出 API + 下沉 collect_export_items/ExportItem/LabelMe 导出
├── bin/
│   └── lab-convert.rs # CLI 薄壳（clap 解析 → 调 import/conversion）
├── project.rs         # 不动（CLI 直接复用 Project API）
└── validation.rs      # 不动
```

- 迁入 `import.rs` 的函数（来自 `lab-gui/src/app/import_export.rs`）：
  `import_from_yolo` / `import_from_voc` / `import_from_coco` /
  `import_from_labelme` / `merge_imported_images` / `rect_polygon` /
  `build_label` / `struct ImportedImage`——全部去掉 `&self`（本就不依赖 GUI
  状态），i18n 错误消息保持 `duplicate_template: &str` 参数化（GUI 传 i18n 串，
  CLI 传英文默认串）。
- 迁入 `conversion.rs` 的函数：`collect_export_items` / `struct ExportItem` /
  `export_labelme_annotation`。
- lab-gui 的 `import_export.rs` 瘦身为：rfd 文件对话框 + i18n + 调用
  `lab_utils::import::*`（预计 909 → ~250 行）。

### 3.2 CLI 接口（clap derive）

```
# 导入：外部数据集 → JLab 项目
lab-convert import --format yolo    <src_root> <project_dir>
lab-convert import --format voc     <src_root> <project_dir>
lab-convert import --format labelme <src_root> <project_dir>
lab-convert import --format coco    <ann.json> --images <dir> <project_dir>

# 导出：JLab 项目 → 外部数据集
lab-convert export --format yolo|voc|coco|labelme <project_dir> <out_dir>
```

- `--format` 为必选枚举参数（yolo/voc/coco/labelme）。
- coco 导入需要两个输入路径（标注 json + 图片目录），`--images <dir>` 为该格式下的
  **必选**参数（clap 按格式校验，缺失即报错）。
- 行为与 GUI 完全一致（调用同一批函数）：
  - 导入 = 解析 → 重名检查（与项目已有图片或导入集内部重复即报错）→ 复制图片
    进 `images/` → 写 `labels/*.yaml`；解析失败的行跳过并打 warn 日志。
  - 导出 = 遍历项目图片 →（无标注的图导出空标注）→ 按格式建目录结构并**复制图片**
    （yolo: `images/`+`labels/`；voc: `JPEGImages/`+`Annotations/`；
    coco: `annotations.json`+图片复制；labelme: 平铺 json+图片）。
- 结束打印统计（如 `imported 354 labels, 5232 boxes`）。

## 4. 依赖与 Cargo 变更

- `[workspace.dependencies]` 新增：
  - `clap = { version = "4.5", features = ["derive"] }`
  - `image = "0.25"`
- `lab-utils` 新增依赖：clap（仅 bin 使用）、image（collect_export_items 算尺寸）。
- `lab-gui` 的 image/clap 改为引 workspace 版本（统一版本单一来源）。
- **删除** `lab-core/examples/import_yolo.rs` 与 `lab-utils/examples/export_yolo.rs`
  （被 CLI 正式取代——替换实现同批删旧，不共存）。

## 5. 错误处理

- 下沉函数维持 `anyhow::Result` 签名（与 GUI 现状一致；lab-utils 定位为应用工具
  库，不为一次性工具函数新造 Error enum）。
- CLI 入口：错误输出到 stderr、进程退出码 1；参数错误由 clap 自动处理。
- 无静默降级：重名冲突直接报错；畸形数据行跳过时必须打 warn 日志（与 GUI 语义一致）。

## 6. 测试策略

- lab-utils 单元测试（新增）：
  - YOLO roundtrip：txt → Label → 导出 → 逐行比对。
  - VOC/COCO/LabelMe 样例解析（小 fixture 文件放 `lab-utils/tests/fixtures/`）。
  - merge 重名冲突报错路径。
- 集成验证：用真实数据（`manual_annotate`，354 张 YOLO 数据集）跑 CLI import/export，
  结果与现有 `manual_annotate_2` / `manual_annotate_2_yolo` 比对一致。
- 完成标准：`cargo test --workspace`、`cargo clippy --workspace`、`cargo fmt --check`
  全绿；GUI 导入导出对话框回归可用。

## 7. 非目标

- 不改 lab-core（含其 `src/export/` 的归属）。
- 不做标注交互功能的 CLI 化。
- 不加 dry-run / 并行 / 进度条（YAGNI，需要时再加）。
