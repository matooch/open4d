//! Alembic core headers, sampled properties, and import to native tracks.

use std::{collections::BTreeMap, sync::Arc};

use forge_core::{
    EditError, Interp, Key, MeshData, NodeId, Track, Tx, Value,
    transform::{POS, ROT, SCALE, Transform},
};
use glam::{DMat4, DVec3};

use super::ogawa::{Cursor, Group, Reader};
use super::{
    Error, MAX_BYTES, MAX_DEPTH, MAX_FRAMES, MAX_OBJECTS, checked_scale, flip, invalid, point, trs,
};

struct SampleTime {
    period: f64,
    times: Vec<f64>,
}

impl SampleTime {
    fn at(&self, i: usize) -> Result<f64, Error> {
        let t = if self.period == f64::MAX / 32.0 {
            *self
                .times
                .get(i)
                .ok_or_else(|| invalid("acyclic sample index out of range"))?
        } else {
            self.times[i % self.times.len()] + self.period * (i / self.times.len()) as f64
        };
        t.is_finite()
            .then_some(t)
            .ok_or_else(|| invalid("non-finite sample time"))
    }
}

struct Property<'a> {
    kind: u32,
    pod: u32,
    extent: usize,
    count: usize,
    first: usize,
    last: usize,
    time: usize,
    meta: &'a str,
    group: u64,
}

impl Property<'_> {
    fn constant(&self) -> bool {
        self.count <= 1 || self.last == 0
    }

    fn physical(&self, logical: usize) -> Result<usize, Error> {
        if logical >= self.count {
            return Err(invalid("invalid property sample index"));
        }
        if self.constant() || logical < self.first {
            return Ok(0);
        }
        if self.first == 0 || self.first > self.last || self.last >= self.count {
            return Err(invalid("invalid repeated sample interval"));
        }
        Ok(logical.min(self.last) - self.first + 1)
    }

    fn expect(&self, kind: u32, extent: usize, pod: u32) -> Result<(), Error> {
        if self.kind != kind || self.extent != extent || self.pod != pod {
            return Err(invalid("unexpected Alembic property data type"));
        }
        Ok(())
    }
}

type Properties<'a> = BTreeMap<&'a str, Property<'a>>;
type Pose = (f64, Transform);

struct Unpack<'a> {
    ogawa: Reader<'a>,
    meta: Vec<&'a str>,
    time: Vec<SampleTime>,
    scale: f64,
    objects: usize,
    decoded: usize,
}

fn required<'a, 'b>(map: &'b Properties<'a>, key: &str) -> Result<&'b Property<'a>, Error> {
    map.get(key)
        .ok_or_else(|| invalid("missing Alembic schema property"))
}

impl<'a> Unpack<'a> {
    fn charge(&mut self, bytes: usize) -> Result<(), Error> {
        self.decoded = self
            .decoded
            .checked_add(bytes)
            .filter(|n| *n <= MAX_BYTES)
            .ok_or_else(|| invalid("decoded sample budget exceeded"))?;
        Ok(())
    }

    fn metadata(&self, id: usize, c: &mut Cursor<'a>, width: u32) -> Result<&'a str, Error> {
        if id == 255 {
            let n = c.width(width)?;
            c.string(n)
        } else {
            self.meta
                .get(id)
                .copied()
                .ok_or_else(|| invalid("invalid metadata index"))
        }
    }

    fn props(&self, ptr: u64) -> Result<Properties<'a>, Error> {
        let group = self.ogawa.group(ptr)?;
        let mut props = BTreeMap::new();
        if group.len() == 0 {
            return Ok(props);
        }
        let mut c = Cursor(self.ogawa.data(group.at(group.len() - 1)?)?);
        while !c.0.is_empty() {
            let bits = c.u32()?;
            let kind = (bits & 3).min(2);
            let w = (bits >> 2) & 3;
            let count = if kind == 0 { 0 } else { c.width(w)? };
            let (first, last) = if kind != 0 && bits & 0x200 != 0 {
                (c.width(w)?, c.width(w)?)
            } else if kind == 0 || bits & 0x800 != 0 {
                (0, 0)
            } else {
                (1, count.saturating_sub(1))
            };
            let time = if kind != 0 && bits & 0x100 != 0 {
                c.width(w)?
            } else {
                0
            };
            let n = c.width(w)?;
            let name = c.string(n)?;
            let meta = self.metadata(((bits >> 20) & 255) as usize, &mut c, w)?;
            if name.is_empty()
                || props.len() >= (group.len() - 1).min(MAX_OBJECTS)
                || time >= self.time.len()
            {
                return Err(invalid("invalid compound property header"));
            }
            let p = Property {
                kind,
                pod: (bits >> 4) & 15,
                extent: ((bits >> 12) & 255) as usize,
                count,
                first,
                last,
                time,
                meta,
                group: group.at(props.len())?,
            };
            if props.insert(name, p).is_some() {
                return Err(invalid("duplicate Alembic property"));
            }
        }
        if props.len() != group.len() - 1 {
            return Err(invalid("compound property count mismatch"));
        }
        Ok(props)
    }

    fn body(&self, p: &Property<'_>, sample: usize, pod_bytes: usize) -> Result<&'a [u8], Error> {
        let g = self.ogawa.group(p.group)?;
        let i = p.physical(sample)?;
        let stride = pod_bytes
            .checked_mul(p.extent)
            .filter(|s| *s != 0)
            .ok_or_else(|| invalid("invalid sample stride"))?;
        let index = if p.kind == 2 {
            i.checked_mul(2)
                .ok_or_else(|| invalid("array sample index overflow"))?
        } else {
            i
        };
        let bytes = self.ogawa.data(g.at(index)?)?;
        let data = if p.kind == 2 && bytes.is_empty() {
            bytes
        } else {
            bytes
                .get(16..)
                .ok_or_else(|| invalid("sample missing its content key"))?
        };
        if (p.kind == 1 && data.len() != stride)
            || !data.len().is_multiple_of(stride)
            || data.len() > MAX_BYTES
        {
            return Err(invalid("property sample size mismatch"));
        }
        if p.kind == 2 {
            let dims = self.ogawa.data(g.at(index + 1)?)?;
            if !dims.is_empty() {
                let mut c = Cursor(dims);
                let mut n = 1usize;
                while !c.0.is_empty() {
                    let d = usize::try_from(c.u64()?)
                        .map_err(|_| invalid("array dimension overflow"))?;
                    n = n
                        .checked_mul(d)
                        .ok_or_else(|| invalid("array element count overflow"))?;
                }
                if n != data.len() / stride {
                    return Err(invalid("array dimensions disagree with payload"));
                }
            }
        }
        Ok(data)
    }

    fn doubles(&self, p: &Property<'_>, i: usize) -> Result<Vec<f64>, Error> {
        if p.pod != 11 || !(1..=2).contains(&p.kind) {
            return Err(invalid("expected a double property"));
        }
        Ok(self
            .body(p, i, 8)?
            .as_chunks::<8>()
            .0
            .iter()
            .copied()
            .map(f64::from_le_bytes)
            .collect())
    }

    fn times(&self, p: &Property<'_>) -> Result<Vec<f64>, Error> {
        let n = if p.constant() { 1 } else { p.count };
        if p.count == 0 || n > MAX_FRAMES {
            return Err(invalid("invalid property sample count"));
        }
        let times: Vec<_> = (0..n)
            .map(|i| self.time[p.time].at(i))
            .collect::<Result<_, _>>()?;
        if times.windows(2).any(|v| v[0] >= v[1]) {
            return Err(invalid("non-increasing sample times"));
        }
        Ok(times)
    }

    fn xform(&mut self, ptr: u64) -> Result<Vec<Pose>, Error> {
        let props = self.props(ptr)?;
        if let Some(p) = props.get(".inherits") {
            p.expect(1, 1, 0)?;
            if !p.constant() || self.body(p, 0, 1)? != [1] {
                return Err(Error::Unsupported("non-inherited transform"));
            }
        }
        let Some(values) = props.get(".vals") else {
            return Ok(vec![(0.0, Transform::IDENTITY)]);
        };
        let ops = required(&props, ".ops")?;
        if ops.kind != 1 || ops.pod != 1 || !ops.constant() {
            return Err(Error::Unsupported("changing transform operations"));
        }
        let ops = self.body(ops, 0, 1)?;
        let times = self.times(values)?;
        self.charge(times.len().saturating_mul(std::mem::size_of::<Pose>()))?;
        times
            .into_iter()
            .enumerate()
            .map(|(i, t)| {
                let values = self.doubles(values, i)?;
                let matrix = operation_stack(ops, &values)?;
                Ok((t, trs(flip(matrix, self.scale))?))
            })
            .collect()
    }

    fn indices(&self, p: &Property<'_>, pod: u32) -> Result<Vec<u32>, Error> {
        p.expect(2, 1, pod)?;
        if !p.constant() {
            return Err(Error::Unsupported("changing topology or UV indexing"));
        }
        let bytes = self.body(p, 0, 4)?;
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .copied()
            .map(|b| {
                let n = u32::from_le_bytes(b);
                if pod == 6 && n > i32::MAX as u32 {
                    Err(invalid("negative mesh index or face count"))
                } else {
                    Ok(n)
                }
            })
            .collect()
    }

    fn points(&mut self, p: &Property<'_>, i: usize) -> Result<Vec<[f64; 3]>, Error> {
        let bytes = self.body(p, i, 4)?;
        self.charge(bytes.len().saturating_mul(2))?;
        bytes
            .as_chunks::<12>()
            .0
            .iter()
            .map(|chunk| {
                let raw = chunk.as_chunks::<4>().0;
                let pos = point(
                    DVec3::new(
                        f64::from(f32::from_le_bytes(raw[0])),
                        f64::from(f32::from_le_bytes(raw[1])),
                        f64::from(f32::from_le_bytes(raw[2])),
                    ),
                    self.scale,
                );
                pos.is_finite()
                    .then_some(pos.to_array())
                    .ok_or_else(|| invalid("non-finite mesh point"))
            })
            .collect()
    }

    fn mesh(&mut self, props: &Properties<'a>) -> Result<Shape, Error> {
        let positions = required(props, "P")?;
        positions.expect(2, 3, 10)?;
        let inds = self.indices(required(props, ".faceIndices")?, 6)?;
        let sizes = self.indices(required(props, ".faceCounts")?, 6)?;
        self.charge(
            inds.len()
                .saturating_mul(4)
                .saturating_add(sizes.len().saturating_mul(std::mem::size_of::<Vec<u32>>())),
        )?;
        let points = self.points(positions, 0)?;
        let mut polys = Vec::new();
        let mut tail = inds.as_slice();
        for n in sizes {
            let (head, rest) = tail
                .split_at_checked(n as usize)
                .ok_or_else(|| invalid("invalid polygon face counts"))?;
            if n < 3 || head.iter().any(|i| *i as usize >= points.len()) {
                return Err(invalid("invalid polygon corners"));
            }
            polys.push(head.to_vec());
            tail = rest;
        }
        if !tail.is_empty() {
            return Err(invalid("unclaimed face indices"));
        }
        let uv = if let Some(uv) = props.get("uv") {
            self.charge(inds.len().saturating_mul(16))?;
            Some(self.uv(uv, &polys)?)
        } else {
            None
        };
        let mut anim = None;
        if !positions.constant() {
            let mut track = Track::default();
            self.charge(points.len().saturating_mul(24))?;
            for (i, t) in self.times(positions)?.into_iter().enumerate() {
                let sample = if i == 0 {
                    points.clone()
                } else {
                    self.points(positions, i)?
                };
                if !track.set_points_key(t, Interp::Linear, sample) {
                    return Err(Error::Unsupported("changing point counts"));
                }
            }
            anim = Some(track);
        }
        Ok(Shape::Mesh(MeshShape {
            mesh: Arc::new(MeshData { points, polys }),
            uv,
            anim,
        }))
    }

    fn uv(&self, p: &Property<'_>, polys: &[Vec<u32>]) -> Result<Vec<f64>, Error> {
        let indexed;
        let (values, indices) = if p.kind == 0 {
            indexed = self.props(p.group)?;
            let values = required(&indexed, ".vals")?;
            let indices = self.indices(required(&indexed, ".indices")?, 5)?;
            (values, Some(indices))
        } else {
            (p, None)
        };
        values.expect(2, 2, 10)?;
        if !values.constant() {
            return Err(Error::Unsupported("animated UV values"));
        }
        let storage = self.body(values, 0, 4)?.as_chunks::<8>().0;
        let scope = token(p.meta, "geoScope")
            .or_else(|| token(values.meta, "geoScope"))
            .unwrap_or("fvr");
        let mut result = Vec::new();
        let mut corner = 0;
        for (i, poly) in polys.iter().enumerate() {
            for vertex in poly {
                let index = match scope {
                    "fvr" => corner,
                    "vtx" | "var" => *vertex as usize,
                    "uni" => i,
                    "con" => 0,
                    _ => return Err(Error::Unsupported("UV geometry scope")),
                };
                let index = if let Some(indices) = &indices {
                    *indices
                        .get(index)
                        .ok_or_else(|| invalid("missing UV index"))? as usize
                } else {
                    index
                };
                let u = storage
                    .get(index)
                    .ok_or_else(|| invalid("UV index outside array"))?;
                for c in u.as_chunks::<4>().0.iter().copied() {
                    let n = f64::from(f32::from_le_bytes(c));
                    if !n.is_finite() {
                        return Err(invalid("non-finite UV value"));
                    }
                    result.push(n);
                }
                corner += 1;
            }
        }
        Ok(result)
    }

    fn camera(&self, props: &Properties<'a>) -> Result<Shape, Error> {
        let core = required(props, ".core")?;
        core.expect(1, 16, 11)?;
        if !core.constant() {
            return Err(Error::Unsupported("animated camera lens"));
        }
        let values = self.doubles(core, 0)?;
        let focal = values[0];
        let sensor = values[1] * 10.0;
        let focus = values[11] * self.scale;
        if ![focal, sensor, focus].into_iter().all(f64::is_finite) || focal <= 0.0 || sensor <= 0.0
        {
            return Err(invalid("invalid camera optics"));
        }
        Ok(Shape::Camera {
            sensor,
            fov: 2.0 * (0.5 * sensor / focal).atan(),
            focus,
        })
    }

    fn children(&mut self, group: Group<'a>, depth: usize) -> Result<Vec<Object>, Error> {
        if group.len() < 2 || depth > MAX_DEPTH {
            return Err(invalid("invalid or excessively deep object hierarchy"));
        }
        let raw = self.ogawa.data(group.at(group.len() - 1)?)?;
        let trim = raw
            .len()
            .checked_sub(32)
            .ok_or_else(|| invalid("missing object digest trailer"))?;
        let mut c = Cursor(&raw[..trim]);
        let mut children = Vec::new();
        let mut i = 1;
        while !c.0.is_empty() {
            let n = c.u32()? as usize;
            let name = c.string(n)?;
            let meta = c.u8()? as usize;
            self.metadata(meta, &mut c, 2)?;
            if name.is_empty() || i >= group.len() - 1 || self.objects >= MAX_OBJECTS {
                return Err(invalid("invalid or oversized object header table"));
            }
            self.objects += 1;
            if let Some(obj) = self.object(name, group.at(i)?, depth)? {
                children.push(obj);
            }
            i += 1;
        }
        if i != group.len() - 1 {
            return Err(invalid("object header count mismatch"));
        }
        Ok(children)
    }

    fn object(&mut self, name: &str, ptr: u64, depth: usize) -> Result<Option<Object>, Error> {
        let group = self.ogawa.group(ptr)?;
        let props = self.props(group.at(0)?)?;
        let poses = props
            .get(".xform")
            .map(|x| self.xform(x.group))
            .transpose()?
            .unwrap_or_default();
        let mut shape = Shape::None;
        if let Some(geom) = props.get(".geom") {
            match token(geom.meta, "schema") {
                Some("AbcGeom_PolyMesh_v1") => shape = self.mesh(&self.props(geom.group)?)?,
                Some("AbcGeom_Camera_v1") => shape = self.camera(&self.props(geom.group)?)?,
                _ => {}
            }
        }
        let mut children = self.children(group, depth + 1)?;
        if !poses.is_empty() && matches!(shape, Shape::None) {
            let mut candidates = children.iter().enumerate().filter(|(_, child)| {
                child.poses.is_empty()
                    && child.children.is_empty()
                    && !matches!(child.shape, Shape::None)
            });
            if let (Some((i, _)), None) = (candidates.next(), candidates.next()) {
                shape = children.remove(i).shape;
            }
        }
        if matches!(shape, Shape::None) && poses.is_empty() && children.is_empty() {
            return Ok(None);
        }
        Ok(Some(Object {
            name: name.into(),
            poses,
            shape,
            children,
        }))
    }
}

fn token<'a>(meta: &'a str, key: &str) -> Option<&'a str> {
    meta.split(';')
        .filter_map(|t| t.split_once('='))
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
}

fn operation_stack(ops: &[u8], values: &[f64]) -> Result<DMat4, Error> {
    let mut rest = values;
    let mut world = DMat4::IDENTITY;
    if values.iter().any(|n| !n.is_finite()) {
        return Err(invalid("non-finite transform channel"));
    }
    for op in ops {
        let kind = op >> 4;
        let n = match kind {
            0 | 1 => 3,
            2 => 4,
            3 => 16,
            4..=6 => 1,
            _ => return Err(Error::Unsupported("unknown Xform operation")),
        };
        let (v, tail) = rest
            .split_at_checked(n)
            .ok_or_else(|| invalid("truncated Xform channels"))?;
        rest = tail;
        let m = match kind {
            0 => DMat4::from_scale(DVec3::from_slice(v)),
            1 => DMat4::from_translation(DVec3::from_slice(v)),
            2 => {
                let axis = DVec3::from_slice(v).try_normalize();
                if v[3] == 0.0 {
                    DMat4::IDENTITY
                } else {
                    DMat4::from_axis_angle(
                        axis.ok_or_else(|| invalid("invalid rotation axis"))?,
                        v[3].to_radians(),
                    )
                }
            }
            // Imath's row-vector matrix is the transpose of our column-vector matrix.
            3 => DMat4::from_cols_array(v.try_into().expect("sixteen channels")),
            4 => DMat4::from_rotation_x(v[0].to_radians()),
            5 => DMat4::from_rotation_y(v[0].to_radians()),
            _ => DMat4::from_rotation_z(v[0].to_radians()),
        };
        world *= m;
    }
    if !rest.is_empty() {
        return Err(invalid("unexpected extra Xform channels"));
    }
    Ok(world)
}

struct MeshShape {
    mesh: Arc<MeshData>,
    uv: Option<Vec<f64>>,
    anim: Option<Track>,
}

enum Shape {
    None,
    Mesh(MeshShape),
    Camera { sensor: f64, fov: f64, focus: f64 },
}

struct Object {
    name: String,
    poses: Vec<Pose>,
    shape: Shape,
    children: Vec<Object>,
}

impl Object {
    fn apply(&self, tx: &mut Tx<'_>, parent: Option<NodeId>) -> Result<NodeId, EditError> {
        let ty = match self.shape {
            Shape::None => "forge.null",
            Shape::Mesh(_) => "forge.polygon",
            Shape::Camera { .. } => "forge.camera",
        };
        let id = tx.create(ty, parent, None)?;
        tx.rename(id, &self.name)?;
        match &self.shape {
            Shape::None => {}
            Shape::Mesh(shape) => {
                tx.set_param(id, "mesh", Value::Mesh(shape.mesh.clone()))?;
                if let Some(uv) = &shape.uv {
                    let tag = tx.attach_tag("forge.uvw", id, None)?;
                    tx.set_param(tag, "uvw", Value::FloatList(uv.clone()))?;
                }
                if let Some(anim) = &shape.anim {
                    tx.set_track(id, "mesh.points", Some(anim.clone()))?;
                }
            }
            Shape::Camera { sensor, fov, focus } => {
                tx.set_param(id, "sensor_size", Value::Float(*sensor))?;
                tx.set_param(id, "fov_horizontal", Value::Float(*fov))?;
                tx.set_param(id, "focus_distance", Value::Float(*focus))?;
            }
        }
        if let Some((_, first)) = self.poses.first() {
            for (param, initial, get) in [
                (
                    POS,
                    first.pos,
                    (|t: Transform| t.pos) as fn(Transform) -> DVec3,
                ),
                (ROT, first.rot, |t: Transform| t.rot),
                (SCALE, first.scale, |t: Transform| t.scale),
            ] {
                tx.set_param(id, param, Value::Vec3(initial.to_array()))?;
                for (axis, suffix) in ["x", "y", "z"].into_iter().enumerate() {
                    let mut last = initial[axis];
                    let keys: Vec<_> = self
                        .poses
                        .iter()
                        .map(|(t, pose)| {
                            let mut v = get(*pose)[axis];
                            if param == ROT {
                                let delta = (v - last) / std::f64::consts::TAU;
                                v -= delta.round() * std::f64::consts::TAU;
                            }
                            last = v;
                            Key::new(*t, v).with_interp(Interp::Linear)
                        })
                        .collect();
                    if keys.iter().any(|k| k.value != initial[axis]) {
                        tx.set_track(id, &format!("{param}.{suffix}"), Some(Track::new(keys)))?;
                    }
                }
            }
        }
        for child in &self.children {
            child.apply(tx, Some(id))?;
        }
        Ok(id)
    }
}

/// A decoded archive, ready for one undoable scene insertion.
pub struct ImportScene(Vec<Object>);

impl ImportScene {
    pub fn apply(&self, tx: &mut Tx<'_>) -> Result<Vec<NodeId>, EditError> {
        self.0.iter().map(|node| node.apply(tx, None)).collect()
    }
}

pub fn read(bytes: &[u8], scale: f64) -> Result<ImportScene, Error> {
    checked_scale(scale)?;
    if bytes.len() > MAX_BYTES {
        return Err(invalid("archive exceeds file size limit"));
    }
    let (ogawa, root) = Reader::open(bytes)?;
    if root.len() < 6
        || ogawa.data(root.at(0)?)? != [0; 4]
        || Cursor(ogawa.data(root.at(1)?)?).u32()? < 9999
    {
        return Err(Error::Unsupported("Alembic core layout"));
    }
    let mut meta = vec![""];
    let mut c = Cursor(ogawa.data(root.at(5)?)?);
    while !c.0.is_empty() {
        if meta.len() >= 255 {
            return Err(invalid("too many metadata entries"));
        }
        let n = c.u8()?;
        meta.push(c.string(n.into())?);
    }
    let mut time = Vec::new();
    let mut c = Cursor(ogawa.data(root.at(4)?)?);
    while !c.0.is_empty() {
        c.u32()?; // advertised maximum sample count is advisory
        let period = c.f64()?;
        let n = c.u32()? as usize;
        if !period.is_finite()
            || period <= 0.0
            || n == 0
            || n > MAX_FRAMES
            || time.len() >= MAX_FRAMES
        {
            return Err(invalid("invalid time sampling"));
        }
        let times = (0..n).map(|_| c.f64()).collect::<Result<_, _>>()?;
        time.push(SampleTime { period, times });
    }
    if time.is_empty() {
        return Err(invalid("missing time sampling"));
    }
    let top = ogawa.group(root.at(2)?)?;
    let mut decode = Unpack {
        ogawa,
        meta,
        time,
        scale,
        objects: 0,
        decoded: 0,
    };
    Ok(ImportScene(decode.children(top, 0)?))
}
