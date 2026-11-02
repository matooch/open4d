//! Attribute panel rows derived from parameter descriptors (docs/specs/UI.md §4).

use forge_core::{
    Category, Document, EditError, EnumOption, NodeId, ParamKind, Scene, Unit, Value,
};

use crate::animate::{AnimCtx, write_param};
use crate::tree::type_label;

#[derive(Debug, Clone, PartialEq)]
pub enum Widget {
    Checkbox,
    IntDrag {
        min: i64,
        max: i64,
    },
    FloatDrag {
        min: f64,
        max: f64,
        unit: Unit,
    },
    Vec3Drag {
        min: f64,
        max: f64,
        unit: Unit,
    },
    Color,
    Combo {
        options: Vec<EnumOption>,
    },
    Text,
    Link {
        accepts: Vec<Category>,
    },
    LinkList {
        accepts: Vec<Category>,
    },
    /// Raw value of a node whose type is not registered.
    ReadOnly,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttrRow {
    pub param: String,
    pub label: String,
    pub widget: Widget,
    /// The active object's value.
    pub value: Value,
    /// The selected objects do not all have the same value (multi-edit).
    pub mixed: bool,
    /// Vec3 only: which components differ between the selected objects.
    pub mixed_axes: [bool; 3],
    /// Scrub step in display units (`UiHint::step`).
    pub step: Option<f64>,
    /// The parameter can carry keys (hook for the keyframe indicators, brief 097).
    pub animatable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttributeView {
    pub name: String,
    pub type_label: String,
    pub editable: bool,
    /// Groups by their full key (`"Tab/Group"` or `"Group"`), first-appearance order.
    pub groups: Vec<(String, Vec<AttrRow>)>,
    /// How many objects the rows edit at once.
    pub count: usize,
}

const DEFAULT_GROUP: &str = "Parameters";
/// The tab of groups whose key has no `Tab/` prefix.
pub const DEFAULT_TAB: &str = "Basic";

/// Splits a group key into (tab, group name): `"Tab/Group"`; no slash means the default tab.
pub fn split_group(key: &str) -> (&str, &str) {
    match key.split_once('/') {
        Some((t, g)) if !t.is_empty() && !g.is_empty() => (t, g),
        _ => (DEFAULT_TAB, key),
    }
}

impl AttributeView {
    /// Tab names in first-appearance order.
    pub fn tabs(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for (key, _) in &self.groups {
            let t = split_group(key).0;
            if !out.iter().any(|o| o == t) {
                out.push(t.to_string());
            }
        }
        out
    }

    /// The groups of `tab` as (group name, rows).
    pub fn groups_of(&self, tab: &str) -> Vec<(&str, &[AttrRow])> {
        self.groups
            .iter()
            .filter(|(k, _)| split_group(k).0 == tab)
            .map(|(k, rows)| (split_group(k).1, rows.as_slice()))
            .collect()
    }
}

pub(crate) fn widget(kind: &ParamKind, unit: Unit) -> Widget {
    match kind {
        ParamKind::Bool => Widget::Checkbox,
        ParamKind::Int { min, max } => Widget::IntDrag {
            min: *min,
            max: *max,
        },
        ParamKind::Float { min, max } => Widget::FloatDrag {
            min: *min,
            max: *max,
            unit,
        },
        ParamKind::Vec3 { min, max } => Widget::Vec3Drag {
            min: *min,
            max: *max,
            unit,
        },
        ParamKind::Color => Widget::Color,
        ParamKind::Enum { options } => Widget::Combo {
            options: options.clone(),
        },
        ParamKind::String => Widget::Text,
        ParamKind::Link { accepts } => Widget::Link {
            accepts: accepts.clone(),
        },
        ParamKind::LinkList { accepts } => Widget::LinkList {
            accepts: accepts.clone(),
        },
        ParamKind::Mesh | ParamKind::IndexList => Widget::ReadOnly,
    }
}

fn push(groups: &mut Vec<(String, Vec<AttrRow>)>, group: &str, row: AttrRow) {
    let group = if group.is_empty() {
        DEFAULT_GROUP
    } else {
        group
    };
    match groups.iter_mut().find(|(g, _)| g == group) {
        Some((_, rows)) => rows.push(row),
        None => groups.push((group.to_string(), vec![row])),
    }
}

/// The attribute view of a node, or `None` if it does not exist.
pub fn attribute_rows(scene: &Scene, id: NodeId) -> Option<AttributeView> {
    attribute_rows_multi(scene, &[id])
}

fn value_of(scene: &Scene, id: NodeId, desc: &forge_core::ParamDesc) -> Value {
    scene
        .param(id, &desc.id)
        .cloned()
        .unwrap_or_else(|| desc.default.clone())
}

/// The attribute view of several objects at once (multi-edit). The first id is the active
/// object: its type decides the rows and its values are shown. Only parameters every other
/// object's type also has, with the same kind, are listed; `mixed` marks rows whose values
/// differ.
pub fn attribute_rows_multi(scene: &Scene, ids: &[NodeId]) -> Option<AttributeView> {
    let id = *ids.first()?;
    let node = scene.node(id)?;
    let (label, missing) = type_label(scene, id);
    let others: Vec<NodeId> = ids[1..]
        .iter()
        .copied()
        .filter(|o| scene.node(*o).is_some())
        .collect();
    let mut groups = Vec::new();
    match scene.node_type(id) {
        Some(ty) => {
            for desc in ty.params.iter().filter(|d| !d.ui.hidden) {
                let common = others.iter().all(|o| {
                    scene
                        .node_type(*o)
                        .and_then(|t| t.find_param(&desc.id))
                        .is_some_and(|d| d.kind == desc.kind)
                });
                if !common {
                    continue;
                }
                let value = value_of(scene, id, desc);
                let mut mixed_axes = [false; 3];
                for o in &others {
                    match (&value, scene.param(*o, &desc.id).unwrap_or(&desc.default)) {
                        (Value::Vec3(a), Value::Vec3(b)) => {
                            for k in 0..3 {
                                mixed_axes[k] |= a[k] != b[k];
                            }
                        }
                        (a, b) => mixed_axes[0] |= a != b,
                    }
                }
                let mixed = mixed_axes.iter().any(|m| *m);
                if !matches!(value, Value::Vec3(_)) {
                    mixed_axes = [mixed; 3];
                }
                let row = AttrRow {
                    param: desc.id.clone(),
                    label: desc.label.clone(),
                    widget: widget(&desc.kind, desc.ui.unit),
                    value,
                    mixed,
                    mixed_axes,
                    step: desc.ui.step,
                    animatable: desc.animatable,
                };
                push(&mut groups, &desc.ui.group, row);
            }
        }
        None => {
            for (param, value) in &node.params {
                let row = AttrRow {
                    param: param.clone(),
                    label: param.clone(),
                    widget: Widget::ReadOnly,
                    value: value.clone(),
                    mixed: false,
                    mixed_axes: [false; 3],
                    step: None,
                    animatable: false,
                };
                push(&mut groups, "", row);
            }
        }
    }
    Some(AttributeView {
        name: node.name.clone(),
        type_label: label,
        editable: !missing,
        groups,
        count: 1 + others.len(),
    })
}

/// Stored value → value shown to the user.
pub fn to_display(unit: Unit, v: f64) -> f64 {
    match unit {
        Unit::Angle => v.to_degrees(),
        Unit::Percent => v * 100.0,
        _ => v,
    }
}

/// Value typed by the user → stored value.
pub fn from_display(unit: Unit, v: f64) -> f64 {
    match unit {
        Unit::Angle => v.to_radians(),
        Unit::Percent => v / 100.0,
        _ => v,
    }
}

/// Suffix shown after a value in this unit.
pub fn unit_suffix(unit: Unit) -> &'static str {
    match unit {
        Unit::Angle => "°",
        Unit::Percent => " %",
        _ => "",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    /// A single, final change.
    Commit,
    /// Part of the continuous drag with this id; merged into one undo step.
    Drag(u64),
}

/// Applies a parameter edit as an undoable step.
pub fn apply_edit(
    doc: &mut Document,
    id: NodeId,
    param: &str,
    value: Value,
    gesture: Gesture,
) -> Result<(), EditError> {
    apply_edit_anim(doc, id, param, value, gesture, None)
}

/// [`apply_edit`] with animation: autokey and rule 5 write keys (docs/specs/ANIM.md §2.4).
pub fn apply_edit_anim(
    doc: &mut Document,
    id: NodeId,
    param: &str,
    value: Value,
    gesture: Gesture,
    ctx: Option<&AnimCtx>,
) -> Result<(), EditError> {
    let label = doc
        .scene()
        .node_type(id)
        .and_then(|t| t.find_param(param))
        .map_or_else(|| param.to_string(), |d| d.label.clone());
    let label = format!("Set {label}");
    let edit = |tx: &mut forge_core::Tx| write_param(tx, id, param, value, ctx);
    match gesture {
        Gesture::Commit => doc.transact(&label, edit),
        Gesture::Drag(n) => doc.transact_merged(&label, &format!("drag:{id}:{param}:{n}"), edit),
    }
}

/// Attribute rows for a tool's settings, valued from `values` (docs/specs/MODELING.md §0.2).
pub fn tool_rows(
    params: Vec<forge_core::ParamDesc>,
    values: &forge_modeling::Settings,
) -> Vec<AttrRow> {
    params
        .into_iter()
        .filter(|d| !d.ui.hidden)
        .map(|d| AttrRow {
            value: values
                .get(&d.id)
                .cloned()
                .unwrap_or_else(|| d.default.clone()),
            widget: widget(&d.kind, d.ui.unit),
            mixed: false,
            mixed_axes: [false; 3],
            step: d.ui.step,
            animatable: false,
            param: d.id,
            label: d.label,
        })
        .collect()
}
