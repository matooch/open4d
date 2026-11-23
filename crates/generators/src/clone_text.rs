//! Clone Text: glyph outlines grouped into independently transformed extrusions (TEXT.md §7).

use std::collections::BTreeMap;

use forge_clone_engine::{CloneData, CloneError, CloneItem, DELAY, apply_effectors};
use forge_core::{Category, EvalCtx, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use forge_geometry::PolyMesh;
use forge_text::{Font, Glyph};
use glam::{DMat4, DVec3};
use spade::{ConstrainedDelaunayTriangulation, Point2, Triangulation};

use crate::spline_sampling::Sampling;
use crate::splines::Curve;
use crate::{BuildError, PARALLEL_POINTS, Params, expand_clones, spline_gens, text_spline};

pub const TYPE: &str = "forge.clone_text";

pub(crate) fn node_type() -> NodeType {
    let t = NodeType::new(TYPE, "Clone Text", Category::Generator)
        .menu_path("Generator", 0)
        .param(ParamDesc::enumeration(
            "group",
            "Group",
            "letters",
            &[
                ("letters", "Letters"),
                ("words", "Words"),
                ("lines", "Lines"),
            ],
        ))
        .param(ParamDesc::float("depth", "Depth", 20.0).unit(Unit::Distance))
        .param(ParamDesc::int("subdivision", "Subdivision", 1).range(1, 4000))
        .param(ParamDesc::bool("start_cap", "Start Cap", true))
        .param(ParamDesc::bool("end_cap", "End Cap", true))
        .param(ParamDesc::link_list(
            "effectors",
            "Effectors",
            &[Category::Effector],
        ));
    text_spline::node_type()
        .params
        .into_iter()
        .filter(|p| !matches!(p.id.as_str(), "plane" | "reverse" | "separate_letters"))
        .fold(t, NodeType::param)
}

struct Extrusion {
    side_params: BTreeMap<String, Value>,
    sampling: Sampling,
    depth: f64,
    start_cap: bool,
    end_cap: bool,
}

impl Extrusion {
    fn from_params(p: &Params<'_>) -> Self {
        let depth = p.float("depth");
        Self {
            side_params: BTreeMap::from([
                ("direction".into(), Value::Enum("z".into())),
                ("offset".into(), Value::Float(depth)),
                ("subdivision".into(), p.get("subdivision")),
                ("cap_start".into(), Value::Enum("none".into())),
                ("cap_end".into(), Value::Enum("none".into())),
            ]),
            sampling: Sampling::from_params(p),
            depth,
            start_cap: p.bool("start_cap"),
            end_cap: p.bool("end_cap"),
        }
    }

    fn glyph(&self, glyph: &Glyph) -> Result<PolyMesh, BuildError> {
        if glyph.character.is_whitespace() {
            return Ok(PolyMesh::default());
        }
        let Some(curve) = text_spline::curve_from_contours(&glyph.contours, false, self.sampling)
        else {
            return Ok(PolyMesh::default());
        };
        let mut mesh = spline_gens::extrude(&curve, &self.side_params)?;
        if self.start_cap || self.end_cap {
            self.cap(&curve, &mut mesh)?;
        }
        Ok(mesh)
    }

    fn cap(&self, curve: &Curve, mesh: &mut PolyMesh) -> Result<(), BuildError> {
        let loops = curve.loops();
        let mut vertices = Vec::new();
        let mut edges = Vec::new();
        for rim in &loops {
            let base = vertices.len();
            for (i, p) in rim.iter().enumerate() {
                vertices.push(Point2::new(p.x, p.y));
                edges.push([base + i, base + (i + 1) % rim.len()]);
            }
        }
        let len = vertices.len();
        let mut conflicting = false;
        let triangles =
            ConstrainedDelaunayTriangulation::<_>::try_bulk_load_cdt(vertices, edges, |_| {
                conflicting = true
            })
            .map_err(|e| BuildError::Failed(format!("text cap triangulation: {e}")))?;
        if conflicting || triangles.num_vertices() != len {
            return Err(BuildError::Unsupported(
                "caps with crossing or touching glyph contours",
            ));
        }
        let last = (mesh.points.len() - len) as u32;
        for f in triangles.inner_faces() {
            let centre = f.center();
            if !inside_ink(centre, &loops) {
                continue;
            }
            let ccw = f.vertices().map(|v| v.index() as u32);
            if self.start_cap {
                let mut front = ccw;
                if self.depth >= 0.0 {
                    front.swap(1, 2);
                }
                mesh.polys.push(front.to_vec());
            }
            if self.end_cap {
                let mut back = ccw.map(|i| last + i);
                if self.depth < 0.0 {
                    back.swap(1, 2);
                }
                mesh.polys.push(back.to_vec());
            }
        }
        Ok(())
    }
}

fn inside_ink(p: Point2<f64>, loops: &[Vec<DVec3>]) -> bool {
    let mut inside = false;
    for rim in loops {
        for (a, b) in rim.iter().zip(rim.iter().cycle().skip(1)) {
            if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
                inside = !inside;
            }
        }
    }
    inside
}

struct PlacedGlyph {
    advance: [f64; 2],
    whitespace: bool,
    mesh: PolyMesh,
}

fn push_group(
    group: &[PlacedGlyph],
    mid_y: f64,
    meshes: &mut Vec<PolyMesh>,
    clones: &mut CloneData,
) {
    let ink = || group.iter().filter(|glyph| !glyph.mesh.points.is_empty());
    let Some(first) = ink().next() else { return };
    let last = ink().next_back().expect("nonempty group");
    let pivot = DVec3::new(f64::midpoint(first.advance[0], last.advance[1]), mid_y, 0.0);
    let mut proto = PolyMesh::default();
    for g in ink() {
        let base = proto.points.len() as u32;
        proto
            .points
            .extend(g.mesh.points.iter().map(|p| *p - pivot));
        proto.polys.extend(
            g.mesh
                .polys
                .iter()
                .map(|f| f.iter().map(|i| base + i).collect()),
        );
    }
    let child = meshes.len() as u32;
    meshes.push(proto);
    clones.push(CloneItem::at(DMat4::from_translation(pivot), DVec3::ZERO));
    clones.child[child as usize] = child;
}

fn instances(
    scene: &Scene,
    id: NodeId,
    ctx: &EvalCtx,
) -> Result<Option<(Vec<PolyMesh>, CloneData)>, BuildError> {
    let Some(node) = scene.node(id).filter(|n| n.type_id == TYPE) else {
        return Ok(None);
    };
    let ty = node_type();
    let p = Params {
        ty: &ty,
        values: &node.params,
    };
    let layout = text_spline::layout_settings(&p);
    if layout.height == 0.0 {
        return Ok(None);
    }
    let grouping = p.enumeration("group");
    let extrusion = Extrusion::from_params(&p);
    let mut meshes = Vec::new();
    let mut clones = CloneData::default();
    for glyphs in Font::resolve(&p.string("font")).layout_glyphs(&p.string("text"), &layout) {
        let mut line = Vec::new();
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for glyph in glyphs {
            let mesh = extrusion.glyph(&glyph)?;
            for point in &mesh.points {
                lo = lo.min(point.y);
                hi = hi.max(point.y);
            }
            line.push(PlacedGlyph {
                advance: glyph.advance,
                whitespace: glyph.character.is_whitespace(),
                mesh,
            });
        }
        if !lo.is_finite() {
            continue;
        }
        let mid = f64::midpoint(lo, hi);
        match grouping.as_str() {
            "lines" => push_group(&line, mid, &mut meshes, &mut clones),
            "words" => {
                for group in line.split(|glyph| glyph.whitespace) {
                    push_group(group, mid, &mut meshes, &mut clones);
                }
            }
            _ => {
                for glyph in &line {
                    push_group(std::slice::from_ref(glyph), mid, &mut meshes, &mut clones);
                }
            }
        }
    }
    if clones.is_empty() {
        return Ok(None);
    }
    if let Value::LinkList(effectors) = p.get("effectors")
        && effectors.into_iter().any(|id| {
            scene.node(id).is_some_and(|n| n.type_id == DELAY)
                && !matches!(scene.param(id, "enabled"), Some(Value::Bool(false)))
        })
    {
        return Err(BuildError::Unsupported("Delay history on Clone Text"));
    }
    apply_effectors(scene, id, &mut clones, ctx).map_err(|err| match err {
        CloneError::Unsupported(what) => BuildError::Unsupported(what),
        CloneError::TooMany => BuildError::Unsupported("too many text clones"),
    })?;
    Ok(Some((meshes, clones)))
}

/// Clone pivots and channels in generator-local space.
pub fn clone_data(
    scene: &Scene,
    id: NodeId,
    ctx: &EvalCtx,
) -> Result<Option<CloneData>, BuildError> {
    Ok(instances(scene, id, ctx)?.map(|(_, data)| data))
}

pub(crate) fn geometry(
    scene: &Scene,
    id: NodeId,
    ctx: &EvalCtx,
) -> Result<Option<PolyMesh>, BuildError> {
    let Some((meshes, clones)) = instances(scene, id, ctx)? else {
        return Ok(None);
    };
    let points = meshes.iter().map(|m| m.points.len()).sum::<usize>();
    Ok(Some(expand_clones(
        &meshes,
        &clones,
        points >= PARALLEL_POINTS,
    )))
}
