# vlabel-core

标注数据模型 + 导出引擎（YOLO / VOC / COCO）。无 GUI 依赖。

## 关键文件

- label.rs: 标注数据类型（Label / Object / PropertyEntry；2026-09 自 v3-types 搬入）
- label_meta.rs: 项目元数据类型（CatDef / PropDef / ValueDef / LocalizedNames 双语名）
- annotation.rs: Label/Object 构造与操作辅助函数
- export/coco.rs: COCO JSON 导出
- export/voc.rs: Pascal VOC XML 导出
- export/yolo.rs: YOLO txt 导出
- io.rs: 标注文件读写
- error.rs: 错误类型定义

## 测试

cargo test -p vlabel-core

## 规范

- 错误处理用 thiserror
- 导出格式必须通过单元测试验证
- 无 GUI 依赖，可独立使用
- 标注类型归本 crate；几何类型（Point/Polygon）仍在 v3-types（path 依赖）
