use lab_core::Point;

/// Check if a point is inside a polygon using ray casting algorithm
pub fn point_in_polygon(point: &Point, polygon: &[Point]) -> bool {
    if polygon.len() < 3 {
        return false;
    }

    let mut inside = false;
    let mut j = polygon.len() - 1;

    for i in 0..polygon.len() {
        let xi = polygon[i].x;
        let yi = polygon[i].y;
        let xj = polygon[j].x;
        let yj = polygon[j].y;

        let intersect = ((yi > point.y) != (yj > point.y))
            && (point.x < (xj - xi) * (point.y - yi) / (yj - yi) + xi);

        if intersect {
            inside = !inside;
        }

        j = i;
    }

    inside
}

/// Calculate distance from point to line segment
pub fn point_to_segment_distance(point: &Point, p1: &Point, p2: &Point) -> f32 {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;

    if dx == 0.0 && dy == 0.0 {
        // p1 and p2 are the same point
        let dx = point.x - p1.x;
        let dy = point.y - p1.y;
        return (dx * dx + dy * dy).sqrt();
    }

    // Calculate the t parameter
    let t = ((point.x - p1.x) * dx + (point.y - p1.y) * dy) / (dx * dx + dy * dy);
    let t = t.clamp(0.0, 1.0);

    // Calculate the closest point on the segment
    let closest_x = p1.x + t * dx;
    let closest_y = p1.y + t * dy;

    // Calculate distance
    let dx = point.x - closest_x;
    let dy = point.y - closest_y;
    (dx * dx + dy * dy).sqrt()
}

/// Calculate bounding box of a polygon
pub fn bounding_box(polygon: &[Point]) -> Option<(Point, Point)> {
    if polygon.is_empty() {
        return None;
    }

    let mut min_x = polygon[0].x;
    let mut min_y = polygon[0].y;
    let mut max_x = polygon[0].x;
    let mut max_y = polygon[0].y;

    for point in polygon.iter().skip(1) {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }

    Some((Point::new(min_x, min_y), Point::new(max_x, max_y)))
}

fn orientation(a: &Point, b: &Point, c: &Point) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn on_segment(a: &Point, b: &Point, c: &Point) -> bool {
    let min_x = a.x.min(b.x);
    let max_x = a.x.max(b.x);
    let min_y = a.y.min(b.y);
    let max_y = a.y.max(b.y);
    let eps = 1e-6;

    c.x >= min_x - eps && c.x <= max_x + eps && c.y >= min_y - eps && c.y <= max_y + eps
}

fn segments_intersect(a1: &Point, a2: &Point, b1: &Point, b2: &Point) -> bool {
    let o1 = orientation(a1, a2, b1);
    let o2 = orientation(a1, a2, b2);
    let o3 = orientation(b1, b2, a1);
    let o4 = orientation(b1, b2, a2);
    let eps = 1e-6;

    if o1.abs() < eps && on_segment(a1, a2, b1) {
        return true;
    }
    if o2.abs() < eps && on_segment(a1, a2, b2) {
        return true;
    }
    if o3.abs() < eps && on_segment(b1, b2, a1) {
        return true;
    }
    if o4.abs() < eps && on_segment(b1, b2, a2) {
        return true;
    }

    (o1 > eps && o2 < -eps || o1 < -eps && o2 > eps)
        && (o3 > eps && o4 < -eps || o3 < -eps && o4 > eps)
}

pub fn fix_self_intersections(points: &mut Vec<Point>) -> bool {
    if points.len() < 4 {
        return false;
    }

    let mut updated = false;
    let mut iterations = 0;
    let max_iterations = points.len() * points.len();

    loop {
        let count = points.len();
        let mut fixed = false;

        for i in 0..count {
            let a1 = points[i];
            let a2 = points[(i + 1) % count];

            for j in (i + 2)..count {
                if i == 0 && j == count - 1 {
                    continue;
                }

                let b1 = points[j];
                let b2 = points[(j + 1) % count];

                if a1 == b1 || a1 == b2 || a2 == b1 || a2 == b2 {
                    continue;
                }

                if segments_intersect(&a1, &a2, &b1, &b2) {
                    points[i + 1..=j].reverse();
                    updated = true;
                    fixed = true;
                    break;
                }
            }

            if fixed {
                break;
            }
        }

        if !fixed {
            break;
        }

        iterations += 1;
        if iterations > max_iterations {
            break;
        }
    }

    updated
}

#[cfg(test)]
mod tests {
    use super::*;

    fn polygon_has_self_intersections(points: &[Point]) -> bool {
        if points.len() < 4 {
            return false;
        }

        let count = points.len();
        for i in 0..count {
            let a1 = &points[i];
            let a2 = &points[(i + 1) % count];

            for j in (i + 2)..count {
                if i == 0 && j == count - 1 {
                    continue;
                }

                let b1 = &points[j];
                let b2 = &points[(j + 1) % count];
                if segments_intersect(a1, a2, b1, b2) {
                    return true;
                }
            }
        }

        false
    }

    #[test]
    fn test_point_in_polygon() {
        let polygon = vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(0.0, 1.0),
        ];

        assert!(point_in_polygon(&Point::new(0.5, 0.5), &polygon));
        assert!(!point_in_polygon(&Point::new(1.5, 0.5), &polygon));
    }

    #[test]
    fn test_point_to_segment_distance() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(1.0, 0.0);
        let point = Point::new(0.5, 0.5);

        let distance = point_to_segment_distance(&point, &p1, &p2);
        assert!((distance - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_bounding_box() {
        let polygon = vec![
            Point::new(0.2, 0.3),
            Point::new(0.8, 0.3),
            Point::new(0.8, 0.7),
            Point::new(0.2, 0.7),
        ];

        let (min, max) = bounding_box(&polygon).unwrap();
        assert_eq!(min.x, 0.2);
        assert_eq!(min.y, 0.3);
        assert_eq!(max.x, 0.8);
        assert_eq!(max.y, 0.7);
    }

    #[test]
    fn test_fix_self_intersections() {
        let mut polygon = vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(0.0, 1.0),
            Point::new(1.0, 0.0),
        ];

        assert!(polygon_has_self_intersections(&polygon));
        assert!(fix_self_intersections(&mut polygon));
        assert!(!polygon_has_self_intersections(&polygon));
    }
}
