//! Object tree rows (docs/specs/UI.md §3).

use std::collections::BTreeSet;

use forge_command::NodeSelection;
use forge_core::visibility::{self, Vis};
use forge_core::{Category, LayerId, NodeId, Scene, layers};
use serde_json::{Value as Json, json};

#[derive(Debug, Clone, PartialEq)]
pub struct TreeRow {
    pub id: NodeId,
    pub depth: usize,
    pub name: String,
    pub type_label: String,
    pub has_children: bool,
    pub expanded: bool,
    /// `(tag id, tag type label)`, in order.
    pub tags: Vec<(NodeId, String)>,
    pub missing_type: bool,
    /// Own editor and render visibility settings and enable flag (UI.md §13.1).
    pub editor_vis: Vis,
    pub render_vis: Vis,
    pub enabled: bool,
    /// Shown only because a descendant matches the search (UI.md §13.5).
    pub dim: bool,
    /// The object's own layer: id, name and colour (UI.md §22.2).
    pub layer: Option<(LayerId, String, [f64; 3])>,
    /// What the evaluation cell shows for this type (UI.md §13.1).
    pub eval: EvalCell,
}

/// The type-dependent cell after the visibility dots (observed, G1 item 2): an evaluation
/// check on most types, an active-camera target on cameras, nothing on nulls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalCell {
    None,
    Check,
    ActiveCamera,
}

/// The evaluation cell of a node type. Which other types lack the check is
/// **parity-pending G1**: ours is every type but the null.
pub fn eval_cell(type_id: &str) -> EvalCell {
    match type_id {
        "forge.null" => EvalCell::None,
        t if t == forge_generators::scene_objects::CAMERA => EvalCell::ActiveCamera,
        _ => EvalCell::Check,
    }
}

/// Label of a node's type, and whether the type is missing from the registry.
pub fn type_label(scene: &Scene, id: NodeId) -> (String, bool) {
    match (scene.node_type(id), scene.node(id)) {
        (Some(t), _) => (t.label.clone(), false),
        (None, Some(n)) => (n.type_id.clone(), true),
        (None, None) => (String::new(), true),
    }
}

fn make_row(scene: &Scene, id: NodeId, depth: usize, expanded: bool) -> TreeRow {
    let n = scene.node(id).expect("row of an existing node");
    let (label, missing) = type_label(scene, id);
    let (editor_vis, render_vis, enabled) = visibility::own(scene, id);
    TreeRow {
        id,
        depth,
        name: n.name.clone(),
        type_label: label,
        has_children: !n.children.is_empty(),
        expanded,
        tags: n
            .tags
            .iter()
            .map(|t| (*t, type_label(scene, *t).0))
            .collect(),
        missing_type: missing,
        editor_vis,
        render_vis,
        enabled,
        dim: false,
        layer: layers::of(scene, id).map(|l| (l.id, l.name.clone(), l.color)),
        eval: eval_cell(&n.type_id),
    }
}

/// Visible rows in depth-first order; children of collapsed nodes are skipped.
pub fn tree_rows(scene: &Scene, expanded: &BTreeSet<NodeId>) -> Vec<TreeRow> {
    let mut out = Vec::new();
    let mut stack: Vec<(NodeId, usize)> = scene.roots().iter().rev().map(|r| (*r, 0)).collect();
    while let Some((id, depth)) = stack.pop() {
        let Some(n) = scene.node(id) else { continue };
        let open = expanded.contains(&id);
        // a layer with the manager off omits the row; its children stay, one level up
        let listed = layers::in_manager(scene, id);
        if listed {
            out.push(make_row(scene, id, depth, open));
        }
        if open || !listed {
            let below = depth + usize::from(listed);
            stack.extend(n.children.iter().rev().map(|c| (*c, below)));
        }
    }
    out
}

// ---- selection (UI.md §13.2) --------------------------------------------------------

/// Applies a click on row `id` to the selection: plain replaces, Shift selects the range
/// from the anchor over `order` (the visible rows), Ctrl/Cmd toggles.
pub fn click_selection(
    sel: &mut NodeSelection,
    order: &[NodeId],
    id: NodeId,
    shift: bool,
    command: bool,
) {
    if shift {
        sel.range(id, order);
    } else if command {
        sel.toggle(id);
    } else {
        sel.click(id);
    }
}

/// Right-click on a row: an unselected row becomes the selection, a selected one keeps it.
pub fn menu_target(sel: &mut NodeSelection, row: NodeId) {
    if !sel.nodes.contains(&row) {
        sel.click(row);
    }
}

// ---- drag and drop (UI.md §13.3) ----------------------------------------------------

/// Where in a row the pointer is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropZone {
    Before,
    Into,
    After,
}

/// The zone for a pointer `frac` (0 = top edge, 1 = bottom edge) down the hovered row.
pub fn drop_zone(frac: f32) -> DropZone {
    if frac < 0.25 {
        DropZone::Before
    } else if frac > 0.75 {
        DropZone::After
    } else {
        DropZone::Into
    }
}

/// Arguments for `object.move`: the new parent and the index among its children without
/// the moved nodes (`None` = last).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropTarget {
    pub parent: Option<NodeId>,
    pub index: Option<usize>,
}

fn children_of(scene: &Scene, parent: Option<NodeId>) -> Vec<NodeId> {
    match parent {
        Some(p) => scene
            .node(p)
            .map(|n| n.children.clone())
            .unwrap_or_default(),
        None => scene.roots().to_vec(),
    }
}

/// True if `id` or one of its ancestors is in `moved`.
fn inside(scene: &Scene, id: NodeId, moved: &[NodeId]) -> bool {
    std::iter::successors(Some(id), |i| scene.node(*i).and_then(|n| n.parent))
        .any(|i| moved.contains(&i))
}

/// The drop target for dropping `moved` on the visible row `hovered` in `zone`, or `None`
/// when the drop is invalid (on a moved node, or into/next to its own subtree).
pub fn drop_target(
    scene: &Scene,
    rows: &[TreeRow],
    hovered: NodeId,
    zone: DropZone,
    moved: &[NodeId],
) -> Option<DropTarget> {
    if moved.is_empty() || moved.contains(&hovered) {
        return None;
    }
    let row = rows.iter().find(|r| r.id == hovered)?;
    let target = match zone {
        DropZone::Into => DropTarget {
            parent: Some(hovered),
            index: None,
        },
        DropZone::After if row.has_children && row.expanded => DropTarget {
            parent: Some(hovered),
            index: Some(0),
        },
        DropZone::Before | DropZone::After => {
            let parent = scene.node(hovered)?.parent;
            let rest: Vec<NodeId> = children_of(scene, parent)
                .into_iter()
                .filter(|c| !moved.contains(c))
                .collect();
            let pos = rest.iter().position(|c| *c == hovered)?;
            DropTarget {
                parent,
                index: Some(pos + usize::from(zone == DropZone::After)),
            }
        }
    };
    match target.parent {
        Some(p) if inside(scene, p, moved) => None,
        _ => Some(target),
    }
}

/// The nodes a drag of `dragged` moves: the selection when it contains the row, else the
/// row alone; in visible order, without nodes below another moved node.
pub fn moved_set(
    scene: &Scene,
    rows: &[TreeRow],
    selection: &BTreeSet<NodeId>,
    dragged: NodeId,
) -> Vec<NodeId> {
    if !selection.contains(&dragged) {
        return vec![dragged];
    }
    let mut ids: Vec<NodeId> = rows
        .iter()
        .map(|r| r.id)
        .filter(|id| selection.contains(id))
        .collect();
    let rest: Vec<NodeId> = selection
        .iter()
        .copied()
        .filter(|id| !ids.contains(id))
        .collect();
    ids.extend(rest);
    ids.retain(|id| scene.node(*id).is_some_and(|n| !n.is_tag()));
    let all = ids.clone();
    ids.retain(|id| {
        let parent = scene.node(*id).and_then(|n| n.parent);
        !parent.is_some_and(|p| inside(scene, p, &all))
    });
    ids
}

// ---- search (UI.md §13.5) -----------------------------------------------------------

/// Rows matching `query` (name, type label or tag label, case-insensitive) with their
/// ancestors, in tree order; empty for an empty query or no match.
pub fn filter_rows(scene: &Scene, query: &str) -> Vec<TreeRow> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let hit = |row: &TreeRow| {
        row.name.to_lowercase().contains(&q)
            || row.type_label.to_lowercase().contains(&q)
            || row.tags.iter().any(|(_, t)| t.to_lowercase().contains(&q))
    };
    // returns the rows of `id`'s subtree that survive, `id` first
    fn walk(
        scene: &Scene,
        id: NodeId,
        depth: usize,
        hit: &dyn Fn(&TreeRow) -> bool,
        out: &mut Vec<TreeRow>,
    ) -> bool {
        let Some(n) = scene.node(id) else {
            return false;
        };
        if !layers::in_manager(scene, id) {
            // the row is omitted, its children are still searched at the same depth
            return n
                .children
                .iter()
                .fold(false, |any, c| walk(scene, *c, depth, hit, out) | any);
        }
        let mut row = make_row(scene, id, depth, false);
        let own = hit(&row);
        let mark = out.len();
        out.push(row.clone());
        let mut below = false;
        for c in &n.children {
            below |= walk(scene, *c, depth + 1, hit, out);
        }
        if !own && !below {
            out.truncate(mark);
            return false;
        }
        row.expanded = below;
        row.dim = !own;
        out[mark] = row;
        true
    }
    let mut out = Vec::new();
    for r in scene.roots() {
        walk(scene, *r, 0, &hit, &mut out);
    }
    out
}

// ---- context menu (UI.md §13.6) -----------------------------------------------------

/// One context menu entry: a registry command with arguments (`None` = the in-place
/// rename, which is panel state).
#[derive(Debug, Clone, PartialEq)]
pub struct MenuItem {
    pub label: &'static str,
    pub command: Option<&'static str>,
    pub args: Json,
}

/// The menu for the selected nodes.
pub fn context_menu(scene: &Scene, selection: &BTreeSet<NodeId>) -> Vec<MenuItem> {
    let ids: Vec<NodeId> = selection
        .iter()
        .copied()
        .filter(|id| scene.node(*id).is_some_and(|n| !n.is_tag()))
        .collect();
    let nodes: Vec<u64> = ids.iter().map(|i| i.0).collect();
    let item = |label, command, args| MenuItem {
        label,
        command: Some(command),
        args,
    };
    let mut out = Vec::new();
    if ids.len() == 1 {
        out.push(MenuItem {
            label: "Rename",
            command: None,
            args: Json::Null,
        });
    }
    if ids.is_empty() {
        return out;
    }
    out.push(item(
        "Group Objects",
        "object.group",
        json!({"nodes": nodes}),
    ));
    if ids.iter().any(|id| {
        scene
            .node_type(*id)
            .is_some_and(|t| t.category == Category::Generator)
    }) {
        out.push(item(
            "Make Editable",
            "mesh.make_editable",
            json!({"nodes": nodes}),
        ));
    }
    let own: Vec<_> = ids.iter().map(|id| visibility::own(scene, *id)).collect();
    if own.iter().all(|(e, r, _)| *e == Vis::Off && *r == Vis::Off) {
        out.push(item(
            "Show",
            "object.visibility",
            json!({"nodes": nodes, "editor": "default", "render": "default"}),
        ));
    } else {
        out.push(item(
            "Hide",
            "object.visibility",
            json!({"nodes": nodes, "editor": "off", "render": "off"}),
        ));
    }
    if own.iter().all(|(_, _, on)| !on) {
        out.push(item(
            "Enable",
            "object.enable",
            json!({"nodes": nodes, "enabled": true}),
        ));
    } else {
        out.push(item(
            "Disable",
            "object.enable",
            json!({"nodes": nodes, "enabled": false}),
        ));
    }
    out.push(item(
        "Move to Root",
        "object.move",
        json!({"nodes": nodes, "parent": null}),
    ));
    out.push(item("Delete", "object.delete", json!({"nodes": nodes})));
    out
}

// ---- layer sub menu (UI.md §22.2) ---------------------------------------------------

/// One entry of the row context menu's "Layer" sub menu.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerMenuItem {
    pub label: String,
    pub command: &'static str,
    pub args: Json,
    /// The whole selection is on this layer already.
    pub checked: bool,
}

/// "No layer", one entry per layer and "New layer from selection", acting on the selected
/// hierarchy nodes; empty when none is selected.
pub fn layer_menu(scene: &Scene, selection: &BTreeSet<NodeId>) -> Vec<LayerMenuItem> {
    let ids: Vec<NodeId> = selection
        .iter()
        .copied()
        .filter(|id| scene.node(*id).is_some_and(|n| !n.is_tag()))
        .collect();
    if ids.is_empty() {
        return Vec::new();
    }
    let nodes: Vec<u64> = ids.iter().map(|i| i.0).collect();
    let on = |l: Option<LayerId>| ids.iter().all(|id| layers::layer_id(scene, *id) == l);
    let mut out = vec![LayerMenuItem {
        label: "No layer".into(),
        command: "layer.assign",
        args: json!({"layer": null, "nodes": nodes}),
        checked: on(None),
    }];
    for l in scene.layers().iter() {
        out.push(LayerMenuItem {
            label: l.name.clone(),
            command: "layer.assign",
            args: json!({"layer": l.id.0, "nodes": nodes}),
            checked: on(Some(l.id)),
        });
    }
    out.push(LayerMenuItem {
        label: "New layer from selection".into(),
        command: "layer.create",
        args: json!({"assign": true, "nodes": nodes}),
        checked: false,
    });
    out
}

// ---- row geometry and hierarchy cues (UI.md §13.1a; observed, G1 item 7) ---------------

/// The numbers of the object manager's rows, in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// Height of a row; rows touch (no gap), so every y of the list belongs to a row.
    /// Observed: the row pitch is 20 px.
    pub row_height: f32,
    /// Indent per depth level. Observed: about 12 px.
    pub indent: f32,
    /// Width of the gutter cell at the start of a row's own column, holding the
    /// disclosure box.
    pub arrow_width: f32,
    /// Side of the square disclosure box (a boxed minus or plus), centred in the cell.
    pub box_size: f32,
    /// How far into its own cell a leaf row's connector runs.
    pub leaf_reach: f32,
}

pub const METRICS: Metrics = Metrics {
    row_height: 20.0,
    indent: 12.0,
    arrow_width: 12.0,
    box_size: 10.0,
    leaf_reach: 8.0,
};

/// Where a row's parts sit, relative to the left edge of the list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowLayout {
    /// Left edge of the row's own column.
    pub start: f32,
    /// The gutter cell, `(left, right)`.
    pub arrow: (f32, f32),
    /// The trunk x of the row's parent, where the row's elbow hangs (none for roots).
    pub parent_trunk: Option<f32>,
}

/// The x of the vertical trunk of a node at `level`: the centre of its gutter cell.
pub fn trunk_x(level: usize) -> f32 {
    METRICS.indent * level as f32 + METRICS.arrow_width / 2.0
}

pub fn row_layout(depth: usize) -> RowLayout {
    let start = METRICS.indent * depth as f32;
    RowLayout {
        start,
        arrow: (start, start + METRICS.arrow_width),
        parent_trunk: depth.checked_sub(1).map(trunk_x),
    }
}

/// The disclosure box of a row at `depth`, relative to the row's top-left corner: a square
/// of [`Metrics::box_size`] centred in the gutter cell, as `(left, top, right, bottom)`.
pub fn disclosure_box(depth: usize) -> (f32, f32, f32, f32) {
    let c = trunk_x(depth);
    let half = METRICS.box_size / 2.0;
    let mid = METRICS.row_height / 2.0;
    (c - half, mid - half, c + half, mid + half)
}

/// The glyph inside the box: a minus when expanded, a plus when collapsed (observed, G1
/// item 7). The segments are relative to the row's top-left corner: the horizontal bar, and
/// for a plus also the vertical bar; each 6 px long, centred in the box.
pub fn disclosure_glyph(depth: usize, expanded: bool) -> Vec<([f32; 2], [f32; 2])> {
    let (l, t, r, b) = disclosure_box(depth);
    let (cx, cy) = ((l + r) / 2.0, (t + b) / 2.0);
    let mut out = vec![([cx - 3.0, cy], [cx + 3.0, cy])];
    if !expanded {
        out.push(([cx, cy - 3.0], [cx, cy + 3.0]));
    }
    out
}

/// Which ancestor trunks pass through a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connector {
    /// One flag per ancestor level `k < depth`: a later row at depth `k + 1` follows within
    /// the ancestor's subtree, so its trunk continues below this row. The last flag
    /// (`k = depth - 1`) belongs to the row's own elbow and means "a later sibling exists".
    pub trunks: Vec<bool>,
}

/// The connectors of the visible rows, from their depth sequence alone.
pub fn connectors(rows: &[TreeRow]) -> Vec<Connector> {
    let mut out = vec![Connector { trunks: Vec::new() }; rows.len()];
    // open[k]: a later row of depth k exists before the list returns above depth k
    let mut open: Vec<bool> = Vec::new();
    for (i, row) in rows.iter().enumerate().rev() {
        let d = row.depth;
        if open.len() <= d {
            open.resize(d + 1, false);
        }
        out[i].trunks = (0..d).map(|k| open[k + 1]).collect();
        open.truncate(d + 1);
        open[d] = true;
    }
    out
}

/// The line segments of a row's connectors, relative to the row's top-left corner (the
/// list's left edge): full-height trunks of the higher ancestors, the elbow (a right angle
/// from the parent's trunk to this row), and for an expanded parent the stub from its
/// disclosure box down to the row's bottom edge, where the children's trunk starts.
pub fn connector_segments(
    depth: usize,
    has_children: bool,
    expanded: bool,
    trunks: &[bool],
) -> Vec<([f32; 2], [f32; 2])> {
    let h = METRICS.row_height;
    let mid = h / 2.0;
    let mut out = Vec::new();
    if let Some(parent) = depth.checked_sub(1) {
        for (k, _) in trunks.iter().enumerate().take(parent).filter(|(_, t)| **t) {
            out.push(([trunk_x(k), 0.0], [trunk_x(k), h]));
        }
        let x = trunk_x(parent);
        let below = trunks.get(parent).copied().unwrap_or(false);
        out.push(([x, 0.0], [x, if below { h } else { mid }]));
        let start = row_layout(depth).start;
        let end = if has_children {
            // up to the left edge of the disclosure box
            disclosure_box(depth).0
        } else {
            start + METRICS.leaf_reach
        };
        out.push(([x, mid], [end, mid]));
    }
    if has_children && expanded {
        let x = trunk_x(depth);
        out.push(([x, disclosure_box(depth).3], [x, h]));
    }
    out
}

/// What a drag hover shows (UI.md §13.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropMark {
    /// The row itself is highlighted: the nodes become its children.
    Into,
    /// A line on the row's top (`below` false) or bottom edge, its dot at the indent of
    /// `depth`, the depth the nodes land at.
    Line { depth: usize, below: bool },
}

pub fn drop_mark(row: &TreeRow, zone: DropZone) -> DropMark {
    match zone {
        DropZone::Into => DropMark::Into,
        DropZone::Before => DropMark::Line {
            depth: row.depth,
            below: false,
        },
        DropZone::After => DropMark::Line {
            // below an expanded row with children the node becomes its first child
            depth: row.depth + usize::from(row.has_children && row.expanded),
            below: true,
        },
    }
}

/// The row colours, derived from the theme tokens so every theme follows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowColours {
    pub guide: egui::Color32,
    pub separator: egui::Color32,
    pub alt: egui::Color32,
    /// The faint fill behind a selected row; the name itself is drawn in the selection colour.
    pub selected: egui::Color32,
    /// The 2 px bar along the left edge of a selected row: the selection colour itself, so a
    /// selected row reads in greyscale too (UI.md §24.1).
    pub bar: egui::Color32,
}

pub fn row_colours(theme: &crate::theme::Theme) -> RowColours {
    RowColours {
        guide: theme.panel.lerp_to_gamma(theme.weak_text, 0.75),
        separator: theme.panel.lerp_to_gamma(theme.weak_text, 0.18),
        alt: theme.panel.lerp_to_gamma(theme.faint, 0.55),
        selected: theme.panel.lerp_to_gamma(theme.selection, 0.16),
        bar: theme.selection,
    }
}

/// The colour of a row's name: the selection colour while selected (observed, G1 item 2: selected
/// names are amber; ours is the theme's `selection`, D22, owner note N11), else `None` for the
/// plain text colour.
pub fn name_colour(theme: &crate::theme::Theme, selected: bool) -> Option<egui::Color32> {
    selected.then_some(theme.selection)
}
