//! Spline Mask (docs/specs/GENERATORS3.md §2; docs/parity/generators-3.md, `spline-mask/*`):
//! the spline booleans of SPLINES.md §8.3 as a spline generator.

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene};

use crate::BuildError;
use crate::editable_spline::{SplineData, SplineType};
use crate::spline_sampling::{Interpolation, Sampling, sample};
use crate::spline_tools::{self, BoolOp, Operand};
use crate::splines::{Curve, is_spline, placed_curve, straight};

pub const TYPE: &str = "forge.spline_mask";

pub(crate) fn node_type() -> NodeType {
    NodeType::new(TYPE, "Spline Mask", Category::Generator)
        .param(ParamDesc::enumeration(
            "mode",
            "Mode",
            "union",
            &[
                ("union", "Union"),
                ("a_minus_b", "A subtract B"),
                ("b_minus_a", "B subtract A"),
                ("and", "And"),
                ("or", "Or"),
                ("intersection", "Intersection"),
            ],
        ))
        .param(ParamDesc::enumeration(
            "axis",
            "Axis",
            "xy",
            &[
                ("xy", "XY (Along Z)"),
                ("zy", "ZY (Along X)"),
                ("xz", "XZ (Along Y)"),
                ("viewport", "Viewport (Render View)"),
            ],
        ))
        .param(ParamDesc::bool("create_cap", "Create Cap", false))
}

/// The two spline children A and B a mask consumes, in child order.
pub fn inputs_of(scene: &Scene, id: NodeId) -> Vec<NodeId> {
    let Some(node) = scene.node(id) else {
        return Vec::new();
    };
    node.children
        .iter()
        .copied()
        .filter(|c| scene.node(*c).is_some_and(|n| is_spline(&n.type_id)))
        .take(2)
        .collect()
}

/// An input as the polygon of its sampled outline (rule 1 of the spec).
fn operand(curve: &Curve) -> Result<Operand, BuildError> {
    if !curve.extra.is_empty() {
        return Err(BuildError::Unsupported(
            "spline mask input with several loops",
        ));
    }
    let points = sample(curve, &curve.sampling);
    if points.len() < 3 || !curve.closed {
        return Err(BuildError::Failed(
            "the mask inputs must be closed splines".into(),
        ));
    }
    if points.iter().any(|p| p.z.abs() >= 1e-6) {
        return Err(BuildError::Failed(
            "the splines must lie in one plane (the mask's XY plane)".into(),
        ));
    }
    Ok(Operand {
        data: SplineData::from_points(points, true),
        ty: SplineType::Linear,
        sampling: Sampling::new(Interpolation::Adaptive),
    })
}

/// The outline polygons of a boolean result as one curve of straight pieces that is sampled
/// Uniform with 8 intermediate points (rule 2 of the spec).
fn outline_curve(data: &SplineData) -> Curve {
    let sampling = Sampling::new(Interpolation::Uniform);
    let mut loops = data.ranges().into_iter().map(|r| {
        let pts = &data.points[r];
        let segments = (0..pts.len())
            .map(|i| straight(pts[i], pts[(i + 1) % pts.len()]))
            .collect();
        Curve::new(segments, false, true, sampling)
    });
    let mut main = loops.next().expect("a result has a loop");
    main.extra = loops.collect();
    main
}

/// The mask's curve in its own space; `None` with fewer than two spline children.
pub fn curve_of(scene: &Scene, id: NodeId) -> Result<Option<Curve>, BuildError> {
    let Some(node) = scene.node(id) else {
        return Ok(None);
    };
    let ty = node_type();
    let p = crate::Params {
        ty: &ty,
        values: &node.params,
    };
    if p.enumeration("axis") != "xy" {
        return Err(BuildError::Unsupported("spline mask axis other than XY"));
    }
    if p.bool("create_cap") {
        return Err(BuildError::Unsupported("spline mask create cap"));
    }
    let children = inputs_of(scene, id);
    if children.len() < 2 {
        return Ok(None);
    }
    let mut operands = Vec::new();
    for c in children {
        let curve = placed_curve(scene, c)?
            .ok_or_else(|| BuildError::Failed("the mask input has no curve".into()))?;
        operands.push(operand(&curve)?);
    }
    let (a, b) = (&operands[0], &operands[1]);
    let result = match p.enumeration("mode").as_str() {
        "union" => spline_tools::boolean(a, b, BoolOp::Union),
        "a_minus_b" => spline_tools::boolean(a, b, BoolOp::Subtract),
        "b_minus_a" => spline_tools::boolean(b, a, BoolOp::Subtract),
        "and" => spline_tools::boolean(a, b, BoolOp::And),
        "or" => spline_tools::boolean(a, b, BoolOp::Or),
        _ => spline_tools::boolean(a, b, BoolOp::Intersect),
    }
    .map_err(|e| BuildError::Failed(e.to_string()))?;
    Ok(Some(outline_curve(&result.data)))
}
