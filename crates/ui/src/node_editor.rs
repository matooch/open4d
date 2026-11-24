//! The expression node editor panel (docs/specs/UI.md §17): `ExpressionModel` shows a
//! `forge.expression` tag's graph on the generic [`crate::node_canvas`], and every canvas
//! action runs as one `graph.*` command. Own design, **parity-pending G6**.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use egui::{Color32, Ui};
use forge_core::{NodeId, Scene};
use forge_material::node_material::{self, NODE_MATERIAL, SURFACE};
use forge_nodes::{
    Graph, GraphNodeId, NodeRegistry, PortRef, PortType, PortValue, TAG, load_graph,
};
use serde_json::{Value as Json, json};

use crate::commands::execute;
use crate::node_canvas::{
    CanvasAction, CanvasModel, CanvasState, Constant, MenuEntry, NodeKey, NodeView, PortAddr,
    PortView, Side, View, cut, fmt_float,
};
use crate::state::EditorState;

/// Port colour by type (UI.md §17.3).
pub fn type_colour(ty: PortType) -> Color32 {
    match ty {
        PortType::Float => Color32::from_rgb(110, 165, 235),
        PortType::Int => Color32::from_rgb(70, 195, 165),
        PortType::Bool => Color32::from_rgb(235, 135, 85),
        PortType::Vec3 => Color32::from_rgb(235, 205, 80),
        PortType::Colour => Color32::from_rgb(225, 110, 205),
        PortType::Matrix => Color32::from_rgb(150, 125, 235),
        PortType::String => Color32::from_gray(170),
        PortType::Node => Color32::from_rgb(90, 215, 235),
        PortType::Any => Color32::from_gray(240),
    }
}

/// The value of a port as shown on the canvas (UI.md §17.5).
pub fn fmt_json(v: &Json) -> String {
    match v {
        Json::Null => "none".to_string(),
        Json::Bool(b) => b.to_string(),
        Json::Number(n) => n
            .as_i64()
            .map_or_else(|| fmt_float(n.as_f64().unwrap_or(0.0)), |i| i.to_string()),
        Json::String(s) => format!("\"{}\"", cut(s, 16)),
        Json::Array(a) if a.len() == 16 => "mat4".to_string(),
        Json::Array(a) => {
            let parts: Vec<String> = a.iter().map(fmt_json).collect();
            format!("({})", parts.join(", "))
        }
        Json::Object(_) => "{...}".to_string(),
    }
}

/// A node as the output of `graph.get`-style data needs it: constant of an input.
fn constant_of(ty: PortType, v: &PortValue) -> Constant {
    match (ty, v) {
        (PortType::Any, v) | (_, v @ (PortValue::Matrix(_) | PortValue::Node(_))) => {
            Constant::Fixed(fmt_json(&v.to_json()))
        }
        (_, PortValue::Float(f)) => Constant::Float(*f),
        (_, PortValue::Int(i)) => Constant::Int(*i),
        (_, PortValue::Bool(b)) => Constant::Bool(*b),
        (_, PortValue::String(s)) => Constant::Text(s.clone()),
        (_, PortValue::Vec3(v) | PortValue::Colour(v)) => Constant::Vec3(*v),
    }
}

/// The add menu entries from the result of `graph.node_types`.
pub fn entries_from_types(types: &Json) -> Vec<MenuEntry> {
    let mut out: Vec<MenuEntry> = types["types"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|t| {
                    Some(MenuEntry {
                        id: t["id"].as_str()?.to_string(),
                        label: t["label"].as_str()?.to_string(),
                        category: t["category"].as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| (&a.category, &a.label, &a.id).cmp(&(&b.category, &b.label, &b.id)));
    out
}

/// A tag's graph on the canvas.
pub struct ExpressionModel {
    pub graph: Arc<Graph>,
    pub entries: Arc<Vec<MenuEntry>>,
    /// `"node.port"` to the value from the dry run.
    pub values: Arc<BTreeMap<String, Json>>,
    registry: &'static NodeRegistry,
}

impl ExpressionModel {
    fn port_key(node: GraphNodeId, port: &str) -> String {
        format!("{node}.{port}")
    }
}

fn graph_node(key: NodeKey) -> GraphNodeId {
    GraphNodeId(key)
}

impl CanvasModel for ExpressionModel {
    fn node_keys(&self) -> Vec<NodeKey> {
        self.graph.nodes().map(|n| n.id.0).collect()
    }

    fn node(&self, key: NodeKey) -> Option<NodeView> {
        let n = self.graph.node(graph_node(key))?;
        let def = self.registry.get(&n.type_id)?;
        let inputs = def
            .inputs
            .iter()
            .map(|p| {
                let to = PortRef::new(n.id, &p.id);
                let link = self.graph.link_into(&to);
                let value_text = link.and_then(|l| {
                    self.values
                        .get(&Self::port_key(l.from.node, &l.from.port))
                        .map(fmt_json)
                });
                let constant = if link.is_none() {
                    let v = n.values.get(&p.id).unwrap_or(&p.default);
                    Some(constant_of(p.ty, v))
                } else {
                    None
                };
                PortView {
                    id: p.id.clone(),
                    label: p.label.clone(),
                    type_name: p.ty.name().to_string(),
                    colour: type_colour(p.ty),
                    linked: link.is_some(),
                    constant,
                    value_text,
                }
            })
            .collect();
        let outputs = def
            .outputs
            .iter()
            .filter(|_| n.type_id != SURFACE)
            .map(|p| PortView {
                id: p.id.clone(),
                label: p.label.clone(),
                type_name: p.ty.name().to_string(),
                colour: type_colour(p.ty),
                linked: self
                    .graph
                    .links()
                    .iter()
                    .any(|l| l.from.node == n.id && l.from.port == p.id),
                constant: None,
                value_text: self.values.get(&Self::port_key(n.id, &p.id)).map(fmt_json),
            })
            .collect();
        Some(NodeView {
            title: def.label.clone(),
            category: def.category.clone(),
            pos: [n.pos[0] as f32, n.pos[1] as f32],
            inputs,
            outputs,
        })
    }

    fn links(&self) -> Vec<(PortAddr, PortAddr)> {
        self.graph
            .links()
            .iter()
            .map(|l| {
                (
                    PortAddr::new(l.from.node.0, &l.from.port, Side::Output),
                    PortAddr::new(l.to.node.0, &l.to.port, Side::Input),
                )
            })
            .collect()
    }

    fn check_link(&self, from: &PortAddr, to: &PortAddr) -> Result<(), String> {
        // The graph itself decides, on a copy: the same checks `graph.connect` makes.
        let mut g = (*self.graph).clone();
        g.connect(
            self.registry,
            PortRef::new(graph_node(from.node), &from.port),
            PortRef::new(graph_node(to.node), &to.port),
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
    }

    fn menu_entries(&self) -> Vec<MenuEntry> {
        (*self.entries).clone()
    }
}

/// What was last read for the chosen tag.
struct Cache {
    key: (u64, i64, NodeId),
    graph: Arc<Graph>,
    values: Arc<BTreeMap<String, Json>>,
    error: Option<String>,
}

/// Panel state (not saved): the chosen tag, the canvas, the cached graph and live values.
#[derive(Default)]
pub struct NodeEditorState {
    pub tag: Option<NodeId>,
    pub canvas: CanvasState,
    materials: bool,
    material: Option<NodeId>,
    material_canvas: CanvasState,
    entries: Option<Arc<Vec<MenuEntry>>>,
    cache: Option<Cache>,
    /// Pan, zoom and selection of tags that are not shown now.
    saved: BTreeMap<NodeId, (View, BTreeSet<NodeKey>)>,
}

impl NodeEditorState {
    /// Shows `tag`, keeping the view and selection of the one shown before.
    pub fn show_tag(&mut self, tag: NodeId) {
        self.materials = false;
        if self.tag == Some(tag) {
            return;
        }
        if let Some(old) = self.tag {
            self.saved.insert(
                old,
                (self.canvas.view, std::mem::take(&mut self.canvas.selection)),
            );
        }
        let (view, selection) = self.saved.remove(&tag).unwrap_or_default();
        self.canvas.view = view;
        self.canvas.selection = selection;
        self.canvas.menu = None;
        self.canvas.notice = None;
        self.tag = Some(tag);
        self.cache = None;
    }

    pub fn show_material(&mut self, material: NodeId) {
        self.materials = true;
        if self.material != Some(material) {
            self.material = Some(material);
            self.material_canvas = CanvasState::default();
        }
    }

    pub(crate) fn active_canvas(&mut self) -> &mut CanvasState {
        if self.materials {
            &mut self.material_canvas
        } else {
            &mut self.canvas
        }
    }

    /// Layout and selection of the currently shown graph.
    pub fn canvas_view(&self) -> &CanvasState {
        if self.materials {
            &self.material_canvas
        } else {
            &self.canvas
        }
    }

    /// The error of the last dry run, if it failed.
    pub fn eval_error(&self) -> Option<&str> {
        self.cache.as_ref().and_then(|c| c.error.as_deref())
    }

    /// The live value of an output port from the last dry run.
    pub fn live_value(&self, node: u64, port: &str) -> Option<&Json> {
        self.cache.as_ref()?.values.get(&format!("{node}.{port}"))
    }
}

/// Every expression tag, as `(tag, "object / tag")`, in document order.
pub fn expression_tags(scene: &Scene) -> Vec<(NodeId, String)> {
    let mut out = Vec::new();
    let mut visit = |id: NodeId| {
        let Some(n) = scene.node(id) else {
            return;
        };
        for t in &n.tags {
            if let Some(tn) = scene.node(*t)
                && tn.type_id == TAG
            {
                out.push((*t, format!("{} / {}", n.name, tn.name)));
            }
        }
    };
    for r in scene.roots() {
        visit(*r);
        for d in scene.descendants(*r) {
            visit(d);
        }
    }
    out
}

/// The tag to show when none was chosen: the first on the active object, else the first.
pub fn default_tag(st: &EditorState) -> Option<NodeId> {
    let scene = st.doc.scene();
    let tags = expression_tags(scene);
    if let Some(active) = st.selection.active
        && let Some(n) = scene.node(active)
        && let Some((t, _)) = tags.iter().find(|(t, _)| n.tags.contains(t))
    {
        return Some(*t);
    }
    tags.first().map(|(t, _)| *t)
}

impl EditorState {
    /// Opens the Node Editor panel on `tag` (double-click on an expression tag).
    pub fn open_node_editor(&mut self, tag: NodeId) {
        self.dock.reveal(crate::dock::Panel::NodeEditor);
        self.node_editor.show_tag(tag);
    }
}

/// Reads the graph and runs the dry run when the scene, the frame or the tag changed.
fn refresh(st: &mut EditorState, tag: NodeId) {
    let key = (st.doc.scene().revision(), st.timeline.current, tag);
    if st.node_editor.cache.as_ref().is_some_and(|c| c.key == key) {
        return;
    }
    if st.node_editor.entries.is_none() {
        let types = execute(st, "graph.node_types", json!({})).unwrap_or(Json::Null);
        st.node_editor.entries = Some(Arc::new(entries_from_types(&types)));
    }
    let graph = load_graph(st.doc.scene(), tag);
    let (graph, mut error) = match graph {
        Ok(g) => (g, None),
        Err(e) => (Graph::new(), Some(e)),
    };
    let mut values = BTreeMap::new();
    if error.is_none() {
        match execute(st, "graph.evaluate", json!({"tag": tag.0})) {
            Ok(r) => {
                if let Some(o) = r["outputs"].as_object() {
                    values = o.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                }
            }
            Err(e) => error = Some(e),
        }
    }
    st.node_editor.cache = Some(Cache {
        key,
        graph: Arc::new(graph),
        values: Arc::new(values),
        error,
    });
}

fn value_json(c: &Constant) -> Json {
    match c {
        Constant::Float(f) => json!(f),
        Constant::Int(i) => json!(i),
        Constant::Bool(b) => json!(b),
        Constant::Text(s) => json!(s),
        Constant::Vec3(v) => json!(v),
        Constant::Fixed(_) => Json::Null,
    }
}

/// Runs one canvas action as a registry command. The error goes to the status bar.
pub fn run_action(st: &mut EditorState, tag: NodeId, action: CanvasAction) {
    st.status.clear();
    let result = match action {
        CanvasAction::AddNode { type_id, pos } => execute(
            st,
            "graph.add_node",
            json!({"tag": tag.0, "type": type_id, "pos": [pos[0], pos[1]]}),
        )
        .map(|r| {
            if let Some(n) = r["node"].as_u64() {
                st.node_editor.canvas.selection = BTreeSet::from([n]);
            }
        }),
        CanvasAction::Move { moves, gesture } => {
            let list: Vec<Json> = moves
                .iter()
                .map(|(n, p)| json!({"node": n, "x": p[0], "y": p[1]}))
                .collect();
            execute(
                st,
                "graph.move",
                json!({"tag": tag.0, "moves": list, "gesture": gesture}),
            )
            .map(|_| ())
        }
        CanvasAction::Connect { from, to, released } => {
            let mut a = json!({
                "tag": tag.0,
                "from_node": from.node, "from_port": from.port,
                "to_node": to.node, "to_port": to.port,
            });
            if let Some(r) = released {
                a["release_node"] = json!(r.node);
                a["release_port"] = json!(r.port);
            }
            execute(st, "graph.connect", a).map(|_| ())
        }
        CanvasAction::Disconnect { to } => execute(
            st,
            "graph.disconnect",
            json!({"tag": tag.0, "to_node": to.node, "to_port": to.port}),
        )
        .map(|_| ()),
        CanvasAction::Delete { nodes } => {
            let r = execute(st, "graph.remove", json!({"tag": tag.0, "nodes": nodes}));
            if r.is_ok() {
                st.node_editor.canvas.selection.clear();
            }
            r.map(|_| ())
        }
        CanvasAction::SetConstant {
            node,
            port,
            value,
            gesture,
        } => {
            let mut a =
                json!({"tag": tag.0, "node": node, "port": port, "value": value_json(&value)});
            if let Some(g) = gesture {
                a["gesture"] = json!(g);
            }
            execute(st, "graph.set_value", a).map(|_| ())
        }
        CanvasAction::Refused(reason) => {
            st.status = reason;
            Ok(())
        }
    };
    if let Err(e) = result {
        st.status = e;
    }
}

/// Draws the panel: the tag chooser, then the canvas.
pub fn node_editor_ui(ui: &mut Ui, st: &mut EditorState) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut st.node_editor.materials, false, "Expressions");
        ui.selectable_value(&mut st.node_editor.materials, true, "Materials");
    });
    if st.node_editor.materials {
        material_editor_ui(ui, st);
        return;
    }
    let tags = expression_tags(st.doc.scene());
    if st
        .node_editor
        .tag
        .is_none_or(|t| !tags.iter().any(|(id, _)| *id == t))
    {
        match default_tag(st) {
            Some(t) => st.node_editor.show_tag(t),
            None => {
                st.node_editor.tag = None;
                st.node_editor.cache = None;
            }
        }
    }
    let mut create = false;
    let mut chosen: Option<NodeId> = None;
    ui.horizontal(|ui| {
        let current = st
            .node_editor
            .tag
            .and_then(|t| tags.iter().find(|(id, _)| *id == t))
            .map_or_else(|| "none".to_string(), |(_, n)| n.clone());
        let label = ui.label("Expression tag");
        egui::ComboBox::from_id_salt("node_editor_tag")
            .selected_text(current)
            .show_ui(ui, |ui| {
                for (id, name) in &tags {
                    if ui
                        .selectable_label(st.node_editor.tag == Some(*id), name)
                        .clicked()
                    {
                        chosen = Some(*id);
                    }
                }
            })
            .response
            .labelled_by(label.id);
        let has_object = st.selection.active.is_some();
        if ui
            .add_enabled(has_object, egui::Button::new("Add expression tag"))
            .clicked()
        {
            create = true;
        }
        if let Some(e) = st.node_editor.eval_error() {
            ui.colored_label(crate::theme::Theme::current(ui.ctx()).error, e);
        }
    });
    if let Some(t) = chosen {
        st.node_editor.show_tag(t);
    }
    if create {
        st.status.clear();
        match execute(st, "object.create", json!({"type": TAG})) {
            Ok(r) => {
                if let Some(t) = r["node"].as_u64() {
                    st.node_editor.show_tag(NodeId(t));
                }
            }
            Err(e) => st.status = e,
        }
    }
    let Some(tag) = st.node_editor.tag else {
        ui.weak("No expression tag. Select an object and add one, or double-click an expression tag in the Objects panel.");
        st.node_editor.canvas.hot = false;
        return;
    };
    refresh(st, tag);
    let cache = st.node_editor.cache.as_ref().expect("refreshed above");
    let model = ExpressionModel {
        graph: cache.graph.clone(),
        entries: st
            .node_editor
            .entries
            .clone()
            .unwrap_or_else(|| Arc::new(Vec::new())),
        values: cache.values.clone(),
        registry: NodeRegistry::builtin(),
    };
    let actions = st.node_editor.canvas.show(ui, &model);
    for a in actions {
        run_action(st, tag, a);
    }
}

fn material_editor_ui(ui: &mut Ui, st: &mut EditorState) {
    let scene = st.doc.scene();
    let list: Vec<_> = scene
        .materials()
        .iter()
        .filter_map(|&id| {
            let node = scene.node(id).filter(|n| n.type_id == NODE_MATERIAL)?;
            Some((id, node.name.clone()))
        })
        .collect();
    let selected = st
        .node_editor
        .material
        .filter(|id| list.iter().any(|(m, _)| m == id))
        .or_else(|| list.first().map(|(id, _)| *id));
    let Some(mut material) = selected else {
        st.node_editor.material_canvas.hot = false;
        ui.weak("No node material. Create one in the Materials panel.");
        return;
    };
    let current = list
        .iter()
        .find(|(id, _)| *id == material)
        .map_or("none", |(_, name)| name);
    ui.horizontal(|ui| {
        let label = ui.label("Material graph");
        egui::ComboBox::from_id_salt("node_material")
            .selected_text(current)
            .show_ui(ui, |ui| {
                for (id, name) in &list {
                    ui.selectable_value(&mut material, *id, name);
                }
            })
            .response
            .labelled_by(label.id);
    });
    st.node_editor.show_material(material);
    let node = st.doc.scene().node(material).expect("listed material");
    let graph = match node_material::load_graph(node).and_then(|g| {
        node_material::compile_graph(&g)?;
        Ok(g)
    }) {
        Ok(g) => g,
        Err(e) => {
            st.node_editor.material_canvas.hot = false;
            ui.colored_label(crate::theme::Theme::current(ui.ctx()).error, e.to_string());
            return;
        }
    };
    let registry = node_material::registry();
    let model = ExpressionModel {
        graph: Arc::new(graph),
        registry,
        entries: Arc::new(
            registry
                .defs()
                .filter(|d| d.id != SURFACE)
                .map(|d| MenuEntry {
                    id: d.id.clone(),
                    label: d.label.clone(),
                    category: d.category.clone(),
                })
                .collect(),
        ),
        values: Arc::default(),
    };
    let actions = st.node_editor.material_canvas.show(ui, &model);
    for action in actions {
        st.status = run_material_action(st, material, action)
            .err()
            .unwrap_or_default();
    }
}

/// Run a shared canvas action against a hosted material graph.
pub fn run_material_action(
    st: &mut EditorState,
    material: NodeId,
    action: CanvasAction,
) -> Result<(), String> {
    let scene = st.doc.scene();
    let node = scene
        .node(material)
        .ok_or_else(|| "material not found".to_string())?;
    let mut graph = node_material::load_graph(node).map_err(|e| e.to_string())?;
    let reg = node_material::registry();
    let graph_error = |e: forge_nodes::GraphError| e.to_string();
    let reference = |p: &PortAddr| PortRef::new(graph_node(p.node), &p.port);
    let mut select = None;
    let mut delete = false;
    let gesture = match action {
        CanvasAction::AddNode { type_id, pos } => {
            let id = graph.add_node(reg, &type_id).map_err(graph_error)?;
            graph
                .set_position(id, pos.map(f64::from))
                .map_err(graph_error)?;
            select = Some(id.0);
            None
        }
        CanvasAction::Move { moves, gesture } => {
            for (id, pos) in moves {
                graph
                    .set_position(graph_node(id), pos.map(f64::from))
                    .map_err(graph_error)?;
            }
            Some(gesture)
        }
        CanvasAction::Connect { from, to, released } => {
            let to = reference(&to);
            if let Some(prev) = released.as_ref().map(reference).filter(|prev| prev != &to) {
                graph.disconnect(&prev).map_err(graph_error)?;
            }
            graph
                .connect(reg, reference(&from), to)
                .map_err(graph_error)?;
            None
        }
        CanvasAction::Disconnect { to } => {
            graph.disconnect(&reference(&to)).map_err(graph_error)?;
            None
        }
        CanvasAction::Delete { nodes } => {
            for id in nodes {
                let key = graph_node(id);
                if graph.node(key).is_some_and(|node| node.type_id == SURFACE) {
                    return Err("The Surface is required".into());
                }
                graph.remove_node(key).map_err(graph_error)?;
            }
            delete = true;
            None
        }
        CanvasAction::SetConstant {
            node,
            port,
            value,
            gesture,
        } => {
            let id = graph_node(node);
            let ty = graph
                .node(id)
                .and_then(|n| reg.get(&n.type_id))
                .and_then(|d| d.find_input(&port))
                .ok_or_else(|| format!("unknown input {port}"))?
                .ty;
            let val = PortValue::from_json(ty, &value_json(&value)).map_err(|e| e.to_string())?;
            graph.set_value(reg, id, &port, val).map_err(graph_error)?;
            gesture
        }
        CanvasAction::Refused(reason) => return Err(reason),
    };
    execute(
        st,
        "material.set_graph",
        json!({
            "material": material.0, "graph": graph.to_json(), "gesture": gesture,
        }),
    )?;
    if let Some(id) = select {
        st.node_editor.material_canvas.selection = BTreeSet::from([id]);
    } else if delete {
        st.node_editor.material_canvas.selection.clear();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_formatted_for_ports() {
        assert_eq!(fmt_json(&json!(1.0)), "1");
        assert_eq!(fmt_json(&json!(1.23456)), "1.235");
        assert_eq!(fmt_json(&json!(3)), "3");
        assert_eq!(fmt_json(&json!(true)), "true");
        assert_eq!(fmt_json(&json!([1.0, 2.5, 0.0])), "(1, 2.5, 0)");
        assert_eq!(
            fmt_json(&json!("abcdefghijklmnopqrstuv")),
            "\"abcdefghijklmnop...\""
        );
        assert_eq!(fmt_json(&Json::from(vec![0.0; 16])), "mat4");
        assert_eq!(fmt_json(&Json::Null), "none");
    }

    #[test]
    fn every_port_type_has_its_own_colour() {
        let mut seen = BTreeSet::new();
        for t in PortType::ALL {
            assert!(seen.insert(type_colour(t).to_array()), "{t:?}");
        }
    }

    #[test]
    fn constants_follow_the_port_type() {
        assert_eq!(
            constant_of(PortType::Float, &PortValue::Float(2.0)),
            Constant::Float(2.0)
        );
        assert_eq!(
            constant_of(PortType::Colour, &PortValue::Colour([1.0, 0.0, 0.5])),
            Constant::Vec3([1.0, 0.0, 0.5])
        );
        assert!(matches!(
            constant_of(PortType::Any, &PortValue::Float(1.0)),
            Constant::Fixed(_)
        ));
        assert!(matches!(
            constant_of(PortType::Node, &PortValue::Node(None)),
            Constant::Fixed(_)
        ));
    }

    #[test]
    fn menu_entries_come_sorted_from_the_command_result() {
        let types = json!({"types": [
            {"id": "vector.add", "label": "Add Vectors", "category": "vector"},
            {"id": "math.sub", "label": "Subtract", "category": "math"},
            {"id": "math.add", "label": "Add", "category": "math"},
        ]});
        let e = entries_from_types(&types);
        let ids: Vec<&str> = e.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["math.add", "math.sub", "vector.add"]);
        assert!(entries_from_types(&Json::Null).is_empty());
    }
}
