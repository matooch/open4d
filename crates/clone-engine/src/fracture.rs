//! Fracture: splits geometry into fragments that are clones of their own
//! (clone-toolset spec F1-F2). This module has the node type, the polygon islands and the
//! clone data of the fragments; the generators crate evaluates the input and assembles the
//! mesh.

use std::collections::BTreeMap;

use forge_core::{NodeType, ParamDesc, Value};
use glam::{DMat4, DVec3};

use crate::node::{cloner_type, settings_of};
use crate::settings::TransformTab;
use crate::{CloneData, CloneItem, hpb};

/// Type id of the Fracture generator.
pub const FRACTURE: &str = "forge.fracture";

/// How the input is split into fragments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FractureMode {
    /// One fragment per input object.
    Straight,
    /// One fragment per polygon island (polygons sharing points).
    ExplodeSegments,
    /// Islands with connectors between them (not derived).
    ExplodeSegmentsConnect,
}

/// Cloner parameters a Fracture shares: the Transform tab, the display and the effector list.
const SHARED: &[&str] = &[
    "effectors",
    "display",
    "clone_position",
    "clone_rotation",
    "clone_scale",
    "clone_color",
    "clone_weight",
];

/// The Fracture node type. Its children are the inputs and are consumed.
pub fn fracture_type() -> NodeType {
    let mut ty = NodeType::new(FRACTURE, "Fracture", forge_core::Category::Generator)
        .consuming_children()
        .param(
            ParamDesc::enumeration(
                "mode",
                "Mode",
                "straight",
                &[
                    ("straight", "Straight"),
                    ("explode_segments", "Explode Segments"),
                    ("explode_segments_connect", "Explode Segments & Connect"),
                ],
            )
            .group("Object"),
        );
    for desc in cloner_type().params {
        if SHARED.contains(&desc.id.as_str()) {
            ty = ty.param(desc);
        }
    }
    ty
}

/// Mode and Transform tab of a Fracture.
pub fn fracture_settings(values: &BTreeMap<String, Value>) -> (FractureMode, TransformTab) {
    let ty = fracture_type();
    let mode = match values.get("mode") {
        Some(Value::Enum(m)) if m == "explode_segments" => FractureMode::ExplodeSegments,
        Some(Value::Enum(m)) if m == "explode_segments_connect" => {
            FractureMode::ExplodeSegmentsConnect
        }
        _ => FractureMode::Straight,
    };
    (mode, settings_of(&ty, values).transform)
}

/// The polygon islands of a mesh: polygons that share a point are one island. Each island is
/// its polygon indices ascending; islands are ordered by their first polygon.
pub fn polygon_islands(points: usize, polys: &[Vec<u32>]) -> Vec<Vec<usize>> {
    // Union-find over points; a polygon joins its points.
    let mut parent: Vec<usize> = (0..points).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    for poly in polys {
        let Some(first) = poly.first() else { continue };
        let a = find(&mut parent, *first as usize);
        for v in &poly[1..] {
            let b = find(&mut parent, *v as usize);
            if a != b {
                parent[b] = a;
            }
        }
    }
    let mut order: Vec<usize> = Vec::new();
    let mut islands: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (i, poly) in polys.iter().enumerate() {
        let Some(first) = poly.first() else { continue };
        let root = find(&mut parent, *first as usize);
        let list = islands.entry(root).or_default();
        if list.is_empty() {
            order.push(root);
        }
        list.push(i);
    }
    order
        .into_iter()
        .map(|r| islands.remove(&r).expect("island"))
        .collect()
}

/// Clone data of the fragments: fragment `i` sits at `pivots[i]` and the Transform tab is
/// applied as on a cloner (position before, rotation and scale after, colour, weight).
pub fn fragment_clones(pivots: &[DVec3], tab: &TransformTab) -> CloneData {
    let after = hpb(tab.rotation) * DMat4::from_scale(tab.scale);
    let before = DMat4::from_translation(tab.position);
    let n = pivots.len().max(1) as f64;
    let mut data = CloneData::default();
    for (i, pivot) in pivots.iter().enumerate() {
        let mut item = CloneItem::at(
            before * DMat4::from_translation(*pivot) * after,
            DVec3::new(i as f64 / (n - 1.0).max(1.0), 0.0, 0.0),
        );
        item.color = tab.color;
        item.weight = tab.weight;
        data.push(item);
    }
    data
}
