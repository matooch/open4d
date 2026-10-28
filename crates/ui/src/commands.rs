//! Editor commands shared by menus, shortcuts and panels (docs/specs/UI.md §5).

use forge_core::{Category, EditError, NodeId, Registry, Tx};
use forge_generators::EditableError;
use forge_viewport::render::Shading;

use crate::components::{EditMode, SelectCmd, run_select};
use crate::state::EditorState;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Create(String),
    DeleteSelected,
    Undo,
    Redo,
    Rename(NodeId, String),
    Reparent(NodeId, Option<NodeId>),
    ToggleExpanded(NodeId),
    SetShading(Shading),
    FrameAll,
    /// Replace each selected generator by a polygon object (docs/specs/EDITABLE.md §1.2).
    MakeEditable,
    /// Add a polygon object with the deformed result after each selected node (§1.3).
    CurrentStateToObject,
    /// Switch what viewport clicks select (docs/specs/EDITABLE.md §2.2).
    SetMode(EditMode),
    /// Select All / Deselect All / Invert / Grow / Shrink on the current mode's set (§2.5).
    Select(SelectCmd),
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

pub fn execute(s: &mut EditorState, cmd: Command) -> Result<(), String> {
    match cmd {
        Command::Create(type_id) => {
            let ty = s
                .doc
                .scene()
                .registry()
                .get(&type_id)
                .cloned()
                .ok_or_else(|| format!("unknown type {type_id}"))?;
            if ty.category == Category::Tag {
                let host = s
                    .selection
                    .active
                    .ok_or("select an object to add a tag to")?;
                s.doc
                    .transact(&format!("Add {}", ty.label), |tx| {
                        tx.attach_tag(&type_id, host, None)
                    })
                    .map_err(err)?;
            } else {
                // New generators come with a smoothing tag (docs/specs/SMOOTHING.md §2).
                let smooth = ty.category == Category::Generator
                    && s.doc
                        .scene()
                        .registry()
                        .get(forge_core::smoothing::TAG)
                        .is_some();
                let id = s
                    .doc
                    .transact(&format!("Create {}", ty.label), |tx| {
                        let id = tx.create(&type_id, None, None)?;
                        if smooth {
                            tx.attach_tag(forge_core::smoothing::TAG, id, None)?;
                        }
                        Ok(id)
                    })
                    .map_err(err)?;
                s.selection.click(id);
            }
        }
        Command::DeleteSelected => {
            let scene = s.doc.scene();
            // Skip nodes inside another selected node's subtree: they go with it.
            let roots: Vec<NodeId> = s
                .selection
                .nodes
                .iter()
                .copied()
                .filter(|id| {
                    let mut p = scene.node(*id).and_then(|n| n.parent.or(n.host));
                    while let Some(pid) = p {
                        if s.selection.nodes.contains(&pid) {
                            return false;
                        }
                        p = scene.node(pid).and_then(|n| n.parent.or(n.host));
                    }
                    scene.contains(*id)
                })
                .collect();
            if !roots.is_empty() {
                s.doc
                    .transact("Delete", |tx| {
                        roots.iter().try_for_each(|id| tx.delete(*id))
                    })
                    .map_err(err)?;
            }
            s.selection.clear();
        }
        Command::Undo => {
            s.doc.undo();
            s.selection.retain_existing(s.doc.scene());
        }
        Command::Redo => {
            s.doc.redo();
            s.selection.retain_existing(s.doc.scene());
        }
        Command::Rename(id, name) => {
            s.doc
                .transact("Rename", |tx| tx.rename(id, &name))
                .map_err(err)?;
        }
        Command::Reparent(id, parent) => {
            s.doc
                .transact("Move", |tx| tx.move_node(id, parent, None))
                .map_err(err)?;
        }
        Command::ToggleExpanded(id) => {
            if !s.expanded.remove(&id) {
                s.expanded.insert(id);
            }
        }
        Command::MakeEditable => {
            convert_selection(s, "Make Editable", forge_generators::make_editable)?;
        }
        Command::CurrentStateToObject => {
            convert_selection(
                s,
                "Current State to Object",
                forge_generators::current_state_to_object,
            )?;
        }
        Command::SetMode(mode) => s.mode = mode,
        Command::Select(cmd) => run_select(s, cmd)?,
        Command::SetShading(shading) => s.shading = shading,
        Command::FrameAll => {
            // No geometry yet (Phase 5): reset to the default view.
            s.camera = EditorState::default_camera();
        }
    }
    Ok(())
}

type Convert = fn(&mut Tx, NodeId) -> Result<NodeId, EditableError>;

/// Runs `convert` on every selected node as one undo step. Nodes without a mesh are
/// skipped with a status message; the selection becomes the new objects.
fn convert_selection(s: &mut EditorState, label: &str, convert: Convert) -> Result<(), String> {
    let ids: Vec<NodeId> = s.selection.nodes.iter().copied().collect();
    if ids.is_empty() {
        return Err(format!("{label}: select an object first"));
    }
    let mut made = Vec::new();
    let mut skipped = Vec::new();
    // A build failure is not an `EditError`: it is parked here and aborts the step with a
    // placeholder error, so the transaction reverts every edit made so far.
    let mut failure = None;
    let result = s.doc.transact(label, |tx| {
        for id in ids {
            // consumed children of an earlier selected node are gone already
            if !tx.scene().contains(id) {
                continue;
            }
            match convert(tx, id) {
                Ok(new) => made.push(new),
                Err(EditableError::NoMesh(name)) => skipped.push(name),
                Err(EditableError::Edit(e)) => return Err(e),
                Err(e @ EditableError::Build(_)) => {
                    failure = Some(e.to_string());
                    return Err(EditError::UnknownNode(id));
                }
            }
        }
        Ok(())
    });
    if let Some(msg) = failure {
        return Err(format!("{label}: {msg}"));
    }
    result.map_err(err)?;
    s.status = skipped
        .iter()
        .map(|name| format!("{label}: {name} has no mesh"))
        .collect::<Vec<_>>()
        .join("; ");
    if let Some((first, rest)) = made.split_first() {
        s.selection.click(*first);
        for id in rest {
            s.selection.toggle(*id);
        }
    }
    s.selection.retain_existing(s.doc.scene());
    Ok(())
}

/// Placeable categories in `Category` order, each with `(type id, label)` sorted by label.
pub fn create_menu(registry: &Registry) -> Vec<(Category, Vec<(String, String)>)> {
    let mut out: Vec<(Category, Vec<(String, String)>)> = Vec::new();
    for t in registry.types() {
        if !(t.category.is_hierarchy() || t.category == Category::Tag) || !t.in_create_menu {
            continue;
        }
        let item = (t.id.clone(), t.label.clone());
        match out.iter_mut().find(|(c, _)| *c == t.category) {
            Some((_, items)) => items.push(item),
            None => out.push((t.category, vec![item])),
        }
    }
    out.sort_by_key(|(c, _)| *c);
    for (_, items) in &mut out {
        items.sort_by(|a, b| a.1.cmp(&b.1));
    }
    out
}

pub fn category_label(c: Category) -> &'static str {
    match c {
        Category::Object => "Object",
        Category::Generator => "Generator",
        Category::Deformer => "Deformer",
        Category::Effector => "Effector",
        Category::Field => "Field",
        Category::Tag => "Tag",
        Category::Material => "Material",
        Category::Shader => "Shader",
        Category::RenderEngine => "Render engine",
    }
}
