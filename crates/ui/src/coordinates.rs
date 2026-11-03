//! Coordinate panel model: local/world values and scale/size (docs/specs/MANIPULATE.md §4,
//! UI.md §20). The maths lives in `forge_app::coords`; this keeps the single-object API.

use forge_core::{Document, EditError, NodeId};
use glam::DVec3;

use crate::animate::AnimEdit;
use crate::attributes::Gesture;
use crate::manipulate::write_transform;

pub use forge_app::coords::{
    CoordView, Mixed, Row, Space, Third, coord_view, coord_view_multi, mesh_size, next_transform,
};

pub fn set_coord(
    doc: &mut Document,
    id: NodeId,
    space: Space,
    row: Row,
    value: DVec3,
    gesture: Gesture,
) -> Result<(), EditError> {
    set_coord_anim(doc, id, space, row, value, gesture, None)
}

/// [`set_coord`] on an animated scene: the values shown come from the evaluated scene and
/// the result is keyed per autokey / rule 5 (docs/specs/ANIM.md §2.4).
pub fn set_coord_anim(
    doc: &mut Document,
    id: NodeId,
    space: Space,
    row: Row,
    value: DVec3,
    gesture: Gesture,
    anim: Option<&AnimEdit>,
) -> Result<(), EditError> {
    let scene = anim.map_or(doc.scene(), |a| a.eval);
    let start = scene
        .local_transform(id)
        .ok_or(EditError::UnknownNode(id))?;
    let parent = scene
        .parent_world_matrix(id)
        .ok_or(EditError::UnknownNode(id))?;
    let next = next_transform(&parent, &start, mesh_size(scene, id), space, row, value);
    let edit = |tx: &mut forge_core::Tx| write_transform(tx, id, &next, anim);
    match gesture {
        Gesture::Commit => doc.transact("Set coordinates", edit),
        Gesture::Drag(n) => {
            doc.transact_merged("Set coordinates", &format!("coords:{id}:{n}"), edit)
        }
    }
}

/// Typed but not yet applied field values of the Manual mode (UI.md §20.1), per row (position
/// or pivot, rotation, third row) and axis, in display units (degrees for rotation).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CoordDraft {
    pub rows: [[Option<f64>; 3]; 3],
}

impl CoordDraft {
    pub fn set(&mut self, row: usize, axis: usize, value: f64) {
        self.rows[row][axis] = Some(value);
    }

    pub fn value(&self, row: usize, axis: usize) -> Option<f64> {
        self.rows[row][axis]
    }

    pub fn is_empty(&self) -> bool {
        self.rows.iter().flatten().all(Option::is_none)
    }

    pub fn clear(&mut self) {
        *self = CoordDraft::default();
    }

    fn row_json(&self, row: usize) -> Option<serde_json::Value> {
        self.rows[row]
            .iter()
            .any(Option::is_some)
            .then(|| serde_json::json!(self.rows[row]))
    }

    /// The `transform.edit` arguments for the draft (UI.md §20.3); `None` when it is empty.
    pub fn transform_args(
        &self,
        space: Space,
        third: Third,
        gesture: Option<u64>,
    ) -> Option<serde_json::Value> {
        if self.is_empty() {
            return None;
        }
        let mut args = serde_json::Map::new();
        args.insert(
            "space".into(),
            match space {
                Space::Local => "local",
                Space::World => "world",
            }
            .into(),
        );
        for (row, key) in [
            (0, "pos"),
            (1, "rot"),
            (
                2,
                match third {
                    Third::Scale => "scale",
                    Third::Size => "size",
                },
            ),
        ] {
            if let Some(v) = self.row_json(row) {
                args.insert(key.into(), v);
            }
        }
        if let Some(g) = gesture {
            args.insert("gesture".into(), g.into());
        }
        Some(args.into())
    }
}
