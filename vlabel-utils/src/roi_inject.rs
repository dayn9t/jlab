//! 批量 ROI 注入：按文件名前缀把 ROI 多边形写进导入的标注。
//!
//! 服务于固定机位多相机流水线（如 n001 crop640）：同一相机的所有帧共享
//! 同一个 ROI，逐帧手画不可行。规则从 JSON5 文件解析（`--roi <file>`）。

use anyhow::Context;
use std::path::Path;
use vlabel_core::{Label, Point, Polygon};

/// Parsed `--roi` rules: file-stem prefix → ROI polygons (normalized coords).
///
/// The JSON5 file maps a key to a list of polygons; each polygon is a list of
/// `[x, y]` points in normalized [0,1] coordinates. One quad ROI for camera-1
/// frames (`1_*.jpg` stems) and a full-frame ROI for camera 2:
///
/// ```json5
/// {
///   "1": [[[0, 0.0012], [1, 0.0012], [1, 0.745], [0, 0.838]]],
///   "2": [[[0, 0], [1, 0], [1, 1], [0, 1]]],
/// }
/// ```
///
/// A key matches every file whose stem *starts with* the key, so a full stem
/// doubles as a per-frame rule. Multiple matching keys are an error — the
/// assignment would be ambiguous; merge or split the rules instead.
#[derive(Debug, Clone, Default)]
pub struct RoiRules {
    rules: Vec<RoiRule>,
}

#[derive(Debug, Clone)]
struct RoiRule {
    key: String,
    polygons: Vec<Polygon<f32>>,
}

impl RoiRules {
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Number of rules (keys) loaded.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Parse rules from a JSON5 file on disk.
    pub fn parse_file(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read ROI rules file {}", path.display()))?;
        Self::parse_str(&content)
            .with_context(|| format!("invalid ROI rules in {}", path.display()))
    }

    /// Parse rules from JSON5 text (kept separate from the file IO for tests).
    fn parse_str(content: &str) -> anyhow::Result<Self> {
        // BTreeMap: duplicate keys are rejected by the deserializer, and the
        // sorted iteration makes ambiguity errors deterministic.
        let raw: std::collections::BTreeMap<String, Vec<Vec<[f32; 2]>>> = json5::from_str(content)?;
        let mut rules = Vec::with_capacity(raw.len());
        for (key, polygons) in raw {
            let mut parsed = Vec::with_capacity(polygons.len());
            for (poly_idx, points) in polygons.iter().enumerate() {
                if points.len() < 3 {
                    anyhow::bail!(
                        "rule {key:?} polygon {poly_idx} has {} point(s); at least 3 required",
                        points.len()
                    );
                }
                for (point_idx, &[x, y]) in points.iter().enumerate() {
                    if !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
                        anyhow::bail!(
                            "rule {key:?} polygon {poly_idx} point {point_idx} is ({x}, {y}); \
                             ROI coordinates must be normalized [0,1]"
                        );
                    }
                }
                parsed.push(Polygon::from(
                    points.iter().map(|&[x, y]| Point { x, y }).collect::<Vec<_>>(),
                ));
            }
            if parsed.is_empty() {
                anyhow::bail!("rule {key:?} has no polygons");
            }
            rules.push(RoiRule { key, polygons: parsed });
        }
        Ok(RoiRules { rules })
    }

    /// ROI polygons for a file name, matched by stem prefix.
    ///
    /// * `Ok(None)` — no rule matches (the frame keeps its existing ROIs; for
    ///   a fresh import that is none).
    /// * `Err` — more than one rule matches: ambiguous, refuse to guess.
    fn rois_for_file(&self, file_name: &str) -> anyhow::Result<Option<&[Polygon<f32>]>> {
        let stem = crate::import::image_stem(file_name);
        let mut matched = self.rules.iter().filter(|rule| stem.starts_with(&rule.key));
        let first = match matched.next() {
            Some(rule) => rule,
            None => return Ok(None),
        };
        if let Some(second) = matched.next() {
            anyhow::bail!(
                "ROI rules are ambiguous for {file_name}: keys {:?} and {:?} both match; \
                 merge or split the rules",
                first.key,
                second.key
            );
        }
        Ok(Some(&first.polygons))
    }

    /// Inject ROIs into one annotation, replacing any existing ROIs.
    /// Returns whether a rule matched.
    pub fn apply_to_label(&self, label: &mut Label, file_name: &str) -> anyhow::Result<bool> {
        match self.rois_for_file(file_name)? {
            Some(polygons) => {
                label.rois = polygons.to_vec();
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAM_RULES: &str = r#"
    {
      "1": [[[0, 0.0012], [1, 0.0012], [1, 0.745], [0, 0.838]]],
      "2": [[[0, 0], [1, 0], [1, 1], [0, 1]]],
    }
    "#;

    fn label() -> Label {
        vlabel_core::new_label("test")
    }

    #[test]
    fn parses_and_matches_by_stem_prefix() {
        let rules = RoiRules::parse_str(CAM_RULES).unwrap();
        assert_eq!(rules.len(), 2);

        let mut cam1 = label();
        assert!(rules.apply_to_label(&mut cam1, "1_0_2026-06-22_08-00-03.000_00000.jpg").unwrap());
        assert_eq!(cam1.rois.len(), 1);
        assert_eq!(cam1.rois[0].0.len(), 4);
        assert_eq!(cam1.rois[0].0[0], Point { x: 0.0, y: 0.0012 });
        assert_eq!(cam1.rois[0].0[3], Point { x: 0.0, y: 0.838 });

        let mut cam2 = label();
        assert!(rules.apply_to_label(&mut cam2, "2_0_frame.jpg").unwrap());
        assert_eq!(cam2.rois[0].0[2], Point { x: 1.0, y: 1.0 });
    }

    #[test]
    fn unmatched_file_keeps_existing_rois() {
        let rules = RoiRules::parse_str(CAM_RULES).unwrap();
        let mut other = label();
        other.rois = vec![Polygon::from(vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
        ])];
        assert!(!rules.apply_to_label(&mut other, "9_frame.jpg").unwrap());
        assert_eq!(other.rois.len(), 1, "no rule matched: existing ROIs stay untouched");
    }

    #[test]
    fn full_stem_key_gives_per_frame_rule() {
        let rules =
            RoiRules::parse_str(r#"{ "only_this_frame": [[[0, 0], [0.5, 0], [0.5, 0.5]]] }"#)
                .unwrap();
        let mut hit = label();
        assert!(rules.apply_to_label(&mut hit, "only_this_frame.png").unwrap());
        // Prefix semantics: only_this_frame2 would still match (longer stem,
        // shared prefix) — a truly different stem does not.
        let mut miss = label();
        assert!(!rules.apply_to_label(&mut miss, "other_frame.png").unwrap());
    }

    #[test]
    fn ambiguous_rules_are_rejected() {
        let rules =
            RoiRules::parse_str(r#"{ "1": [[[0,0],[1,0],[1,1]]], "1_0": [[[0,0],[1,0],[1,1]]] }"#)
                .unwrap();
        let err = rules.apply_to_label(&mut label(), "1_0_x.jpg").unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("ambiguous"), "message was: {msg}");
        assert!(msg.contains("\"1\"") && msg.contains("\"1_0\""), "message was: {msg}");
    }

    #[test]
    fn rejects_short_polygon() {
        let err = RoiRules::parse_str(r#"{ "1": [[[0,0],[1,1]]] }"#).unwrap_err();
        assert!(format!("{err:#}").contains("at least 3 required"), "message was: {err:#}");
    }

    #[test]
    fn rejects_out_of_range_coordinates() {
        let err = RoiRules::parse_str(r#"{ "1": [[[0,0],[1.5,0],[1,1]]] }"#).unwrap_err();
        assert!(format!("{err:#}").contains("normalized [0,1]"), "message was: {err:#}");
    }

    #[test]
    fn rejects_rule_without_polygons() {
        let err = RoiRules::parse_str(r#"{ "1": [] }"#).unwrap_err();
        assert!(format!("{err:#}").contains("no polygons"), "message was: {err:#}");
    }

    #[test]
    fn parse_file_reports_missing_file() {
        let err = RoiRules::parse_file(Path::new("/nonexistent/roi.json5")).unwrap_err();
        assert!(format!("{err:#}").contains("failed to read"), "message was: {err:#}");
    }
}
