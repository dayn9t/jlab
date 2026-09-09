# feature/convert-pipeline 分支审阅摘要

> 8 commits（2026-09-08 ~ 09-09），基于 `2ceeeca` (v0.8.0)。
> master tip 已前进至 `1a016e3`（JXL parity GUI 膜，另一会话提交），两侧仅
> v0.8.0 处分叉、无文件冲突（merge-tree 预演通过），合并为 merge commit。
> 生成于人工审阅前，测试全绿（见文末）。

## 分支目的

为检测样本流水线（n001 crop640 固定机位场景）补齐 `vlabel-convert` CLI 的导入/导出/对账能力：
负样本保留、批量 ROI 注入、轻量 YOLO 导出、round-trip 逐框对账、ImageFolder 分类集导出
（R1-R6 六项需求全部落地）。GUI 行为不变（仅一处 export 调用点传默认参数）。

## Commit 清单（时间序，含需求编号）

| Hash | 需求 | 主题 | 一句话 |
|------|------|------|--------|
| e458de5 | R1 | test(import): 负样本帧导入为空 vlabel | blank/missing label 文件必须落成 0-object vlabel 而非跳过；import.rs + project.rs 两级测试锁定既有行为（纯测试，+55 行） |
| ed5742c | R2 | feat(import): `--roi` 批量 ROI 注入 | 新扁平模块 `roi_inject.rs`（218 行/8 测试）：JSON5 规则按文件 stem 前缀注入 ROI 多边形；多 key 匹配歧义即报错，未匹配帧仅 warn |
| ee1bf4b | R4 | feat(export): yolo `--no-mask` / `--symlink` | `YoloExportOptions` + `link_or_copy`（unix-only）；非 yolo 格式传参直接 bail；附 R5 TODO 设计稿；`export_image_roi_masked` 更名 `export_image_yolo` |
| a7f9775 | R3 | feat(cli): `verify-roundtrip` 子命令 | 新模块 `roundtrip.rs`（698 行/13 测试）：YOLO 源 → 临时项目 → no-mask 重导出 → `reconcile()` 纯函数逐框比对；严格解析，mismatch 非零退出 |
| a8fab84 | — | docs(readme): CLI 文档 | README 补 `--roi` / `--no-mask` / `--symlink` / `verify-roundtrip` 用法与 ROI 规则示例 |
| e175ea4 | — | refactor(utils): 拆出 `dataset_export.rs` | conversion.rs 达 944 行超 800 阈值，四个数据集驱动 verbatim 平移至 `dataset_export.rs`（605 行/9 测试），无行为变更 |
| a2c1ed0 | R6 | fix(cli): 默认 IoU 容差 0.001→0.01 | 亚像素小框过 6 位小数量化后 IoU 只能到 0.99x，旧默认 0.999 误报 missing+extra（n001 dataset_v3 实测 3/17,491）；新增 `DEFAULT_IOU_TOLERANCE` 单一数据源 + diff 条目 `tiny` 标记 |
| 1c3bd52 | R5 | feat(export): image-folder 按属性导出分类集 | 新模块 `image_folder_export.rs`（652 行/11 测试，TDD）：`export --format image-folder --property <name> [--crop <margin>]`，每对象裁切 bbox+margin 重编码 JPEG q95，按属性值名归入 `out/<value>/` 类目录；缺属性/special value → `unlabeled/` 桶（不静默丢弃）；两遍 driver（先全量校验后写文件，schema drift 零副作用失败）；sanitize 冲突/保留名 `unlabeled` 预检拒绝；`save_image_high_quality` 从 `export_image_yolo` 提取共享。开放项：`--split train:val` 未实现 |

## 改动面统计

- 11 个文件，**+2440 / −442**；4 个新模块均为扁平独立（符合 flat-modular 原则）：
  - `vlabel-utils/src/roundtrip.rs` — 698 行，13 测试（reconcile 纯函数 + 端到端）
  - `vlabel-utils/src/dataset_export.rs` — 605 行，9 测试（自 conversion.rs 平移）
  - `vlabel-utils/src/roi_inject.rs` — 218 行，8 测试（parse 与 IO 分离，可测性优先）
  - `vlabel-utils/src/image_folder_export.rs` — 652 行，11 测试（分类集导出，TDD）
- conversion.rs 944 → 378 行；`Format` 枚举增加 `PartialEq, Eq` derive（支撑 yolo-only 参数校验）
- GUI 仅一处调用点改动：传 `YoloExportOptions::default()`，行为不变
- README 同步更新（含 R6 容差口径说明、R5 image-folder 用法）

## 合并前重点过目项（建议 4 处）

1. **R6 默认容差放宽的影响面**（`roundtrip.rs:36` `DEFAULT_IOU_TOLERANCE = 0.01`）
   验证工具的默认通过线从 IoU≥0.999 放宽到 ≥0.99。若 verify-roundtrip 被用作数据完整性
   闸门，下游脚本/文档需知悉默认口径已变（严格模式需显式 `--iou-tolerance 0.001`）。
   另 `TINY_AREA = 0.002`（`roundtrip.rs:41`，≈28×28px@640）是启发式阈值，只影响 JSONL
   报告的 `tiny` 标注、不影响 pass/fail——确认这个量级符合预期。

2. **`--roi` 未匹配帧 warn 不失败**（`roi_inject.rs` `apply_to_label`）
   未命中规则的帧保留无 ROI 状态，导出时不做 letterbox masking——有可见计数 + `log::warn`
   但继续执行。介于 No Silent Degradation 边界上（可见但非 fail-fast），需确认这个取舍。
   另注意注入是**整体替换** `label.rois`，重复导入+注入会覆盖手工 ROI。

3. **`--symlink` 导出的非自包含性**（`dataset_export.rs` `link_or_copy`）
   symlink 指向 canonicalize 后的项目原图路径，导出目录不再自包含：移动/删除 project 即断链；
   且 masked 图仍是真实文件（symlink 混合形态）。若导出产物是最终训练集（通常要求自包含），
   symlink 应只用于中间态/省盘场景。非 unix 平台直接 bail（无降级，合规）。

4. **`dataset_export.rs` 拆分可快速过目**（e175ea4）
   verbatim 平移、无行为变更，重点只需确认三个调用点（bin/GUI/roundtrip）import 路径正确、
   conversion.rs 残留无孤儿引用（编译+测试已验证）。
5. **R5 `unlabeled/` 桶语义**（1c3bd52）
   缺 property 或 special value（如 occluded）的对象进 `unlabeled/` 而非丢弃——下游训练
   自行决定是否排除；若误将 unlabeled/ 当作一个真实类喂给分类器会造成标签噪声，使用方需知。
   开放项 `--split train:val` 未实现（设计稿内已声明）。

R5（properties-export，VLabel properties → ImageFolder 分类集导出）已在 1c3bd52 实现。

## 测试状态

`cargo test --workspace`（feature/convert-pipeline @ 1c3bd52）：**全绿，108 passed / 0 failed**

| Crate | 结果 |
|-------|------|
| vlabel-core | 10 passed |
| vlabel-gui | 19 passed |
| vlabel-utils（含新模块 roundtrip 13 / roi_inject 8 / dataset_export 9 / image_folder_export 11） | 79 passed |

R6 含双层测试：纯 `reconcile()` 用例（严格失败/默认通过/tiny 标记）+ 端到端复现
（全精度亚像素源 round-trip 到 IoU 0.998，落在噪声带 0.99..0.999 内）。
R5 为 TDD：11 个测试先写于 `todo!()` 桩（布局/分组、margin 数学+钳制、裁切几何按像素
颜色断言、unlabeled 桶、sanitize 冲突/保留名、schema-drift 先失败零写入、退化多边形跳过、
负 margin、项目根守卫）。
