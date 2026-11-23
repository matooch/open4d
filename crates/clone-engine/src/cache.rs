//! Versioned frame buffers for the Clone Cache tag (clone-toolset spec O5).

use forge_core::{Category, EvalCtx, Node, NodeId, NodeType, ParamDesc, Scene, Value};
use glam::{DMat4, DVec3};

use crate::{CloneData, CloneItem};

pub const CLONE_CACHE: &str = "forge.clone_cache";

const VERSION: f64 = 1.0;
const HEADER: usize = 4;
const ROW: usize = 27;
const MAX_FRAMES: usize = 10_000;
const MAX_VALUES: usize = 8_000_000;

pub(crate) fn node_type() -> NodeType {
    NodeType::new(CLONE_CACHE, "Clone Cache", Category::Tag)
        .param(ParamDesc::bool("enabled", "Enabled", true).group("Cache"))
        .param(ParamDesc::float_list("frames", "Frames"))
}

pub fn tag(scene: &Scene, host: NodeId) -> Option<&Node> {
    scene
        .node(host)?
        .tags
        .iter()
        .filter_map(|id| scene.node(*id))
        .find(|tag| tag.type_id == CLONE_CACHE)
}

/// A cache tag is a time source for its host and downstream consumers.
pub fn reads_time(scene: &Scene, node: NodeId) -> bool {
    scene.node(node).is_some_and(|n| {
        n.type_id == CLONE_CACHE
            && !matches!(n.params.get("enabled"), Some(Value::Bool(false)))
            && matches!(n.params.get("frames"), Some(Value::FloatList(buf)) if !buf.is_empty())
    })
}

/// Layout: version, FPS, first frame, frame count, `count + 1` absolute offsets;
/// then fixed-stride clone records in frame order. Offsets allow variable clone
/// counts and direct lookup when scrubbing or evaluating out of order.
pub struct Writer {
    buf: Vec<f64>,
    count: usize,
    written: usize,
}

impl Writer {
    pub fn new(start: i64, end: i64, fps: u32) -> Option<Self> {
        let first = i32::try_from(start).ok()?;
        let last = i32::try_from(end).ok()?;
        let count = usize::try_from(i64::from(last) - i64::from(first) + 1).ok()?;
        if count == 0 || count > MAX_FRAMES || fps == 0 {
            return None;
        }
        let mut buf = vec![0.0; HEADER + count + 1];
        buf[..HEADER].copy_from_slice(&[VERSION, f64::from(fps), f64::from(first), count as f64]);
        Some(Self {
            buf,
            count,
            written: 0,
        })
    }

    pub fn push(&mut self, data: &CloneData) -> Option<()> {
        if self.written >= self.count
            || !data.is_consistent()
            || data.len() > (MAX_VALUES - self.buf.len()) / ROW
        {
            return None;
        }
        let start = self.buf.len();
        for i in 0..data.len() {
            self.buf.extend(data.matrices[i].to_cols_array());
            self.buf.extend(data.colors[i]);
            self.buf.push(data.weights[i]);
            self.buf.push(f64::from(u8::from(data.visible[i])));
            self.buf.extend(data.uvw[i].to_array());
            self.buf.push(f64::from(data.index[i]));
            self.buf.push(data.time[i]);
            self.buf.push(f64::from(data.child[i]));
        }
        if !self.buf[start..].iter().all(|v| v.is_finite()) {
            self.buf.truncate(start);
            return None;
        }
        self.buf[HEADER + self.written] = start as f64;
        self.written += 1;
        Some(())
    }

    pub fn finish(mut self) -> Option<Vec<f64>> {
        if self.written != self.count {
            return None;
        }
        self.buf[HEADER + self.count] = self.buf.len() as f64;
        Some(self.buf)
    }
}

pub fn read(scene: &Scene, host: NodeId, ctx: &EvalCtx) -> Option<CloneData> {
    let tag = tag(scene, host)?;
    if matches!(tag.params.get("enabled"), Some(Value::Bool(false))) {
        return None;
    }
    let Value::FloatList(buf) = tag.params.get("frames")? else {
        return None;
    };
    decode(buf, ctx)
}

fn uint(v: f64) -> Option<u32> {
    let n = v as u32;
    (f64::from(n) == v).then_some(n)
}

fn decode(buf: &[f64], ctx: &EvalCtx) -> Option<CloneData> {
    let &[version, fps, first, count] = buf.first_chunk::<HEADER>()?;
    if version != VERSION || uint(fps)? == 0 || fps != ctx.fps || first != f64::from(first as i32) {
        return None;
    }
    let count = uint(count)? as usize;
    let at = uint((ctx.time * fps).round() - first)? as usize;
    if at >= count || count > MAX_FRAMES || buf.len() > MAX_VALUES {
        return None;
    }
    let table = buf.get(HEADER..=HEADER + count)?;
    let begin = uint(table[at])? as usize;
    let end = uint(table[at + 1])? as usize;
    if begin < HEADER + count + 1 {
        return None;
    }
    let frame = buf.get(begin..end)?;
    let (records, remainder) = frame.as_chunks::<ROW>();
    if !remainder.is_empty() {
        return None;
    }
    let mut data = CloneData::default();
    for row in records {
        if !row.iter().all(|v| v.is_finite()) || !matches!(row[20], 0.0 | 1.0) {
            return None;
        }
        data.push(CloneItem {
            matrix: DMat4::from_cols_array(row[..16].try_into().ok()?),
            color: row[16..19].try_into().ok()?,
            weight: row[19],
            visible: row[20] == 1.0,
            uvw: DVec3::from_slice(&row[21..24]),
            time: row[25],
        });
        let i = data.len() - 1;
        data.index[i] = uint(row[24])?;
        data.child[i] = uint(row[26])?;
    }
    Some(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_channel_round_trips_across_frames_of_different_lengths() {
        let mut one = CloneData::default();
        one.push(CloneItem {
            matrix: DMat4::from_translation(DVec3::new(1.0, -2.0, 3.0)),
            color: [0.1, 0.2, 0.3],
            weight: 0.75,
            visible: false,
            uvw: DVec3::new(0.25, 0.5, 0.75),
            time: -0.125,
        });
        one.index[0] = 8;
        one.child[0] = 4;
        let mut writer = Writer::new(-1, 0, 30).unwrap();
        writer.push(&one).unwrap();
        writer.push(&CloneData::default()).unwrap();
        let mut buf = writer.finish().unwrap();
        assert_eq!(decode(&buf, &EvalCtx::at_frame(-1, 30.0)), Some(one));
        assert_eq!(
            decode(&buf, &EvalCtx::at_frame(0, 30.0)),
            Some(CloneData::default())
        );
        assert!(decode(&buf, &EvalCtx::at_frame(-1, 24.0)).is_none());
        buf[HEADER + 1] -= 1.0;
        assert!(decode(&buf, &EvalCtx::at_frame(-1, 30.0)).is_none());
    }
}
