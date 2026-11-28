//! Shader node types (Category::Shader, not placeable in a scene yet; SHADERS.md S10).

use forge_core::{Category, NodeType, ParamDesc, Plugin, Registry, RegistryError, Unit};

pub const PREFIX: &str = "forge.shader.";

const BLEND: &[(&str, &str)] = &[
    ("normal", "Normal"),
    ("add", "Add"),
    ("subtract", "Subtract"),
    ("multiply", "Multiply"),
    ("screen", "Screen"),
    ("overlay", "Overlay"),
    ("darken", "Darken"),
    ("lighten", "Lighten"),
    ("difference", "Difference"),
];

/// The noise types of O42 (32 entries); the ones without a public definition build as
/// `Unsupported`.
pub const NOISE_TYPES: &[(&str, &str)] = &[
    ("box", "Box"),
    ("blistered_turbulence", "Blistered Turbulence"),
    ("buya", "Buya"),
    ("cell_noise", "Cell Noise"),
    ("cranal", "Cranal"),
    ("dents", "Dents"),
    ("displaced_turbulence", "Displaced Turbulence"),
    ("electric", "Electric"),
    ("fbm", "FBM"),
    ("fire", "Fire"),
    ("gaseous", "Gaseous"),
    ("hama", "Hama"),
    ("luka", "Luka"),
    ("mod_noise", "Mod Noise"),
    ("naki", "Naki"),
    ("noise", "Noise"),
    ("nutous", "Nutous"),
    ("ober", "Ober"),
    ("pezo", "Pezo"),
    ("poxo", "Poxo"),
    ("sema", "Sema"),
    ("stupl", "Stupl"),
    ("turbulence", "Turbulence"),
    ("vl_noise", "VL Noise"),
    ("wavy_turbulence", "Wavy Turbulence"),
    ("cell_voronoi", "Cell Voronoi"),
    ("displaced_voronoi", "Displaced Voronoi"),
    ("sparse_convolution", "Sparse Convolution"),
    ("voronoi_1", "Voronoi 1"),
    ("voronoi_2", "Voronoi 2"),
    ("voronoi_3", "Voronoi 3"),
    ("zada", "Zada"),
];

fn ty(name: &str, label: &str) -> NodeType {
    NodeType::new(&format!("{PREFIX}{name}"), label, Category::Shader).not_in_create_menu()
}

fn pct(id: &str, label: &str, v: f64) -> ParamDesc {
    ParamDesc::float(id, label, v).unit(Unit::Percent)
}

fn col(id: &str, label: &str, v: [f64; 3]) -> ParamDesc {
    ParamDesc::color(id, label, v)
}

fn gradient_params(t: NodeType) -> NodeType {
    t.param(col("color_start", "Start Colour", [0.0; 3]))
        .param(col("color_end", "End Colour", [1.0; 3]))
        .param(ParamDesc::enumeration(
            "interpolation",
            "Interpolation",
            "cubic",
            &[("none", "None"), ("linear", "Linear"), ("cubic", "Cubic")],
        ))
}

/// Every shader node type.
pub fn shader_types() -> Vec<NodeType> {
    let blur = |t: NodeType| {
        t.param(pct("blur_offset", "Blur Offset", 0.0))
            .param(pct("blur_scale", "Blur Scale", 0.0))
    };
    vec![
        blur(ty("color", "Color"))
            .param(col("color", "Color", [0.8; 3]))
            .param(pct("brightness", "Brightness", 1.0)),
        gradient_params(blur(ty("gradient", "Gradient")))
            .param(ParamDesc::enumeration(
                "type",
                "Type",
                "u",
                &[
                    ("u", "2D - U"),
                    ("v", "2D - V"),
                    ("diagonal", "2D - Diagonal"),
                    ("circular", "2D - Circular"),
                    ("radial", "2D - Radial"),
                    ("linear_3d", "3D - Linear"),
                    ("spherical_3d", "3D - Spherical"),
                    ("cylindrical_3d", "3D - Cylindrical"),
                ],
            ))
            .param(ParamDesc::bool("cycle", "Cycle", true))
            .param(ParamDesc::vec3("start", "Start", [-100.0, 0.0, 0.0]))
            .param(ParamDesc::vec3("end", "End", [100.0, 0.0, 0.0]))
            .param(ParamDesc::float("radius", "Radius", 100.0).range(0.0, 1e9))
            .param(pct("turbulence", "Turbulence", 0.0))
            .param(ParamDesc::int("octaves", "Octaves", 5).range(1, 16))
            .param(pct("scale", "Scale", 1.0))
            .param(ParamDesc::int("seed", "Seed", 0).range(0, 1_000_000)),
        blur(ty("noise", "Noise"))
            .param(col("color_1", "Colour 1", [0.0; 3]))
            .param(col("color_2", "Colour 2", [1.0; 3]))
            .param(ParamDesc::int("seed", "Seed", 665).range(0, 1_000_000))
            .param(ParamDesc::enumeration(
                "noise",
                "Noise",
                "noise",
                NOISE_TYPES,
            ))
            .param(ParamDesc::int("octaves", "Octaves", 5).range(1, 16))
            .param(ParamDesc::enumeration(
                "space",
                "Space",
                "texture",
                &[
                    ("texture", "Texture"),
                    ("object", "Object"),
                    ("world", "World"),
                    ("uv", "UV"),
                ],
            ))
            .param(pct("global_scale", "Global Scale", 1.0).range(0.0, 1e6))
            .param(ParamDesc::vec3(
                "relative_scale",
                "Relative Scale",
                [1.0; 3],
            ))
            .param(ParamDesc::float("animation_speed", "Animation Speed", 0.0))
            .param(ParamDesc::bool("absolute", "Absolute", false))
            .param(pct("low_clip", "Low Clip", 0.0))
            .param(pct("high_clip", "High Clip", 1.0))
            .param(pct("brightness", "Brightness", 0.0))
            .param(pct("contrast", "Contrast", 0.0)),
        blur(ty("fresnel", "Fresnel"))
            .param(ParamDesc::float("ior", "IOR", 1.33283).range(1.0, 10.0))
            .param(ParamDesc::bool("invert", "Invert", false))
            .param(ParamDesc::bool("physical", "Physical", false)),
        gradient_params(blur(ty("falloff", "Falloff")))
            .param(ParamDesc::bool("invert", "Invert", false)),
        blur(ty("layer", "Layer"))
            .param(ParamDesc::enumeration(
                "blend_mode",
                "Blend Mode",
                "normal",
                BLEND,
            ))
            .param(pct("opacity", "Opacity", 1.0).range(0.0, 1.0)),
        gradient_params(blur(ty("colorizer", "Colorizer")))
            .param(ParamDesc::enumeration(
                "input",
                "Input",
                "luminance",
                &[
                    ("luminance", "Luminance"),
                    ("red", "Red"),
                    ("green", "Green"),
                    ("blue", "Blue"),
                ],
            ))
            .param(ParamDesc::bool("cycle", "Cycle", false)),
        blur(ty("bitmap", "Bitmap"))
            .param(ParamDesc::bool("srgb_input", "Decode sRGB Color", false))
            .param(ParamDesc::file_path("file", "File", ""))
            .param(ParamDesc::bool("repeat", "Repeat", true))
            .param(ParamDesc::bool("interpolate", "Interpolate", true)),
        blur(ty("checkerboard", "Checkerboard"))
            .param(col("color_1", "Colour 1", [1.0; 3]))
            .param(col("color_2", "Colour 2", [0.0; 3]))
            .param(ParamDesc::float("u_frequency", "U Frequency", 10.0).range(0.0, 1e6))
            .param(ParamDesc::float("v_frequency", "V Frequency", 10.0).range(0.0, 1e6)),
        blur(ty("tiles", "Tiles"))
            .param(col("grout_color", "Grout Colour", [0.0; 3]))
            .param(col("tiles_color_1", "Tiles Colour 1", [0.5, 0.0, 0.0]))
            .param(col("tiles_color_2", "Tiles Colour 2", [0.5, 0.5, 0.5]))
            .param(col("tiles_color_3", "Tiles Colour 3", [0.0, 0.0, 0.5]))
            .param(ParamDesc::enumeration(
                "pattern",
                "Pattern",
                "squares",
                &[
                    ("squares", "Squares"),
                    ("hexagons", "Hexagons"),
                    ("triangles", "Triangles"),
                    ("bricks", "Bricks"),
                ],
            ))
            .param(pct("grout_width", "Grout Width", 0.05))
            .param(pct("bevel_width", "Bevel Width", 0.30))
            .param(ParamDesc::bool("bevel", "Bevel", true))
            .param(ParamDesc::bool(
                "randomize_color",
                "Randomize Colour",
                false,
            ))
            .param(ParamDesc::enumeration(
                "orientation",
                "Orientation",
                "u",
                &[("u", "U"), ("v", "V")],
            ))
            .param(pct("global_scale", "Global Scale", 1.0).range(0.0, 1e6))
            .param(pct("u_scale", "U Scale", 1.0).range(0.0, 1e6))
            .param(pct("v_scale", "V Scale", 1.0).range(0.0, 1e6))
            .param(ParamDesc::float("rotate", "Rotate", 0.0).unit(Unit::Angle))
            .param(ParamDesc::int("seed", "Seed", 665).range(0, 1_000_000)),
        blur(ty("brick", "Brick"))
            .param(pct("scale", "Scale", 0.25))
            .param(pct("brick_width", "Brick Width", 0.5))
            .param(pct("brick_height", "Brick Height", 0.25))
            .param(pct("shift", "Shift", 0.5))
            .param(ParamDesc::int("seed", "Seed", 555).range(0, 1_000_000)),
        blur(ty("fusion", "Fusion"))
            .param(ParamDesc::enumeration(
                "blend_mode",
                "Blend Mode",
                "normal",
                BLEND,
            ))
            .param(pct("opacity", "Opacity", 1.0).range(0.0, 1.0)),
        blur(ty("filter", "Filter"))
            .param(ParamDesc::float("hue", "Hue", 0.0).unit(Unit::Angle))
            .param(pct("saturation", "Saturation", 0.0))
            .param(pct("lightness", "Lightness", 0.0))
            .param(pct("contrast", "Contrast", 0.0))
            .param(ParamDesc::float("gamma", "Gamma", 1.0).range(0.01, 100.0))
            .param(ParamDesc::bool("invert", "Invert", false)),
        blur(ty("normalizer", "Normalizer"))
            .param(pct("low", "Low", 0.0))
            .param(pct("high", "High", 1.0)),
        blur(ty("vertex_map", "Vertex Map"))
            .param(col("color_low", "Colour at 0", [0.0; 3]))
            .param(col("color_high", "Colour at 1", [1.0; 3])),
        blur(ty("field", "Field")).param(ParamDesc::string("field", "Field", "")),
        blur(ty("ao", "Ambient Occlusion"))
            .param(col("color_low", "Occluded Colour", [0.0; 3]))
            .param(col("color_high", "Open Colour", [1.0; 3])),
    ]
}

/// Registers the shader node types.
pub struct ShadersPlugin;

impl Plugin for ShadersPlugin {
    fn id(&self) -> &str {
        "forge.shaders"
    }
    fn register(&self, registry: &mut Registry) -> Result<(), RegistryError> {
        for t in shader_types() {
            registry.register(t)?;
        }
        Ok(())
    }
}
