//! Editor commands shared by menus, shortcuts and panels (docs/specs/UI.md §5).

use forge_core::{Category, NodeId, Registry};
use forge_viewport::render::Shading;

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
        Command::SetShading(shading) => s.shading = shading,
        Command::FrameAll => {
            // No geometry yet (Phase 5): reset to the default view.
            s.camera = EditorState::default_camera();
        }
    }
    Ok(())
}

/// Placeable categories in `Category` order, each with `(type id, label)` sorted by label.
pub fn create_menu(registry: &Registry) -> Vec<(Category, Vec<(String, String)>)> {
    let mut out: Vec<(Category, Vec<(String, String)>)> = Vec::new();
    for t in registry.types() {
        if !(t.category.is_hierarchy() || t.category == Category::Tag) {
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
