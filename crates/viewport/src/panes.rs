//! Viewport panes and the 4-view layout maths (docs/specs/VIEWPORT.md §9). Own design,
//! parity-pending G8.

use crate::camera::Camera;
use crate::nav::{StandardView, view_camera};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Pane {
    Perspective,
    Top,
    Front,
    Right,
}

impl Pane {
    pub const ALL: [Pane; 4] = [Pane::Perspective, Pane::Top, Pane::Front, Pane::Right];
    /// Top-left, top-right, bottom-left, bottom-right of the 4-view layout.
    pub const QUAD: [Pane; 4] = [Pane::Perspective, Pane::Top, Pane::Right, Pane::Front];

    /// Index into per-pane arrays (the position in [`Pane::ALL`]).
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn id(self) -> &'static str {
        match self {
            Pane::Perspective => "perspective",
            Pane::Top => "top",
            Pane::Front => "front",
            Pane::Right => "right",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Pane::Perspective => "Perspective",
            Pane::Top => "Top",
            Pane::Front => "Front",
            Pane::Right => "Right",
        }
    }

    pub fn from_id(id: &str) -> Option<Pane> {
        Pane::ALL.into_iter().find(|p| p.id() == id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    #[default]
    Single,
    Four,
}

/// A pane's area in pixels, origin top-left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaneRect {
    pub pane: Pane,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl PaneRect {
    fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// The panes drawn, in drawing order.
pub fn visible(layout: Layout, active: Pane, maximized: Option<Pane>) -> Vec<Pane> {
    match (layout, maximized) {
        (Layout::Single, _) => vec![active],
        (Layout::Four, Some(p)) => vec![p],
        (Layout::Four, None) => Pane::QUAD.to_vec(),
    }
}

/// Splits `total` into two parts separated by `gap`: `floor((total - gap) / 2)` and the rest.
fn split(total: f32, gap: f32) -> (f32, f32) {
    let first = ((total - gap).max(0.0) / 2.0).floor();
    (first, (total - gap).max(0.0) - first)
}

/// Where each visible pane goes in a `width` x `height` area.
pub fn arrange(
    layout: Layout,
    active: Pane,
    maximized: Option<Pane>,
    width: f32,
    height: f32,
    gap: f32,
) -> Vec<PaneRect> {
    let panes = visible(layout, active, maximized);
    if let [pane] = panes[..] {
        return vec![PaneRect {
            pane,
            x: 0.0,
            y: 0.0,
            w: width,
            h: height,
        }];
    }
    let (w1, w2) = split(width, gap);
    let (h1, h2) = split(height, gap);
    let (x2, y2) = (w1 + gap, h1 + gap);
    let cells = [
        (0.0, 0.0, w1, h1),
        (x2, 0.0, w2, h1),
        (0.0, y2, w1, h2),
        (x2, y2, w2, h2),
    ];
    panes
        .into_iter()
        .zip(cells)
        .map(|(pane, (x, y, w, h))| PaneRect { pane, x, y, w, h })
        .collect()
}

/// The pane whose rectangle contains the point; gaps and the outside give `None`.
pub fn pane_at(rects: &[PaneRect], x: f32, y: f32) -> Option<Pane> {
    rects.iter().find(|r| r.contains(x, y)).map(|r| r.pane)
}

/// A pane's default camera, derived from the editor's default perspective camera `base`.
pub fn pane_camera(pane: Pane, base: &Camera) -> Camera {
    let view = match pane {
        Pane::Perspective => return *base,
        Pane::Top => StandardView::Top,
        Pane::Front => StandardView::Front,
        Pane::Right => StandardView::Right,
    };
    view_camera(base, view)
}
