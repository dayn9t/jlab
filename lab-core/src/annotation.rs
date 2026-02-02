use crate::geometry::{Point, Polygon};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Annotation data for a single image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    /// Format version
    pub version: String,

    /// User agent that created this annotation
    pub user_agent: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last modification timestamp
    pub last_modified: DateTime<Utc>,

    /// Region of Interest polygons
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rois: Vec<Vec<Point>>,

    /// Annotated objects
    pub objects: Vec<Object>,
}

impl Annotation {
    /// Create a new empty annotation
    pub fn new(user_agent: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            version: "2.0".to_string(),
            user_agent: user_agent.into(),
            created_at: now,
            last_modified: now,
            rois: Vec::new(),
            objects: Vec::new(),
        }
    }

    /// Add an object to the annotation
    pub fn add_object(&mut self, object: Object) {
        self.objects.push(object);
        self.last_modified = Utc::now();
    }

    /// Remove an object by ID
    pub fn remove_object(&mut self, id: i32) -> bool {
        let len_before = self.objects.len();
        self.objects.retain(|obj| obj.id != id);
        let removed = self.objects.len() < len_before;
        if removed {
            self.last_modified = Utc::now();
        }
        removed
    }

    /// Find an object by ID
    pub fn find_object(&self, id: i32) -> Option<&Object> {
        self.objects.iter().find(|obj| obj.id == id)
    }

    /// Find an object by ID (mutable)
    pub fn find_object_mut(&mut self, id: i32) -> Option<&mut Object> {
        self.objects.iter_mut().find(|obj| obj.id == id)
    }

    /// Update the last modified timestamp
    pub fn touch(&mut self) {
        self.last_modified = Utc::now();
    }

    /// Get the next available object ID
    pub fn next_object_id(&self) -> i32 {
        self.objects
            .iter()
            .map(|obj| obj.id)
            .max()
            .map(|max_id| max_id + 1)
            .unwrap_or(0)
    }
}

/// An annotated object in an image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    /// Unique ID within this annotation
    pub id: i32,

    /// Category ID (references Meta.categories)
    pub category: i32,

    /// Confidence score (0.0 to 1.0), typically 1.0 for manual annotations
    pub confidence: f32,

    /// Object shape as a polygon
    pub polygon: Vec<Point>,

    /// Object properties (property_id -> list of values with confidence)
    #[serde(default)]
    pub properties: HashMap<String, Vec<PropertyValueWithConfidence>>,
}

impl Object {
    /// Create a new object
    pub fn new(id: i32, category: i32, polygon: Vec<Point>) -> Self {
        Self {
            id,
            category,
            confidence: 1.0,
            polygon,
            properties: HashMap::new(),
        }
    }

    /// Set a property value
    pub fn set_property(&mut self, property_id: i32, value: i32, confidence: f32) {
        let key = property_id.to_string();
        self.properties
            .insert(key, vec![PropertyValueWithConfidence { value, confidence }]);
    }

    /// Get a property value
    pub fn get_property(&self, property_id: i32) -> Option<&PropertyValueWithConfidence> {
        let key = property_id.to_string();
        self.properties.get(&key).and_then(|values| values.first())
    }

    /// Remove a property
    pub fn remove_property(&mut self, property_id: i32) {
        let key = property_id.to_string();
        self.properties.remove(&key);
    }

    /// Convert polygon to Polygon type
    pub fn as_polygon(&self) -> Polygon {
        Polygon::new(self.polygon.clone())
    }
}

/// Property value with confidence score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyValueWithConfidence {
    /// Property value ID
    pub value: i32,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn test_annotation_creation() {
        let mut annotation = Annotation::new("test-tool");
        assert_eq!(annotation.version, "2.0");
        assert_eq!(annotation.objects.len(), 0);

        let obj = Object::new(
            0,
            1,
            vec![
                Point::new(0.0, 0.0),
                Point::new(1.0, 0.0),
                Point::new(1.0, 1.0),
                Point::new(0.0, 1.0),
            ],
        );

        annotation.add_object(obj);
        assert_eq!(annotation.objects.len(), 1);
    }

    #[test]
    fn test_object_properties() {
        let mut obj = Object::new(0, 1, vec![]);
        obj.set_property(0, 5, 1.0);

        let prop = obj.get_property(0).unwrap();
        assert_eq!(prop.value, 5);
        assert_eq!(prop.confidence, 1.0);
    }

    #[test]
    fn test_next_object_id() {
        let mut annotation = Annotation::new("test");
        assert_eq!(annotation.next_object_id(), 0);

        annotation.add_object(Object::new(0, 1, vec![]));
        assert_eq!(annotation.next_object_id(), 1);

        annotation.add_object(Object::new(5, 1, vec![]));
        assert_eq!(annotation.next_object_id(), 6);
    }

    #[test]
    fn test_rois_deserialize_single() {
        let yaml = r#"
version: "2.0"
user_agent: "test"
created_at: "2024-01-01T00:00:00Z"
last_modified: "2024-01-01T00:00:00Z"
rois:
  - - {x: 0.1, y: 0.1}
    - {x: 0.2, y: 0.1}
    - {x: 0.2, y: 0.2}
objects: []
"#;

        let annotation: Annotation = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(annotation.rois.len(), 1);
        assert_eq!(annotation.rois[0].len(), 3);
    }

    #[test]
    fn test_rois_deserialize_multiple() {
        let yaml = r#"
version: "2.0"
user_agent: "test"
created_at: "2024-01-01T00:00:00Z"
last_modified: "2024-01-01T00:00:00Z"
rois:
  - - {x: 0.1, y: 0.1}
    - {x: 0.2, y: 0.1}
    - {x: 0.2, y: 0.2}
  - - {x: 0.4, y: 0.4}
    - {x: 0.5, y: 0.4}
    - {x: 0.5, y: 0.5}
objects: []
"#;

        let annotation: Annotation = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(annotation.rois.len(), 2);
        assert_eq!(annotation.rois[0].len(), 3);
    }
}
