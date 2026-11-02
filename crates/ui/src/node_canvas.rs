//! A generic node canvas (docs/specs/UI.md §17): pan and zoom, node boxes with typed ports,
//! wires, selection, an add menu. It knows nothing about what the nodes mean: it draws a
//! [`CanvasModel`] and returns [`CanvasAction`]s; the owner turns them into registry commands.
//! Own design, **parity-pending G6**. Node materials (brief 124) implement the same trait.

use std::collections::BTreeSet;

use egui::{
    Align2, Color32, CornerRadius, FontId, Key, Pos2, Rect, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, Vec2, pos2, vec2,
};

/// Identifies a node of the model.
pub type NodeKey = u64;

/// World units (see UI.md §17.3).
pub const NODE_W: f32 = 200.0;
pub const TITLE_H: f32 = 24.0;
pub const ROW_H: f32 = 22.0;
pub const NODE_PAD: f32 = 8.0;
pub const PORT_R: f32 = 5.0;
/// Hit radius of a port, in screen pixels.
pub const PORT_HIT: f32 = 10.0;
/// Hit distance of a link, in screen pixels.
pub const LINK_HIT: f32 = 6.0;
pub const ZOOM_MIN: f32 = 0.25;
pub const ZOOM_MAX: f32 = 2.5;
/// Below this zoom the inline editors are drawn as text.
pub const EDITOR_ZOOM: f32 = 0.75;
/// Samples of a link curve.
const CURVE_STEPS: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Side {
    Input,
    Output,
}

impl Side {
    pub fn opposite(self) -> Side {
        match self {
            Side::Input => Side::Output,
            Side::Output => Side::Input,
        }
    }
}

/// One port of one node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PortAddr {
    pub node: NodeKey,
    pub port: String,
    pub side: Side,
}

impl PortAddr {
    pub fn new(node: NodeKey, port: &str, side: Side) -> PortAddr {
        PortAddr {
            node,
            port: port.to_string(),
            side,
        }
    }
}

/// The value of an unconnected input, as the canvas edits it.
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Float(f64),
    Int(i64),
    Bool(bool),
    Text(String),
    Vec3([f64; 3]),
    /// Shown as text, not editable (matrix, node link).
    Fixed(String),
}

#[derive(Debug, Clone)]
pub struct PortView {
    pub id: String,
    pub label: String,
    pub type_name: String,
    pub colour: Color32,
    pub linked: bool,
    /// Unlinked inputs only.
    pub constant: Option<Constant>,
    /// Live value text: an output's value, or the value arriving at a linked input.
    pub value_text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NodeView {
    pub title: String,
    pub category: String,
    /// Top-left corner, world units.
    pub pos: [f32; 2],
    pub inputs: Vec<PortView>,
    pub outputs: Vec<PortView>,
}

/// An entry of the add menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuEntry {
    pub id: String,
    pub label: String,
    pub category: String,
}

/// What the canvas draws. Implemented by the expression editor and, later, node materials.
pub trait CanvasModel {
    /// Keys in draw order (the last is on top).
    fn node_keys(&self) -> Vec<NodeKey>;
    fn node(&self, key: NodeKey) -> Option<NodeView>;
    /// `(output, input)` pairs.
    fn links(&self) -> Vec<(PortAddr, PortAddr)>;
    /// Whether an output may feed an input; the error is the reason shown to the user.
    fn check_link(&self, from: &PortAddr, to: &PortAddr) -> Result<(), String>;
    fn menu_entries(&self) -> Vec<MenuEntry>;
}

/// What the user asked for; the owner runs it and the next frame shows the result.
#[derive(Debug, Clone, PartialEq)]
pub enum CanvasAction {
    AddNode {
        type_id: String,
        pos: [f32; 2],
    },
    /// Absolute positions; the moves of one drag share a gesture number.
    Move {
        moves: Vec<(NodeKey, [f32; 2])>,
        gesture: u64,
    },
    /// `released` is an input whose link is moved to `to` in the same step.
    Connect {
        from: PortAddr,
        to: PortAddr,
        released: Option<PortAddr>,
    },
    Disconnect {
        to: PortAddr,
    },
    Delete {
        nodes: Vec<NodeKey>,
    },
    SetConstant {
        node: NodeKey,
        port: String,
        value: Constant,
        gesture: Option<u64>,
    },
    /// A connection the model refused, with the reason.
    Refused(String),
}

// --- view ------------------------------------------------------------------------------------

/// Pan and zoom: `screen = origin + pan + world * zoom`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub pan: Vec2,
    pub zoom: f32,
}

impl Default for View {
    fn default() -> View {
        View {
            pan: vec2(60.0, 40.0),
            zoom: 1.0,
        }
    }
}

impl View {
    pub fn to_screen(&self, origin: Pos2, p: Pos2) -> Pos2 {
        origin + self.pan + p.to_vec2() * self.zoom
    }

    pub fn to_world(&self, origin: Pos2, p: Pos2) -> Pos2 {
        ((p - origin - self.pan) / self.zoom).to_pos2()
    }

    pub fn rect_to_screen(&self, origin: Pos2, r: Rect) -> Rect {
        Rect::from_min_max(self.to_screen(origin, r.min), self.to_screen(origin, r.max))
    }

    /// Multiplies the zoom by `factor` (clamped) keeping the world point under `at` fixed.
    pub fn zoom_about(&mut self, origin: Pos2, at: Pos2, factor: f32) {
        let world = self.to_world(origin, at);
        self.zoom = (self.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
        let moved = self.to_screen(origin, world);
        self.pan += at - moved;
    }
}

// --- layout and hit testing (pure) -----------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NodeBox {
    pub key: NodeKey,
    pub view: NodeView,
    /// World rectangle.
    pub rect: Rect,
}

pub fn node_height(outputs: usize, inputs: usize) -> f32 {
    TITLE_H + ROW_H * (outputs + inputs) as f32 + NODE_PAD
}

impl NodeBox {
    /// Row index (outputs first, then inputs) of a port.
    pub fn row_of(&self, port: &str, side: Side) -> Option<usize> {
        match side {
            Side::Output => self.view.outputs.iter().position(|p| p.id == port),
            Side::Input => self
                .view
                .inputs
                .iter()
                .position(|p| p.id == port)
                .map(|i| i + self.view.outputs.len()),
        }
    }

    /// Centre of the row `row`, world units, at the box's left or right edge.
    pub fn row_y(&self, row: usize) -> f32 {
        self.rect.top() + TITLE_H + ROW_H * (row as f32 + 0.5)
    }

    pub fn port_world(&self, port: &str, side: Side) -> Option<Pos2> {
        let row = self.row_of(port, side)?;
        let x = match side {
            Side::Output => self.rect.right(),
            Side::Input => self.rect.left(),
        };
        Some(pos2(x, self.row_y(row)))
    }
}

/// What is under a point.
#[derive(Debug, Clone, PartialEq)]
pub enum Hit {
    Port(PortAddr),
    Node(NodeKey),
    /// Index into [`Layout::links`].
    Link(usize),
    Nothing,
}

/// The model laid out in world units.
#[derive(Debug, Clone, Default)]
pub struct Layout {
    pub boxes: Vec<NodeBox>,
    pub links: Vec<(PortAddr, PortAddr)>,
}

impl Layout {
    pub fn build(model: &dyn CanvasModel) -> Layout {
        let boxes = model
            .node_keys()
            .into_iter()
            .filter_map(|key| {
                let view = model.node(key)?;
                let h = node_height(view.outputs.len(), view.inputs.len());
                let rect = Rect::from_min_size(pos2(view.pos[0], view.pos[1]), vec2(NODE_W, h));
                Some(NodeBox { key, view, rect })
            })
            .collect();
        Layout {
            boxes,
            links: model.links(),
        }
    }

    pub fn node(&self, key: NodeKey) -> Option<&NodeBox> {
        self.boxes.iter().find(|b| b.key == key)
    }

    pub fn port_world(&self, a: &PortAddr) -> Option<Pos2> {
        self.node(a.node)?.port_world(&a.port, a.side)
    }

    /// Screen position of a port.
    pub fn port_screen(&self, view: &View, origin: Pos2, a: &PortAddr) -> Option<Pos2> {
        self.port_world(a).map(|p| view.to_screen(origin, p))
    }

    /// Screen rectangle of a node.
    pub fn node_screen(&self, view: &View, origin: Pos2, key: NodeKey) -> Option<Rect> {
        self.node(key).map(|b| view.rect_to_screen(origin, b.rect))
    }

    /// The port nearest to `p` within [`PORT_HIT`] pixels, restricted to `side` if given.
    pub fn hit_port(
        &self,
        view: &View,
        origin: Pos2,
        p: Pos2,
        side: Option<Side>,
    ) -> Option<PortAddr> {
        let mut best: Option<(f32, PortAddr)> = None;
        for b in self.boxes.iter().rev() {
            for (sd, ports) in [
                (Side::Output, &b.view.outputs),
                (Side::Input, &b.view.inputs),
            ] {
                if side.is_some_and(|s| s != sd) {
                    continue;
                }
                for port in ports {
                    let Some(w) = b.port_world(&port.id, sd) else {
                        continue;
                    };
                    let d = view.to_screen(origin, w).distance(p);
                    if d <= PORT_HIT && best.as_ref().is_none_or(|(bd, _)| d < *bd) {
                        best = Some((d, PortAddr::new(b.key, &port.id, sd)));
                    }
                }
            }
        }
        best.map(|(_, a)| a)
    }

    /// The topmost node whose box contains `p`.
    pub fn hit_node(&self, view: &View, origin: Pos2, p: Pos2) -> Option<NodeKey> {
        self.boxes
            .iter()
            .rev()
            .find(|b| view.rect_to_screen(origin, b.rect).contains(p))
            .map(|b| b.key)
    }

    /// The link whose curve passes within [`LINK_HIT`] pixels of `p` (the nearest).
    pub fn hit_link(&self, view: &View, origin: Pos2, p: Pos2) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for (i, (from, to)) in self.links.iter().enumerate() {
            let (Some(a), Some(b)) = (
                self.port_screen(view, origin, from),
                self.port_screen(view, origin, to),
            ) else {
                continue;
            };
            let pts = curve(a, b, view.zoom);
            let d = pts
                .windows(2)
                .map(|w| segment_distance(p, w[0], w[1]))
                .fold(f32::INFINITY, f32::min);
            if d <= LINK_HIT && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, i));
            }
        }
        best.map(|(_, i)| i)
    }

    /// Ports first, then nodes, then links.
    pub fn hit(&self, view: &View, origin: Pos2, p: Pos2) -> Hit {
        if let Some(a) = self.hit_port(view, origin, p, None) {
            return Hit::Port(a);
        }
        if let Some(n) = self.hit_node(view, origin, p) {
            return Hit::Node(n);
        }
        match self.hit_link(view, origin, p) {
            Some(i) => Hit::Link(i),
            None => Hit::Nothing,
        }
    }

    /// Nodes whose box intersects the world rectangle.
    pub fn nodes_in_rect(&self, world: Rect) -> Vec<NodeKey> {
        self.boxes
            .iter()
            .filter(|b| b.rect.intersects(world))
            .map(|b| b.key)
            .collect()
    }

    /// The link into an input, if any (index and the output it comes from).
    pub fn link_into(&self, to: &PortAddr) -> Option<(usize, &PortAddr)> {
        self.links
            .iter()
            .enumerate()
            .find(|(_, (_, t))| t.node == to.node && t.port == to.port)
            .map(|(i, (f, _))| (i, f))
    }
}

/// A cubic Bézier from `a` to `b` with horizontal tangents, as `CURVE_STEPS + 1` points.
pub fn curve(a: Pos2, b: Pos2, zoom: f32) -> Vec<Pos2> {
    let dx = (40.0 * zoom).max((b.x - a.x).abs() / 2.0);
    let (c1, c2) = (a + vec2(dx, 0.0), b - vec2(dx, 0.0));
    (0..=CURVE_STEPS)
        .map(|i| {
            let t = i as f32 / CURVE_STEPS as f32;
            let u = 1.0 - t;
            let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            pos2(
                w[0] * a.x + w[1] * c1.x + w[2] * c2.x + w[3] * b.x,
                w[0] * a.y + w[1] * c1.y + w[2] * c2.y + w[3] * b.y,
            )
        })
        .collect()
}

/// Distance from `p` to the segment `a`-`b`.
pub fn segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_sq();
    if len2 == 0.0 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// The add menu entries matching `query` (case-insensitive substring of id, label or category).
pub fn filter_entries<'a>(entries: &'a [MenuEntry], query: &str) -> Vec<&'a MenuEntry> {
    let q = query.trim().to_lowercase();
    entries
        .iter()
        .filter(|e| {
            q.is_empty()
                || e.id.to_lowercase().contains(&q)
                || e.label.to_lowercase().contains(&q)
                || e.category.to_lowercase().contains(&q)
        })
        .collect()
}

/// A title bar colour from the category (stable hash to a hue).
pub fn category_colour(category: &str) -> Color32 {
    let h = category.bytes().fold(2166136261u32, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(16777619)
    });
    let hue = (h % 360) as f32 / 360.0;
    egui::epaint::Hsva::new(hue, 0.45, 0.42, 1.0).into()
}

// --- state and drawing -----------------------------------------------------------------------

/// The pointer interaction in progress.
#[derive(Debug, Clone)]
enum Drag {
    None,
    Pan,
    Move {
        /// Selected nodes with their world positions at the start.
        start: Vec<(NodeKey, Pos2)>,
        /// Pointer (screen) at the start.
        origin: Pos2,
        gesture: u64,
    },
    Box {
        start: Pos2,
        additive: bool,
    },
    Wire {
        anchor: PortAddr,
        /// The input whose link was picked up.
        released: Option<PortAddr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuTarget {
    /// The add menu at a world position.
    Add {
        world: Pos2,
    },
    Node(NodeKey),
    /// A link, by its input.
    Link(PortAddr),
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub screen: Pos2,
    pub target: MenuTarget,
    pub query: String,
    opened: bool,
}

/// Canvas state that is not model state: view, selection, the drag in progress, the menu.
pub struct CanvasState {
    pub view: View,
    pub selection: BTreeSet<NodeKey>,
    /// Screen rectangle of the canvas as last drawn.
    pub rect: Rect,
    /// The model as last drawn (tests and owners read port positions from it).
    pub layout: Layout,
    /// The pointer is over the canvas (the owner keeps Delete away from `object.delete`).
    pub hot: bool,
    /// Set by the owner when Delete was pressed while `hot` and the key map swallowed it.
    pub delete_requested: bool,
    pub menu: Option<Menu>,
    /// The last refusal, until the next interaction.
    pub notice: Option<String>,
    /// While a wire is dragged: the port under the pointer and whether the link is allowed
    /// (green ring) or why not (red ring and text).
    pub wire_hover: Option<(PortAddr, Result<(), String>)>,
    drag: Drag,
    gesture_seq: u64,
    value_gesture: Option<u64>,
    /// The pointer position last frame, for wire feedback.
    pointer: Option<Pos2>,
}

impl Default for CanvasState {
    fn default() -> CanvasState {
        CanvasState {
            view: View::default(),
            selection: BTreeSet::new(),
            rect: Rect::from_min_size(Pos2::ZERO, vec2(100.0, 100.0)),
            layout: Layout::default(),
            hot: false,
            delete_requested: false,
            menu: None,
            notice: None,
            wire_hover: None,
            drag: Drag::None,
            gesture_seq: 0,
            value_gesture: None,
            pointer: None,
        }
    }
}

const BG: Color32 = Color32::from_rgb(30, 32, 36);
const GRID: Color32 = Color32::from_rgb(40, 43, 48);
const NODE_FILL: Color32 = Color32::from_rgb(46, 49, 56);
const NODE_STROKE: Color32 = Color32::from_rgb(78, 82, 92);
const ACCENT: Color32 = Color32::from_rgb(240, 170, 60);
const OK: Color32 = Color32::from_rgb(90, 200, 110);
const BAD: Color32 = Color32::from_rgb(230, 80, 70);
const TEXT: Color32 = Color32::from_gray(225);
const WEAK: Color32 = Color32::from_gray(140);

fn font(size: f32, zoom: f32) -> FontId {
    FontId::proportional((size * zoom).clamp(5.0, 32.0))
}

impl CanvasState {
    /// The port under the pointer while a wire is dragged, with the verdict.
    fn wire_feedback(
        &self,
        model: &dyn CanvasModel,
        anchor: &PortAddr,
        p: Pos2,
    ) -> Option<(PortAddr, Result<(), String>)> {
        let target =
            self.layout
                .hit_port(&self.view, self.rect.min, p, Some(anchor.side.opposite()))?;
        let (from, to) = match anchor.side {
            Side::Output => (anchor, &target),
            Side::Input => (&target, anchor),
        };
        let verdict = model.check_link(from, to);
        Some((target, verdict))
    }

    /// Draws the canvas and returns what the user did this frame.
    pub fn show(&mut self, ui: &mut Ui, model: &dyn CanvasModel) -> Vec<CanvasAction> {
        let mut actions = Vec::new();
        let size = ui.available_size_before_wrap().max(vec2(160.0, 120.0));
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let resp = resp.accessible_name("Node canvas");
        self.rect = rect;
        self.layout = Layout::build(model);
        let alive: BTreeSet<NodeKey> = self.layout.boxes.iter().map(|b| b.key).collect();
        self.selection.retain(|k| alive.contains(k));
        self.hot = resp.hovered() || self.menu.is_some();
        let pointer = ui.input(|i| i.pointer.latest_pos());
        self.pointer = pointer;

        self.zoom_input(ui, &resp, pointer);
        self.pointer_input(ui, &resp, model, &mut actions);
        self.key_input(ui, &resp, &mut actions);

        self.wire_hover = match (&self.drag, self.pointer) {
            (Drag::Wire { anchor, .. }, Some(p)) => self.wire_feedback(model, anchor, p),
            _ => None,
        };
        let painter = ui.painter_at(rect);
        self.draw(&painter);
        self.editors(ui, &mut actions);
        self.menu_ui(ui, model, &mut actions);
        for a in &actions {
            if let CanvasAction::Refused(m) = a {
                self.notice = Some(m.clone());
            }
        }
        if let Some(n) = &self.notice {
            painter.text(
                rect.left_bottom() + vec2(10.0, -8.0),
                Align2::LEFT_BOTTOM,
                n,
                FontId::proportional(13.0),
                BAD,
            );
        }
        actions
    }

    fn zoom_input(&mut self, ui: &Ui, resp: &egui::Response, pointer: Option<Pos2>) {
        if !resp.hovered() || self.menu.is_some() {
            return;
        }
        let Some(at) = pointer else {
            return;
        };
        let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
        let factor = if (pinch - 1.0).abs() > f32::EPSILON {
            pinch
        } else if scroll != 0.0 {
            1.0015f32.powf(scroll)
        } else {
            return;
        };
        ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
        self.view.zoom_about(self.rect.min, at, factor);
    }

    fn pointer_input(
        &mut self,
        ui: &Ui,
        resp: &egui::Response,
        model: &dyn CanvasModel,
        actions: &mut Vec<CanvasAction>,
    ) {
        use egui::PointerButton::{Middle, Primary};
        let origin = self.rect.min;
        let (mods, space) = ui.input(|i| (i.modifiers, i.key_down(Key::Space)));
        let additive = mods.shift || mods.command;

        if resp.drag_started() {
            self.notice = None;
            self.menu = None;
            let press = ui
                .input(|i| i.pointer.press_origin())
                .unwrap_or(self.rect.center());
            if resp.drag_started_by(Middle) || (mods.alt || space) {
                self.drag = Drag::Pan;
            } else if resp.drag_started_by(Primary) {
                self.drag = match self.layout.hit(&self.view, origin, press) {
                    Hit::Port(addr) => self.start_wire(addr),
                    Hit::Node(key) => {
                        if !self.selection.contains(&key) {
                            if !additive {
                                self.selection.clear();
                            }
                            self.selection.insert(key);
                        }
                        self.gesture_seq += 1;
                        let start = self
                            .selection
                            .iter()
                            .filter_map(|k| Some((*k, self.layout.node(*k)?.rect.min)))
                            .collect();
                        Drag::Move {
                            start,
                            origin: press,
                            gesture: self.gesture_seq,
                        }
                    }
                    _ => Drag::Box {
                        start: self.view.to_world(origin, press),
                        additive,
                    },
                };
            }
        }

        if resp.dragged() {
            let now = resp.interact_pointer_pos().or(self.pointer);
            match &self.drag {
                Drag::Pan => self.view.pan += resp.drag_delta(),
                Drag::Move {
                    start,
                    origin: press,
                    gesture,
                } => {
                    if let Some(now) = now {
                        let d = (now - *press) / self.view.zoom;
                        let moves: Vec<(NodeKey, [f32; 2])> = start
                            .iter()
                            .map(|(k, p)| (*k, [p.x + d.x, p.y + d.y]))
                            .collect();
                        if d != Vec2::ZERO {
                            actions.push(CanvasAction::Move {
                                moves,
                                gesture: *gesture,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        if resp.drag_stopped() {
            let end = self.pointer.or_else(|| resp.interact_pointer_pos());
            match std::mem::replace(&mut self.drag, Drag::None) {
                Drag::Box { start, additive } => {
                    if let Some(end) = end {
                        let a = start;
                        let b = self.view.to_world(origin, end);
                        let hits = self.layout.nodes_in_rect(Rect::from_two_pos(a, b));
                        if !additive {
                            self.selection.clear();
                        }
                        self.selection.extend(hits);
                    }
                }
                Drag::Wire { anchor, released } => {
                    self.finish_wire(model, anchor, released, end, actions);
                }
                _ => {}
            }
        }

        if resp.clicked_by(Primary) {
            self.notice = None;
            self.menu = None;
            if let Some(p) = resp.interact_pointer_pos() {
                match self.layout.hit(&self.view, origin, p) {
                    Hit::Node(key) => {
                        if additive {
                            if !self.selection.remove(&key) {
                                self.selection.insert(key);
                            }
                        } else {
                            self.selection = BTreeSet::from([key]);
                        }
                    }
                    Hit::Nothing | Hit::Link(_) => {
                        if !additive {
                            self.selection.clear();
                        }
                    }
                    Hit::Port(_) => {}
                }
            }
        }
        if resp.double_clicked_by(Primary)
            && let Some(p) = resp.interact_pointer_pos()
            && self.layout.hit(&self.view, origin, p) == Hit::Nothing
        {
            self.open_menu(
                p,
                MenuTarget::Add {
                    world: self.view.to_world(origin, p),
                },
            );
        }
        if resp.secondary_clicked()
            && let Some(p) = resp.interact_pointer_pos()
        {
            let target = match self.layout.hit(&self.view, origin, p) {
                Hit::Node(key) => {
                    if !self.selection.contains(&key) {
                        self.selection = BTreeSet::from([key]);
                    }
                    MenuTarget::Node(key)
                }
                Hit::Link(i) => MenuTarget::Link(self.layout.links[i].1.clone()),
                _ => MenuTarget::Add {
                    world: self.view.to_world(origin, p),
                },
            };
            self.open_menu(p, target);
        }
    }

    fn start_wire(&self, addr: PortAddr) -> Drag {
        match addr.side {
            Side::Output => Drag::Wire {
                anchor: addr,
                released: None,
            },
            Side::Input => match self.layout.link_into(&addr) {
                Some((_, from)) => Drag::Wire {
                    anchor: from.clone(),
                    released: Some(addr),
                },
                None => Drag::Wire {
                    anchor: addr,
                    released: None,
                },
            },
        }
    }

    fn finish_wire(
        &mut self,
        model: &dyn CanvasModel,
        anchor: PortAddr,
        released: Option<PortAddr>,
        end: Option<Pos2>,
        actions: &mut Vec<CanvasAction>,
    ) {
        let origin = self.rect.min;
        let Some(end) = end else {
            return;
        };
        let over_port = self
            .layout
            .hit_port(&self.view, origin, end, None)
            .is_some();
        match self.wire_feedback(model, &anchor, end) {
            Some((target, Ok(()))) => {
                let (from, to) = match anchor.side {
                    Side::Output => (anchor, target),
                    Side::Input => (target, anchor),
                };
                // Dropping a picked-up link back on its own input changes nothing.
                if released.as_ref() == Some(&to) {
                    return;
                }
                actions.push(CanvasAction::Connect { from, to, released });
            }
            Some((_, Err(reason))) => actions.push(CanvasAction::Refused(reason)),
            None => {
                // Empty canvas: a picked-up link is cut; a new wire is dropped. Dropping on a
                // port of the wrong side is neither.
                if !over_port && let Some(to) = released {
                    actions.push(CanvasAction::Disconnect { to });
                }
            }
        }
    }

    fn open_menu(&mut self, screen: Pos2, target: MenuTarget) {
        self.menu = Some(Menu {
            screen,
            target,
            query: String::new(),
            opened: false,
        });
    }

    fn key_input(&mut self, ui: &Ui, resp: &egui::Response, actions: &mut Vec<CanvasAction>) {
        let pressed = ui.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace));
        let typing = ui.ctx().egui_wants_keyboard_input();
        let want = std::mem::take(&mut self.delete_requested) || (pressed && !typing);
        if want && (resp.hovered() || self.hot) && self.menu.is_none() && !self.selection.is_empty()
        {
            actions.push(CanvasAction::Delete {
                nodes: self.selection.iter().copied().collect(),
            });
        }
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            self.menu = None;
            if matches!(self.drag, Drag::Wire { .. }) {
                self.drag = Drag::None;
            }
        }
    }

    fn draw(&self, painter: &egui::Painter) {
        let (rect, view) = (self.rect, self.view);
        let origin = rect.min;
        painter.rect_filled(rect, 0.0, BG);
        // Grid.
        let step = 40.0 * view.zoom;
        if step >= 8.0 {
            let off = (origin.to_vec2() + view.pan).to_pos2();
            let mut x = rect.left() + (off.x - rect.left()).rem_euclid(step);
            while x < rect.right() {
                painter.line_segment(
                    [pos2(x, rect.top()), pos2(x, rect.bottom())],
                    Stroke::new(1.0, GRID),
                );
                x += step;
            }
            let mut y = rect.top() + (off.y - rect.top()).rem_euclid(step);
            while y < rect.bottom() {
                painter.line_segment(
                    [pos2(rect.left(), y), pos2(rect.right(), y)],
                    Stroke::new(1.0, GRID),
                );
                y += step;
            }
        }

        // Links, except a picked-up one.
        let picked = match &self.drag {
            Drag::Wire { released, .. } => released.clone(),
            _ => None,
        };
        for (from, to) in &self.layout.links {
            if picked.as_ref() == Some(to) {
                continue;
            }
            let (Some(a), Some(b)) = (
                self.layout.port_screen(&view, origin, from),
                self.layout.port_screen(&view, origin, to),
            ) else {
                continue;
            };
            let colour = self.port_colour(from);
            painter.add(egui::Shape::line(
                curve(a, b, view.zoom),
                Stroke::new((2.0 * view.zoom).max(1.0), colour),
            ));
        }

        // Nodes.
        let zoom = view.zoom;
        let r = CornerRadius::same((5.0 * zoom).clamp(2.0, 8.0) as u8);
        for b in &self.layout.boxes {
            let sr = view.rect_to_screen(origin, b.rect);
            if !sr.intersects(rect) {
                continue;
            }
            let selected = self.selection.contains(&b.key);
            painter.rect_filled(sr, r, NODE_FILL);
            let title = Rect::from_min_size(sr.min, vec2(sr.width(), TITLE_H * zoom));
            painter.rect_filled(
                title,
                CornerRadius {
                    nw: r.nw,
                    ne: r.ne,
                    sw: 0,
                    se: 0,
                },
                category_colour(&b.view.category),
            );
            painter.rect_stroke(
                sr,
                r,
                if selected {
                    Stroke::new(2.0, ACCENT)
                } else {
                    Stroke::new(1.0, NODE_STROKE)
                },
                StrokeKind::Inside,
            );
            if zoom >= 0.4 {
                painter.text(
                    title.left_center() + vec2(8.0 * zoom, 0.0),
                    Align2::LEFT_CENTER,
                    &b.view.title,
                    font(13.0, zoom),
                    TEXT,
                );
            }
            let n_out = b.view.outputs.len();
            for (i, p) in b.view.outputs.iter().enumerate() {
                let y = view.to_screen(origin, pos2(0.0, b.row_y(i))).y;
                if zoom >= 0.4 {
                    painter.text(
                        pos2(sr.right() - 12.0 * zoom, y),
                        Align2::RIGHT_CENTER,
                        &p.label,
                        font(12.0, zoom),
                        TEXT,
                    );
                    if let Some(v) = &p.value_text {
                        painter.text(
                            pos2(sr.left() + 10.0 * zoom, y),
                            Align2::LEFT_CENTER,
                            v,
                            font(11.0, zoom),
                            WEAK,
                        );
                    }
                }
                self.dot(painter, pos2(sr.right(), y), p);
            }
            for (j, p) in b.view.inputs.iter().enumerate() {
                let y = view.to_screen(origin, pos2(0.0, b.row_y(n_out + j))).y;
                if zoom >= 0.4 {
                    painter.text(
                        pos2(sr.left() + 12.0 * zoom, y),
                        Align2::LEFT_CENTER,
                        &p.label,
                        font(12.0, zoom),
                        TEXT,
                    );
                    if !self.shows_editor(p)
                        && let Some(v) = p.value_text.clone().or_else(|| constant_text(p))
                    {
                        painter.text(
                            pos2(sr.right() - 8.0 * zoom, y),
                            Align2::RIGHT_CENTER,
                            v,
                            font(11.0, zoom),
                            WEAK,
                        );
                    }
                }
                self.dot(painter, pos2(sr.left(), y), p);
            }
        }

        // The wire being dragged and the verdict on the port under the pointer.
        if let Drag::Wire { anchor, .. } = &self.drag
            && let Some(a) = self.layout.port_screen(&view, origin, anchor)
            && let Some(p) = self.pointer
        {
            let verdict = self.wire_hover.clone();
            let end = verdict
                .as_ref()
                .and_then(|(t, _)| self.layout.port_screen(&view, origin, t))
                .unwrap_or(p);
            let (from, to) = match anchor.side {
                Side::Output => (a, end),
                Side::Input => (end, a),
            };
            painter.add(egui::Shape::line(
                curve(from, to, view.zoom),
                Stroke::new((2.0 * zoom).max(1.0), self.port_colour(anchor)),
            ));
            if let Some((t, v)) = &verdict
                && let Some(c) = self.layout.port_screen(&view, origin, t)
            {
                let (colour, text) = match v {
                    Ok(()) => (OK, None),
                    Err(m) => (BAD, Some(m.as_str())),
                };
                painter.circle_stroke(c, (PORT_R + 4.0) * zoom.max(0.8), Stroke::new(2.0, colour));
                if let Some(t) = text {
                    painter.text(
                        p + vec2(14.0, 10.0),
                        Align2::LEFT_TOP,
                        t,
                        FontId::proportional(13.0),
                        BAD,
                    );
                }
            }
        }

        // The box selection rectangle.
        if let (Drag::Box { start, .. }, Some(p)) = (&self.drag, self.pointer) {
            let r = Rect::from_two_pos(view.to_screen(origin, *start), p);
            painter.rect_filled(r, 0.0, ACCENT.gamma_multiply(0.12));
            painter.rect_stroke(r, 0.0, Stroke::new(1.0, ACCENT), StrokeKind::Inside);
        }
    }

    fn port_colour(&self, a: &PortAddr) -> Color32 {
        self.layout
            .node(a.node)
            .and_then(|b| {
                let ports = match a.side {
                    Side::Output => &b.view.outputs,
                    Side::Input => &b.view.inputs,
                };
                ports.iter().find(|p| p.id == a.port).map(|p| p.colour)
            })
            .unwrap_or(WEAK)
    }

    fn dot(&self, painter: &egui::Painter, c: Pos2, p: &PortView) {
        let r = (PORT_R * self.view.zoom).max(2.5);
        if p.linked {
            painter.circle_filled(c, r, p.colour);
        } else {
            painter.circle_filled(c, r, NODE_FILL);
            painter.circle_stroke(c, r, Stroke::new(1.5, p.colour));
        }
    }

    fn shows_editor(&self, p: &PortView) -> bool {
        !p.linked
            && self.view.zoom >= EDITOR_ZOOM
            && matches!(&p.constant, Some(c) if !matches!(c, Constant::Fixed(_)))
    }

    /// The inline editors of unlinked inputs, drawn as real widgets over the canvas.
    fn editors(&mut self, ui: &mut Ui, actions: &mut Vec<CanvasAction>) {
        let (view, origin, canvas) = (self.view, self.rect.min, self.rect);
        let boxes = self.layout.boxes.clone();
        for b in &boxes {
            let n_out = b.view.outputs.len();
            for (j, p) in b.view.inputs.iter().enumerate() {
                if !self.shows_editor(p) {
                    continue;
                }
                let Some(constant) = &p.constant else {
                    continue;
                };
                let y = view.to_screen(origin, pos2(0.0, b.row_y(n_out + j))).y;
                let sr = view.rect_to_screen(origin, b.rect);
                let left = sr.left() + 70.0 * view.zoom;
                let row = Rect::from_min_max(
                    pos2(left, y - ROW_H * view.zoom * 0.5 + 1.0),
                    pos2(
                        sr.right() - 8.0 * view.zoom,
                        y + ROW_H * view.zoom * 0.5 - 1.0,
                    ),
                );
                if !row.intersects(canvas) {
                    continue;
                }
                let mut child = ui.new_child(
                    UiBuilder::new()
                        .id_salt(("node_input", b.key, &p.id))
                        .max_rect(row)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                child.set_clip_rect(canvas.intersect(ui.clip_rect()));
                child.spacing_mut().item_spacing.x = 2.0;
                child.spacing_mut().interact_size.y = (row.height()).max(14.0);
                let name = format!("{} #{} {}", b.view.title, b.key, p.label);
                if let Some((value, dragging, started)) = edit_constant(&mut child, constant, &name)
                {
                    if started {
                        self.gesture_seq += 1;
                        self.value_gesture = Some(self.gesture_seq);
                    }
                    let gesture = if dragging { self.value_gesture } else { None };
                    actions.push(CanvasAction::SetConstant {
                        node: b.key,
                        port: p.id.clone(),
                        value,
                        gesture,
                    });
                }
            }
        }
    }

    fn menu_ui(&mut self, ui: &mut Ui, model: &dyn CanvasModel, actions: &mut Vec<CanvasAction>) {
        let Some(menu) = self.menu.clone() else {
            return;
        };
        let ctx = ui.ctx().clone();
        let mut close = false;
        let mut query = menu.query.clone();
        let area = egui::Area::new(ui.scope_id().with("node_canvas_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(menu.screen)
            .show(&ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(220.0);
                    match &menu.target {
                        MenuTarget::Add { world } => {
                            let r = ui.add(
                                egui::TextEdit::singleline(&mut query)
                                    .hint_text("Search nodes")
                                    .desired_width(f32::INFINITY),
                            );
                            r.clone().accessible_name("Search nodes");
                            if !menu.opened {
                                r.request_focus();
                            }
                            let entries = model.menu_entries();
                            let found = filter_entries(&entries, &query);
                            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                            let mut chosen: Option<String> = None;
                            if enter && let Some(first) = found.first() {
                                chosen = Some(first.id.clone());
                            }
                            egui::ScrollArea::vertical()
                                .max_height(280.0)
                                .id_salt("node_menu_scroll")
                                .show(ui, |ui| {
                                    if found.is_empty() {
                                        ui.weak("No match");
                                    }
                                    let mut last = "";
                                    for e in &found {
                                        if e.category != last {
                                            ui.weak(&e.category);
                                            last = &e.category;
                                        }
                                        if ui.button(format!("{} ({})", e.label, e.id)).clicked() {
                                            chosen = Some(e.id.clone());
                                        }
                                    }
                                });
                            if let Some(type_id) = chosen {
                                actions.push(CanvasAction::AddNode {
                                    type_id,
                                    pos: [world.x, world.y],
                                });
                                close = true;
                            }
                        }
                        MenuTarget::Node(key) => {
                            let nodes: Vec<NodeKey> = if self.selection.contains(key) {
                                self.selection.iter().copied().collect()
                            } else {
                                vec![*key]
                            };
                            let label = if nodes.len() > 1 {
                                format!("Delete {} nodes", nodes.len())
                            } else {
                                "Delete node".to_string()
                            };
                            if ui.button(label).clicked() {
                                actions.push(CanvasAction::Delete { nodes });
                                close = true;
                            }
                            if let Some(b) = self.layout.node(*key) {
                                for p in b.view.inputs.iter().filter(|p| p.linked) {
                                    if ui.button(format!("Disconnect {}", p.label)).clicked() {
                                        actions.push(CanvasAction::Disconnect {
                                            to: PortAddr::new(*key, &p.id, Side::Input),
                                        });
                                        close = true;
                                    }
                                }
                            }
                        }
                        MenuTarget::Link(to) => {
                            if ui.button("Disconnect").clicked() {
                                actions.push(CanvasAction::Disconnect { to: to.clone() });
                                close = true;
                            }
                        }
                    }
                });
            });
        let outside = ctx.input(|i| i.pointer.any_pressed())
            && ctx
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|p| !area.response.rect.contains(p))
            && menu.opened;
        if let Some(m) = self.menu.as_mut() {
            m.query = query;
            m.opened = true;
        }
        if close || outside {
            self.menu = None;
        }
    }
}

fn constant_text(p: &PortView) -> Option<String> {
    Some(match p.constant.as_ref()? {
        Constant::Float(f) => fmt_float(*f),
        Constant::Int(i) => i.to_string(),
        Constant::Bool(b) => b.to_string(),
        Constant::Text(s) => format!("\"{}\"", cut(s, 16)),
        Constant::Vec3(v) => format!(
            "({}, {}, {})",
            fmt_float(v[0]),
            fmt_float(v[1]),
            fmt_float(v[2])
        ),
        Constant::Fixed(s) => s.clone(),
    })
}

/// A float with at most 3 decimals and no trailing zeros.
pub fn fmt_float(f: f64) -> String {
    let s = format!("{f:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" || s == "-0" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// The first `n` characters, with an ellipsis when cut.
pub fn cut(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let head: String = s.chars().take(n).collect();
        format!("{head}...")
    }
}

/// Draws the editor of a constant; the new value, whether it is a drag and whether the drag
/// just started, if the user changed it.
fn edit_constant(ui: &mut Ui, c: &Constant, name: &str) -> Option<(Constant, bool, bool)> {
    match c {
        Constant::Float(v) => {
            let mut x = *v;
            let r = ui
                .add(egui::DragValue::new(&mut x).speed(0.01).max_decimals(4))
                .accessible_name(name);
            r.changed()
                .then(|| (Constant::Float(x), r.dragged(), r.drag_started()))
        }
        Constant::Int(v) => {
            let mut x = *v;
            let r = ui
                .add(egui::DragValue::new(&mut x).speed(0.1))
                .accessible_name(name);
            r.changed()
                .then(|| (Constant::Int(x), r.dragged(), r.drag_started()))
        }
        Constant::Bool(v) => {
            let mut x = *v;
            let r = ui.checkbox(&mut x, "").on_hover_text(name);
            let r = r.clone().accessible_name(name);
            r.changed().then_some((Constant::Bool(x), false, false))
        }
        Constant::Text(s) => {
            // Typed text is kept in a buffer while the box has focus and committed when it
            // loses it (Enter or a click elsewhere): one edit, one undo step.
            let id = ui.scope_id().with("text_buffer");
            let editing: Option<String> = ui.data(|d| d.get_temp(id));
            let mut buf = editing.unwrap_or_else(|| s.clone());
            let r = ui
                .add(egui::TextEdit::singleline(&mut buf).desired_width(f32::INFINITY))
                .accessible_name(name);
            if r.has_focus() {
                ui.data_mut(|d| d.insert_temp(id, buf.clone()));
            }
            if r.lost_focus() {
                ui.data_mut(|d| d.remove::<String>(id));
                return (buf != *s).then_some((Constant::Text(buf), false, false));
            }
            None
        }
        Constant::Vec3(v) => {
            let mut x = *v;
            let mut out = None;
            for (i, axis) in ["x", "y", "z"].into_iter().enumerate() {
                let r = ui
                    .add(egui::DragValue::new(&mut x[i]).speed(0.01).max_decimals(3))
                    .accessible_name(format!("{name} {axis}"));
                if r.changed() {
                    out = Some((r.dragged(), r.drag_started()));
                }
            }
            out.map(|(d, s)| (Constant::Vec3(x), d, s))
        }
        Constant::Fixed(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        nodes: Vec<(NodeKey, [f32; 2], usize, usize)>,
        links: Vec<(PortAddr, PortAddr)>,
    }

    fn port(id: &str) -> PortView {
        PortView {
            id: id.into(),
            label: id.into(),
            type_name: "float".into(),
            colour: Color32::WHITE,
            linked: false,
            constant: None,
            value_text: None,
        }
    }

    impl CanvasModel for Fake {
        fn node_keys(&self) -> Vec<NodeKey> {
            self.nodes.iter().map(|n| n.0).collect()
        }

        fn node(&self, key: NodeKey) -> Option<NodeView> {
            let (_, pos, i, o) = self.nodes.iter().find(|n| n.0 == key)?;
            Some(NodeView {
                title: format!("n{key}"),
                category: "math".into(),
                pos: *pos,
                inputs: (0..*i).map(|k| port(&format!("i{k}"))).collect(),
                outputs: (0..*o).map(|k| port(&format!("o{k}"))).collect(),
            })
        }

        fn links(&self) -> Vec<(PortAddr, PortAddr)> {
            self.links.clone()
        }

        fn check_link(&self, _: &PortAddr, _: &PortAddr) -> Result<(), String> {
            Ok(())
        }

        fn menu_entries(&self) -> Vec<MenuEntry> {
            Vec::new()
        }
    }

    fn fake() -> Fake {
        Fake {
            nodes: vec![(1, [0.0, 0.0], 2, 1), (2, [300.0, 40.0], 2, 1)],
            links: vec![(
                PortAddr::new(1, "o0", Side::Output),
                PortAddr::new(2, "i1", Side::Input),
            )],
        }
    }

    fn close(a: Pos2, b: Pos2) -> bool {
        a.distance(b) < 1e-3
    }

    #[test]
    fn view_transforms_round_trip() {
        let v = View {
            pan: vec2(13.0, -7.0),
            zoom: 1.7,
        };
        let o = pos2(100.0, 50.0);
        let w = pos2(33.0, -12.0);
        assert!(close(v.to_world(o, v.to_screen(o, w)), w));
        assert!(close(
            View::default().to_screen(o, pos2(0.0, 0.0)),
            pos2(160.0, 90.0)
        ));
    }

    #[test]
    fn zoom_keeps_the_point_under_the_pointer_and_clamps() {
        let mut v = View::default();
        let o = pos2(10.0, 10.0);
        let at = pos2(300.0, 200.0);
        let world = v.to_world(o, at);
        v.zoom_about(o, at, 1.5);
        assert!((v.zoom - 1.5).abs() < 1e-6);
        assert!(close(v.to_screen(o, world), at));
        v.zoom_about(o, at, 100.0);
        assert_eq!(v.zoom, ZOOM_MAX);
        assert!(close(v.to_screen(o, world), at));
        v.zoom_about(o, at, 1e-6);
        assert_eq!(v.zoom, ZOOM_MIN);
    }

    #[test]
    fn node_and_port_geometry() {
        let l = Layout::build(&fake());
        let b = l.node(1).unwrap();
        assert_eq!(b.rect.height(), node_height(1, 2));
        assert_eq!(b.rect.width(), NODE_W);
        // Outputs come first, right edge; inputs after, left edge.
        let o = l.port_world(&PortAddr::new(1, "o0", Side::Output)).unwrap();
        assert!(close(o, pos2(NODE_W, TITLE_H + ROW_H * 0.5)));
        let i1 = l.port_world(&PortAddr::new(1, "i1", Side::Input)).unwrap();
        assert!(close(i1, pos2(0.0, TITLE_H + ROW_H * 2.5)));
        let n2 = l.port_world(&PortAddr::new(2, "i0", Side::Input)).unwrap();
        assert!(close(n2, pos2(300.0, 40.0 + TITLE_H + ROW_H * 1.5)));
        assert!(l.port_world(&PortAddr::new(1, "zz", Side::Input)).is_none());
    }

    #[test]
    fn hit_testing_prefers_ports_then_nodes_then_links() {
        let l = Layout::build(&fake());
        let (v, o) = (View::default(), pos2(0.0, 0.0));
        let port_pos = l
            .port_screen(&v, o, &PortAddr::new(1, "o0", Side::Output))
            .unwrap();
        // Just outside the box but within the hit radius: a port.
        assert_eq!(
            l.hit(&v, o, port_pos + vec2(6.0, 0.0)),
            Hit::Port(PortAddr::new(1, "o0", Side::Output))
        );
        // Inside the box away from ports: the node.
        assert_eq!(
            l.hit(&v, o, v.to_screen(o, pos2(100.0, 20.0))),
            Hit::Node(1)
        );
        // Far away: nothing.
        assert_eq!(l.hit(&v, o, pos2(900.0, 900.0)), Hit::Nothing);
        // On the middle of the link between the two nodes.
        let a = l
            .port_screen(&v, o, &PortAddr::new(1, "o0", Side::Output))
            .unwrap();
        let b = l
            .port_screen(&v, o, &PortAddr::new(2, "i1", Side::Input))
            .unwrap();
        let mid = curve(a, b, 1.0)[CURVE_STEPS / 2];
        assert_eq!(l.hit(&v, o, mid), Hit::Link(0));
        assert_eq!(l.hit(&v, o, mid + vec2(0.0, 40.0)), Hit::Nothing);
    }

    #[test]
    fn overlapping_nodes_hit_the_last_drawn() {
        let m = Fake {
            nodes: vec![(1, [0.0, 0.0], 0, 1), (2, [20.0, 10.0], 0, 1)],
            links: vec![],
        };
        let l = Layout::build(&m);
        let (v, o) = (View::default(), Pos2::ZERO);
        assert_eq!(l.hit_node(&v, o, v.to_screen(o, pos2(50.0, 30.0))), Some(2));
        assert_eq!(l.hit_node(&v, o, v.to_screen(o, pos2(5.0, 5.0))), Some(1));
    }

    #[test]
    fn hit_radius_is_in_screen_pixels_whatever_the_zoom() {
        let l = Layout::build(&fake());
        let o = Pos2::ZERO;
        let a = PortAddr::new(1, "o0", Side::Output);
        for zoom in [0.5, 1.0, 2.0] {
            let v = View {
                pan: vec2(20.0, 20.0),
                zoom,
            };
            let c = l.port_screen(&v, o, &a).unwrap();
            assert_eq!(
                l.hit_port(&v, o, c + vec2(PORT_HIT - 1.0, 0.0), None),
                Some(a.clone())
            );
            assert_eq!(l.hit_port(&v, o, c + vec2(PORT_HIT + 1.0, 0.0), None), None);
            // Side restriction.
            assert_eq!(l.hit_port(&v, o, c, Some(Side::Input)), None);
        }
    }

    #[test]
    fn box_select_takes_every_intersecting_node() {
        let l = Layout::build(&fake());
        assert_eq!(
            l.nodes_in_rect(Rect::from_min_max(pos2(-10.0, -10.0), pos2(5.0, 5.0))),
            vec![1]
        );
        assert_eq!(
            l.nodes_in_rect(Rect::from_min_max(pos2(150.0, 20.0), pos2(320.0, 60.0))),
            vec![1, 2]
        );
        assert!(
            l.nodes_in_rect(Rect::from_min_max(pos2(220.0, 200.0), pos2(280.0, 260.0)))
                .is_empty()
        );
        // The rectangle may be dragged in any direction.
        assert_eq!(
            l.nodes_in_rect(Rect::from_two_pos(pos2(5.0, 5.0), pos2(-10.0, -10.0))),
            vec![1]
        );
    }

    #[test]
    fn link_into_finds_the_output() {
        let l = Layout::build(&fake());
        let to = PortAddr::new(2, "i1", Side::Input);
        let (i, from) = l.link_into(&to).unwrap();
        assert_eq!(i, 0);
        assert_eq!(from.node, 1);
        assert!(l.link_into(&PortAddr::new(2, "i0", Side::Input)).is_none());
    }

    #[test]
    fn curves_start_and_end_on_the_ports_and_distance_is_exact() {
        let (a, b) = (pos2(0.0, 0.0), pos2(200.0, 100.0));
        let c = curve(a, b, 1.0);
        assert_eq!(c.len(), CURVE_STEPS + 1);
        assert!(close(c[0], a) && close(c[CURVE_STEPS], b));
        assert!(
            (segment_distance(pos2(5.0, 5.0), pos2(0.0, 0.0), pos2(10.0, 0.0)) - 5.0).abs() < 1e-6
        );
        assert!(
            (segment_distance(pos2(-3.0, 4.0), pos2(0.0, 0.0), pos2(10.0, 0.0)) - 5.0).abs() < 1e-6
        );
        assert_eq!(
            segment_distance(pos2(1.0, 1.0), pos2(2.0, 2.0), pos2(2.0, 2.0)),
            2f32.sqrt()
        );
    }

    #[test]
    fn menu_search_matches_id_label_and_category() {
        let e = |id: &str, label: &str, cat: &str| MenuEntry {
            id: id.into(),
            label: label.into(),
            category: cat.into(),
        };
        let all = vec![
            e("math.add", "Add", "math"),
            e("vector.add", "Add Vectors", "vector"),
            e("time.now", "Time", "time"),
        ];
        assert_eq!(filter_entries(&all, "").len(), 3);
        assert_eq!(filter_entries(&all, " ADD ").len(), 2);
        assert_eq!(filter_entries(&all, "vector.").len(), 1);
        assert_eq!(filter_entries(&all, "tim")[0].id, "time.now");
        assert!(filter_entries(&all, "zzz").is_empty());
    }

    #[test]
    fn number_and_text_formatting() {
        assert_eq!(fmt_float(1.0), "1");
        assert_eq!(fmt_float(1.2345), "1.234");
        assert_eq!(fmt_float(-0.0001), "0");
        assert_eq!(fmt_float(-2.5), "-2.5");
        assert_eq!(cut("short", 16), "short");
        assert_eq!(cut("abcdefghijklmnopqrstuvwxyz", 4), "abcd...");
    }

    #[test]
    fn category_colours_are_stable_and_differ() {
        assert_eq!(category_colour("math"), category_colour("math"));
        assert_ne!(category_colour("math"), category_colour("vector"));
    }
}
