//! Canonical Ogawa blocks for standard Xform, PolyMesh and Camera schemas.

use std::collections::BTreeSet;

use forge_core::{EvalCtx, NodeId, Scene, Value};
use forge_geometry::{CornerUvs, PolyMesh};
use glam::DVec3;

use super::ogawa::{DATA, Writer};
use super::{
    CAMERA, Error, MAX_BYTES, MAX_DEPTH, MAX_FRAMES, MAX_OBJECTS, MESH, POINT, UV, XFORM,
    checked_scale, flip, invalid, point, trs,
};

// The sample key is a deterministic content id; Ogawa readers use it for lookup,
// not checksum verification. Two independent byte orders distinguish samples.
fn key(bytes: &[u8]) -> [u8; 16] {
    let hash = |bytes: &mut dyn Iterator<Item = &u8>, seed: u64| {
        bytes.fold(seed, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100000001b3))
    };
    let a = hash(&mut bytes.iter(), 0xcbf29ce484222325);
    let b = hash(&mut bytes.iter().rev(), 0x9e3779b97f4a7c15);
    let mut out = [0; 16];
    out[..8].copy_from_slice(&a.to_le_bytes());
    out[8..].copy_from_slice(&b.to_le_bytes());
    out
}

#[derive(Default)]
struct Series {
    last: Vec<u8>,
    ptrs: Vec<u64>,
}

impl Series {
    fn push(&mut self, writer: &mut Writer, data: Vec<u8>) -> Result<(), Error> {
        let ptr = match self.ptrs.last() {
            Some(ptr) if data == self.last => *ptr,
            _ => {
                if data.len() > MAX_BYTES.saturating_sub(writer.len()).saturating_sub(24) {
                    return Err(invalid("archive exceeds file size limit"));
                }
                let mut sample = key(&data).to_vec();
                sample.extend_from_slice(&data);
                self.last = data;
                writer.data(&sample)
            }
        };
        self.ptrs.push(ptr);
        Ok(())
    }

    fn property(
        self,
        writer: &mut Writer,
        name: &'static str,
        pod: u32,
        extent: u32,
        array: bool,
        meta: &'static str,
    ) -> Property {
        let constant = self.ptrs.iter().all(|p| *p == self.ptrs[0]);
        let slots = if constant {
            &self.ptrs[..1]
        } else {
            self.ptrs.as_slice()
        };
        let group = if array {
            writer.group(&slots.iter().flat_map(|p| [*p, DATA]).collect::<Vec<_>>())
        } else {
            writer.group(slots)
        };
        let flags = if array { 2 } else { 1 } | (pod << 4) | (extent << 12) | 0x400;
        let flags = flags | if constant { 0x800 } else { 0x100 };
        Property {
            name,
            meta,
            group,
            flags,
            count: slots.len() as u32,
            time: u32::from(!constant),
        }
    }
}

struct Property {
    name: &'static str,
    meta: &'static str,
    group: u64,
    flags: u32,
    count: u32,
    time: u32,
}

impl Property {
    fn compound(
        writer: &mut Writer,
        name: &'static str,
        meta: &'static str,
        props: &[Self],
    ) -> Self {
        Self {
            name,
            meta,
            group: properties(writer, props),
            flags: 0,
            count: 0,
            time: 0,
        }
    }
}

fn string(out: &mut Vec<u8>, s: &str) {
    out.extend((s.len() as u32).to_le_bytes());
    out.extend(s.as_bytes());
}

fn properties(writer: &mut Writer, props: &[Property]) -> u64 {
    if props.is_empty() {
        return 0;
    }
    let mut table = Vec::new();
    let mut ptrs = Vec::new();
    for p in props {
        let flags = p.flags | 8 | if p.meta.is_empty() { 0 } else { 255 << 20 };
        table.extend(flags.to_le_bytes());
        if p.flags & 3 != 0 {
            table.extend(p.count.to_le_bytes());
            if p.time != 0 {
                table.extend(p.time.to_le_bytes());
            }
        }
        string(&mut table, p.name);
        if !p.meta.is_empty() {
            string(&mut table, p.meta);
        }
        ptrs.push(p.group);
    }
    ptrs.push(writer.data(&table));
    writer.group(&ptrs)
}

struct Written {
    name: String,
    meta: String,
    group: u64,
}

fn object(writer: &mut Writer, props: u64, children: Vec<Written>) -> u64 {
    let mut headers = Vec::new();
    let mut ptrs = vec![props];
    for child in children {
        string(&mut headers, &child.name);
        if child.meta.is_empty() {
            headers.push(0);
        } else {
            headers.push(255);
            string(&mut headers, &child.meta);
        }
        ptrs.push(child.group);
    }
    // Two content-key trailers describe properties and the subtree. These do not
    // participate in locating child records or in decoding properties.
    headers.extend(key(&props.to_le_bytes()));
    let children: Vec<_> = ptrs[1..].iter().flat_map(|p| p.to_le_bytes()).collect();
    headers.extend(key(&children));
    ptrs.push(writer.data(&headers));
    writer.group(&ptrs)
}

fn unique(name: &str, names: &mut BTreeSet<String>) -> String {
    let name = if name.is_empty() {
        "Object".into()
    } else {
        name.replace(['/', '\0'], "_")
    };
    if names.insert(name.clone()) {
        return name;
    }
    for i in 2.. {
        let candidate = format!("{name}_{i}");
        if names.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!()
}

struct Mesh {
    count: usize,
    polys: Vec<Vec<u32>>,
    uv: Option<CornerUvs>,
    positions: Series,
    bounds: Series,
}

impl Mesh {
    fn push(
        &mut self,
        writer: &mut Writer,
        mesh: &PolyMesh,
        scale: f64,
        uv: Option<CornerUvs>,
    ) -> Result<(), Error> {
        if mesh.points.len() != self.count || mesh.polys != self.polys {
            return Err(Error::Unsupported("changing mesh topology"));
        }
        if uv != self.uv {
            return Err(Error::Unsupported("changing UV values"));
        }
        let mut p = Vec::new();
        let (mut min, mut max) = (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY));
        for v in &mesh.points {
            let v = point(*v, scale);
            min = min.min(v);
            max = max.max(v);
            let components = v.to_array().map(|v| v as f32);
            if !components.into_iter().all(f32::is_finite) {
                return Err(invalid("mesh point exceeds the Alembic float range"));
            }
            p.extend(components.into_iter().flat_map(f32::to_le_bytes));
        }
        self.positions.push(writer, p)?;
        self.bounds.push(
            writer,
            doubles(min.to_array().into_iter().chain(max.to_array())),
        )
    }

    fn finish(self, writer: &mut Writer) -> Result<Property, Error> {
        let mut indices = Series::default();
        let mut counts = Series::default();
        indices.push(
            writer,
            self.polys
                .iter()
                .flatten()
                .flat_map(|i| i.to_le_bytes())
                .collect(),
        )?;
        counts.push(
            writer,
            self.polys
                .iter()
                .flat_map(|p| (p.len() as u32).to_le_bytes())
                .collect(),
        )?;
        let mut props = vec![
            self.bounds
                .property(writer, ".selfBnds", 11, 6, false, "interpretation=box"),
            self.positions.property(writer, "P", 10, 3, true, POINT),
            indices.property(writer, ".faceIndices", 6, 1, true, ""),
            counts.property(writer, ".faceCounts", 6, 1, true, ""),
        ];
        if let Some(uv) = self.uv {
            let mut data = Vec::new();
            for v in uv.corners.iter().flatten().flatten() {
                let v = *v as f32;
                if !v.is_finite() {
                    return Err(invalid("UV exceeds the Alembic float range"));
                }
                data.extend(v.to_le_bytes());
            }
            let mut uv = Series::default();
            uv.push(writer, data)?;
            props.push(uv.property(writer, "uv", 10, 2, true, UV));
        }
        Ok(Property::compound(writer, ".geom", MESH, &props))
    }
}

enum Shape {
    Empty,
    Mesh(Mesh),
    Camera([f64; 16]),
}

fn camera(scene: &Scene, id: NodeId, scale: f64) -> Result<[f64; 16], Error> {
    if !matches!(scene.param(id, "projection"), Some(Value::Enum(p)) if p == "perspective") {
        return Err(Error::Unsupported("non-perspective camera"));
    }
    let float = |param| match scene.param(id, param) {
        Some(Value::Float(v)) => Ok(*v),
        _ => Err(invalid("missing camera parameter")),
    };
    let sensor = float("sensor_size")?;
    let focal = 0.5 * sensor / (0.5 * float("fov_horizontal")?).tan();
    let focus = float("focus_distance")? * scale;
    if ![sensor, focal, focus].into_iter().all(f64::is_finite) || focal <= 0.0 {
        return Err(invalid("invalid camera optics"));
    }
    Ok([
        focal,
        sensor / 10.0,
        0.0,
        sensor / 15.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        8.0,
        focus,
        0.0,
        0.0,
        0.1,
        100000.0,
    ])
}

fn doubles(values: impl IntoIterator<Item = f64>) -> Vec<u8> {
    values.into_iter().flat_map(f64::to_le_bytes).collect()
}

struct Node {
    id: NodeId,
    name: String,
    detached: bool,
    xform: Series,
    shape: Shape,
    kids: Vec<Node>,
}

impl Node {
    fn collect(
        scene: &Scene,
        id: NodeId,
        depth: usize,
        names: &mut BTreeSet<String>,
        budget: &mut usize,
    ) -> Result<Option<Self>, Error> {
        let Some(node) = scene.node(id) else {
            return Ok(None);
        };
        if node.is_tag()
            || !scene
                .node_type(id)
                .is_some_and(|ty| ty.category.is_hierarchy())
            || forge_generators::is_consumed(scene, id)
        {
            return Ok(None);
        }
        if depth > MAX_DEPTH || *budget == 0 {
            return Err(invalid("oversized export hierarchy"));
        }
        *budget -= 1;
        let mut child_names = BTreeSet::new();
        let kids = node
            .children
            .iter()
            .map(|id| Self::collect(scene, *id, depth + 1, &mut child_names, budget))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(Some(Self {
            id,
            name: unique(&node.name, names),
            detached: depth == 0 && node.parent.is_some(),
            xform: Series::default(),
            shape: Shape::Empty,
            kids,
        }))
    }

    fn sample(
        &mut self,
        evaluated: &Scene,
        ctx: &EvalCtx,
        writer: &mut Writer,
        scale: f64,
        initial: bool,
    ) -> Result<(), Error> {
        let mat = if self.detached {
            evaluated.world_matrix(self.id)
        } else {
            evaluated.local_transform(self.id).map(|t| t.matrix())
        }
        .ok_or_else(|| invalid("missing export transform"))?;
        let matrix = flip(mat, scale);
        trs(matrix)?;
        self.xform.push(writer, doubles(matrix.to_cols_array()))?;
        if evaluated
            .node(self.id)
            .is_some_and(|n| n.type_id == "forge.camera")
        {
            let camera = camera(evaluated, self.id, scale)?;
            if initial {
                self.shape = Shape::Camera(camera);
            } else if !matches!(self.shape, Shape::Camera(v) if v == camera) {
                return Err(Error::Unsupported("animated lens"));
            }
        } else {
            let mesh = forge_generators::evaluate(evaluated, self.id, ctx)
                .map_err(|_| Error::Unsupported("a scene generator could not be evaluated"))?
                .filter(|mesh| !mesh.polys.is_empty());
            if let Some(mesh) = mesh {
                let uv = crate::scene::node_uvs(evaluated, self.id, &mesh);
                if initial {
                    if mesh.points.len() > i32::MAX as usize
                        || mesh.polys.iter().any(|p| {
                            p.len() < 3
                                || p.len() > i32::MAX as usize
                                || p.iter().any(|i| *i as usize >= mesh.points.len())
                        })
                    {
                        return Err(invalid("invalid export polygon topology"));
                    }
                    self.shape = Shape::Mesh(Mesh {
                        count: mesh.points.len(),
                        polys: mesh.polys.clone(),
                        uv: uv.clone(),
                        positions: Series::default(),
                        bounds: Series::default(),
                    });
                }
                let Shape::Mesh(shape) = &mut self.shape else {
                    return Err(Error::Unsupported("changing mesh presence"));
                };
                shape.push(writer, &mesh, scale, uv)?;
            } else if matches!(self.shape, Shape::Mesh(_)) {
                return Err(Error::Unsupported("changing mesh presence"));
            }
        }
        for child in &mut self.kids {
            child.sample(evaluated, ctx, writer, scale, initial)?;
        }
        Ok(())
    }

    fn finish(self, writer: &mut Writer, count: &mut usize) -> Result<Written, Error> {
        let Self {
            name,
            xform,
            shape,
            kids,
            ..
        } = self;
        let props = {
            let mut ops = Series::default();
            ops.push(writer, vec![0x30])?;
            let mut flag = Series::default();
            flag.push(writer, vec![1])?;
            let props = [
                flag.clone_for(writer, ".inherits", 0),
                ops.property(writer, ".ops", 1, 1, false, ""),
                xform.property(writer, ".vals", 11, 16, false, ""),
                flag.property(writer, "isNotConstantIdentity", 0, 1, false, ""),
            ];
            let prop = Property::compound(writer, ".xform", XFORM, &props);
            properties(writer, &[prop])
        };
        let mut children: Vec<_> = kids
            .into_iter()
            .map(|c| c.finish(writer, count))
            .collect::<Result<_, _>>()?;
        let geom = match shape {
            Shape::Empty => None,
            Shape::Mesh(mesh) => Some((mesh.finish(writer)?, "AbcGeom_PolyMesh_v1")),
            Shape::Camera(data) => {
                let mut series = Series::default();
                series.push(writer, doubles(data))?;
                let core = series.property(writer, ".core", 11, 16, false, "");
                Some((
                    Property::compound(writer, ".geom", CAMERA, &[core]),
                    "AbcGeom_Camera_v1",
                ))
            }
        };
        if let Some((geom, schema)) = geom {
            *count += 1;
            let meta = format!("{};schemaObjTitle={schema}:.geom", geom.meta);
            let geom_props = properties(writer, &[geom]);
            let mut names = children.iter().map(|child| child.name.clone()).collect();
            let shape_name = unique(&format!("{name}Shape"), &mut names);
            children.insert(
                0,
                Written {
                    name: shape_name,
                    meta,
                    group: object(writer, geom_props, Vec::new()),
                },
            );
        }
        let group = object(writer, props, children);
        Ok(Written {
            name,
            meta: format!("{XFORM};schemaObjTitle=AbcGeom_Xform_v3:.xform"),
            group,
        })
    }
}

impl Series {
    fn clone_for(&self, writer: &mut Writer, name: &'static str, pod: u32) -> Property {
        Self {
            last: Vec::new(),
            ptrs: self.ptrs.clone(),
        }
        .property(writer, name, pod, 1, false, "")
    }
}

pub fn write(
    scene: &Scene,
    roots: Option<&[NodeId]>,
    scale: f64,
) -> Result<(Vec<u8>, usize), Error> {
    checked_scale(scale)?;
    let frames = scene
        .settings()
        .end
        .saturating_sub(scene.settings().start)
        .saturating_add(1);
    if frames <= 0 || frames > MAX_FRAMES as i64 {
        return Err(invalid("project exceeds Alembic sample limit"));
    }
    let selected: BTreeSet<_> = roots.unwrap_or(scene.roots()).iter().copied().collect();
    let mut names = BTreeSet::new();
    let mut budget = MAX_OBJECTS;
    let mut nodes = Vec::new();
    let mut seen = BTreeSet::new();
    for id in roots.unwrap_or(scene.roots()).iter().copied() {
        if !seen.insert(id) {
            continue;
        }
        let mut parent = scene.node(id).and_then(|n| n.parent);
        let mut covered = false;
        while let Some(id) = parent {
            if selected.contains(&id) {
                covered = true;
                break;
            }
            parent = scene.node(id).and_then(|n| n.parent);
        }
        if !covered && let Some(node) = Node::collect(scene, id, 0, &mut names, &mut budget)? {
            nodes.push(node);
        }
    }
    if nodes.is_empty() {
        return Err(Error::Unsupported("no exportable objects"));
    }
    if (MAX_OBJECTS - budget).saturating_mul(frames as usize) > 2_000_000 {
        return Err(invalid("too many object-frame samples"));
    }
    let mut writer = Writer::new();
    let fps = f64::from(scene.settings().fps);
    let start = scene.settings().start;
    let tracked = forge_anim::track_nodes(scene);
    for frame in start..=scene.settings().end {
        let ctx = EvalCtx::at_frame(frame, fps);
        let (evaluated, _) =
            scene.with_params(forge_anim::overrides_exact(scene, &tracked, ctx.time));
        for node in &mut nodes {
            node.sample(&evaluated, &ctx, &mut writer, scale, frame == start)?;
        }
    }
    let times = [
        (1u32, 1.0, 0.0),
        (frames as u32, 1.0 / fps, start as f64 / fps),
    ];
    let mut time_data = Vec::new();
    for (count, period, first) in times {
        time_data.extend(count.to_le_bytes());
        time_data.extend(period.to_le_bytes());
        time_data.extend(1u32.to_le_bytes());
        time_data.extend(first.to_le_bytes());
    }
    let mut count = 0;
    let children = nodes
        .into_iter()
        .map(|n| n.finish(&mut writer, &mut count))
        .collect::<Result<_, _>>()?;
    let top = object(&mut writer, 0, children);
    let root = [
        writer.data(&0u32.to_le_bytes()),
        writer.data(&10809u32.to_le_bytes()),
        top,
        writer.data(b"application=Forge3D"),
        writer.data(&time_data),
        DATA,
    ];
    let root = writer.group(&root);
    if writer.len() > MAX_BYTES {
        return Err(invalid("archive exceeds file size limit"));
    }
    Ok((writer.finish(root), count))
}
