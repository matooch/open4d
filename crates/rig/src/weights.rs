//! The weight tag: joints list, sparse per-point weights and the bind pose
//! (docs/specs/RIG.md §2).

use std::collections::BTreeMap;

use forge_core::{Category, EditError, NodeId, NodeType, ParamDesc, Scene, Tx, Value};
use glam::{DMat4, DVec3};

/// Type id of the weight tag.
pub const WEIGHT_TAG: &str = "forge.weight_tag";

pub(crate) fn weight_tag_type() -> NodeType {
    NodeType::new(WEIGHT_TAG, "Weight", Category::Tag)
        .param(ParamDesc::float("weight", "Weight", 1.0))
        .param(ParamDesc::bool(
            "expose_on_object",
            "Expose on Object",
            false,
        ))
        .param(ParamDesc::link_list(
            "joints",
            "Joints",
            &[Category::Object],
        ))
        .param(ParamDesc::vec3_list("weights", "Weights"))
        .param(ParamDesc::vec3_list("bind_pose", "Bind pose"))
}

/// Sparse weights: per point, `(joint slot, weight)` pairs sorted by slot; zero weights are
/// not stored.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WeightMap {
    rows: BTreeMap<u32, Vec<(u32, f64)>>,
}

impl WeightMap {
    pub fn new() -> WeightMap {
        WeightMap::default()
    }

    /// From dense rows: `rows[point][slot]`.
    pub fn from_dense(rows: &[Vec<f64>]) -> WeightMap {
        let mut m = WeightMap::new();
        for (p, row) in rows.iter().enumerate() {
            m.set(
                p as u32,
                row.iter()
                    .enumerate()
                    .map(|(s, w)| (s as u32, *w))
                    .collect(),
            );
        }
        m
    }

    /// The weights of `point` (empty when none).
    pub fn get(&self, point: u32) -> &[(u32, f64)] {
        self.rows.get(&point).map_or(&[], Vec::as_slice)
    }

    /// Replaces the weights of `point`; entries are sorted by slot, duplicates added up,
    /// zeros and non-finite values dropped.
    pub fn set(&mut self, point: u32, row: Vec<(u32, f64)>) {
        let mut merged: BTreeMap<u32, f64> = BTreeMap::new();
        for (s, w) in row {
            if w.is_finite() {
                *merged.entry(s).or_insert(0.0) += w;
            }
        }
        let row: Vec<(u32, f64)> = merged.into_iter().filter(|(_, w)| *w != 0.0).collect();
        if row.is_empty() {
            self.rows.remove(&point);
        } else {
            self.rows.insert(point, row);
        }
    }

    /// Points that have at least one weight, ascending.
    pub fn points(&self) -> impl Iterator<Item = u32> + '_ {
        self.rows.keys().copied()
    }

    /// Scales every row to sum 1; rows that sum to 0 are left alone.
    pub fn normalise(&mut self) {
        for row in self.rows.values_mut() {
            let sum: f64 = row.iter().map(|(_, w)| *w).sum();
            if sum != 0.0 {
                for (_, w) in row.iter_mut() {
                    *w /= sum;
                }
            }
        }
    }

    /// Drops weights below `threshold` in absolute value.
    pub fn prune(&mut self, threshold: f64) {
        let points: Vec<u32> = self.rows.keys().copied().collect();
        for p in points {
            let row: Vec<(u32, f64)> = self.rows[&p]
                .iter()
                .copied()
                .filter(|(_, w)| w.abs() >= threshold)
                .collect();
            self.set(p, row);
        }
    }

    /// The stored form: `(slot, point, weight)` triples sorted by point, then slot.
    pub fn to_triples(&self) -> Vec<[f64; 3]> {
        let mut out = Vec::new();
        for (p, row) in &self.rows {
            for (s, w) in row {
                out.push([f64::from(*s), f64::from(*p), *w]);
            }
        }
        out
    }

    /// Reads the stored form; malformed triples (negative or fractional indices) are skipped.
    pub fn from_triples(triples: &[[f64; 3]]) -> WeightMap {
        let mut m = WeightMap::new();
        let mut rows: BTreeMap<u32, Vec<(u32, f64)>> = BTreeMap::new();
        for [s, p, w] in triples {
            if *s >= 0.0 && *p >= 0.0 && s.fract() == 0.0 && p.fract() == 0.0 {
                rows.entry(*p as u32).or_default().push((*s as u32, *w));
            }
        }
        for (p, row) in rows {
            m.set(p, row);
        }
        m
    }

    /// The map as a param value.
    pub fn to_value(&self) -> Value {
        Value::Vec3List(self.to_triples())
    }
}

/// The first weight tag on `object`.
pub fn find_weight_tag(scene: &Scene, object: NodeId) -> Option<NodeId> {
    scene
        .node(object)?
        .tags
        .iter()
        .copied()
        .find(|t| scene.node(*t).is_some_and(|n| n.type_id == WEIGHT_TAG))
}

/// The tag's joint list.
pub fn tag_joints(scene: &Scene, tag: NodeId) -> Vec<NodeId> {
    match scene.param(tag, "joints") {
        Some(Value::LinkList(l)) => l.clone(),
        _ => Vec::new(),
    }
}

/// The tag's weights.
pub fn read_weights(scene: &Scene, tag: NodeId) -> WeightMap {
    match scene.param(tag, "weights") {
        Some(Value::Vec3List(t)) => WeightMap::from_triples(t),
        _ => WeightMap::new(),
    }
}

/// The stored bind matrices (object space), one per joint slot; identity for slots without
/// an entry.
pub fn read_bind_pose(scene: &Scene, tag: NodeId, joints: usize) -> Vec<DMat4> {
    let list = match scene.param(tag, "bind_pose") {
        Some(Value::Vec3List(l)) => l.as_slice(),
        _ => &[],
    };
    (0..joints)
        .map(|j| match list.get(4 * j..4 * j + 4) {
            Some([x, y, z, t]) => DMat4::from_cols(
                DVec3::from_array(*x).extend(0.0),
                DVec3::from_array(*y).extend(0.0),
                DVec3::from_array(*z).extend(0.0),
                DVec3::from_array(*t).extend(1.0),
            ),
            _ => DMat4::IDENTITY,
        })
        .collect()
}

/// A bind matrix in its stored form.
pub fn matrix_to_entries(m: &DMat4) -> [[f64; 3]; 4] {
    [
        m.x_axis.truncate().to_array(),
        m.y_axis.truncate().to_array(),
        m.z_axis.truncate().to_array(),
        m.w_axis.truncate().to_array(),
    ]
}

/// The bind pose of `joints` for `object` in the current scene:
/// `inverse(object world) · joint world` per joint (identity for a missing joint).
pub fn current_bind_pose(scene: &Scene, object: NodeId, joints: &[NodeId]) -> Vec<DMat4> {
    let inv = scene
        .world_matrix(object)
        .map_or(DMat4::IDENTITY, |m| m.inverse());
    joints
        .iter()
        .map(|j| scene.world_matrix(*j).map_or(DMat4::IDENTITY, |m| inv * m))
        .collect()
}

/// Binds: stores the current joint poses (relative to the host object) as the bind pose.
///
/// # Errors
/// Fails for a node that is not a weight tag or that has no host object.
pub fn bind(tx: &mut Tx<'_>, tag: NodeId) -> Result<(), EditError> {
    let scene = tx.scene();
    let node = scene
        .node(tag)
        .filter(|n| n.type_id == WEIGHT_TAG)
        .ok_or(EditError::UnknownNode(tag))?;
    let host = node.host.ok_or(EditError::UnknownNode(tag))?;
    let joints = tag_joints(scene, tag);
    let pose = current_bind_pose(scene, host, &joints);
    let entries: Vec<[f64; 3]> = pose.iter().flat_map(matrix_to_entries).collect();
    tx.set_param(tag, "bind_pose", Value::Vec3List(entries))
}
