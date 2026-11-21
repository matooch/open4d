//! The clone-toolset spline generator (spec S1): Simple, Spline and Turtle modes. Simple makes
//! a straight line along Z; Spline copies a linked spline; Turtle is not derived yet.

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use glam::DVec3;

use crate::spline_sampling::{Interpolation, Sampling, add_params};
use crate::splines::{Curve, placed_curve};
use crate::{BuildError, Params};

pub const TYPE: &str = "forge.clone_spline";

pub(crate) fn node_type() -> NodeType {
    let t = NodeType::new(TYPE, "Clone Spline", Category::Generator)
        .param(ParamDesc::enumeration(
            "mode",
            "Mode",
            "simple",
            &[
                ("simple", "Simple"),
                ("spline", "Spline"),
                ("turtle", "Turtle"),
            ],
        ))
        .param(ParamDesc::int("count", "Count", 100).range(1, 100_000))
        .param(
            ParamDesc::float("length", "Length", 200.0)
                .range(0.0, f64::MAX)
                .unit(Unit::Distance),
        )
        .param(ParamDesc::link(
            "source",
            "Source spline",
            &[Category::Object, Category::Generator],
        ));
    add_params(t, Interpolation::None)
}

/// The generator's curve in its own frame; `None` in Spline mode without a source.
pub fn curve_of(scene: &Scene, id: NodeId) -> Result<Option<Curve>, BuildError> {
    let Some(node) = scene.node(id) else {
        return Ok(None);
    };
    let ty = node_type();
    let p = Params {
        ty: &ty,
        values: &node.params,
    };
    match p.enumeration("mode").as_str() {
        "simple" => {
            let n = p.int("count").max(1);
            let length = p.float("length");
            let points: Vec<DVec3> = (0..=n)
                .map(|i| DVec3::new(0.0, 0.0, length * i as f64 / n as f64))
                .collect();
            Ok(Some(Curve::polyline(
                &points,
                false,
                Sampling::from_params(&p),
            )))
        }
        "spline" => match node.params.get("source") {
            Some(Value::Link(Some(src))) if *src != id => {
                // The copy is always an open spline of the source's sampled points; a closed
                // source repeats its first point (`clone-spline/spline-mode`: 37 points).
                let Some(source) = placed_curve(scene, *src)? else {
                    return Ok(None);
                };
                let mut points = source.points();
                if source.closed
                    && let Some(first) = points.first().copied()
                {
                    points.push(first);
                }
                if points.len() < 2 {
                    return Ok(None);
                }
                Ok(Some(Curve::polyline(
                    &points,
                    false,
                    Sampling::from_params(&p),
                )))
            }
            _ => Ok(None),
        },
        _ => Err(BuildError::Unsupported("Clone Spline Turtle mode")),
    }
}
