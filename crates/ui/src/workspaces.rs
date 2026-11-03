//! Named workspaces and saved layouts (docs/specs/UI.md §21; brief 156). Pure data and file
//! handling: the editor applies the results. View state only, never the document.

use std::path::PathBuf;

use crate::dock::{Area, Dock, Panel};
use crate::layout::{Layout, Loaded, Sizes};

/// The built-in arrangements. The names are our own (parity-pending G1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workspace {
    Standard,
    Modeling,
    Animate,
    Uv,
    Rigging,
    Node,
}

impl Workspace {
    pub const ALL: [Workspace; 6] = [
        Workspace::Standard,
        Workspace::Modeling,
        Workspace::Animate,
        Workspace::Uv,
        Workspace::Rigging,
        Workspace::Node,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Workspace::Standard => "Standard",
            Workspace::Modeling => "Modeling",
            Workspace::Animate => "Animate",
            Workspace::Uv => "UV",
            Workspace::Rigging => "Rigging",
            Workspace::Node => "Node",
        }
    }

    /// The workspace called `name`, ignoring case and surrounding space.
    pub fn from_name(name: &str) -> Option<Workspace> {
        let name = name.trim();
        Workspace::ALL
            .into_iter()
            .find(|w| w.name().eq_ignore_ascii_case(name))
    }

    /// The arrangement (UI.md §21.2).
    pub fn layout(self) -> Layout {
        use Area::{Bottom, Center, Left, RightBottom, RightTop, Top};
        use Panel::{
            Attributes, CommandBar, Console, Coordinates, History, NodeEditor, ObjectTree, Script,
            Timeline, ToolPalette, Viewport,
        };
        let sizes = |left_width, right_width, right_top_fraction, bottom_height| Sizes {
            left_width,
            right_width,
            right_top_fraction,
            bottom_height,
        };
        let (areas, sizes): (Vec<(Area, &[Panel])>, Sizes) = match self {
            Workspace::Standard => return Layout::standard(),
            Workspace::Modeling => (
                vec![
                    (Top, &[CommandBar]),
                    (Left, &[ToolPalette]),
                    (Center, &[Viewport]),
                    (RightTop, &[ObjectTree]),
                    (RightBottom, &[Attributes, Coordinates]),
                    (Bottom, &[History, Console]),
                ],
                sizes(150.0, 360.0, 0.35, 150.0),
            ),
            Workspace::Animate => (
                vec![
                    (Top, &[CommandBar]),
                    (Left, &[ToolPalette]),
                    (Center, &[Viewport]),
                    (RightTop, &[ObjectTree]),
                    (RightBottom, &[Attributes, Coordinates]),
                    (Bottom, &[Timeline, History]),
                ],
                sizes(150.0, 340.0, 0.40, 240.0),
            ),
            Workspace::Uv => (
                vec![
                    (Top, &[CommandBar]),
                    (Left, &[ToolPalette]),
                    (Center, &[Viewport]),
                    (RightBottom, &[Attributes, Coordinates, ObjectTree]),
                    (Bottom, &[History]),
                ],
                sizes(150.0, 400.0, 0.45, 120.0),
            ),
            Workspace::Rigging => (
                vec![
                    (Top, &[CommandBar]),
                    (Left, &[ToolPalette]),
                    (Center, &[Viewport]),
                    (RightTop, &[ObjectTree]),
                    (RightBottom, &[Attributes]),
                    (Bottom, &[Timeline, Coordinates, Script]),
                ],
                sizes(150.0, 380.0, 0.60, 160.0),
            ),
            Workspace::Node => (
                vec![
                    (Top, &[CommandBar]),
                    (Center, &[NodeEditor, Viewport]),
                    (RightTop, &[ObjectTree]),
                    (RightBottom, &[Attributes]),
                    (Bottom, &[Console, History, Script]),
                ],
                sizes(150.0, 340.0, 0.45, 200.0),
            ),
        };
        Layout {
            version: crate::layout::LAYOUT_VERSION,
            dock: Dock::arranged(&areas),
            sizes,
            workspace: self.name().into(),
        }
    }
}

/// Checks a user layout name and returns it trimmed (UI.md §21.3).
pub fn check_layout_name(name: &str) -> Result<String, String> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 48 {
        return Err("a layout name has 1 to 48 characters".into());
    }
    if !n.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        || !n
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_'))
    {
        return Err(format!(
            "bad layout name \"{n}\": use letters, digits, space, - and _, starting with a letter or digit"
        ));
    }
    if Workspace::from_name(n).is_some() {
        return Err(format!("\"{n}\" is the name of a workspace"));
    }
    Ok(n.to_string())
}

const LAST: &str = "_last";

/// The folder of saved layouts. Without a folder every write fails and the list is empty.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LayoutStore {
    dir: Option<PathBuf>,
}

impl LayoutStore {
    pub fn new(dir: Option<PathBuf>) -> LayoutStore {
        LayoutStore { dir }
    }

    /// The per-OS folder of this process (PLATFORM.md §8).
    pub fn platform() -> LayoutStore {
        LayoutStore::new(forge_platform::layouts_dir())
    }

    fn file(&self, stem: &str) -> Result<PathBuf, String> {
        let dir = self.dir.as_ref().ok_or("no layout folder")?;
        Ok(dir.join(format!("{stem}.json")))
    }

    fn write(&self, stem: &str, layout: &Layout) -> Result<(), String> {
        let path = self.file(stem)?;
        let dir = path.parent().ok_or("no layout folder")?;
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        let tmp = dir.join(format!("{stem}.json.tmp"));
        std::fs::write(&tmp, layout.to_json())
            .and_then(|()| std::fs::rename(&tmp, &path))
            .map_err(|e| format!("cannot write {}: {e}", path.display()))
    }

    fn read(&self, stem: &str) -> Result<Option<Loaded>, String> {
        let path = self.file(stem)?;
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(Some(Layout::load_text(&text))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("cannot read {}: {e}", path.display())),
        }
    }

    /// Saves `layout` as `name` (replacing a file of that name).
    pub fn save(&self, name: &str, layout: &Layout) -> Result<(), String> {
        self.write(&check_layout_name(name)?, layout)
    }

    /// Loads the layout `name`; a broken or old file gives the standard layout and a warning.
    pub fn load(&self, name: &str) -> Result<Loaded, String> {
        let name = check_layout_name(name)?;
        self.read(&name)?
            .ok_or_else(|| format!("no layout named \"{name}\""))
    }

    pub fn delete(&self, name: &str) -> Result<(), String> {
        let name = check_layout_name(name)?;
        let path = self.file(&name)?;
        std::fs::remove_file(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                format!("no layout named \"{name}\"")
            } else {
                format!("cannot delete {}: {e}", path.display())
            }
        })
    }

    /// The saved layout names, sorted; files that are not valid names are ignored.
    pub fn list(&self) -> Vec<String> {
        let Some(dir) = &self.dir else {
            return Vec::new();
        };
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let path = e.path();
                (path.extension()? == "json").then_some(())?;
                let stem = path.file_stem()?.to_str()?.to_string();
                check_layout_name(&stem).ok().filter(|n| *n == stem)
            })
            .collect();
        names.sort();
        names
    }

    /// Remembers the arrangement at exit.
    pub fn save_last(&self, layout: &Layout) -> Result<(), String> {
        self.write(LAST, layout)
    }

    /// The remembered arrangement, if there is one.
    pub fn load_last(&self) -> Result<Option<Loaded>, String> {
        self.read(LAST)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, LayoutStore) {
        let d = tempfile::tempdir().unwrap();
        let s = LayoutStore::new(Some(d.path().join("layouts")));
        (d, s)
    }

    #[test]
    fn names_are_matched_without_case() {
        assert_eq!(Workspace::from_name("uv"), Some(Workspace::Uv));
        assert_eq!(Workspace::from_name(" Node "), Some(Workspace::Node));
        assert_eq!(Workspace::from_name("sculpt"), None);
        for w in Workspace::ALL {
            assert_eq!(Workspace::from_name(w.name()), Some(w));
        }
    }

    #[test]
    fn every_workspace_is_valid_and_shows_the_viewport() {
        for w in Workspace::ALL {
            let l = w.layout();
            assert!(l.dock.is_valid(), "{}", w.name());
            assert_eq!(l.workspace, w.name());
            assert!(l.dock.area_of(Panel::Viewport).is_some(), "{}", w.name());
            assert!(l.dock.area_of(Panel::CommandBar).is_some(), "{}", w.name());
            // JSON round trip
            assert_eq!(Layout::from_json(&l.to_json()).unwrap(), l);
        }
        assert_eq!(Workspace::Standard.layout(), Layout::standard());
    }

    #[test]
    fn workspaces_differ_and_use_the_newer_panels() {
        let layouts: Vec<Layout> = Workspace::ALL.iter().map(|w| w.layout()).collect();
        for (i, a) in layouts.iter().enumerate() {
            for b in &layouts[i + 1..] {
                assert_ne!(a.dock, b.dock);
            }
        }
        let node = Workspace::Node.layout();
        assert_eq!(node.dock.active(Area::Center), Some(Panel::NodeEditor));
        assert_eq!(node.dock.area_of(Panel::Console), Some(Area::Bottom));
        assert_eq!(
            Workspace::Modeling.layout().dock.area_of(Panel::History),
            Some(Area::Bottom)
        );
        assert_eq!(
            Workspace::Rigging.layout().dock.area_of(Panel::Script),
            Some(Area::Bottom)
        );
    }

    #[test]
    fn layout_names_are_checked() {
        assert_eq!(check_layout_name("  My layout-2 ").unwrap(), "My layout-2");
        for bad in [
            "", "  ", "../x", "a/b", "_last", "-x", "x.y", "Modeling", "uv",
        ] {
            assert!(check_layout_name(bad).is_err(), "{bad}");
        }
        assert!(check_layout_name(&"a".repeat(49)).is_err());
        assert!(check_layout_name(&"a".repeat(48)).is_ok());
    }

    #[test]
    fn save_load_list_delete_round_trip() {
        let (_d, s) = store();
        assert!(s.list().is_empty());
        let mut l = Workspace::Animate.layout();
        l.dock.move_panel(Panel::Script, Area::RightBottom);
        l.sizes.right_width = 123.5;
        s.save("Mine", &l).unwrap();
        s.save("Another one", &Workspace::Node.layout()).unwrap();
        assert_eq!(s.list(), ["Another one", "Mine"]);
        let loaded = s.load("Mine").unwrap();
        assert_eq!(loaded.layout, l);
        assert!(loaded.warning.is_none());
        // saving again replaces
        s.save("Mine", &Layout::standard()).unwrap();
        assert_eq!(s.load("Mine").unwrap().layout, Layout::standard());
        s.delete("Mine").unwrap();
        assert_eq!(s.list(), ["Another one"]);
        assert!(s.delete("Mine").unwrap_err().contains("no layout"));
        assert!(s.load("Mine").unwrap_err().contains("no layout"));
    }

    #[test]
    fn no_folder_means_no_saving() {
        let s = LayoutStore::new(None);
        assert!(s.list().is_empty());
        assert!(
            s.save("x", &Layout::standard())
                .unwrap_err()
                .contains("no layout folder")
        );
        assert!(s.load("x").is_err());
        assert!(s.save_last(&Layout::standard()).is_err());
    }

    #[test]
    fn broken_old_and_newer_files_fall_back_with_a_warning() {
        let (d, s) = store();
        s.save("ok", &Workspace::Modeling.layout()).unwrap();
        let dir = d.path().join("layouts");
        let good = std::fs::read_to_string(dir.join("ok.json")).unwrap();
        let doubled = good.replacen("\"CommandBar\"", "\"Viewport\"", 1);
        let cases = [
            ("garbage", "{".to_string(), "parsing"),
            ("empty", String::new(), "EOF"),
            (
                "old",
                good.replace("\"version\": 1", "\"version\": 0"),
                "older version 0",
            ),
            (
                "newer",
                good.replace("\"version\": 1", "\"version\": 9"),
                "newer version 9",
            ),
            ("fields", "{\"version\": 1}".to_string(), "missing field"),
            ("doubled", doubled, "twice"),
        ];
        for (name, text, why) in cases {
            std::fs::write(dir.join(format!("{name}.json")), text).unwrap();
            let l = s.load(name).unwrap();
            assert_eq!(l.layout, Layout::standard(), "{name}");
            let w = l.warning.unwrap_or_default();
            assert!(
                w.contains("standard layout") && w.contains(why),
                "{name}: {w}"
            );
        }
        // other files in the folder are not layouts
        std::fs::write(dir.join("notes.txt"), "x").unwrap();
        std::fs::write(dir.join("bad.name.json"), "{}").unwrap();
        assert!(!s.list().contains(&"notes".to_string()));
        assert!(!s.list().iter().any(|n| n.contains('.')));
    }

    #[test]
    fn the_last_layout_is_remembered_but_not_listed() {
        let (_d, s) = store();
        assert!(s.load_last().unwrap().is_none());
        s.save_last(&Workspace::Uv.layout()).unwrap();
        assert_eq!(
            s.load_last().unwrap().unwrap().layout,
            Workspace::Uv.layout()
        );
        assert!(s.list().is_empty());
    }
}
