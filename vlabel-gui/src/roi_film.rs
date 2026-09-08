//! ROI 外蓝色膜：JXL `make_roi_surround_color` 的 Rust 复刻。
//!
//! 语义：`rois` 为空 → 不加膜（全图即 ROI）；非空 → 所有 ROI 并集之外的
//! 像素替换为蓝色单色（亮度 = 原像素灰度，R=G=0，B=gray），ROI 内保持原彩。
//! 纯函数，烘焙进显示纹理（egui painter 不支持多边形镂空填充）。

use vlabel_core::Polygon;

/// 对 RGBA 像素缓冲应用 ROI 膜。像素判定坐标取像素中心
/// ((ix+0.5)/w, (iy+0.5)/h)，与归一化 ROI 多边形（[0,1]）对齐。
pub fn apply_roi_film(pixels: &mut [u8], width: usize, height: usize, rois: &[Polygon<f32>]) {
    if rois.is_empty() || width == 0 || height == 0 {
        return;
    }
    for iy in 0..height {
        let cy = (iy as f32 + 0.5) / height as f32;
        for ix in 0..width {
            let cx = (ix as f32 + 0.5) / width as f32;
            if rois.iter().any(|roi| point_in_polygon(cx, cy, roi)) {
                continue;
            }
            let i = (iy * width + ix) * 4;
            let gray = luma(pixels[i], pixels[i + 1], pixels[i + 2]);
            pixels[i] = 0;
            pixels[i + 1] = 0;
            pixels[i + 2] = gray;
        }
    }
}

/// 偶奇规则射线法：点是否在多边形内（边界归属由浮点比较自然决定，
/// 调用方应避免依赖恰好落在边上的采样点）。
fn point_in_polygon(px: f32, py: f32, poly: &Polygon<f32>) -> bool {
    let pts = &poly.0;
    if pts.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let (xi, yi) = (pts[i].x, pts[i].y);
        let (xj, yj) = (pts[j].x, pts[j].y);
        if (yi > py) != (yj > py) && px < (xj - xi) * (py - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// BT.601 亮度。
fn luma(r: u8, g: u8, b: u8) -> u8 {
    (0.299_f32 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use vlabel_core::Point;

    /// 4×4 全 (200, 100, 50) 的测试图
    fn solid_image() -> Vec<u8> {
        let mut v = Vec::with_capacity(4 * 4 * 4);
        for _ in 0..4 * 4 {
            v.extend_from_slice(&[200, 100, 50, 255]);
        }
        v
    }

    /// 亮度：0.299R + 0.587G + 0.114B，四舍五入
    fn expect_gray() -> u8 {
        (0.299_f32 * 200.0 + 0.587 * 100.0 + 0.114 * 50.0).round() as u8 // = 124
    }

    fn rect_roi(x0: f32, y0: f32, x1: f32, y1: f32) -> Polygon<f32> {
        Polygon::from(vec![
            Point { x: x0, y: y0 },
            Point { x: x1, y: y0 },
            Point { x: x1, y: y1 },
            Point { x: x0, y: y1 },
        ])
    }

    fn px(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
        let i = (y * 4 + x) * 4;
        [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
    }

    #[test]
    fn empty_rois_is_no_op() {
        let mut pixels = solid_image();
        apply_roi_film(&mut pixels, 4, 4, &[]);
        assert_eq!(pixels, solid_image());
    }

    #[test]
    fn rect_roi_films_outside_keeps_inside() {
        let mut pixels = solid_image();
        // 中心 2×2 像素（像素中心 0.125~0.875，ROI 覆盖 0.25~0.75 → 内侧为 (1,1)(2,1)(1,2)(2,2)）
        let rois = [rect_roi(0.25, 0.25, 0.75, 0.75)];
        apply_roi_film(&mut pixels, 4, 4, &rois);

        for y in 0..4 {
            for x in 0..4 {
                let p = px(&pixels, x, y);
                if (1..=2).contains(&x) && (1..=2).contains(&y) {
                    assert_eq!(p, [200, 100, 50, 255], "inside pixel ({x},{y}) must stay original");
                } else {
                    assert_eq!(
                        p,
                        [0, 0, expect_gray(), 255],
                        "outside pixel ({x},{y}) must be blue monochrome"
                    );
                }
            }
        }
    }

    #[test]
    fn multiple_rois_union_keeps_both() {
        let mut pixels = solid_image();
        // 左半 + 右半 → 全图都在并集内，无膜
        let rois = [rect_roi(0.0, 0.0, 0.5, 1.0), rect_roi(0.5, 0.0, 1.0, 1.0)];
        apply_roi_film(&mut pixels, 4, 4, &rois);
        assert_eq!(pixels, solid_image());
    }

    #[test]
    fn triangle_roi_diagonal_edge() {
        let mut pixels = solid_image();
        // 三角 (0,0),(1,0),(0,1)：内侧 = x+y<1 的像素中心
        let rois = [Polygon::from(vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 0.0, y: 1.0 },
        ])];
        apply_roi_film(&mut pixels, 4, 4, &rois);

        // 像素中心 (0.125,0.125)~：sum=x+y
        let inside = [(0usize, 0usize), (1, 0), (0, 1), (1, 1)]; // sum 0.25/0.5/0.5/0.75
        let outside = [(2, 2), (3, 3), (3, 2), (2, 3)]; // sum 1.25/1.75/1.5/1.5
        for (x, y) in inside {
            assert_eq!(px(&pixels, x, y), [200, 100, 50, 255], "({x},{y}) inside triangle");
        }
        for (x, y) in outside {
            assert_eq!(px(&pixels, x, y), [0, 0, expect_gray(), 255], "({x},{y}) outside triangle");
        }
    }
}
