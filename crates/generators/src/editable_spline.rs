//! The editable spline object `forge.spline` (docs/specs/SPLINES.md): points, Bezier
//! handles, segments, closed flags and the interpolation type, plus Make Editable for the
//! spline primitives. All the data rules are pure functions on [`SplineData`].

use std::collections::BTreeMap;

use forge_core::{Category, NodeType, ParamDesc, Value};
use forge_geometry::spline::{Spline, SplineKind};
use glam::DVec3;

use crate::spline_sampling::{Interpolation, Sampling, add_params};
use crate::splines::Curve;
use crate::{BuildError, Params};

pub const TYPE: &str = "forge.spline";

/// The reference app's public tangent read returns this fraction of the geometric Bezier
/// offset (`spline-tools/editable-circle`: 83 for a handle of 110.667; SPLINES.md §1.1).
pub const TANGENT_READBACK: f64 = 0.75;

/// A handle as the reference app's public tangent read would report it.
pub fn tangent_readback(handle: DVec3) -> DVec3 {
    handle * TANGENT_READBACK
}

/// The interpolation type of an editable spline (`type` param).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplineType {
    Linear,
    Cubic,
    Akima,
    BSpline,
    Bezier,
}

impl SplineType {
    pub const ALL: [SplineType; 5] = [
        SplineType::Linear,
        SplineType::Cubic,
        SplineType::Akima,
        SplineType::BSpline,
        SplineType::Bezier,
    ];

    pub fn id(self) -> &'static str {
        match self {
            SplineType::Linear => "linear",
            SplineType::Cubic => "cubic",
            SplineType::Akima => "akima",
            SplineType::BSpline => "bspline",
            SplineType::Bezier => "bezier",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SplineType::Linear => "Linear",
            SplineType::Cubic => "Cubic",
            SplineType::Akima => "Akima",
            SplineType::BSpline => "B-Spline",
            SplineType::Bezier => "Bezier",
        }
    }

    pub fn from_id(id: &str) -> Option<SplineType> {
        Self::ALL.into_iter().find(|t| t.id() == id)
    }
}

pub(crate) fn node_type() -> NodeType {
    let options: Vec<(&str, &str)> = SplineType::ALL
        .iter()
        .map(|t| (t.id(), t.label()))
        .collect();
    add_params(
        NodeType::new(TYPE, "Spline", Category::Generator)
            .param(ParamDesc::vec3_list("points", "Points"))
            .param(ParamDesc::vec3_list("left", "Left tangents"))
            .param(ParamDesc::vec3_list("right", "Right tangents"))
            .param(ParamDesc::index_list("segment_ends", "Segment ends"))
            .param(ParamDesc::index_list("closed_segments", "Closed segments"))
            .param(ParamDesc::enumeration(
                "type",
                "Type",
                SplineType::Bezier.id(),
                &options,
            ))
            .not_in_create_menu(),
        Interpolation::Uniform,
    )
}

/// One segment (a separate loop or chain) of an editable spline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    /// Number of points.
    pub len: usize,
    pub closed: bool,
}

/// The point data of an editable spline.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SplineData {
    pub points: Vec<DVec3>,
    pub left: Vec<DVec3>,
    pub right: Vec<DVec3>,
    pub segments: Vec<Segment>,
}

fn list(values: &BTreeMap<String, Value>, id: &str) -> Vec<DVec3> {
    match values.get(id) {
        Some(Value::Vec3List(l)) => l.iter().map(|p| DVec3::from_array(*p)).collect(),
        _ => Vec::new(),
    }
}

fn indices(values: &BTreeMap<String, Value>, id: &str) -> Vec<usize> {
    match values.get(id) {
        Some(Value::IndexList(l)) => l.iter().map(|i| *i as usize).collect(),
        _ => Vec::new(),
    }
}

fn vec_list(l: &[DVec3]) -> Value {
    Value::Vec3List(l.iter().map(|p| p.to_array()).collect())
}

impl SplineData {
    /// Reads and checks the five data params (SPLINES.md §1.1).
    pub fn from_values(values: &BTreeMap<String, Value>) -> Result<SplineData, BuildError> {
        let bad = |m: &str| BuildError::Failed(format!("invalid spline: {m}"));
        let points = list(values, "points");
        let (left, right) = (list(values, "left"), list(values, "right"));
        if points.is_empty() {
            return Err(bad("no points"));
        }
        if left.len() != points.len() || right.len() != points.len() {
            return Err(bad("points and tangents differ in length"));
        }
        let ends = indices(values, "segment_ends");
        let ends = if ends.is_empty() {
            vec![points.len()]
        } else {
            ends
        };
        if ends.last() != Some(&points.len()) {
            return Err(bad("segment ends do not cover the points"));
        }
        let closed = indices(values, "closed_segments");
        if closed.iter().any(|c| *c >= ends.len()) {
            return Err(bad("a closed flag names a missing segment"));
        }
        let mut segments = Vec::new();
        let mut start = 0;
        for (i, end) in ends.iter().enumerate() {
            if *end < start + 2 {
                return Err(bad("a segment needs at least two points"));
            }
            segments.push(Segment {
                len: end - start,
                closed: closed.contains(&i),
            });
            start = *end;
        }
        Ok(SplineData {
            points,
            left,
            right,
            segments,
        })
    }

    /// The data params, ready for `set_param`.
    pub fn to_values(&self) -> Vec<(String, Value)> {
        let mut end = 0;
        let ends: Vec<u32> = self
            .segments
            .iter()
            .map(|s| {
                end += s.len;
                end as u32
            })
            .collect();
        let closed: Vec<u32> = self
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.closed)
            .map(|(i, _)| i as u32)
            .collect();
        vec![
            ("points".into(), vec_list(&self.points)),
            ("left".into(), vec_list(&self.left)),
            ("right".into(), vec_list(&self.right)),
            ("segment_ends".into(), Value::IndexList(ends)),
            ("closed_segments".into(), Value::IndexList(closed)),
        ]
    }

    /// Index range of every segment in the point lists.
    pub fn ranges(&self) -> Vec<std::ops::Range<usize>> {
        let mut start = 0;
        self.segments
            .iter()
            .map(|s| {
                let r = start..start + s.len;
                start += s.len;
                r
            })
            .collect()
    }

    /// An open or closed chain of `points` with zero handles.
    pub fn from_points(points: Vec<DVec3>, closed: bool) -> SplineData {
        let n = points.len();
        SplineData {
            left: vec![DVec3::ZERO; n],
            right: vec![DVec3::ZERO; n],
            segments: vec![Segment { len: n, closed }],
            points,
        }
    }

    /// Inserts a point so that it ends up at `index` of the list (SPLINES.md §5): into the
    /// segment of point `index - 1` (the first segment for `index` 0).
    pub fn insert(
        &mut self,
        index: usize,
        position: DVec3,
        left: DVec3,
        right: DVec3,
    ) -> Result<(), String> {
        if index > self.points.len() {
            return Err(format!(
                "index {index} is past the {} points",
                self.points.len()
            ));
        }
        let seg = if index == 0 {
            0
        } else {
            self.ranges()
                .iter()
                .position(|r| r.contains(&(index - 1)))
                .unwrap_or(self.segments.len() - 1)
        };
        self.points.insert(index, position);
        self.left.insert(index, left);
        self.right.insert(index, right);
        self.segments[seg].len += 1;
        Ok(())
    }

    /// Removes the points at `indices` (any order, no repeats); a segment may not drop
    /// below two points.
    pub fn delete(&mut self, indices: &[usize]) -> Result<(), String> {
        let mut sorted = indices.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        if sorted.len() != indices.len() {
            return Err("an index is listed twice".into());
        }
        if let Some(i) = sorted.iter().find(|i| **i >= self.points.len()) {
            return Err(format!("index {i} is out of range"));
        }
        let mut lens = Vec::new();
        for r in self.ranges() {
            let gone = sorted.iter().filter(|i| r.contains(i)).count();
            if r.len() - gone < 2 {
                return Err("a segment would keep fewer than two points".into());
            }
            lens.push(r.len() - gone);
        }
        for i in sorted.iter().rev() {
            self.points.remove(*i);
            self.left.remove(*i);
            self.right.remove(*i);
        }
        for (s, len) in self.segments.iter_mut().zip(lens) {
            s.len = len;
        }
        Ok(())
    }

    /// The control points of segment `seg`.
    fn chain(&self, range: &std::ops::Range<usize>) -> Vec<DVec3> {
        self.points[range.clone()].to_vec()
    }

    /// One segment as a [`Curve`] for the shared sampler (SPLINES.md §1.2).
    fn loop_curve(
        &self,
        range: &std::ops::Range<usize>,
        closed: bool,
        ty: SplineType,
        sampling: Sampling,
    ) -> Curve {
        let pts = self.chain(range);
        // None: the control points only, whatever the type (`interp-*` cases)
        if sampling.mode == Interpolation::None || ty == SplineType::Linear {
            return Curve::polyline(&pts, closed, sampling);
        }
        let n = pts.len();
        let spans = if closed { n } else { n - 1 };
        let segments = match ty {
            SplineType::Bezier => (0..spans)
                .map(|i| {
                    let (a, b) = (range.start + i, range.start + (i + 1) % n);
                    [
                        self.points[a],
                        self.points[a] + self.right[a],
                        self.points[b] + self.left[b],
                        self.points[b],
                    ]
                })
                .collect(),
            other => {
                let kind = match other {
                    SplineType::Cubic => SplineKind::CatmullRom,
                    SplineType::Akima => SplineKind::Akima,
                    _ => SplineKind::BSpline,
                };
                let spline = Spline::new(kind, pts, closed).expect("a segment has two points");
                (0..spans)
                    .map(|i| {
                        let (p0, p1) = (spline.eval(i, 0.0), spline.eval(i, 1.0));
                        [
                            p0,
                            p0 + spline.derivative(i, 0.0) / 3.0,
                            p1 - spline.derivative(i, 1.0) / 3.0,
                            p1,
                        ]
                    })
                    .collect()
            }
        };
        Curve::new(segments, false, closed, sampling)
    }

    /// The whole spline: the first segment is the main loop, the others its extra loops.
    pub fn curve(&self, ty: SplineType, sampling: Sampling) -> Curve {
        let mut loops = self
            .ranges()
            .into_iter()
            .zip(&self.segments)
            .map(|(r, s)| self.loop_curve(&r, s.closed, ty, sampling));
        let mut main = loops.next().expect("a spline has a segment");
        main.extra = loops.collect();
        main
    }

    /// The editable form of a curve (Make Editable, SPLINES.md §3).
    pub fn from_curve(curve: &Curve) -> Result<SplineData, BuildError> {
        let mut data = SplineData::default();
        for lp in std::iter::once(curve).chain(curve.extra.iter()) {
            let n = lp.segments.len();
            let nxt = |i: usize| (i + 1) % n;
            for i in 0..n {
                let chained = (lp.segments[i][3] - lp.segments[nxt(i)][0]).length() < 1e-9;
                if (lp.closed || i + 1 < n) && !chained {
                    return Err(BuildError::Unsupported("spline pieces that do not chain"));
                }
            }
            let count = if lp.closed { n } else { n + 1 };
            if count < 2 {
                return Err(BuildError::Unsupported("a spline loop of one point"));
            }
            let base = data.points.len();
            for i in 0..count {
                let seg = |k: usize| &lp.segments[k];
                let (point, right, left) = if i < n {
                    let s = seg(i);
                    let left = if i > 0 || lp.closed {
                        let p = seg((i + n - 1) % n);
                        p[2] - p[3]
                    } else {
                        // open start: the unused left handle mirrors the right one
                        s[0] - s[1]
                    };
                    (s[0], s[1] - s[0], left)
                } else {
                    // open end: the unused right handle mirrors the left one
                    let s = seg(n - 1);
                    (s[3], s[3] - s[2], s[2] - s[3])
                };
                data.points.push(point);
                let linear = lp.linear;
                data.left.push(if linear { DVec3::ZERO } else { left });
                data.right.push(if linear { DVec3::ZERO } else { right });
            }
            data.segments.push(Segment {
                len: data.points.len() - base,
                closed: lp.closed,
            });
        }
        Ok(data)
    }
}

/// The curve of a `forge.spline` node's params, in its local space.
pub(crate) fn build_curve(values: &BTreeMap<String, Value>) -> Result<Curve, BuildError> {
    let ty = node_type();
    let p = Params { ty: &ty, values };
    let sampling = Sampling::from_params(&p);
    let kind = SplineType::from_id(&p.enumeration("type")).unwrap_or(SplineType::Bezier);
    Ok(SplineData::from_values(values)?.curve(kind, sampling))
}

/// The params of the editable spline that replaces the spline primitive `type_id` with
/// `values` (SPLINES.md §3); `None` if it is not a spline primitive.
pub(crate) fn from_primitive(
    type_id: &str,
    values: &BTreeMap<String, Value>,
) -> Result<Option<Vec<(String, Value)>>, BuildError> {
    if type_id == TYPE || !crate::splines::is_spline(type_id) {
        return Ok(None);
    }
    let curve = crate::splines::build_curve(type_id, values)?;
    let data = SplineData::from_curve(&curve)?;
    let kind = if curve.linear {
        SplineType::Linear
    } else {
        SplineType::Bezier
    };
    let s = curve.sampling;
    let mut out = data.to_values();
    out.push(("type".into(), Value::Enum(kind.id().into())));
    out.push(("intermediate".into(), Value::Enum(s.mode.id().into())));
    out.push(("intermediate_number".into(), Value::Int(s.number as i64)));
    out.push(("intermediate_angle".into(), Value::Float(s.angle)));
    out.push(("intermediate_max_length".into(), Value::Float(s.max_length)));
    Ok(Some(out))
}

/// Reads a `forge.spline` node's data (for the commands and the viewport).
pub fn data_of(
    scene: &forge_core::Scene,
    id: forge_core::NodeId,
) -> Result<SplineData, BuildError> {
    let node = scene
        .node(id)
        .filter(|n| n.type_id == TYPE)
        .ok_or_else(|| BuildError::UnknownType(format!("node {id}")))?;
    SplineData::from_values(&node.params)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> SplineData {
        SplineData::from_points(
            vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(10.0, 0.0, 0.0),
                DVec3::new(10.0, 10.0, 0.0),
                DVec3::new(0.0, 10.0, 0.0),
            ],
            true,
        )
    }

    #[test]
    fn values_round_trip() {
        let d = square();
        let values: BTreeMap<String, Value> = d.to_values().into_iter().collect();
        assert_eq!(SplineData::from_values(&values).unwrap(), d);
    }

    #[test]
    fn insert_goes_into_the_segment_of_the_previous_point() {
        let mut d = square();
        d.segments = vec![
            Segment {
                len: 2,
                closed: false,
            },
            Segment {
                len: 2,
                closed: true,
            },
        ];
        d.insert(2, DVec3::X, DVec3::ZERO, DVec3::ZERO).unwrap();
        assert_eq!(d.segments.iter().map(|s| s.len).collect::<Vec<_>>(), [3, 2]);
        d.insert(0, DVec3::Y, DVec3::ZERO, DVec3::ZERO).unwrap();
        assert_eq!(d.segments.iter().map(|s| s.len).collect::<Vec<_>>(), [4, 2]);
        assert!(d.insert(7, DVec3::Y, DVec3::ZERO, DVec3::ZERO).is_err());
    }

    #[test]
    fn delete_keeps_two_points_per_segment() {
        let mut d = square();
        d.delete(&[1]).unwrap();
        assert_eq!(d.points.len(), 3);
        assert!(d.delete(&[0, 1]).is_err());
        assert_eq!(d.points.len(), 3, "a refused delete changes nothing");
        assert!(d.delete(&[9]).is_err());
        assert!(d.delete(&[0, 0]).is_err());
    }

    #[test]
    fn invalid_data_is_an_error_not_a_panic() {
        let mut v: BTreeMap<String, Value> = square().to_values().into_iter().collect();
        v.insert("left".into(), Value::Vec3List(vec![]));
        assert!(SplineData::from_values(&v).is_err());
        let mut v: BTreeMap<String, Value> = square().to_values().into_iter().collect();
        v.insert("closed_segments".into(), Value::IndexList(vec![3]));
        assert!(SplineData::from_values(&v).is_err());
        assert!(SplineData::from_values(&BTreeMap::new()).is_err());
    }

    #[test]
    fn bspline_with_none_keeps_the_control_points() {
        let d = square();
        let c = d.curve(SplineType::BSpline, Sampling::new(Interpolation::None));
        assert_eq!(c.points(), d.points);
    }

    #[test]
    fn other_types_pass_through_their_points_at_the_ends() {
        let d = SplineData::from_points(
            vec![
                DVec3::ZERO,
                DVec3::new(10.0, 5.0, 0.0),
                DVec3::new(20.0, 0.0, 0.0),
            ],
            false,
        );
        for ty in [SplineType::Cubic, SplineType::Akima] {
            let c = d.curve(ty, Sampling::new(Interpolation::Natural));
            let pts = c.points();
            assert_eq!(pts[0], DVec3::ZERO);
            assert!((pts.last().unwrap().x - 20.0).abs() < 1e-9, "{ty:?}");
            assert!(pts.len() > 3, "{ty:?} gets interior points");
        }
    }
}
