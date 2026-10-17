//! Our own dock model: panels in named areas with tabs (docs/specs/UI.md §2).

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Area {
    Left,
    RightTop,
    RightBottom,
    Bottom,
    Center,
}

impl Area {
    pub const ALL: [Area; 5] = [
        Area::Left,
        Area::RightTop,
        Area::RightBottom,
        Area::Bottom,
        Area::Center,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Area::Left => "left",
            Area::RightTop => "right top",
            Area::RightBottom => "right bottom",
            Area::Bottom => "bottom",
            Area::Center => "centre",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Panel {
    Viewport,
    ObjectTree,
    Attributes,
    Timeline,
    Coordinates,
}

impl Panel {
    pub const ALL: [Panel; 5] = [
        Panel::Viewport,
        Panel::ObjectTree,
        Panel::Attributes,
        Panel::Timeline,
        Panel::Coordinates,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Panel::Viewport => "Viewport",
            Panel::ObjectTree => "Objects",
            Panel::Attributes => "Attributes",
            Panel::Timeline => "Timeline",
            Panel::Coordinates => "Coordinates",
        }
    }

    pub fn default_area(self) -> Area {
        match self {
            Panel::Viewport => Area::Center,
            Panel::ObjectTree => Area::RightTop,
            Panel::Attributes => Area::RightBottom,
            Panel::Timeline => Area::Bottom,
            Panel::Coordinates => Area::Bottom,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct AreaState {
    tabs: Vec<Panel>,
    active: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dock {
    areas: [AreaState; 5],
}

impl Default for Dock {
    fn default() -> Self {
        let mut d = Dock {
            areas: Default::default(),
        };
        for p in Panel::ALL {
            d.areas[p.default_area() as usize].tabs.push(p);
        }
        d
    }
}

impl Dock {
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

    /// Reopens a closed panel in its default area.
    pub fn open(&mut self, panel: Panel) {
        if self.area_of(panel).is_none() {
            self.move_panel(panel, panel.default_area());
        }
    }
}
