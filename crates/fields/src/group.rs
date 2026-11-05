//! Group field: a field list used as one field (docs/specs/FIELDS.md F7).

use std::collections::BTreeMap;

use forge_core::{EvalCtx, NodeType, Value};
use glam::DVec3;

use crate::node::{FIELD, FIELD_LIST, Params, field_list_param, field_type};
use crate::remap::Remap;
use crate::{Field, FieldError, FieldList, FieldSample};

pub const GROUP: &str = "forge.field.group";

pub fn group_type() -> NodeType {
    let mut ty = field_type(
        GROUP,
        "Group Field",
        vec![field_list_param(FIELD_LIST, "Layers").group(FIELD)],
        0.0,
        false,
        false,
    );
    // The observed Group field has Remapping off (`group-field`, O23).
    for p in &mut ty.params {
        if p.id == "enable_remapping" {
            p.default = Value::Bool(false);
        }
    }
    ty
}

/// The list of a Group field, then its own Remapping tab.
pub struct GroupField {
    pub list: FieldList,
    pub remap: Remap,
}

impl GroupField {
    fn finish(&self, mut s: FieldSample, empty: bool) -> FieldSample {
        // A group without layers has nothing to show (parity-pending); the list itself would
        // answer "full effect".
        s.value = if empty {
            0.0
        } else {
            self.remap.apply(s.value)
        };
        s
    }
}

impl Field for GroupField {
    fn sample(&self, p_world: DVec3, ctx: &EvalCtx) -> FieldSample {
        self.finish(self.list.sample(p_world, ctx), self.list.is_empty())
    }

    fn sample_many(&self, points: &[DVec3], ctx: &EvalCtx) -> Vec<FieldSample> {
        let empty = self.list.is_empty();
        self.list
            .sample_many(points, ctx)
            .into_iter()
            .map(|s| self.finish(s, empty))
            .collect()
    }
}

/// The group of a node's parameters around its built list.
pub(crate) fn field(
    values: &BTreeMap<String, Value>,
    list: FieldList,
) -> Result<GroupField, FieldError> {
    let ty = group_type();
    let p = Params { ty: &ty, values };
    let remap = p.remap();
    remap.check()?;
    Ok(GroupField { list, remap })
}
