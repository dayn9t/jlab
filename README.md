# VLabel

2D 目标检测与属性分类的图像标注工具。

## 功能

- 多边形标注（目标与 ROI）
- 三种模式：浏览、绘制、编辑
- 支持 YOLO、Pascal VOC、COCO JSON 导出
- 中英文界面
- 可配置快捷键
- 主题管理（深色/浅色/跟随系统）
- 外观设置（字体大小、界面缩放）
- 选项对话框统一管理所有设置

## 构建

```bash
cargo build --release
```

## 运行

```bash
# 直接启动
cargo run --release -p vlabel-gui

# 打开项目目录
cargo run --release -p vlabel-gui -- /path/to/project
```

### 格式转换 CLI（非交互）

```bash
# 导入（YOLO 源 = images/ + labels/）；--roi 按 stem 前缀批量注入 ROI
cargo run --release -p vlabel-utils --bin vlabel-convert -- import --format yolo [--roi roi.json5] <src_root> <project_dir>

# 导出；--no-mask / --symlink 仅 yolo 生效
cargo run --release -p vlabel-utils --bin vlabel-convert -- export --format yolo [--no-mask] [--symlink] <project_dir> <out_dir>
cargo run --release -p vlabel-utils --bin vlabel-convert -- export --format coco <project_dir> <out_dir>

# round-trip 对账：import → export（no-mask）→ 与源逐框比对（IoU 容差），差异可落 jsonl
cargo run --release -p vlabel-utils --bin vlabel-convert -- verify-roundtrip [--iou-tolerance 0.001] [--report diffs.jsonl] <yolo_src_root>
```

`--roi` 规则文件（JSON5，key = 文件名 stem 前缀，坐标归一化 [0,1]）：

```json5
{
  "1": [[[0, 0.0012], [1, 0.0012], [1, 0.745], [0, 0.838]]],  // cam1 四边形 ROI
  "2": [[[0, 0], [1, 0], [1, 1], [0, 1]]],                     // cam2 全图
}
```

## 项目结构

```
label_root/
├── meta.json5   # 项目配置（类别、属性）
├── images/      # 图像文件
└── vlabels/     # 标注文件
```

## 快捷键

| 操作 | 快捷键 |
|------|--------|
| 上/下一张 | A / D |
| 前进/后退 10 张 | W / S |
| 保存 | Ctrl+S |
| 模式切换 | Shift+1/2/3 |
| 完成绘制 | Space / 双击 |
| 删除 | Del |
| 取消 | Esc |
| 缩放 | 滚轮 |
| 平移 | Ctrl+拖动 / 中键 |
| 适应画布 | F |
| 缩放到 100% | Z |
| 切换左侧栏 | (可自定义) |
| 切换右侧栏 | (可自定义) |
| 切换自动保存 | (可自定义) |
| 移动对象 | 方向键 / Shift+方向键 (编辑模式) |
| 缩放对象 | +/- (编辑模式) |
| 变成矩形 | R (编辑模式) |
| 自交修正 | B (编辑模式) |

## 选项对话框

通过**文件菜单 → 选项**打开选项对话框，包含两个标签页：

### 常规标签
- **语言**：中文 / English
- **外观设置**：
  - 字体大小 (10-30)
  - 界面缩放 (0.5x-2.0x)
  - 主题（深色/浅色/跟随系统）
  - 显示滚动条
- **项目设置**：
  - 自动保存开关
- **恢复默认值**按钮

### 快捷键标签
- 全局搜索框
- 类别筛选（全部/文件/编辑/模式/导航/视图/工具）
- 快捷键编辑与冲突检测
- 重置默认值

## 配置文件

用户配置保存在 `~/.config/vlabel/`：

- `ui_settings.json` - 字体、缩放、滚动条等外观设置
- `theme.json` - 主题设置
- `auto_save.json` - 自动保存设置
- `shortcuts.json5` - 快捷键配置

## 许可证

MIT OR Apache-2.0
