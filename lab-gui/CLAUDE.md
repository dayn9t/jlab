# lab-gui

egui GUI 应用。依赖 lab-core 和 lab-utils。

## 关键文件

- app.rs: 主应用逻辑、UI 布局
- app/panels.rs: 左右侧边栏面板
- app/toolbar.rs: 工具栏
- canvas.rs: 画布渲染（多边形绘制、顶点编辑）
- state.rs: EditMode 枚举（Normal / Editing）、应用状态
- geometry.rs: 几何计算（线段相交、多边形面积等）
- shortcuts.rs: 快捷键管理
- i18n.rs: 国际化（zh-CN / en-US）
- tools.rs: 工具定义

## 运行

cargo run --release -p lab-gui
cargo run --release -p lab-gui -- /path/to/project

## 测试

cargo test -p lab-gui

## 规范

- 错误处理用 anyhow
- 所有用户可见文本通过 i18n 模块管理
- 画布渲染与业务逻辑分离
