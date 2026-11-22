//! Spline Mask (docs/specs/GENERATORS3.md §2; docs/parity/generators-3.md, `spline-mask/*`):
//! the spline booleans of SPLINES.md §8.3 as a spline generator.

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene};
use forge_geometry::PolyMesh;
use forge_geometry::triangulate::triangulate_delaunay;
use glam::{DMat4, DVec4};

use crate::BuildError;
use crate::editable_spline::{SplineData, SplineType};
use crate::spline_sampling::{Interpolation, Sampling, sample};
use crate::spline_tools::{self, BoolOp, Operand};
use crate::splines::{Curve, is_spline, placed_curve, straight};

pub const TYPE: &str = "forge.spline_mask";

pub(crate) fn node_type() -> NodeType {
    NodeType::new(TYPE, "Spline Mask", Category::Generator)
        .menu_path("Generator", 0)
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

/// Swaps the x and z axes (its own inverse): the frame of the ZY plane.
const SWAP_XZ: DMat4 = DMat4::from_cols(
    DVec4::new(0.0, 0.0, 1.0, 0.0),
    DVec4::new(0.0, 1.0, 0.0, 0.0),
    DVec4::new(1.0, 0.0, 0.0, 0.0),
    DVec4::new(0.0, 0.0, 0.0, 1.0),
);

/// Create Cap on: the mask is a polygon object, its outline ring triangulated
/// (`spline-mask/cap`; GENERATORS3.md §2 rule 5). `None` with Create Cap off or no curve.
pub fn cap_mesh(scene: &Scene, id: NodeId) -> Result<Option<PolyMesh>, BuildError> {
    let on = scene.node(id).is_some_and(|n| {
        matches!(
            n.params.get("create_cap"),
            Some(forge_core::Value::Bool(true))
        )
    });
    if !on {
        return Ok(None);
    }
    let Some(curve) = curve_of(scene, id)? else {
        return Ok(None);
    };
    if !curve.extra.is_empty() {
        return Err(BuildError::Unsupported("spline mask cap of several loops"));
    }
    let points = curve.points();
    let ring: Vec<u32> = (0..points.len() as u32).collect();
    let polys = triangulate_delaunay(&points, &ring)
        .into_iter()
        .map(|t| t.to_vec())
        .collect();
    Ok(Some(PolyMesh { points, polys }))
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
fn outline_curve(data: &SplineData, sampling: Sampling) -> Curve {
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
    // The mask works in the plane of its Axis: XY, or ZY (`spline-mask/axis-zy`), whose
    // frame (u, v) = (z, y) is the XY frame with x and z swapped.
    let swap = match p.enumeration("axis").as_str() {
        "xy" => false,
        "zy" => true,
        _ => return Err(BuildError::Unsupported("spline mask axis XZ and viewport")),
    };
    let children = inputs_of(scene, id);
    if children.len() < 2 {
        return Ok(None);
    }
    let mut operands = Vec::new();
    let mut sampling = None;
    for c in children {
        let mut curve = placed_curve(scene, c)?
            .ok_or_else(|| BuildError::Failed("the mask input has no curve".into()))?;
        if swap {
            curve = curve.transformed(&SWAP_XZ);
        }
        sampling.get_or_insert(curve.sampling);
        operands.push(operand(&curve)?);
    }
    let (a, b) = (&operands[0], &operands[1]);
    let result = match p.enumeration("mode").as_str() {
        "union" => spline_tools::boolean(a, b, BoolOp::Union),
        "a_minus_b" => spline_tools::boolean(a, b, BoolOp::Subtract),
        // B outside A with A inside B: the first loop of Or (`spline-mask/b-minus-a`)
        "b_minus_a" => spline_tools::boolean(a, b, BoolOp::Or).map(|mut r| {
            let first = r.data.segments[0].len;
            r.data.points.truncate(first);
            r.data.left.truncate(first);
            r.data.right.truncate(first);
            r.data.segments.truncate(1);
            r
        }),
        "and" => spline_tools::boolean(a, b, BoolOp::And),
        "or" => spline_tools::boolean(a, b, BoolOp::Or),
        _ => spline_tools::boolean(a, b, BoolOp::Intersect),
    }
    .map_err(|e| BuildError::Failed(e.to_string()))?;
    let curve = outline_curve(&result.data, sampling.expect("two children"));
    Ok(Some(if swap {
        curve.transformed(&SWAP_XZ)
    } else {
        curve
    }))
}
