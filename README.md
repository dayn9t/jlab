# JLab

2D 目标检测与属性分类的图像标注工具。

## 功能

- 多边形标注（目标与 ROI）
- 三种模式：浏览、绘制、编辑
- 支持 YOLO、Pascal VOC、COCO JSON 导出
- 中英文界面
- 可配置快捷键

## 构建

```bash
cargo build --release
```

## 运行

```bash
# 直接启动
cargo run --release -p lab-gui

# 打开项目目录
cargo run --release -p lab-gui -- /path/to/project
```

## 项目结构

```
label_root/
├── meta.yaml    # 项目配置（类别、属性）
├── images/      # 图像文件
└── labels/      # 标注文件
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

## 许可证

MIT OR Apache-2.0
