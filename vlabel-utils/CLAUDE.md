# vlabel-utils

工具库（类型转换、项目管理、数据验证）。无 GUI 依赖。

## 关键文件

- conversion.rs: 坐标格式转换（归一化 ↔ 像素）
- project.rs: 标注项目管理（meta.json5 读写）
- validation.rs: 数据验证

## 测试

cargo test -p vlabel-utils

## 规范

- 纯工具函数，无副作用
- 无 GUI 依赖
