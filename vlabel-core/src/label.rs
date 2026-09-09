//! 标注数据类型。
//!
//! 2026-09 自 v3-types 搬入本项目（标注类型归 vlabel；几何类型
//! Point/Polygon 仍留 v3-types，本文件跨 crate 引用）。

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use v3_types::geometry::Polygon;

/// 属性条目（单值）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyEntry {
    /// 属性 ID。
    pub id: i32,
    /// 属性值 ID。
    pub value: i32,
    /// 置信度 [0, 1]。
    pub confidence: f32,
}

/// 标注目标。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Object {
    /// 目标 ID。
    pub id: i32,
    /// 类别 ID。
    pub category: i32,
    /// 置信度 [0, 1]。
    pub confidence: f32,
    /// 目标多边形。
    pub polygon: Polygon<f32>,
    /// 附加属性列表。
    #[serde(default)]
    pub properties: Vec<PropertyEntry>,
}

/// 标注文件（单个样本）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Label {
    /// 协议版本（String 支持 semver）。
    pub version: String,
    /// 创建工具标识。
    pub user_agent: String,
    /// 创建时间。
    pub created_at: DateTime<Local>,
    /// 最后修改时间。
    pub last_modified: DateTime<Local>,
    /// 感兴趣区域（多个，可为空）。
    #[serde(default)]
    pub rois: Vec<Polygon<f32>>,
    /// 标注目标列表。
    pub objects: Vec<Object>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use v3_types::geometry::Point;

    #[test]
    fn property_entry_roundtrip() {
        let entry = PropertyEntry { id: 1, value: 2, confidence: 0.9 };
        let json = serde_json::to_string(&entry).unwrap();
        let back: PropertyEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back, entry);
    }

    #[test]
    fn property_entry_json_keys() {
        let entry = PropertyEntry { id: 5, value: 10, confidence: 0.75 };
        let val = serde_json::to_value(&entry).unwrap();
        assert_eq!(val["id"], 5);
        assert_eq!(val["value"], 10);
        assert_eq!(val["confidence"], 0.75);
    }

    #[test]
    fn object_roundtrip() {
        let obj = Object {
            id: 1,
            category: 0,
            confidence: 1.0,
            polygon: Polygon::from(vec![
                Point { x: 0.1, y: 0.2 },
                Point { x: 0.3, y: 0.2 },
                Point { x: 0.3, y: 0.4 },
            ]),
            properties: vec![PropertyEntry { id: 1, value: 2, confidence: 1.0 }],
        };
        let json = serde_json::to_string(&obj).unwrap();
        let back: Object = serde_json::from_str(&json).unwrap();
        assert_eq!(back, obj);
    }

    #[test]
    fn object_default_properties() {
        let json = r#"{"id":1,"category":0,"confidence":1.0,"polygon":[{"x":0.0,"y":0.0}]}"#;
        let obj: Object = serde_json::from_str(json).unwrap();
        assert!(obj.properties.is_empty());
    }

    #[test]
    fn label_full_roundtrip() {
        let label = Label {
            version: "1.0.0".to_string(),
            user_agent: "vlabel".to_string(),
            created_at: Local::now(),
            last_modified: Local::now(),
            rois: vec![Polygon::from(vec![
                Point { x: 0.0, y: 0.0 },
                Point { x: 1.0, y: 0.0 },
                Point { x: 1.0, y: 1.0 },
                Point { x: 0.0, y: 1.0 },
            ])],
            objects: vec![Object {
                id: 1,
                category: 0,
                confidence: 1.0,
                polygon: Polygon::from(vec![Point { x: 0.1, y: 0.2 }, Point { x: 0.3, y: 0.4 }]),
                properties: vec![],
            }],
        };
        let json = serde_json::to_string_pretty(&label).unwrap();
        let back: Label = serde_json::from_str(&json).unwrap();
        assert_eq!(back.version, label.version);
        assert_eq!(back.user_agent, label.user_agent);
        assert_eq!(back.objects, label.objects);
        assert_eq!(back.rois, label.rois);
    }

    #[test]
    fn label_minimal_json() {
        let json = r#"{
            "version": "1.0",
            "user_agent": "test",
            "created_at": "2024-01-01T00:00:00+08:00",
            "last_modified": "2024-01-01T00:00:00+08:00",
            "objects": []
        }"#;
        let label: Label = serde_json::from_str(json).unwrap();
        assert!(label.rois.is_empty());
        assert!(label.objects.is_empty());
    }

    #[test]
    fn label_datetime_json_format() {
        // 验证 DateTime<Local> 序列化为 ISO 8601 字符串
        let label = Label {
            version: "1.0".to_string(),
            user_agent: "test".to_string(),
            created_at: Local::now(),
            last_modified: Local::now(),
            rois: vec![],
            objects: vec![],
        };
        let val = serde_json::to_value(&label).unwrap();
        // created_at 和 last_modified 应为字符串（ISO 8601）
        assert!(val["created_at"].is_string());
        assert!(val["last_modified"].is_string());
    }
}
