//! ROI masking: paint uniform gray over pixels outside all ROI polygons.

use image::GenericImageView;
use vlabel_core::{Point, Polygon};

/// YOLO letterbox gray (RGB 114,114,114) applied to masked-out pixels.
/// Same value as `jvi.drawing.color.YOLO_GRAY` / ultralytics yolov5.
pub const MASK_GRAY: u8 = 114;

/// Paint every pixel outside all ROI polygons with uniform gray.
///
/// ROI coordinates are normalized ([0,1] x [0,1]); a pixel is kept when its
/// center lies inside any ROI polygon. With no ROIs every pixel is outside,
/// so the whole image is painted gray.
pub fn mask_outside_rois(img: &image::DynamicImage, rois: &[Polygon<f32>]) -> image::DynamicImage {
    let (width, height) = img.dimensions();
    let mut rgba = img.to_rgba8();
    let buf = rgba.as_mut();

    // Per-ROI bounding boxes in normalized coords (prefilter for the ray cast)
    let bboxes: Vec<(f32, f32, f32, f32)> = rois
        .iter()
        .map(|roi| {
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            for p in &roi.0 {
                min_x = min_x.min(p.x);
                min_y = min_y.min(p.y);
                max_x = max_x.max(p.x);
                max_y = max_y.max(p.y);
            }
            (min_x, min_y, max_x, max_y)
        })
        .collect();

    for y in 0..height {
        // Pixel center in normalized coordinates
        let ny = (y as f32 + 0.5) / height as f32;
        for x in 0..width {
            let nx = (x as f32 + 0.5) / width as f32;
            let keep = rois.iter().zip(&bboxes).any(|(roi, &(min_x, min_y, max_x, max_y))| {
                nx >= min_x
                    && nx <= max_x
                    && ny >= min_y
                    && ny <= max_y
                    && point_in_polygon(nx, ny, &roi.0)
            });
            if !keep {
                let i = ((y * width + x) * 4) as usize;
                buf[i] = MASK_GRAY;
                buf[i + 1] = MASK_GRAY;
                buf[i + 2] = MASK_GRAY;
            }
        }
    }

    image::DynamicImage::ImageRgba8(rgba)
}

/// Ray-casting point-in-polygon test. Polygons with fewer than 3 vertices
/// contain nothing.
fn point_in_polygon(px: f32, py: f32, poly: &[Point<f32>]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (pi, pj) = (&poly[i], &poly[j]);
        if (pi.y > py) != (pj.y > py) && px < (pj.x - pi.x) * (py - pi.y) / (pj.y - pi.y) + pi.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    const RED: Rgba<u8> = Rgba([200, 10, 10, 255]);

    fn solid_image(w: u32, h: u32) -> image::DynamicImage {
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(w, h, RED))
    }

    fn poly(coords: &[(f32, f32)]) -> Polygon<f32> {
        Polygon::from(coords.iter().map(|&(x, y)| Point { x, y }).collect::<Vec<_>>())
    }

    fn left_half_roi() -> Polygon<f32> {
        poly(&[(0.0, 0.0), (0.5, 0.0), (0.5, 1.0), (0.0, 1.0)])
    }

    #[test]
    fn keeps_inside_and_paints_outside_uniform_gray() {
        // 4x4 image: pixel centers x = .125 .375 (inside left half), .625 .875 (outside)
        let img = solid_image(4, 4);
        let masked = mask_outside_rois(&img, &[left_half_roi()]);

        assert_eq!(masked.get_pixel(0, 0), RED); // inside
        assert_eq!(masked.get_pixel(1, 3), RED); // inside
        let gray = Rgba([MASK_GRAY, MASK_GRAY, MASK_GRAY, 255]);
        assert_eq!(masked.get_pixel(2, 0), gray); // outside
        assert_eq!(masked.get_pixel(3, 3), gray); // outside
    }

    #[test]
    fn pixel_inside_any_roi_is_kept() {
        let img = solid_image(4, 4);
        // Two thin vertical ROIs: left quarter and right quarter
        let rois = vec![
            poly(&[(0.0, 0.0), (0.25, 0.0), (0.25, 1.0), (0.0, 1.0)]),
            poly(&[(0.75, 0.0), (1.0, 0.0), (1.0, 1.0), (0.75, 1.0)]),
        ];
        let masked = mask_outside_rois(&img, &rois);

        assert_eq!(masked.get_pixel(0, 0), RED); // x=.125 in first ROI
        assert_eq!(masked.get_pixel(3, 2), RED); // x=.875 in second ROI
        let gray = Rgba([MASK_GRAY, MASK_GRAY, MASK_GRAY, 255]);
        assert_eq!(masked.get_pixel(1, 0), gray); // x=.375 outside both
        assert_eq!(masked.get_pixel(2, 0), gray); // x=.625 outside both
    }

    #[test]
    fn no_rois_paints_everything() {
        let img = solid_image(4, 4);
        let masked = mask_outside_rois(&img, &[]);

        let gray = Rgba([MASK_GRAY, MASK_GRAY, MASK_GRAY, 255]);
        assert_eq!(masked.get_pixel(0, 0), gray);
        assert_eq!(masked.get_pixel(3, 3), gray);
    }

    #[test]
    fn degenerate_polygon_contains_nothing() {
        let img = solid_image(4, 4);
        let masked = mask_outside_rois(&img, &[poly(&[(0.0, 0.0), (1.0, 1.0)])]);

        let gray = Rgba([MASK_GRAY, MASK_GRAY, MASK_GRAY, 255]);
        assert_eq!(masked.get_pixel(0, 0), gray);
        assert_eq!(masked.get_pixel(3, 3), gray);
    }
}
