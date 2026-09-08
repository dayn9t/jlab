# 标注格式 YAML → JSON5 迁移设计

**日期**: 2026-09-08
**状态**: 已与用户逐节确认
**主题**: 磁盘标注格式从 YAML 全家迁移到 JSON5，保留显式迁移转换器，主读写路径彻底移除 serde_yaml

## 1. 背景与动机

现行磁盘格式：`meta.yaml` + `vlabels/<stem>.yaml` + `shortcuts.yaml`，由
`vlabel-core/src/io.rs` 经 `serde_yaml 0.9` 读写。迁往 JSON5 的动机（用户裁决）：

1. **手编辑友好** —— 注释、尾逗号、无引号 key；YAML 嵌套序列缩进
   （`rois: - - x:`）难读难改
2. **类型严格** —— 摆脱 YAML 隐式类型陷阱（yes/no/on/off→bool、裸字符串、
   `'2.0'` 引号问题）；JSON5 是 JSON 超集，类型可预期
3. **LLM/agent 友好** —— 标注文件会被 LLM/VLM 流水线读取生成，JSON 系格式
   token 效率与生成可靠性更高

附带维护性收益：serde_yaml 上游已归档停维护。格式消费面确认：原始格式仅
vlabel 自身消费，训练管线消费导出产物（YOLO/VOC/COCO/LabelMe），不受影响。

## 2. 设计决策（用户裁决记录）

| 决策点 | 裁决 | 否决的备选 |
|--------|------|-----------|
| 写出策略 | 真 JSON5 双向：读 `json5::from_str`，写 `json5::to_string`（多行缩进、key 无引号、尾逗号） | 宽容读 + serde_json 严格写（两个依赖、产出丢 JSON5 风味） |
| 扩展名 | `.json5`（诚实信号：注释合法） | `.json`（误导人以为不能写注释） |
| 迁移范围 | 全家：meta + vlabels + shortcuts（全局+项目两处） | 仅 meta+labels（shortcuts 会留下 serde_yaml 悬空依赖）、仅 labels（格式两半不一致） |
| serde_yaml 去留 | 从 vlabel-core/vlabel-gui/顶层 workspace 移除；降级为 vlabel-utils 私有依赖，仅供迁移子命令读旧文件 | 全仓库删除（与「保留迁移转换器」硬需求冲突） |
| version 字段 | 保持 `"2.0"`——schema 零变化，纯序列化替换，`.json5` 扩展名即时代标记 | 升版本号（无 schema 变化，不触发版本演进规则） |
| 迁移转换器 | 硬需求，长期保留：`vlabel-convert migrate-yaml` | 一次性用后即弃脚本（用户明确要求保留） |

JSON5 库选型：`json5` crate 1.3.1（活跃维护 fork，serde 双向，JSON5 专用中最快，
序列化输出即手编辑风格）。`v3-types` 类型层上游已有 serde_json round-trip 测试，
JSON 序列化路径已被验证。

## 3. 改动面

### 3.1 读写层（vlabel-core/src/io.rs）

`serde_yaml::{from_str, to_string}` → `json5::{from_str, to_string}`，四个函数
（load/save × meta/annotation）签名不变。读宽容：注释/尾逗号/无引号 key 均接受。
`error.rs` 的 serde_yaml 错误变体替换为 json5 变体。

### 3.2 依赖布局

- workspace 顶层移除 `serde_yaml`，新增 `json5`
- `vlabel-core` 依赖 `json5`（io）
- `vlabel-gui` 依赖 `json5`（shortcuts）
- `vlabel-utils` 依赖 `serde_yaml`（仅 migrate 路径）——转换器是显式兼容桥，
  非静默兼容层；主代码路径不读 YAML（No Compatibility Code）

### 3.3 路径推导

- `vlabel-utils/src/project.rs`：`meta.yaml`→`meta.json5`（:24）、
  `{}.yaml`→`{}.json5`（:43）；stem 碰撞守卫按 stem 工作，不受影响
- `vlabel-gui/src/shortcuts.rs`：全局配置目录与项目目录两处
  `shortcuts.yaml`→`shortcuts.json5`（:829、:854）

## 4. 迁移转换器

```
vlabel-convert migrate-yaml <project_dir> [--global]
```

- 覆盖：`meta.yaml`、`vlabels/*.yaml`、项目级 `shortcuts.yaml`；
  `--global` 额外迁移全局快捷键配置（路径解析复用 shortcuts.rs 逻辑）
- 逐文件算法：serde_yaml 严格解析 → `json5::to_string` 写出 `.json5` →
  **回读验证通过** → 删除旧 `.yaml`
- 守卫（沿袭 stem-collision guard 哲学）：
  - 目标 `.json5` 已存在 → 报错拒跑（不静默覆盖）
  - 任何解析/验证失败 → 带文件路径 fail-fast，不产出半迁移状态
  - 无可迁移文件 → 明确报「无可迁移」
- 结束打印迁移计数（meta/labels/shortcuts 分列）

## 5. 测试计划

- io.rs round-trip 测试改 `.json5`；新增「含注释+尾逗号的手改文件可解析」用例
- migrate-yaml 集成测试：YAML 项目 fixture → 迁移 → 新文件可解析 + 旧文件消失
  + 重复执行报「无可迁移」+ 目标已存在时报错
- shortcuts 读写 round-trip（json5）
- 已知褶皱（测试先行解决）：`assets/labels/0001.yaml` 的 `properties: {}` 是
  Python 时代 dict 残留，serde_yaml 能否解进 `Vec<PropertyEntry>` 待验证；
  不能则 fixture 先修为 `[]`

## 6. 存量与文档

- 仓库内 fixture（`assets/meta.yaml`、`assets/labels/0001.yaml`）同 commit 迁移；
  所有测试路径同步改（io.rs、project.rs、state_persistence.rs、conversion.rs、
  import.rs）
- README / CLAUDE.md / `docs/图像标注工具设计.md` 中格式描述更新
- `docs/标注格式对比-Rust与Python.md` 为存档不动，仅顶部加一行
  「2026-09-08 磁盘格式已迁至 JSON5」
- 用户存量数据集用 migrate-yaml 一次性迁移

## 7. 已知取舍

- json5 序列化器格式「fairly basic」：每个 Point 占约 4 行（YAML 为 2 行），
  文件略膨胀——先接受，真碍事再做自定义格式化（YAGNI）
- json5 产出非严格 JSON，Python `json.loads` 不能直接读——原始格式仅 vlabel
  消费，外部消费走导出格式，可接受
