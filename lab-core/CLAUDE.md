# lab-core

标注数据模型 + 导出引擎（YOLO / VOC / COCO）。无 GUI 依赖。

## 关键文件

- annotation.rs: Annotation 数据模型（点集、类别、属性）
- export/coco.rs: COCO JSON 导出
- export/voc.rs: Pascal VOC XML 导出
- export/yolo.rs: YOLO txt 导出
- io.rs: 标注文件读写
- error.rs: 错误类型定义

## 测试

cargo test -p lab-core

## 规范

- 错误处理用 thiserror
- 导出格式必须通过单元测试验证
- 无 GUI 依赖，可独立使用
