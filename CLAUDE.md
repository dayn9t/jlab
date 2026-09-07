# VLabel — 2D 图像标注工具

Rust workspace (3 crates): vlabel-core（标注数据模型 + 导出）、vlabel-gui（egui GUI）、vlabel-utils（工具库）

## 术语表

| 术语 | 含义 | 非此含义 |
|------|------|---------|
| Annotation | 多边形标注数据（点集 + 类别 + 属性），存储为 YAML | ≠ Label（标签文本） |
| ROI | Region of Interest，感兴趣区域标注 | ≠ 普通目标标注 |
| Mode | 编辑模式（Normal / Editing），决定画布行为 | ≠ 编辑器模式 |
| Project | 标注项目 = meta.yaml + images/ + vlabels/ | ≠ Cargo 项目 |
| Export | 将标注导出为 YOLO / VOC / COCO 格式 | ≠ 序列化 |

## 构建 & 测试

```bash
cargo build --release          # 构建全部
cargo run --release -p vlabel-gui # 启动 GUI
cargo test --workspace         # 运行全部测试
cargo clippy --workspace       # lint
cargo fmt --check              # 格式检查
```

## 设计目标

1. **标注效率** — 快捷键优先，减少鼠标操作
2. **格式兼容** — 支持 YOLO / VOC / COCO 三种导出
3. **轻量原生** — Rust + egui，无 Electron，启动快

## 设计原则

- 不可变数据优先：标注操作创建新状态，不修改原状态
- workspace 分层：vlabel-core 无 GUI 依赖，vlabel-gui 只做展示
- i18n：所有用户可见文本通过 i18n 模块管理

## 仓库规范

- 分支: feature/xxx, fix/xxx
- 提交: conventional commits（feat/fix/refactor/docs/test/chore）
- PR base: master
