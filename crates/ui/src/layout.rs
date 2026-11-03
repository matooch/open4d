//! The default window arrangement and the command bar / tool palette contents
//! (docs/specs/UI.md §11). Plain, serialisable data: the egui code only draws it.
//!
//! Every button is a registry command (`id` plus JSON `args`); there is no private logic
//! behind a button. The arrangement is our own design (**parity-pending**, GUI session G1).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::dock::Dock;

/// Bumped when the serialised form changes.
pub const LAYOUT_VERSION: u32 = 1;

/// Initial sizes of the resizable areas, in points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Sizes {
    /// Width of the tool palette column.
    pub left_width: f32,
    /// Width of the manager column on the right.
    pub right_width: f32,
    /// Share of the right column's height taken by the top manager (0..1).
    pub right_top_fraction: f32,
    /// Height of the bottom strip.
    pub bottom_height: f32,
}

impl Default for Sizes {
    fn default() -> Sizes {
        Sizes {
            left_width: 150.0,
            right_width: 340.0,
            right_top_fraction: 0.45,
            bottom_height: 120.0,
        }
    }
}

/// A window layout: which panels sit in which area, and the area sizes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub version: u32,
    pub dock: Dock,
    pub sizes: Sizes,
    /// The workspace the arrangement started from (UI.md §21.1); empty for none.
    #[serde(default)]
    pub workspace: String,
}

/// A layout read leniently: the layout to use and why it is not the one in the file.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub layout: Layout,
    pub warning: Option<String>,
}

impl Layout {
    /// The default arrangement: command bar on top, tool palette left, viewport centre,
    /// Object manager top right, Attribute manager bottom right, timeline strip (with the
    /// Coordinates manager as a second tab) at the bottom.
    pub fn standard() -> Layout {
        Layout {
            version: LAYOUT_VERSION,
            dock: Dock::default(),
            sizes: Sizes::default(),
            workspace: "Standard".into(),
        }
    }

    /// Reads a layout file's text; anything unusable (not JSON, bad fields, a repeated
    /// panel, another version) gives the standard layout and a warning (UI.md §21.1).
    pub fn load_text(text: &str) -> Loaded {
        let fallback = |why: String| Loaded {
            layout: Layout::standard(),
            warning: Some(format!(
                "Layout file not usable ({why}): using the standard layout"
            )),
        };
        // read the version first so an old or new file is named as such
        match serde_json::from_str::<Value>(text) {
            Ok(v) => match v.get("version").and_then(Value::as_u64) {
                Some(n) if n < u64::from(LAYOUT_VERSION) => {
                    return fallback(format!(
                        "older version {n}, this build writes {LAYOUT_VERSION}"
                    ));
                }
                Some(n) if n > u64::from(LAYOUT_VERSION) => {
                    return fallback(format!(
                        "newer version {n}, this build reads {LAYOUT_VERSION}"
                    ));
                }
                _ => {}
            },
            Err(e) => return fallback(e.to_string()),
        }
        match Layout::from_json(text) {
            Ok(layout) => Loaded {
                layout,
                warning: None,
            },
            Err(e) => fallback(e),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a layout is plain data")
    }

    /// Parses a layout; rejects other versions, repeated panels and bad active indices.
    pub fn from_json(text: &str) -> Result<Layout, String> {
        let layout: Layout = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if layout.version != LAYOUT_VERSION {
            return Err(format!("unsupported layout version {}", layout.version));
        }
        if !layout.dock.is_valid() {
            return Err("layout lists a panel twice or has a bad active tab".into());
        }
        Ok(layout)
    }
}

/// One button: the command it runs and how it looks. `glyph` is a simple placeholder
/// symbol (icons are brief 148); `label` is the button text, `tip` the hover text.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandButton {
    pub command: &'static str,
    pub args: Value,
    pub glyph: &'static str,
    pub label: &'static str,
    pub tip: &'static str,
}

impl CommandButton {
    /// The text on the button, e.g. `"<- Undo"`. Different from the menu item texts, so a
    /// label identifies one widget.
    pub fn text(&self) -> String {
        format!("{} {}", self.glyph, self.label)
    }
}

/// A named run of buttons separated from the next group.
#[derive(Debug, Clone, PartialEq)]
pub struct ButtonGroup {
    pub name: &'static str,
    pub buttons: Vec<CommandButton>,
}

fn button(
    command: &'static str,
    args: Value,
    glyph: &'static str,
    label: &'static str,
    tip: &'static str,
) -> CommandButton {
    CommandButton {
        command,
        args,
        glyph,
        label,
        tip,
    }
}

fn create(
    ty: &'static str,
    glyph: &'static str,
    label: &'static str,
    tip: &'static str,
) -> CommandButton {
    button("object.create", json!({"type": ty}), glyph, label, tip)
}

/// The command bar along the top (**parity-pending** order): history, object creation,
/// view, animation.
pub fn command_bar() -> Vec<ButtonGroup> {
    let none = || json!({});
    vec![
        ButtonGroup {
            name: "History",
            buttons: vec![
                button("edit.undo", none(), "<-", "Undo", "Undo the last step"),
                button("edit.redo", none(), "->", "Redo", "Redo the undone step"),
                button(
                    "object.delete",
                    none(),
                    "x",
                    "Remove",
                    "Delete the selected objects",
                ),
            ],
        },
        ButtonGroup {
            name: "Objects",
            buttons: vec![
                create("forge.cube", "#", "Cube", "Create a cube"),
                create("forge.sphere", "o", "Sphere", "Create a sphere"),
                create("forge.cylinder", "|", "Cylinder", "Create a cylinder"),
                create("forge.cone", "^", "Cone", "Create a cone"),
                create("forge.plane", "-", "Plane", "Create a plane"),
                create("forge.torus", "O", "Torus", "Create a torus"),
                button(
                    "mesh.make_editable",
                    none(),
                    "*",
                    "Make editable",
                    "Convert the selection to polygon objects",
                ),
            ],
        },
        ButtonGroup {
            name: "View",
            buttons: vec![
                button(
                    "view.shading",
                    json!({"mode": "smooth"}),
                    "(",
                    "Shaded",
                    "Smooth shading",
                ),
                button(
                    "view.shading",
                    json!({"mode": "wire"}),
                    "%",
                    "Wire",
                    "Wireframe",
                ),
                button(
                    "view.frame_all",
                    none(),
                    "+",
                    "Frame all",
                    "Fit the whole scene in the view",
                ),
            ],
        },
        ButtonGroup {
            name: "Animation",
            buttons: vec![
                button("timeline.play", none(), ">", "Run", "Play the animation"),
                button("timeline.stop", none(), "#", "Halt", "Stop playback"),
                button(
                    "key.record",
                    none(),
                    "o",
                    "Key",
                    "Record keys for the active objects",
                ),
            ],
        },
    ]
}

/// The tool palette down the left (**parity-pending** order): what a viewport click
/// selects, selection operations, then the mesh commands that need no pointer input.
/// Interactive tools (cuts, bridge, pen) stay in the Mesh menu.
pub fn tool_palette() -> Vec<ButtonGroup> {
    let none = || json!({});
    let mode = |m: &'static str, glyph, label, tip| {
        button("select.mode", json!({"mode": m}), glyph, label, tip)
    };
    vec![
        ButtonGroup {
            name: "Mode",
            buttons: vec![
                mode("model", "[]", "Objects", "Click selects whole objects"),
                mode("points", ".", "Pts", "Click selects points"),
                mode("edges", "/", "Edg", "Click selects edges"),
                mode("polys", "\\", "Pol", "Click selects polygons"),
            ],
        },
        ButtonGroup {
            name: "Selection",
            buttons: vec![
                button("select.all", none(), "**", "All", "Select all"),
                button("select.none", none(), "0", "None", "Deselect all"),
                button("select.invert", none(), "~", "Invert", "Invert selection"),
                button("select.grow", none(), "+", "Grow", "Grow the selection"),
                button(
                    "select.shrink",
                    none(),
                    "-",
                    "Shrink",
                    "Shrink the selection",
                ),
            ],
        },
        ButtonGroup {
            name: "Modeling",
            buttons: vec![
                button("mesh.extrude", none(), "^^", "Extrude", "Extrude polygons"),
                button(
                    "mesh.extrude_inner",
                    none(),
                    "[+]",
                    "Inner",
                    "Extrude polygons inwards",
                ),
                button("mesh.bevel", none(), "/|", "Bevel", "Bevel the selection"),
                button(
                    "mesh.subdivide",
                    none(),
                    "##",
                    "Subdiv",
                    "Subdivide polygons",
                ),
                button(
                    "mesh.triangulate",
                    none(),
                    "/\\",
                    "Tris",
                    "Triangulate polygons",
                ),
                button("mesh.optimize", none(), "@", "Tidy", "Remove unused points"),
                button(
                    "mesh.reverse_normals",
                    none(),
                    "<>",
                    "Flip",
                    "Reverse normals",
                ),
            ],
        },
    ]
}

/// Every button of the command bar and the tool palette.
pub fn all_buttons() -> Vec<CommandButton> {
    command_bar()
        .into_iter()
        .chain(tool_palette())
        .flat_map(|g| g.buttons)
        .collect()
}
