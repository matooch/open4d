use std::collections::BTreeSet;
use std::fmt::{Display, Write};

use forge_core::transform::{Transform, hpb_from_rotation};
use forge_core::{EvalCtx, NodeId, Scene, Value};
use forge_geometry::PolyMesh;
use forge_geometry::normals::{Weighting, corner_normals};
use glam::{DMat3, DVec3, EulerRot};

use super::{BASIS, COMPONENTS, Error, OPS, PARAMS, invalid, validate_topology};

/// Write the supported scene as one ASCII USD layer. Returns the mesh count.
pub fn write(scene: &Scene, selection: Option<&[NodeId]>) -> Result<(String, usize), Error> {
    let fps = f64::from(scene.settings().fps);
    let mut w = Writer {
        scene,
        fps,
        count: 0,
        materials: BTreeSet::new(),
        out: format!(
            "#usda 1.0\n(\n    upAxis = \"Y\"\n    metersPerUnit = 0.01\n    timeCodesPerSecond = {fps}\n    framesPerSecond = {fps}\n)\n"
        ),
    };
    if let Some(selection) = selection {
        let ids: BTreeSet<_> = selection.iter().copied().collect();
        for &id in &ids {
            let mut parent = scene.node(id).and_then(|n| n.parent);
            while let Some(p) = parent {
                if ids.contains(&p) {
                    break;
                }
                parent = scene.node(p).and_then(|n| n.parent);
            }
            if parent.is_none() {
                w.node(id, 0)?;
            }
        }
    } else {
        for &id in scene.roots() {
            w.node(id, 0)?;
        }
    }
    if w.count == 0 {
        return Err(Error::Empty);
    }
    w.materials()?;
    Ok((w.out, w.count))
}

struct Writer<'a> {
    scene: &'a Scene,
    out: String,
    fps: f64,
    count: usize,
    materials: BTreeSet<NodeId>,
}

impl Writer<'_> {
    fn node(&mut self, id: NodeId, depth: usize) -> Result<(), Error> {
        if depth >= 128 {
            return Err(invalid("prim nesting limit"));
        }
        let Some(n) = self.scene.node(id) else {
            return Ok(());
        };
        if n.is_tag()
            || forge_generators::is_consumed(self.scene, id)
            || self
                .scene
                .node_type(id)
                .is_some_and(|ty| ty.category == forge_core::Category::Deformer)
        {
            return Ok(());
        }
        if matches!(n.type_id.as_str(), "forge.camera" | "forge.light") {
            return Ok(());
        }
        let Some(mut t) = self.scene.local_transform(id) else {
            return Ok(());
        };
        if depth == 0 && n.parent.is_some() {
            if !n.tracks.is_empty() {
                return Err(Error::Unsupported(
                    "animated selected export root with omitted parent",
                ));
            }
            let world = self
                .scene
                .world_matrix(id)
                .ok_or_else(|| invalid("missing world transform"))?;
            let (scale, r, pos) = world.to_scale_rotation_translation();
            t = Transform {
                pos,
                rot: hpb_from_rotation(DMat3::from_quat(r)),
                scale,
            };
            if !world.abs_diff_eq(t.matrix(), 1e-8) {
                return Err(Error::Unsupported("sheared selection root"));
            }
        }
        let mesh = forge_generators::evaluate(self.scene, id, &EvalCtx::default())
            .map_err(|e| invalid(e.to_string()))?
            .filter(|m| !m.polys.is_empty());
        let mat = mesh.as_ref().and_then(|_| {
            n.tags.iter().find_map(|&tag| {
                let node = self.scene.node(tag)?;
                if node.type_id != forge_material::TEXTURE_TAG {
                    return None;
                }
                let Some(Value::Link(Some(mat))) = self.scene.param(tag, "material") else {
                    return None;
                };
                self.scene
                    .node(*mat)
                    .filter(|m| m.type_id == forge_material::MATERIAL)
                    .map(|_| *mat)
            })
        });
        let kind = if mesh.is_some() { "Mesh" } else { "Xform" };
        let display = quote(&n.name)?;
        self.line(depth, format_args!("def {kind} \"N{}\" (", id.0));
        self.line(depth + 1, format_args!("displayName = {display}"));
        if mat.is_some() {
            self.line(depth + 1, "prepend apiSchemas = [\"MaterialBindingAPI\"]");
        }
        self.line(depth, ") {");
        self.transforms(id, t, depth + 1)?;
        if let Some(mesh) = &mesh {
            self.mesh(id, mesh, depth + 1)?;
            self.count += 1;
            if let Some(mat) = mat {
                self.materials.insert(mat);
                self.line(
                    depth + 1,
                    format_args!("rel material:binding = </ForgeMaterials/M{}>", mat.0),
                );
            }
        }
        for &child in &n.children {
            self.node(child, depth + 1)?;
        }
        self.line(depth, "}");
        Ok(())
    }

    fn transforms(&mut self, id: NodeId, t: Transform, depth: usize) -> Result<(), Error> {
        let n = self.scene.node(id).expect("valid node");
        for (op, base) in [t.pos, t.rot, t.scale].into_iter().enumerate() {
            let label = OPS[op];
            let convert = |v: DVec3| match op {
                0 => BASIS * v,
                1 => {
                    let r = Transform {
                        rot: v,
                        ..Transform::IDENTITY
                    }
                    .rotation();
                    let (z, y, x) = (BASIS * r * BASIS).to_euler(EulerRot::ZYX);
                    DVec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees())
                }
                _ => v,
            };
            self.line(
                depth,
                format_args!("double3 {label} = {}", xyz(convert(base))),
            );
            let tracks = COMPONENTS.map(|axis| n.tracks.get(&format!("{}.{axis}", PARAMS[op])));
            let mut times: Vec<_> = tracks
                .iter()
                .flatten()
                .flat_map(|tr| tr.keys.iter().map(|k| k.time))
                .collect();
            if times.is_empty() {
                continue;
            }
            times.sort_by(f64::total_cmp);
            let first = (times[0] * self.fps).ceil() as i64;
            let last = (times[times.len() - 1] * self.fps).floor() as i64;
            if last.saturating_sub(first) > 1_000_000 {
                return Err(Error::Unsupported(
                    "animation exceeds one million baked frames",
                ));
            }
            times.extend((first..=last).map(|f| f as f64 / self.fps));
            times.sort_by(f64::total_cmp);
            times.dedup();
            self.line(depth, format_args!("double3 {label}.timeSamples = {{"));
            for time in times {
                let v = if op == 1 && tracks.iter().flatten().any(|tr| tr.quaternion) {
                    forge_anim::quaternion_rotation_at(tracks, base, time)
                } else {
                    DVec3::from_array(std::array::from_fn(|i| {
                        tracks[i].map_or(base[i], |tr| forge_anim::value_at_exact(tr, &[], time))
                    }))
                };
                let source_time = time * self.fps;
                if !source_time.is_finite() {
                    return Err(invalid("nonfinite time code"));
                }
                self.line(
                    depth + 1,
                    format_args!("{source_time}: {},", xyz(convert(v))),
                );
            }
            self.line(depth, "}");
        }
        self.line(depth, "uniform token[] xformOpOrder = [\"xformOp:translate\", \"xformOp:rotateXYZ\", \"xformOp:scale\"]");
        Ok(())
    }

    fn mesh(&mut self, id: NodeId, m: &PolyMesh, depth: usize) -> Result<(), Error> {
        validate_topology(m)?;
        self.line(depth, "uniform token subdivisionScheme = \"none\"");
        self.array(
            depth,
            "point3f[] points",
            m.points.iter().map(|&p| xyz(BASIS * p)),
        );
        self.array(
            depth,
            "int[] faceVertexCounts",
            m.polys.iter().map(Vec::len),
        );
        self.array(
            depth,
            "int[] faceVertexIndices",
            m.polys.iter().flat_map(|p| p.iter().rev().copied()),
        );
        let angle = forge_core::smoothing::smoothing_angle(self.scene, id).unwrap_or(0.0);
        let normals = corner_normals(m, angle, Weighting::Angle);
        self.array(
            depth,
            "normal3f[] normals",
            normals
                .iter()
                .flat_map(|f| f.iter().rev().map(|&p| xyz(BASIS * p))),
        );
        self.line(depth, "(interpolation = \"faceVarying\")");
        if let Some(uv) = crate::scene::node_uvs(self.scene, id, m) {
            let st = uv
                .corners
                .iter()
                .flat_map(|f| f.iter().rev())
                .map(|[s, t]| format!("({s}, {t})"));
            self.array(depth, "texCoord2f[] primvars:st", st);
            self.line(depth, "(interpolation = \"faceVarying\")");
        }
        Ok(())
    }

    fn materials(&mut self) -> Result<(), Error> {
        if self.materials.is_empty() {
            return Ok(());
        }
        self.line(0, "def Scope \"ForgeMaterials\" {");
        for id in std::mem::take(&mut self.materials) {
            let mat = self.scene.node(id).expect("known material");
            let name = quote(&mat.name)?;
            let color = match self.scene.param(id, "color.color") {
                Some(Value::Color(color)) => DVec3::from_array(*color),
                _ => DVec3::splat(0.18),
            };
            let roughness = match self.scene.param(id, "color.roughness") {
                Some(Value::Float(v)) => v.clamp(0.0, 1.0),
                _ => 0.5,
            };
            self.line(
                1,
                format_args!("def Material \"M{}\" (displayName = {name}) {{", id.0),
            );
            self.line(
                2,
                format_args!(
                    "token outputs:surface.connect = </ForgeMaterials/M{}/Preview.outputs:surface>",
                    id.0
                ),
            );
            self.line(2, "def Shader \"Preview\" {");
            self.line(3, "uniform token info:id = \"UsdPreviewSurface\"");
            self.line(
                3,
                format_args!("color3f inputs:diffuseColor = {}", xyz(color)),
            );
            self.line(3, format_args!("float inputs:roughness = {roughness}"));
            self.line(3, "token outputs:surface");
            self.line(2, "}");
            self.line(1, "}");
        }
        self.line(0, "}");
        Ok(())
    }

    fn array(&mut self, depth: usize, label: &str, values: impl IntoIterator<Item = impl Display>) {
        self.line(depth, format_args!("{label} = ["));
        for value in values {
            self.line(depth + 1, format_args!("{value},"));
        }
        self.line(depth, "]");
    }

    fn line(&mut self, depth: usize, value: impl Display) {
        writeln!(self.out, "{:width$}{value}", "", width = 4 * depth).expect("write to String");
    }
}

fn xyz(v: DVec3) -> impl Display {
    struct Vector(DVec3);
    impl Display for Vector {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let [x, y, z] = self.0.to_array();
            write!(f, "({x}, {y}, {z})")
        }
    }
    Vector(v)
}

fn quote(s: &str) -> Result<String, Error> {
    serde_json::to_string(s).map_err(|e| invalid(e.to_string()))
}
