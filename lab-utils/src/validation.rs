use lab_core::{Annotation, Meta};

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
pub fn validate_annotation(annotation: &Annotation, meta: &Meta) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Check if annotation has objects
    if annotation.objects.is_empty() {
        result.add_warning("Annotation has no objects".to_string());
    }

    for (idx, obj) in annotation.objects.iter().enumerate() {
        // Validate category exists
        if meta.find_category(obj.category).is_none() {
            result.add_error(format!(
                "Object {} has invalid category ID: {}",
                idx, obj.category
            ));
        }

        // Validate polygon has at least 3 points
        if obj.polygon.len() < 3 {
            result.add_error(format!(
                "Object {} has invalid polygon with {} points (minimum 3 required)",
                idx,
                obj.polygon.len()
            ));
        }

        // Validate coordinates are normalized (0.0 to 1.0)
        for (point_idx, point) in obj.polygon.iter().enumerate() {
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
        if let Some(category) = meta.find_category(obj.category) {
            for prop in &category.properties {
                let prop_key = prop.id.to_string();

                if let Some(values) = obj.properties.get(&prop_key) {
                    // Find the property type
                    if let Some(prop_type) = meta
                        .property_types
                        .iter()
                        .find(|pt| pt.name == prop.property_type)
                    {
                        for value in values {
                            // Check if value exists in property type
                            if prop_type.find_value(value.value).is_none()
                                && !meta
                                    .property_special_values
                                    .iter()
                                    .any(|sv| sv.id == value.value)
                            {
                                result.add_error(format!(
                                    "Object {} has invalid property value: property={}, value={}",
                                    idx, prop.id, value.value
                                ));
                            }
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
    use lab_core::annotation::Object;
    use lab_core::geometry::Point;

    #[test]
    fn test_validate_empty_annotation() {
        let annotation = Annotation::new("test");
        let meta = lab_core::Meta {
            id: 1,
            name: "test".to_string(),
            description: "test".to_string(),
            shape: lab_core::meta::ShapeConfig {
                title_style: 1,
                thickness: 2,
                auto_save: true,
                vertex_radius: 10.0,
            },
            roi: lab_core::meta::RoiConfig {
                color: "#800080".to_string(),
            },
            categories: vec![],
            property_types: vec![],
            property_special_values: vec![],
        };

        let result = validate_annotation(&annotation, &meta);
        assert!(result.is_valid);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_validate_invalid_category() {
        let mut annotation = Annotation::new("test");
        let obj = Object::new(
            0,
            999, // Invalid category
            vec![
                Point::new(0.0, 0.0),
                Point::new(1.0, 0.0),
                Point::new(0.5, 1.0),
            ],
        );
        annotation.add_object(obj);

        let meta = lab_core::Meta {
            id: 1,
            name: "test".to_string(),
            description: "test".to_string(),
            shape: lab_core::meta::ShapeConfig {
                title_style: 1,
                thickness: 2,
                auto_save: true,
                vertex_radius: 10.0,
            },
            roi: lab_core::meta::RoiConfig {
                color: "#800080".to_string(),
            },
            categories: vec![],
            property_types: vec![],
            property_special_values: vec![],
        };

        let result = validate_annotation(&annotation, &meta);
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }
}
