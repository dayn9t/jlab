//! 标注数据类型。
//!
//! 从 v3-types 重新导出 Label、Object、PropertyEntry，
//! 并提供构造和操作辅助函数。

use chrono::Local;
pub use v3_types::label::{Label, Object, PropertyEntry};
pub use v3_types::{Point, Polygon};

/// 创建一个空的 Label。
pub fn new_label(user_agent: impl Into<String>) -> Label {
    let now = Local::now();
    Label {
        version: "2.0".to_string(),
        user_agent: user_agent.into(),
        created_at: now,
        last_modified: now,
        rois: Vec::new(),
        objects: Vec::new(),
    }
}

/// 向 Label 添加一个 Object，并更新 last_modified。
pub fn add_object(label: &mut Label, object: Object) {
    label.objects.push(object);
    label.last_modified = Local::now();
}

/// 按 ID 移除 Object。返回是否成功移除。
pub fn remove_object(label: &mut Label, id: i32) -> bool {
    let len_before = label.objects.len();
    label.objects.retain(|obj| obj.id != id);
    let removed = label.objects.len() < len_before;
    if removed {
        label.last_modified = Local::now();
    }
    removed
}

/// 按 ID 查找 Object 的不可变引用。
pub fn find_object(label: &Label, id: i32) -> Option<&Object> {
    label.objects.iter().find(|obj| obj.id == id)
}

/// 按 ID 查找 Object 的可变引用。
pub fn find_object_mut(label: &mut Label, id: i32) -> Option<&mut Object> {
    label.objects.iter_mut().find(|obj| obj.id == id)
}

/// 更新 last_modified 为当前时间。
pub fn touch(label: &mut Label) {
    label.last_modified = Local::now();
}

/// 返回下一个可用的 Object ID。
pub fn next_object_id(label: &Label) -> i32 {
    label.objects.iter().map(|obj| obj.id).max().map(|max_id| max_id + 1).unwrap_or(0)
}

/// 创建一个新 Object（便捷构造）。
pub fn new_object(id: i32, category: i32, polygon: Polygon<f32>) -> Object {
    Object { id, category, confidence: 1.0, polygon, properties: Vec::new() }
}

/// 为 Object 设置单值属性（查找并替换，或追加）。
pub fn set_object_property(object: &mut Object, property_id: i32, value: i32, confidence: f32) {
    if let Some(entry) = object.properties.iter_mut().find(|e| e.id == property_id) {
        entry.value = value;
        entry.confidence = confidence;
    } else {
        object.properties.push(PropertyEntry { id: property_id, value, confidence });
    }
}

/// 获取 Object 的属性值。
pub fn get_object_property(object: &Object, property_id: i32) -> Option<&PropertyEntry> {
    object.properties.iter().find(|e| e.id == property_id)
}

/// 移除 Object 的属性。
pub fn remove_object_property(object: &mut Object, property_id: i32) {
    object.properties.retain(|e| e.id != property_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annotation_creation() {
        let mut label = new_label("test-tool");
        assert_eq!(label.version, "2.0");
        assert_eq!(label.objects.len(), 0);

        let obj = new_object(
            0,
            1,
            Polygon::from(vec![
                Point { x: 0.0, y: 0.0 },
                Point { x: 1.0, y: 0.0 },
                Point { x: 1.0, y: 1.0 },
                Point { x: 0.0, y: 1.0 },
            ]),
        );

        add_object(&mut label, obj);
        assert_eq!(label.objects.len(), 1);
    }

    #[test]
    fn test_object_properties() {
        let mut obj = new_object(0, 1, Polygon::empty());
        set_object_property(&mut obj, 0, 5, 1.0);

        let prop = get_object_property(&obj, 0).unwrap();
        assert_eq!(prop.value, 5);
        assert!((prop.confidence - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_next_object_id() {
        let mut label = new_label("test");
        assert_eq!(next_object_id(&label), 0);

        add_object(&mut label, new_object(0, 1, Polygon::empty()));
        assert_eq!(next_object_id(&label), 1);

        add_object(&mut label, new_object(5, 1, Polygon::empty()));
        assert_eq!(next_object_id(&label), 6);
    }

    #[test]
    fn test_rois_deserialize_single() {
        let yaml = r#"
version: "2.0"
user_agent: "test"
created_at: "2024-01-01T00:00:00+08:00"
last_modified: "2024-01-01T00:00:00+08:00"
rois:
  - - {x: 0.1, y: 0.1}
    - {x: 0.2, y: 0.1}
    - {x: 0.2, y: 0.2}
objects: []
"#;

        let label: Label = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(label.rois.len(), 1);
        assert_eq!(label.rois[0].0.len(), 3);
    }

    #[test]
    fn test_rois_deserialize_multiple() {
        let yaml = r#"
version: "2.0"
user_agent: "test"
created_at: "2024-01-01T00:00:00+08:00"
last_modified: "2024-01-01T00:00:00+08:00"
rois:
  - - {x: 0.1, y: 0.1}
    - {x: 0.2, y: 0.1}
    - {x: 0.2, y: 0.2}
  - - {x: 0.4, y: 0.4}
    - {x: 0.5, y: 0.4}
    - {x: 0.5, y: 0.5}
objects: []
"#;

        let label: Label = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(label.rois.len(), 2);
        assert_eq!(label.rois[0].0.len(), 3);
    }
}
