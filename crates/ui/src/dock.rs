//! Our own dock model: panels in named areas with tabs (docs/specs/UI.md §2).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Area {
    Top,
    Left,
    RightTop,
    RightBottom,
    Bottom,
    Center,
    LowerRight,
}

impl Area {
    pub const ALL: [Area; 7] = [
        Area::Top,
        Area::Left,
        Area::RightTop,
        Area::RightBottom,
        Area::Bottom,
        Area::Center,
        Area::LowerRight,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Area::Top => "top",
            Area::Left => "left",
            Area::RightTop => "right top",
            Area::RightBottom => "right bottom",
            Area::Bottom => "bottom",
            Area::Center => "centre",
            Area::LowerRight => "below attributes",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Panel {
    CommandBar,
    ToolPalette,
    Viewport,
    ObjectTree,
    Attributes,
    Timeline,
    Coordinates,
    Script,
    NodeEditor,
    History,
    Console,
    Layers,
    ContentBrowser,
    WeightManager,
    DopeSheet,
    ValueGraph,
    Materials,
    Preferences,
    KeyMap,
    DynamicsSettings,
    RenderSettings,
    PictureViewer,
    InteractiveRender,
    UvEditor,
}

impl Panel {
    /// Every panel, in Window menu order.
    pub const ALL: [Panel; 23] = [
        Panel::CommandBar,
        Panel::ToolPalette,
        Panel::Viewport,
        Panel::ObjectTree,
        Panel::Attributes,
        Panel::Timeline,
        Panel::Coordinates,
        Panel::Script,
        Panel::NodeEditor,
        Panel::History,
        Panel::Console,
        Panel::Layers,
        Panel::ContentBrowser,
        Panel::WeightManager,
        Panel::DopeSheet,
        Panel::ValueGraph,
        Panel::Materials,
        Panel::Preferences,
        Panel::KeyMap,
        Panel::DynamicsSettings,
        Panel::RenderSettings,
        Panel::PictureViewer,
        Panel::UvEditor,
    ];

    /// The panels `Dock::default()` opens; the others open from the Window menu.
    pub const DEFAULT: [Panel; 10] = [
        Panel::CommandBar,
        Panel::ToolPalette,
        Panel::Viewport,
        Panel::ObjectTree,
        Panel::Attributes,
        Panel::Timeline,
        Panel::DopeSheet,
        Panel::ValueGraph,
        Panel::Coordinates,
        Panel::Materials,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Panel::CommandBar => "Command bar",
            Panel::ToolPalette => "Tools",
            Panel::Viewport => "Viewport",
            Panel::ObjectTree => "Objects",
            Panel::Attributes => "Attributes",
            Panel::Timeline => "Timeline",
            Panel::Coordinates => "Coordinates",
            Panel::Script => "Script Manager",
            Panel::NodeEditor => "Node Editor",
            Panel::History => "History",
            Panel::Console => "Console",
            Panel::Layers => "Layers",
            Panel::ContentBrowser => "Content Browser",
            Panel::WeightManager => "Weight Manager",
            Panel::DopeSheet => "Dope Sheet",
            Panel::ValueGraph => "Value Graph",
            Panel::Materials => "Materials",
            Panel::Preferences => "Preferences",
            Panel::KeyMap => "Key map",
            Panel::DynamicsSettings => "Dynamics Settings",
            Panel::RenderSettings => "Render Settings",
            Panel::PictureViewer => "Picture Viewer",
            Panel::InteractiveRender => "Interactive Render",
            Panel::UvEditor => "UV Editor",
        }
    }

    pub(crate) fn animation_tab(self) -> Option<(&'static str, &'static str, &'static str)> {
        match self {
            Panel::Timeline => Some((
                "clock",
                "Timeline",
                "Scrub frames and manage the playback range and markers",
            )),
            Panel::DopeSheet => Some((
                "key",
                "Dope Sheet",
                "Select and retime keys across animation tracks",
            )),
            Panel::ValueGraph => Some((
                "graph_node",
                "F-Curve",
                "Edit values, speeds and tangents across frames",
            )),
            _ => None,
        }
    }

    pub fn default_area(self) -> Area {
        match self {
            Panel::CommandBar => Area::Top,
            Panel::ToolPalette => Area::Left,
            Panel::Viewport => Area::Center,
            Panel::ObjectTree => Area::RightTop,
            Panel::Attributes => Area::RightBottom,
            Panel::Timeline => Area::Bottom,
            Panel::Coordinates => Area::Bottom,
            Panel::Script => Area::Bottom,
            Panel::NodeEditor => Area::Center,
            Panel::History => Area::RightBottom,
            Panel::Console => Area::Bottom,
            Panel::Layers => Area::RightBottom,
            Panel::ContentBrowser => Area::RightBottom,
            Panel::WeightManager => Area::RightBottom,
            Panel::Materials => Area::Bottom,
            Panel::RenderSettings | Panel::DynamicsSettings => Area::RightBottom,
            Panel::Preferences | Panel::KeyMap => Area::Center,
            Panel::PictureViewer => Area::Center,
            Panel::InteractiveRender => Area::Center,
            Panel::UvEditor => Area::Center,
            Panel::DopeSheet | Panel::ValueGraph => Area::Bottom,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct AreaState {
    tabs: Vec<Panel>,
    active: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dock {
    #[serde(deserialize_with = "read_areas")]
    areas: [AreaState; 7],
}

fn read_areas<'de, D: serde::Deserializer<'de>>(d: D) -> Result<[AreaState; 7], D::Error> {
    let mut areas = Vec::<AreaState>::deserialize(d)?;
    if areas.len() == 6 {
        areas.push(AreaState::default());
    }
    areas
        .try_into()
        .map_err(|_| serde::de::Error::custom("invalid number of dock areas"))
}

impl Default for Dock {
    fn default() -> Self {
        let mut d = Dock {
            areas: Default::default(),
        };
        for p in Panel::DEFAULT {
            d.areas[p.default_area() as usize].tabs.push(p);
        }
        d
    }
}

impl Dock {
    /// A dock with exactly the listed tabs per area (the first tab of each is shown); areas
    /// not listed are empty. A panel listed twice keeps its first place.
    pub fn arranged(areas: &[(Area, &[Panel])]) -> Dock {
        let mut d = Dock {
            areas: Default::default(),
        };
        for (area, tabs) in areas {
            for p in *tabs {
                if d.area_of(*p).is_none() {
                    d.areas[*area as usize].tabs.push(*p);
                }
            }
        }
        d
    }

    /// True if no panel appears twice and every active index is in range (a loaded dock
    /// must satisfy this; the editing methods keep it).
    pub fn is_valid(&self) -> bool {
        let mut seen = Vec::new();
        for a in &self.areas {
            if a.active > a.tabs.len().saturating_sub(1) {
                return false;
            }
            for p in &a.tabs {
                if seen.contains(p) {
                    return false;
                }
                seen.push(*p);
            }
        }
        true
    }

    pub fn tabs(&self, area: Area) -> &[Panel] {
        &self.areas[area as usize].tabs
    }

    pub fn active(&self, area: Area) -> Option<Panel> {
        let a = &self.areas[area as usize];
        a.tabs.get(a.active).copied()
    }

    pub fn area_of(&self, panel: Panel) -> Option<Area> {
        Area::ALL
            .into_iter()
            .find(|a| self.tabs(*a).contains(&panel))
    }

    pub fn set_active(&mut self, area: Area, index: usize) {
        let a = &mut self.areas[area as usize];
        if index < a.tabs.len() {
            a.active = index;
        }
    }

    /// Removes a panel; its area keeps showing the same active panel if it can.
    pub fn close(&mut self, panel: Panel) {
        let Some(area) = self.area_of(panel) else {
            return;
        };
        let a = &mut self.areas[area as usize];
        let current = a.tabs.get(a.active).copied();
        a.tabs.retain(|p| *p != panel);
        a.active = match current.and_then(|c| a.tabs.iter().position(|p| *p == c)) {
            Some(i) => i,
            None => a.active.min(a.tabs.len().saturating_sub(1)),
        };
    }

    pub fn move_panel(&mut self, panel: Panel, to: Area) {
        if self.area_of(panel) == Some(to) {
            return;
        }
        self.close(panel);
        let a = &mut self.areas[to as usize];
        a.tabs.push(panel);
        a.active = a.tabs.len() - 1;
    }

    /// Opens the panel if it is closed and makes it the shown tab of its area.
    pub fn reveal(&mut self, panel: Panel) {
        self.open(panel);
        if let Some(area) = self.area_of(panel)
            && let Some(i) = self.tabs(area).iter().position(|p| *p == panel)
        {
            self.set_active(area, i);
        }
    }

    /// Reopens a closed panel in its default area.
    pub fn open(&mut self, panel: Panel) {
        if self.area_of(panel).is_none() {
            self.move_panel(panel, panel.default_area());
        }
    }
}

/// Dropping inside a dock joins its tabs. The edges of the center provide a target
/// for side docks, including ones that currently have no tabs.
pub(crate) fn drop_target(
    rects: &[Option<egui::Rect>; Area::ALL.len()],
    pointer: egui::Pos2,
) -> Option<(Area, egui::Rect)> {
    use egui::{Rect, pos2};
    if let Some(center) = rects[Area::Center as usize].filter(|r| r.contains(pointer)) {
        let edge_y = (center.height() * 0.18).min(38.0);
        let edge_x = (center.width() * 0.18).min(44.0);
        if pointer.y <= center.top() + edge_y {
            let preview =
                Rect::from_min_max(center.min, pos2(center.right(), center.top() + edge_y));
            return Some((Area::Top, preview));
        }
        if pointer.y >= center.bottom() - edge_y {
            let preview =
                Rect::from_min_max(pos2(center.left(), center.bottom() - edge_y), center.max);
            return Some((Area::Bottom, preview));
        }
        if pointer.x <= center.left() + edge_x {
            let preview =
                Rect::from_min_max(center.min, pos2(center.left() + edge_x, center.bottom()));
            return Some((Area::Left, preview));
        }
        if pointer.x >= center.right() - edge_x {
            let y = center.center().y;
            let (target, top, bottom) = if pointer.y < y {
                (Area::RightTop, center.top(), y)
            } else {
                (Area::RightBottom, y, center.bottom())
            };
            return Some((
                target,
                Rect::from_min_max(
                    pos2(center.right() - edge_x, top),
                    pos2(center.right(), bottom),
                ),
            ));
        }
        return Some((Area::Center, center));
    }
    if let Some(r) = rects[Area::RightBottom as usize]
        && r.contains(pointer)
        && pointer.y > r.bottom() - r.height().min(120.0) * 0.3
    {
        return Some((
            Area::LowerRight,
            Rect::from_min_max(pos2(r.left(), r.center().y), r.max),
        ));
    }
    Area::ALL.into_iter().find_map(|area| {
        rects[area as usize]
            .filter(|r| r.contains(pointer))
            .map(|r| (area, r))
    })
}
