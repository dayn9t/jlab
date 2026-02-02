use serde::{Deserialize, Serialize};

/// Main metadata structure for an annotation project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub id: i32,
    pub name: String,
    pub description: String,

    /// Shape configuration
    pub shape: ShapeConfig,

    /// ROI (Region of Interest) configuration
    pub roi: RoiConfig,

    /// Object categories
    pub categories: Vec<Category>,

    /// Property type definitions
    pub property_types: Vec<PropertyType>,

    /// Special property values (error, excluded, pending, etc.)
    pub property_special_values: Vec<PropertySpecialValue>,
}

impl Meta {
    /// Find a category by ID
    pub fn find_category(&self, id: i32) -> Option<&Category> {
        self.categories.iter().find(|c| c.id == id)
    }

    /// Find a property type by ID
    pub fn find_property_type(&self, id: i32) -> Option<&PropertyType> {
        self.property_types.iter().find(|pt| pt.id == id)
    }

    /// Find a special property value by ID
    pub fn find_special_value(&self, id: i32) -> Option<&PropertySpecialValue> {
        self.property_special_values.iter().find(|sv| sv.id == id)
    }
}

/// Shape display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeConfig {
    /// Title style (0, 1, 2, etc.)
    pub title_style: i32,

    /// Line thickness for drawing shapes
    pub thickness: i32,

    /// Auto-save when switching images
    #[serde(default = "default_auto_save")]
    pub auto_save: bool,

    /// Vertex detection radius in pixels
    #[serde(default = "default_vertex_radius")]
    pub vertex_radius: f32,
}

fn default_auto_save() -> bool {
    true
}

fn default_vertex_radius() -> f32 {
    10.0
}

/// ROI (Region of Interest) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoiConfig {
    /// Color for ROI display (hex format like "#800080")
    pub color: String,
}

/// Object category definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub description: String,

    /// Keyboard shortcut for quick selection
    pub hotkey: String,

    /// Display color (hex format like "#FF0000")
    pub color: String,

    /// Properties associated with this category
    #[serde(default)]
    pub properties: Vec<CategoryProperty>,
}

/// Property reference in a category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryProperty {
    pub id: i32,
    pub name: String,

    /// Property type name (references PropertyType)
    #[serde(rename = "type")]
    pub property_type: String,
}

/// Property type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyType {
    pub id: i32,
    pub name: String,
    pub description: String,

    /// Possible values for this property type
    pub values: Vec<PropertyValue>,
}

impl PropertyType {
    /// Find a property value by ID
    pub fn find_value(&self, id: i32) -> Option<&PropertyValue> {
        self.values.iter().find(|v| v.id == id)
    }
}

/// A possible value for a property type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyValue {
    pub id: i32,
    pub name: String,
    pub description: String,

    /// Keyboard shortcut
    pub hotkey: String,

    /// Display color
    pub color: String,

    /// Short sign for display on image (e.g., "M" for male, "F" for female)
    pub sign: String,
}

/// Special property values (error, excluded, pending, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySpecialValue {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub hotkey: String,
    pub color: String,
    pub sign: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_serialization() {
        let meta = Meta {
            id: 1,
            name: "Test Project".to_string(),
            description: "Test description".to_string(),
            shape: ShapeConfig {
                title_style: 1,
                thickness: 2,
                auto_save: true,
                vertex_radius: 10.0,
            },
            roi: RoiConfig {
                color: "#800080".to_string(),
            },
            categories: vec![],
            property_types: vec![],
            property_special_values: vec![],
        };

        let yaml = serde_yaml::to_string(&meta).unwrap();
        let deserialized: Meta = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(meta.id, deserialized.id);
        assert_eq!(meta.name, deserialized.name);
    }
}
