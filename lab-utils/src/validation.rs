use lab_core::{
    find_category, find_prop_value, find_property_type, find_special_value, Label, LabelMeta,
};

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.is_valid = false;
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

/// Validate an annotation against metadata
pub fn validate_annotation(label: &Label, meta: &LabelMeta) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Check if annotation has objects
    if label.objects.is_empty() {
        result.add_warning("Annotation has no objects".to_string());
    }

    for (idx, obj) in label.objects.iter().enumerate() {
        // Validate category exists
        if find_category(meta, obj.category).is_none() {
            result.add_error(format!(
                "Object {} has invalid category ID: {}",
                idx, obj.category
            ));
        }

        // Validate polygon has at least 3 points
        if obj.polygon.0.len() < 3 {
            result.add_error(format!(
                "Object {} has invalid polygon with {} points (minimum 3 required)",
                idx,
                obj.polygon.0.len()
            ));
        }

        // Validate coordinates are normalized (0.0 to 1.0)
        for (point_idx, point) in obj.polygon.0.iter().enumerate() {
            if point.x < 0.0 || point.x > 1.0 {
                result.add_error(format!(
                    "Object {} point {} has invalid x coordinate: {} (must be 0.0-1.0)",
                    idx, point_idx, point.x
                ));
            }
            if point.y < 0.0 || point.y > 1.0 {
                result.add_error(format!(
                    "Object {} point {} has invalid y coordinate: {} (must be 0.0-1.0)",
                    idx, point_idx, point.y
                ));
            }
        }

        // Validate properties
        if let Some(category) = find_category(meta, obj.category) {
            for prop in &category.properties {
                // Find the matching PropertyEntry on this Object
                if let Some(entry) = obj.properties.iter().find(|e| e.id == prop.id) {
                    // Find the PropDef by name
                    if let Some(prop_def) = find_property_type(meta, prop.id) {
                        // Check if value exists in PropDef values or special values
                        if find_prop_value(prop_def, entry.value).is_none()
                            && find_special_value(meta, entry.value).is_none()
                        {
                            result.add_error(format!(
                                "Object {} has invalid property value: property={}, value={}",
                                idx, prop.id, entry.value
                            ));
                        }
                    }
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use lab_core::{new_label, new_object, Point, Polygon, RoiConfig, ShapeConfig};

    #[test]
    fn test_validate_empty_annotation() {
        let label = new_label("test");
        let meta = LabelMeta {
            id: 1,
            name: "test".to_string(),
            description: "test".to_string(),
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

        let result = validate_annotation(&label, &meta);
        assert!(result.is_valid);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_validate_invalid_category() {
        let mut label = new_label("test");
        let obj = new_object(
            0,
            999, // Invalid category
            Polygon::from(vec![
                Point { x: 0.0, y: 0.0 },
                Point { x: 1.0, y: 0.0 },
                Point { x: 0.5, y: 1.0 },
            ]),
        );
        lab_core::add_object(&mut label, obj);

        let meta = LabelMeta {
            id: 1,
            name: "test".to_string(),
            description: "test".to_string(),
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

        let result = validate_annotation(&label, &meta);
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }
}
