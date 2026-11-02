//! Instance buffers for clones: layout, chunking and draw lists (docs/specs/VIEWPORT.md §10.1,
//! §10.5). Pure: no GPU, so it is testable and benchmarkable on its own.
//! Ref: Morton (1966) for the spatial order of the instances.

use bytemuck::{Pod, Zeroable};
use glam::{DMat4, DVec3};

use crate::camera::Camera;
use crate::cull::{Frustum, transform_aabb, union_aabb};
use crate::lod;

/// Instances per culling chunk.
pub const INSTANCE_CHUNK: usize = 2048;

/// What the GPU reads per instance: rows 0..3 of the affine matrix (xyz = linear part, w =
/// translation) and the clone colour.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct InstanceRaw {
    pub row0: [f32; 4],
    pub row1: [f32; 4],
    pub row2: [f32; 4],
    pub color: [f32; 4],
}

impl InstanceRaw {
    pub fn new(m: &DMat4, color: [f64; 3]) -> InstanceRaw {
        let m = m.as_mat4();
        let row = |i: usize| [m.x_axis[i], m.y_axis[i], m.z_axis[i], m.w_axis[i]];
        InstanceRaw {
            row0: row(0),
            row1: row(1),
            row2: row(2),
            color: [color[0] as f32, color[1] as f32, color[2] as f32, 1.0],
        }
    }
}

/// Borrowed per-clone arrays (the fields of `CloneData`); all the same length.
#[derive(Debug, Clone, Copy)]
pub struct InstanceInput<'a> {
    pub matrices: &'a [DMat4],
    pub colors: &'a [[f64; 3]],
    pub visible: &'a [bool],
    pub child: &'a [u32],
}

impl InstanceInput<'_> {
    pub fn len(&self) -> usize {
        self.matrices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.matrices.is_empty()
    }
}

/// A run of instances of one prototype that is culled as a unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstanceChunk {
    pub proto: u32,
    /// First instance slot and number of slots.
    pub first: u32,
    pub count: u32,
    /// World box of the member instances.
    pub min: DVec3,
    pub max: DVec3,
    /// Largest half-diagonal of a member's world box.
    pub max_radius: f64,
}

/// The instance buffer content and its chunks.
#[derive(Debug, Clone, Default)]
pub struct InstanceLayout {
    pub raw: Vec<InstanceRaw>,
    /// Clone index of each slot.
    pub clone_of_slot: Vec<u32>,
    pub chunks: Vec<InstanceChunk>,
    pub bounds: Option<(DVec3, DVec3)>,
}

/// Z-order key of a cell `(x, y, z)`, 10 bits each.
fn morton(x: u32, y: u32, z: u32) -> u32 {
    fn spread(mut v: u32) -> u32 {
        v &= 0x3ff;
        v = (v | (v << 16)) & 0x0300_00ff;
        v = (v | (v << 8)) & 0x0300_f00f;
        v = (v | (v << 4)) & 0x030c_30c3;
        v = (v | (v << 2)) & 0x0924_9249;
        v
    }
    spread(x) | (spread(y) << 1) | (spread(z) << 2)
}

/// Builds the layout of docs/specs/VIEWPORT.md §10.1. `proto_bounds[i]` is the local box of prototype
/// `i` (`None` for an empty one: its clones get no slot); `world` is applied to every clone matrix.
pub fn build_layout(
    proto_bounds: &[Option<(DVec3, DVec3)>],
    input: &InstanceInput,
    world: &DMat4,
    chunk: usize,
) -> InstanceLayout {
    let chunk = chunk.max(1);
    // (proto, world matrix, world box, clone index)
    struct Item {
        proto: u32,
        clone: u32,
        matrix: DMat4,
        lo: DVec3,
        hi: DVec3,
        key: u32,
    }
    let mut items: Vec<Item> = Vec::with_capacity(input.len());
    for i in 0..input.len() {
        if !input.visible.get(i).copied().unwrap_or(true) {
            continue;
        }
        let proto = input.child.get(i).copied().unwrap_or(0);
        let Some(Some((pl, ph))) = proto_bounds.get(proto as usize) else {
            continue;
        };
        let matrix = *world * input.matrices[i];
        let (lo, hi) = transform_aabb(&matrix, *pl, *ph);
        items.push(Item {
            proto,
            clone: i as u32,
            matrix,
            lo,
            hi,
            key: 0,
        });
    }
    // Morton keys over the centres of each prototype group
    let mut group_box: Vec<Option<(DVec3, DVec3)>> = vec![None; proto_bounds.len()];
    for it in &items {
        let c = (it.lo + it.hi) / 2.0;
        let g = &mut group_box[it.proto as usize];
        *g = Some(g.map_or((c, c), |(a, b)| (a.min(c), b.max(c))));
    }
    for it in &mut items {
        let (lo, hi) = group_box[it.proto as usize].expect("group has a member");
        let size = (hi - lo).max(DVec3::splat(1e-12));
        let t = ((it.lo + it.hi) / 2.0 - lo) / size * 1023.0;
        let q = |v: f64| v.clamp(0.0, 1023.0) as u32;
        it.key = morton(q(t.x), q(t.y), q(t.z));
    }
    items.sort_by_key(|it| (it.proto, it.key, it.clone));

    let mut layout = InstanceLayout {
        raw: Vec::with_capacity(items.len()),
        clone_of_slot: Vec::with_capacity(items.len()),
        chunks: Vec::with_capacity(items.len() / chunk + 1),
        bounds: None,
    };
    let mut start = 0;
    while start < items.len() {
        let proto = items[start].proto;
        let mut end = start;
        while end < items.len() && end - start < chunk && items[end].proto == proto {
            end += 1;
        }
        let members = &items[start..end];
        let (mut lo, mut hi) = (members[0].lo, members[0].hi);
        let mut max_radius = 0.0f64;
        for m in members {
            lo = lo.min(m.lo);
            hi = hi.max(m.hi);
            max_radius = max_radius.max((m.hi - m.lo).length() / 2.0);
        }
        layout.chunks.push(InstanceChunk {
            proto,
            first: layout.raw.len() as u32,
            count: members.len() as u32,
            min: lo,
            max: hi,
            max_radius,
        });
        layout
            .bounds
            .replace(layout.bounds.map_or((lo, hi), |b| union_aabb(b, (lo, hi))));
        for m in members {
            layout
                .raw
                .push(InstanceRaw::new(&m.matrix, input.colors[m.clone as usize]));
            layout.clone_of_slot.push(m.clone);
        }
        start = end;
    }
    layout
}

/// A contiguous run of instance slots drawn with one call per prototype chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawRange {
    pub proto: u32,
    pub first: u32,
    pub count: u32,
    /// Draw the prototype's LOD proxy.
    pub lod: bool,
}

/// How [`visible_ranges`] picks the LOD level of a chunk.
#[derive(Clone, Copy)]
pub struct LodView<'a> {
    pub camera: &'a Camera,
    pub viewport_h: f64,
    pub min_px: f64,
}

/// The draw list: chunks that intersect `frustum`, in buffer order, adjacent chunks of one
/// prototype and LOD level merged.
pub fn visible_ranges(
    chunks: &[InstanceChunk],
    frustum: &Frustum,
    lod_view: Option<LodView>,
) -> Vec<DrawRange> {
    let mut out: Vec<DrawRange> = Vec::new();
    for c in chunks {
        if !frustum.intersects_aabb(c.min, c.max) {
            continue;
        }
        let use_lod = lod_view.is_some_and(|v| {
            // the chunk's largest member against the nearest point of the chunk
            let d = crate::cull::distance_to_aabb(v.camera.eye_world(), c.min, c.max);
            lod::screen_diameter_px(c.max_radius, d, v.camera, v.viewport_h) < v.min_px
        });
        match out.last_mut() {
            Some(r) if r.proto == c.proto && r.lod == use_lod && r.first + r.count == c.first => {
                r.count += c.count;
            }
            _ => out.push(DrawRange {
                proto: c.proto,
                first: c.first,
                count: c.count,
                lod: use_lod,
            }),
        }
    }
    out
}
