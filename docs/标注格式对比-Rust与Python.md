# 标注格式对比与演进决策：Rust（VLabel）取代 Python（jxl）

> 存档日期：2026-09-07。
> 结论先行：**Rust 格式为今后唯一标注格式**（类型随收编计划从 `v3-types` 迁入本项目
> `vlabel-core`）；**Python 格式（jxl `A2dImageLabel`）废弃**，仅保留少量候选内容
> 按需提取补充到 Rust 格式（见 §7）。

> 2026-09-08 注：磁盘格式已由 YAML 迁至 JSON5（见 specs/2026-09-08-yaml-to-json5-design.md），本文其余 YAML 表述为历史记录。

## 1. 两代格式身份

| 维度 | Rust（现行，VLabel） | Python（退役，jxl） |
|------|----------------------|---------------------|
| 权威定义 | `v3-types/src/{label,label_meta,geometry}.rs`（收编后为 `vlabel-core`） | `py/jxl/src/jxl/label/{a2d/dd,meta,prop}.py` |
| 框架 | serde struct（纯数据） | pydantic BaseModel（数据+行为方法） |
| 磁盘格式 | **YAML**（`vlabels/<stem>.yaml`） | **JSON**（`<stem>.json`） |
| 项目布局 | `<project>/{meta.yaml, images/, vlabels/}` | `{format_name}_m<meta_id>/`（meta_id 进路径） |
| 坐标 | 归一化 f32 [0,1] | 归一化 NCS（绘制时 `points_ncs_trans_in_win` 转像素） |
| 消费者 | vlabel（唯一） | shop-track、jxl 转换器家族（darknet/coco/kitti/tile/hop/ias） |
| 分叉副本 | `v3-s2/engine/v3-types`（label.rs 同步；label_meta.rs 落后一版） | `py/jxl-s2`（dd.py 逐字节相同） |

两代坐标体系一致（均为归一化），数据层可互转；但**版本号同写 `2.0` 而 schema 有代差，
迁移不能只看版本号**。

## 2. Label（单图标注文件）逐字段

| 字段 | Rust `Label` | Python `A2dImageLabel` | 评注 |
|------|--------------|------------------------|------|
| version | `String "2.0"`（可承载 semver） | `float 2.0` | Rust 扩展性更好，保留 |
| user_agent | ✓ | ✓ | 同 |
| 创建时间 | `created_at: DateTime<Local>`（结构化） | `date: str`（ISO 字符串） | 字段名+类型双差 |
| last_modified | `DateTime<Local>` | `str` | 类型差 |
| host | ❌ | `str` | Python 独有，环境冗余 → 丢弃 |
| sensor | ❌ | `int` | Python 独有，传感器来源 → 丢弃 |
| ROI | `rois: Vec<Polygon>`（多 ROI，缺省空） | `roi: Points`（单 ROI，缺省全图 `ROI_FULL`） | 语义差最大：多 vs 单、空 vs 全图 |
| objects | `Vec<Object>` | `list[A2dObjectLabel]` | 同概念 |

## 3. Object（单个目标）逐字段

| 字段 | Rust `Object` | Python `A2dObjectLabel` | 评注 |
|------|---------------|--------------------------|------|
| id | `i32` | `int` | 同 |
| 类别+置信度 | `category: i32` + `confidence: f32`（扁平） | `prob_class: ProbValue(value, conf)`（打包） | 序列化形状不同；保留 Rust 扁平形 |
| polygon | `Polygon<f32>`（serde transparent → 点列表） | `Points`（点列表） | 几乎一致 |
| properties | `Vec<PropertyEntry{id, value, confidence}>`（列表） | `dict[prop_id → ProbValue(value, conf)]`（映射，字段名 `conf`） | 容器与字段名双差；保留 Rust 列表形 |

Python 哨兵值体系（`dd.py:24-32`、`prop.py:15-19`）：`ID_ERROR=-3 / ID_EXCLUDE=-2 /
ID_PENDING=-1 / CAT_PENDING=-1 / CONF_ERROR=-2.0 / CONF_EXCLUDE=-1.0`——把特殊语义编码
进值域；Rust 对应概念集中在 `meta.property_special_values`，类型层干净。

## 4. Meta（项目配置）逐字段

| Rust `LabelMeta` | Python `LabelMeta` | 处置 |
|------------------|---------------------|------|
| id / name / description | id / name / description | 同 |
| `shape: {title_style, thickness, auto_save, vertex_radius}` | `label: {title_style, thickness}` + 顶层 `auto_save` | 保留 Rust 结构 |
| `roi: {color}` | ❌ | Rust 独有，保留 |
| `categories: Vec<CatDef>`（hotkey；properties 带 id） | `categories: list[CatMeta]`（keys；properties 无 id + `filter: FilterCfg`） | 保留 Rust；filter 见 §7 |
| `property_types: Vec<PropDef{id,name,desc,values}>` | `properties: list[PropMeta>`（多 `size`/`border_extend`/`color`） | 保留 Rust；PropMeta 附加项见 §7 |
| `property_special_values`（=`ValueDef` 别名，SSOT 产物） | ❌ | Rust 独有，保留 |
| ❌ | `view_size` / `object_size` / `sample: SampleCfg` | Python 独有（UI+采样生成）→ 丢弃 |

`ValueDef` vs `ValueCfg`：基本同构（id/name/description/热键/color/sign），Python 多
`conf: float = 2.0` 默认置信度；热键字段名 `hotkey` vs `keys`。保留 Rust 形。

## 5. 设计哲学差异（保留 Rust 形态的理由）

- **Python 模型即工具箱**：`A2dImageLabel` 挂 `from_d2d/roi_rect/move/draw_on` 等行为方法；
  Rust 侧纯数据 + 操作函数分离——符合数据客观性 / Functional Core 原则，收编保持。
- **Python 配套生态更厚**：darknet/coco/kitti/tile/hop/ias/blend/extractor/viewer 转换器
  家族 + `jxl_label.py` CLI。VLabel 目前仅 YOLO/VOC/COCO/LabelMe——能力缺口按需移植，
  不随格式迁移。
- 同为 `2.0` 版本号但 schema 有代差，**任何旧数据迁移必须走显式字段映射，不可依赖版本号**。

## 6. Python 格式退役清单（确认无消费者后直接丢弃）

- `host`、`sensor`（环境冗余字段）
- `view_size`、`object_size`、`sample: SampleCfg`（标注器 UI 与采样生成配置）
- `date` 字符串时间（由 `created_at` 结构化时间取代）
- 单 `roi` 默认全图语义（全图 ROI 在 Rust 中以空 `rois` 表达）
- 哨兵常量编码进值域的方式（由 `meta.property_special_values` 取代）
- `prob_class` 打包形 / `conf` 字段名 / 属性 dict 容器

## 7. 候选提取项（可能少量补充到 Rust 格式）

按需评估，不默认采纳；采纳须走格式版本演进（更新 `version`）：

1. **属性默认置信度**（`ValueCfg.conf: float = 2.0`）——属性值级的默认置信度语义，
   若 VLabel 未来做半自动属性预标注则有用。
2. **目标过滤器**（`CatMeta.filter: FilterCfg`）——按几何条件过滤目标，导出/统计场景
   可能需要。
3. **属性分类器链配置**（`PropMeta.size` 输入尺寸、`border_extend` 边界扩展）——若
   VLabel 未来接属性分类器训练管线再提取，属工具链配置而非标注格式本体。
4. **jxl 转换器格式清单**（darknet/kitti/tile/hop/ias）——作为 VLabel 导入导出格式的
   需求参考，非 schema 内容。

## 8. 旧数据迁移备忘（若需要）

一次性脚本（JSON → YAML），显式字段映射：

```
date          → created_at（ISO 字符串 → DateTime<Local>）
prob_class    → category + confidence（拆包）
properties    → dict → Vec<PropertyEntry>（键即 id，conf → confidence）
roi（单）     → rois: Vec<Polygon>（ROI_FULL 全图 → 空 rois）
host/sensor   → 丢弃
```

迁移产物用 VLabel 打开抽检；脚本用后即弃，不写入代码兼容层。
