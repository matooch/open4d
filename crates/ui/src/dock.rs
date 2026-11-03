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
}

impl Area {
    pub const ALL: [Area; 6] = [
        Area::Top,
        Area::Left,
        Area::RightTop,
        Area::RightBottom,
        Area::Bottom,
        Area::Center,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Area::Top => "top",
            Area::Left => "left",
            Area::RightTop => "right top",
            Area::RightBottom => "right bottom",
            Area::Bottom => "bottom",
            Area::Center => "centre",
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
}

impl Panel {
    /// Every panel, in Window menu order.
    pub const ALL: [Panel; 10] = [
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
    ];

    /// The panels `Dock::default()` opens; the others open from the Window menu.
    pub const DEFAULT: [Panel; 7] = [
        Panel::CommandBar,
        Panel::ToolPalette,
        Panel::Viewport,
        Panel::ObjectTree,
        Panel::Attributes,
        Panel::Timeline,
        Panel::Coordinates,
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
    areas: [AreaState; 6],
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
