//! 标注项目元数据类型。

use serde::{Deserialize, Serialize};

/// 形状显示配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeConfig {
    /// 标题样式。
    pub title_style: i32,
    /// 线宽。
    pub thickness: i32,
    /// 是否自动保存。
    #[serde(default = "default_true")]
    pub auto_save: bool,
    /// 顶点半径。
    #[serde(default = "default_vertex_radius")]
    pub vertex_radius: f32,
}

fn default_true() -> bool {
    true
}

fn default_vertex_radius() -> f32 {
    10.0
}

/// ROI 配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoiConfig {
    /// 显示颜色。
    pub color: String,
}

/// 本地化名称：`en` 与 `zh` 平级，无主次。
///
/// `en` 兼任导出/互操作标识（YOLO classes、数据集类目录等面向机器的
/// 锚点，必须存在）；`zh` 为中文显示名（GUI zh-CN 模式显示，缺失时
/// 回退 `en`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalizedNames {
    /// 英文规范名（导出名）。
    pub en: String,
    /// 中文显示名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zh: Option<String>,
}

impl LocalizedNames {
    /// 仅英文名（zh 缺失，显示回退 en）。
    pub fn en(name: impl Into<String>) -> Self {
        Self { en: name.into(), zh: None }
    }
}

/// 属性值定义。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueDef {
    /// 值 ID。
    pub id: i32,
    /// 本地化名称。
    pub names: LocalizedNames,
    /// 描述。
    pub description: String,
    /// 快捷键。
    pub hotkey: String,
    /// 显示颜色。
    pub color: String,
    /// 标记符号。
    pub sign: String,
}

/// 属性类型定义。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropDef {
    /// 属性 ID。
    pub id: i32,
    /// 本地化名称。
    pub names: LocalizedNames,
    /// 描述。
    pub description: String,
    /// 可选值列表。
    pub values: Vec<ValueDef>,
}

/// 类别属性引用。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatProperty {
    /// 属性 ID。
    pub id: i32,
    /// 本地化名称。
    pub names: LocalizedNames,
    /// 属性类型。
    #[serde(rename = "type")]
    pub property_type: String,
}

/// 类别定义。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatDef {
    /// 类别 ID。
    pub id: i32,
    /// 本地化名称。
    pub names: LocalizedNames,
    /// 描述。
    pub description: String,
    /// 快捷键。
    pub hotkey: String,
    /// 显示颜色。
    pub color: String,
    /// 关联的属性列表。
    #[serde(default)]
    pub properties: Vec<CatProperty>,
}

/// 特殊值定义（标注协议历史名称）。
///
/// 历史上 `SpecialValue` 与 `ValueDef` 是字段完全同构的两个独立 struct
/// （id/name/description/hotkey/color/sign，类型全同），属
/// single-source-of-truth 债务（j-design-principles #8）。合并为 `ValueDef`
/// 的类型别名：保留 `SpecialValue` 名字以兼容外部消费者与
/// `LabelMeta.property_special_values` 字段的语义标签，但内存布局、serde
/// 形态、trait impl 全部复用 `ValueDef`。
pub type SpecialValue = ValueDef;

/// 标注项目元数据。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabelMeta {
    /// 项目 ID。
    pub id: i32,
    /// 项目名称。
    pub name: String,
    /// 描述。
    pub description: String,
    /// 形状配置。
    pub shape: ShapeConfig,
    /// ROI 配置。
    pub roi: RoiConfig,
    /// 类别定义列表。
    pub categories: Vec<CatDef>,
    /// 属性类型列表。
    pub property_types: Vec<PropDef>,
    /// 特殊值列表。
    #[serde(default)]
    pub property_special_values: Vec<SpecialValue>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn shape_config_defaults() {
        let json = r#"{"title_style": 0, "thickness": 2}"#;
        let sc: ShapeConfig = serde_json::from_str(json).unwrap();
        assert!(sc.auto_save);
        assert!((sc.vertex_radius - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn shape_config_roundtrip() {
        let sc = ShapeConfig { title_style: 1, thickness: 3, auto_save: false, vertex_radius: 5.0 };
        let json = serde_json::to_string(&sc).unwrap();
        let back: ShapeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, sc);
    }

    #[test]
    fn roi_config_roundtrip() {
        let roi = RoiConfig { color: "#00FF00".to_string() };
        let json = serde_json::to_string(&roi).unwrap();
        let back: RoiConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, roi);
    }

    #[test]
    fn local_names_roundtrip_with_zh() {
        let names = LocalizedNames { en: "trash".to_string(), zh: Some("垃圾桶".to_string()) };
        let json = serde_json::to_string(&names).unwrap();
        assert!(json.contains("垃圾桶"), "zh must serialize: {json}");
        let back: LocalizedNames = serde_json::from_str(&json).unwrap();
        assert_eq!(back, names);
    }

    #[test]
    fn local_names_zh_skipped_when_none() {
        let names = LocalizedNames::en("dry");
        let json = serde_json::to_string(&names).unwrap();
        assert!(!json.contains("zh"), "None zh must not serialize: {json}");
        let back: LocalizedNames = serde_json::from_str(&json).unwrap();
        assert_eq!(back, names);
    }

    #[test]
    fn local_names_zh_defaults_to_none() {
        let back: LocalizedNames = serde_json::from_str(r#"{"en": "dry"}"#).unwrap();
        assert_eq!(back, LocalizedNames::en("dry"));
    }

    #[test]
    fn cat_property_roundtrip() {
        let cp = CatProperty {
            id: 1,
            names: LocalizedNames::en("sort"),
            property_type: "single_select".to_string(),
        };
        let json = serde_json::to_string(&cp).unwrap();
        let val = serde_json::to_value(&cp).unwrap();
        // serde rename: 字段名应为 "type" 而非 "property_type"
        assert!(val.get("type").is_some());
        assert!(val.get("property_type").is_none());
        let back: CatProperty = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cp);
    }

    #[test]
    fn value_def_roundtrip() {
        let vd = ValueDef {
            id: 0,
            names: LocalizedNames { en: "dry".to_string(), zh: Some("干燥".to_string()) },
            description: "干燥状态".to_string(),
            hotkey: "q".to_string(),
            color: "#00FF00".to_string(),
            sign: "D".to_string(),
        };
        let json = serde_json::to_string(&vd).unwrap();
        let back: ValueDef = serde_json::from_str(&json).unwrap();
        assert_eq!(back, vd);
    }

    #[test]
    fn prop_def_roundtrip() {
        let pd = PropDef {
            id: 0,
            names: LocalizedNames::en("sort"),
            description: "分类属性".to_string(),
            values: vec![ValueDef {
                id: 0,
                names: LocalizedNames::en("dry"),
                description: "干燥".to_string(),
                hotkey: "q".to_string(),
                color: "#00FF00".to_string(),
                sign: "D".to_string(),
            }],
        };
        let json = serde_json::to_string(&pd).unwrap();
        let back: PropDef = serde_json::from_str(&json).unwrap();
        assert_eq!(back, pd);
    }

    #[test]
    fn cat_def_roundtrip() {
        let cat = CatDef {
            id: 0,
            names: LocalizedNames::en("opening"),
            description: "洞口".to_string(),
            hotkey: "1".to_string(),
            color: "#FF0000".to_string(),
            properties: vec![CatProperty {
                id: 1,
                names: LocalizedNames::en("sort"),
                property_type: "single_select".to_string(),
            }],
        };
        let json = serde_json::to_string(&cat).unwrap();
        let back: CatDef = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cat);
    }

    #[test]
    fn cat_def_default_properties() {
        let json = r##"{"id":0,"names":{"en":"test"},"description":"测试","hotkey":"1","color":"#FF0000"}"##;
        let cat: CatDef = serde_json::from_str(json).unwrap();
        assert!(cat.properties.is_empty());
    }

    #[test]
    fn special_value_roundtrip() {
        let sv = SpecialValue {
            id: -1,
            names: LocalizedNames::en("occluded"),
            description: "被遮挡".to_string(),
            hotkey: "x".to_string(),
            color: "#888888".to_string(),
            sign: "O".to_string(),
        };
        let json = serde_json::to_string(&sv).unwrap();
        let back: SpecialValue = serde_json::from_str(&json).unwrap();
        assert_eq!(back, sv);
    }

    #[test]
    fn label_meta_full_roundtrip() {
        let meta = LabelMeta {
            id: 1,
            name: "shtm-detect".to_string(),
            description: "水害检测项目".to_string(),
            shape: ShapeConfig {
                title_style: 0,
                thickness: 2,
                auto_save: true,
                vertex_radius: 10.0,
            },
            roi: RoiConfig { color: "#0000FF".to_string() },
            categories: vec![CatDef {
                id: 0,
                names: LocalizedNames { en: "opening".to_string(), zh: Some("洞口".to_string()) },
                description: "洞口".to_string(),
                hotkey: "1".to_string(),
                color: "#FF0000".to_string(),
                properties: vec![CatProperty {
                    id: 1,
                    names: LocalizedNames::en("sort"),
                    property_type: "single_select".to_string(),
                }],
            }],
            property_types: vec![PropDef {
                id: 0,
                names: LocalizedNames::en("sort"),
                description: "分类".to_string(),
                values: vec![ValueDef {
                    id: 0,
                    names: LocalizedNames::en("dry"),
                    description: "干燥".to_string(),
                    hotkey: "q".to_string(),
                    color: "#00FF00".to_string(),
                    sign: "D".to_string(),
                }],
            }],
            property_special_values: vec![SpecialValue {
                id: -1,
                names: LocalizedNames::en("occluded"),
                description: "被遮挡".to_string(),
                hotkey: "x".to_string(),
                color: "#888888".to_string(),
                sign: "O".to_string(),
            }],
        };
        let json = serde_json::to_string_pretty(&meta).unwrap();
        let back: LabelMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(back, meta);
    }

    #[test]
    fn label_meta_default_special_values() {
        let json = r##"{
            "id": 1,
            "name": "test",
            "description": "测试",
            "shape": {"title_style": 0, "thickness": 2},
            "roi": {"color": "#000000"},
            "categories": [],
            "property_types": []
        }"##;
        let meta: LabelMeta = serde_json::from_str(json).unwrap();
        assert!(meta.property_special_values.is_empty());
    }
}
