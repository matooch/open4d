//! Outputs and the AOV model (RENDER.md R3).

use std::collections::BTreeMap;

use forge_image::Image;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Aov {
    Depth,
    Normal,
    Albedo,
    Position,
    ObjectId,
    MaterialId,
    Shadow,
    Diffuse,
    Specular,
    Emission,
}

impl Aov {
    pub const ALL: [Aov; 10] = [
        Aov::Depth,
        Aov::Normal,
        Aov::Albedo,
        Aov::Position,
        Aov::ObjectId,
        Aov::MaterialId,
        Aov::Shadow,
        Aov::Diffuse,
        Aov::Specular,
        Aov::Emission,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Aov::Depth => "depth",
            Aov::Normal => "normal",
            Aov::Albedo => "albedo",
            Aov::Position => "position",
            Aov::ObjectId => "object_id",
            Aov::MaterialId => "material_id",
            Aov::Shadow => "shadow",
            Aov::Diffuse => "diffuse",
            Aov::Specular => "specular",
            Aov::Emission => "emission",
        }
    }

    pub fn parse(s: &str) -> Option<Aov> {
        Aov::ALL.into_iter().find(|a| a.name() == s)
    }

    /// Passes that carry numbers, not colours: never colour-managed when written.
    pub fn is_data(self) -> bool {
        matches!(
            self,
            Aov::Depth
                | Aov::Normal
                | Aov::Position
                | Aov::ObjectId
                | Aov::MaterialId
                | Aov::Shadow
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderStats {
    pub triangles: usize,
    pub instances: usize,
    pub tiles: usize,
    pub samples: u64,
    /// Material channels that were switched on but have no render rule (R7).
    pub ignored_channels: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RenderOutput {
    pub width: u32,
    pub height: u32,
    /// Linear RGBA, straight alpha = coverage.
    pub beauty: Image,
    pub aovs: BTreeMap<Aov, Image>,
    pub stats: RenderStats,
}
