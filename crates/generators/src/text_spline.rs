//! The Text spline (docs/specs/TEXT.md): font outlines laid out by `forge-text`, one closed
//! contour per loop, through the shared Intermediate Points sampler.

use std::collections::BTreeMap;

use forge_core::{Category, NodeType, ParamDesc, Unit, Value};
use forge_text::{Align, Contour, Font, Layout};
use glam::DVec3;

use crate::spline_sampling::{Interpolation, Sampling};
use crate::splines::{Curve, common, on_plane};
use crate::{BuildError, Params};

pub const TYPE: &str = "forge.text_spline";

fn distance(id: &str, label: &str, default: f64) -> ParamDesc {
    ParamDesc::float(id, label, default)
        .range(-1e20, 1e20)
        .unit(Unit::Distance)
}

pub(crate) fn node_type() -> NodeType {
    common(
        NodeType::new(TYPE, "Text", Category::Generator)
            .param(ParamDesc::string("text", "Text", "Text"))
            .param(ParamDesc::string("font", "Font", forge_text::DEFAULT_FONT))
            .param(ParamDesc::enumeration(
                "align",
                "Align",
                "left",
                &[("left", "Left"), ("middle", "Middle"), ("right", "Right")],
            ))
            .param(distance("height", "Height", 200.0))
            .param(distance("horizontal_spacing", "Horizontal spacing", 0.0))
            .param(distance("vertical_spacing", "Vertical spacing", 0.0))
            .param(ParamDesc::bool(
                "separate_letters",
                "Separate letters",
                false,
            )),
        Interpolation::Adaptive,
    )
}

fn point(p: [f64; 2]) -> DVec3 {
    DVec3::new(p[0], p[1], 0.0)
}

/// A closed curve of the contour's pieces, optionally run backwards.
fn curve_of(c: &Contour, reverse: bool, sampling: Sampling) -> Curve {
    let piece = |p: &[[f64; 2]; 4]| {
        if reverse {
            [point(p[3]), point(p[2]), point(p[1]), point(p[0])]
        } else {
            p.map(point)
        }
    };
    let mut segments: Vec<[DVec3; 4]> = c.pieces.iter().map(piece).collect();
    if reverse {
        segments.reverse();
    }
    Curve::new(segments, false, true, sampling)
}

/// The Text spline in its local space (SPLINES.md section 6.1 frame, then its plane): the
/// first contour is the spline, the others its extra loops.
pub(crate) fn build_curve(values: &BTreeMap<String, Value>) -> Result<Curve, BuildError> {
    let ty = node_type();
    let p = Params { ty: &ty, values };
    if p.bool("separate_letters") {
        return Err(BuildError::Unsupported("separate letters"));
    }
    let height = p.float("height");
    if height == 0.0 {
        return Err(BuildError::Failed("the text height is 0".into()));
    }
    let align = match p.enumeration("align").as_str() {
        "middle" => Align::Middle,
        "right" => Align::Right,
        _ => Align::Left,
    };
    let layout = Layout {
        height,
        align,
        horizontal_spacing: p.float("horizontal_spacing"),
        vertical_spacing: p.float("vertical_spacing"),
    };
    let contours = Font::resolve(&p.string("font")).layout(&p.string("text"), &layout);
    let sampling = Sampling::from_params(&p);
    let reverse = p.bool("reverse");
    let mut curves = contours.iter().map(|c| curve_of(c, reverse, sampling));
    let mut curve = curves
        .next()
        .ok_or_else(|| BuildError::Failed("the text has no outline".into()))?;
    curve.extra = curves.collect();
    let plane = p.enumeration("plane");
    if plane != "xy" {
        let place = |c: &mut Curve| {
            for seg in &mut c.segments {
                *seg = seg.map(|q| on_plane(&plane, q));
            }
        };
        place(&mut curve);
        curve.extra.iter_mut().for_each(place);
    }
    Ok(curve)
}
