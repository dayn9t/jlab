use serde::{Deserialize, Serialize};

/// A 2D point with normalized coordinates (0.0 to 1.0)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Convert from pixel coordinates to normalized coordinates
    pub fn from_pixel(x: f32, y: f32, image_width: u32, image_height: u32) -> Self {
        Self {
            x: x / image_width as f32,
            y: y / image_height as f32,
        }
    }

    /// Convert to pixel coordinates
    pub fn to_pixel(&self, image_width: u32, image_height: u32) -> (f32, f32) {
        (self.x * image_width as f32, self.y * image_height as f32)
    }

    /// Calculate distance to another point
    pub fn distance_to(&self, other: &Point) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// A polygon defined by a sequence of points
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Polygon {
    pub points: Vec<Point>,
}

impl Polygon {
    pub fn new(points: Vec<Point>) -> Self {
        Self { points }
    }

    /// Create an empty polygon
    pub fn empty() -> Self {
        Self { points: Vec::new() }
    }

    /// Add a point to the polygon
    pub fn add_point(&mut self, point: Point) {
        self.points.push(point);
    }

    /// Check if the polygon is valid (at least 3 points)
    pub fn is_valid(&self) -> bool {
        self.points.len() >= 3
    }

    /// Calculate the area of the polygon using the shoelace formula
    pub fn area(&self) -> f32 {
        if self.points.len() < 3 {
            return 0.0;
        }

        let mut sum = 0.0;
        for i in 0..self.points.len() {
            let j = (i + 1) % self.points.len();
            sum += self.points[i].x * self.points[j].y;
            sum -= self.points[j].x * self.points[i].y;
        }

        (sum / 2.0).abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_distance() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);
        assert_eq!(p1.distance_to(&p2), 5.0);
    }

    #[test]
    fn test_polygon_area() {
        // Unit square
        let poly = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(0.0, 1.0),
        ]);
        assert!((poly.area() - 1.0).abs() < 0.001);
    }
}
