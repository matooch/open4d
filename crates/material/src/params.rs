//! Node types and parameter tables (MATERIAL.md M1, M2, M4).

use forge_core::{Category, NodeType, ParamDesc, Plugin, Registry, RegistryError, Unit};

pub const MATERIAL: &str = "forge.material.standard";
pub const TEXTURE_TAG: &str = "forge.tag.texture";
/// The UVW tag: corner UVs written by Generate UVW (UV.md U5).
pub const UVW_TAG: &str = "forge.uvw";

const MIX: &[(&str, &str)] = &[
    ("normal", "Normal"),
    ("add", "Add"),
    ("subtract", "Subtract"),
    ("multiply", "Multiply"),
];

struct B {
    ty: NodeType,
    group: &'static str,
}

impl B {
    fn add(&mut self, d: ParamDesc) {
        let d = d.group(self.group);
        self.ty.params.push(d);
    }
    /// A texture slot: a link to the root of the slot's shader tree (MATERIAL.md M6).
    fn slot(&mut self, id: &str, label: &str) {
        self.add(ParamDesc::link(id, label, &[Category::Shader]));
    }
    fn on(&mut self, id: &str, label: &str, v: bool) {
        self.add(ParamDesc::bool(id, label, v));
    }
    fn col(&mut self, id: &str, label: &str, v: [f64; 3]) {
        self.add(ParamDesc::color(id, label, v));
    }
    /// A percent field stored as a fraction.
    fn pct(&mut self, id: &str, label: &str, pct: f64, min: f64, max: f64) {
        self.add(
            ParamDesc::float(id, label, pct / 100.0)
                .range(min / 100.0, max / 100.0)
                .unit(Unit::Percent),
        );
    }
    fn num(&mut self, id: &str, label: &str, v: f64, min: f64, max: f64) {
        self.add(ParamDesc::float(id, label, v).range(min, max));
    }
    fn dist(&mut self, id: &str, label: &str, v: f64, min: f64, max: f64) {
        self.add(
            ParamDesc::float(id, label, v)
                .range(min, max)
                .unit(Unit::Distance),
        );
    }
    fn int(&mut self, id: &str, label: &str, v: i64, min: i64, max: i64) {
        self.add(ParamDesc::int(id, label, v).range(min, max));
    }
    fn en(&mut self, id: &str, label: &str, d: &str, o: &[(&str, &str)]) {
        self.add(ParamDesc::enumeration(id, label, d, o));
    }
    fn tex(&mut self, p: &str, mode_default: &str) {
        self.slot(&format!("{p}.texture"), "Texture");
        self.en(&format!("{p}.mix_mode"), "Mix Mode", mode_default, MIX);
        self.pct(
            &format!("{p}.mix_strength"),
            "Mix Strength",
            100.0,
            0.0,
            100.0,
        );
    }
}

/// The Standard material (values from O29 section 4).
pub fn standard_type() -> NodeType {
    let mut b = B {
        ty: NodeType::new(MATERIAL, "Material", Category::Material).not_in_create_menu(),
        group: "Basic",
    };
    b.col("icon_color", "Color", [1.0, 0.9, 0.4]);
    for (c, on) in [
        ("color", true),
        ("diffusion", false),
        ("luminance", false),
        ("transparency", false),
        ("reflectance", true),
        ("environment", false),
        ("fog", false),
        ("bump", false),
        ("normal", false),
        ("alpha", false),
        ("glow", false),
        ("displacement", false),
    ] {
        b.on(&format!("enable.{c}"), &format!("Enable {c}"), on);
    }
    b.group = "Color";
    b.col("color.color", "Color", [0.8, 0.8, 0.8]);
    b.pct("color.brightness", "Brightness", 100.0, 0.0, 10000.0);
    b.tex("color", "normal");
    b.slot("color.occlusion_texture", "Occlusion Map");
    b.en(
        "color.model",
        "Model",
        "lambertian",
        &[("lambertian", "Lambertian"), ("oren_nayar", "Oren-Nayar")],
    );
    b.pct(
        "color.diffuse_falloff",
        "Diffuse Falloff",
        0.0,
        -1000.0,
        1000.0,
    );
    b.pct("color.diffuse_level", "Diffuse Level", 100.0, 0.0, 1000.0);
    b.pct("color.roughness", "Roughness", 50.0, 0.0, 200.0);
    b.group = "Diffusion";
    b.pct(
        "diffusion.brightness",
        "Diffusion Brightness",
        100.0,
        0.0,
        100.0,
    );
    b.on("diffusion.affect_luminance", "Affect Luminance", false);
    b.on("diffusion.affect_specular", "Affect Specular", true);
    b.on("diffusion.affect_reflection", "Affect Reflection", false);
    b.tex("diffusion", "normal");
    b.group = "Luminance";
    b.col("luminance.color", "Luminance Color", [1.0, 1.0, 1.0]);
    b.pct(
        "luminance.brightness",
        "Luminance Brightness",
        100.0,
        0.0,
        99999999900.0,
    );
    b.tex("luminance", "normal");
    b.group = "Transparency";
    b.col("transparency.color", "Transparency Color", [1.0, 1.0, 1.0]);
    b.pct(
        "transparency.brightness",
        "Transparency Brightness",
        100.0,
        0.0,
        100.0,
    );
    b.en(
        "transparency.refraction_preset",
        "Refraction Preset",
        "glass",
        &[
            ("custom", "Custom"),
            ("beer", "Beer"),
            ("diamond", "Diamond"),
            ("emerald", "Emerald"),
            ("ethanol", "Ethanol"),
            ("glass", "Glass"),
            ("jade", "Jade"),
            ("milk", "Milk"),
            ("oil_vegetable", "Oil (Vegetable)"),
            ("pearl", "Pearl"),
            ("pet", "PET"),
            ("plexiglas", "Plexiglas"),
            ("ruby", "Ruby"),
            ("sapphire", "Sapphire"),
            ("water", "Water"),
            ("water_ice", "Water (Ice)"),
            ("whiskey", "Whiskey"),
        ],
    );
    b.num("transparency.refraction", "Refraction", 1.517, 0.25, 4.0);
    b.on(
        "transparency.total_internal_reflection",
        "Total Internal Reflection",
        true,
    );
    b.on("transparency.exit_reflections", "Exit Reflections", true);
    b.pct(
        "transparency.fresnel_reflectivity",
        "Fresnel Reflectivity",
        100.0,
        0.0,
        100.0,
    );
    b.on("transparency.additive", "Additive", false);
    b.tex("transparency", "normal");
    b.col(
        "transparency.absorption_color",
        "Absorption Color",
        [1.0, 1.0, 1.0],
    );
    b.dist(
        "transparency.absorption_distance",
        "Absorption Distance",
        100.0,
        0.0,
        999999999.0,
    );
    b.pct("transparency.blurriness", "Blurriness", 0.0, 0.0, 100.0);
    b.int("transparency.min_samples", "Min Samples", 5, 1, 10000);
    b.int("transparency.max_samples", "Max Samples", 128, 1, 10000);
    b.pct("transparency.accuracy", "Accuracy", 50.0, 0.0, 100.0);
    b.group = "Reflectance";
    b.pct(
        "reflectance.global_reflection",
        "Global Reflection Brightness",
        100.0,
        0.0,
        1000.0,
    );
    b.pct(
        "reflectance.global_specular",
        "Global Specular Brightness",
        100.0,
        0.0,
        1000.0,
    );
    b.on("reflectance.separate_pass", "Separate Pass", false);
    b.en(
        "reflectance.spec.type",
        "Type",
        "blinn_legacy",
        &[
            ("beckmann", "Beckmann"),
            ("ggx", "GGX"),
            ("phong", "Phong"),
            ("ward", "Ward"),
            ("anisotropic", "Anisotropic"),
            ("lambertian", "Lambertian (Diffuse)"),
            ("oren_nayar", "Oren-Nayar (Diffuse)"),
            ("irawan", "Irawan (Woven Cloth)"),
            ("reflection_legacy", "Reflection (Legacy)"),
            ("blinn_legacy", "Specular - Blinn (Legacy)"),
            ("phong_legacy", "Specular - Phong (Legacy)"),
        ],
    );
    b.en(
        "reflectance.spec.attenuation",
        "Attenuation",
        "additive",
        &[("additive", "Additive"), ("metal", "Metal")],
    );
    b.slot("reflectance.spec.roughness_texture", "Roughness Map");
    b.slot("reflectance.spec.metallic_texture", "Metallic Map");
    b.slot("reflectance.spec.strength_texture", "Specular Level Map");
    b.pct("reflectance.spec.width", "Width", 50.0, 0.0, 100.0);
    b.pct("reflectance.spec.falloff", "Falloff", 0.0, -100.0, 100.0);
    b.pct(
        "reflectance.spec.inner_width",
        "Inner Width",
        0.0,
        0.0,
        100.0,
    );
    b.pct(
        "reflectance.spec.strength",
        "Specular Strength",
        20.0,
        0.0,
        10000.0,
    );
    b.on("reflectance.spec.colored", "Colored", false);
    b.pct(
        "reflectance.spec.bump_strength",
        "Bump Strength",
        100.0,
        0.0,
        100.0,
    );
    b.en(
        "reflectance.spec.bump_mode",
        "Bump Mode",
        "default",
        &[
            ("default", "Default"),
            ("bump_map", "Custom Bump Map"),
            ("normal_map", "Custom Normal Map"),
        ],
    );
    b.on(
        "reflectance.spec.separate_pass",
        "Layer Separate Pass",
        false,
    );
    b.col("reflectance.spec.color", "Layer Color", [1.0, 1.0, 1.0]);
    b.pct(
        "reflectance.spec.color_brightness",
        "Layer Color Brightness",
        100.0,
        0.0,
        10000.0,
    );
    b.slot("reflectance.spec.color_texture", "Layer Color Texture");
    b.en(
        "reflectance.spec.color_mix_mode",
        "Layer Color Mix Mode",
        "normal",
        MIX,
    );
    b.pct(
        "reflectance.spec.color_mix_strength",
        "Layer Color Mix Strength",
        100.0,
        0.0,
        100.0,
    );
    b.pct(
        "reflectance.spec.mask_amount",
        "Layer Mask Amount",
        100.0,
        0.0,
        100.0,
    );
    b.col(
        "reflectance.spec.mask_color",
        "Layer Mask Color",
        [1.0, 1.0, 1.0],
    );
    b.slot("reflectance.spec.mask_texture", "Layer Mask Texture");
    b.en(
        "reflectance.spec.mask_mix_mode",
        "Layer Mask Mix Mode",
        "normal",
        MIX,
    );
    b.pct(
        "reflectance.spec.mask_mix_strength",
        "Layer Mask Mix Strength",
        100.0,
        0.0,
        100.0,
    );
    b.group = "Environment";
    b.col("environment.color", "Environment Color", [1.0, 1.0, 1.0]);
    b.pct(
        "environment.brightness",
        "Environment Brightness",
        100.0,
        0.0,
        10000.0,
    );
    b.tex("environment", "multiply");
    b.num("environment.tiles_x", "Tiles X", 1.0, 0.0, 999999999.0);
    b.num("environment.tiles_y", "Tiles Y", 1.0, 0.0, 999999999.0);
    b.on("environment.exclusive", "Exclusive", true);
    b.group = "Subsurface";
    b.dist(
        "subsurface.distance",
        "Scattering Distance",
        0.0,
        0.0,
        999999999.0,
    );
    b.group = "Fog";
    b.col("fog.color", "Fog Color", [1.0, 1.0, 1.0]);
    b.pct("fog.brightness", "Fog Brightness", 100.0, 0.0, 100.0);
    b.dist("fog.distance", "Fog Distance", 1000.0, 0.0, 999999999.0);
    b.group = "Bump";
    b.pct(
        "bump.strength",
        "Bump Strength",
        20.0,
        -1_000_000.0,
        1_000_000.0,
    );
    b.on("bump.mip_falloff", "MIP Falloff", true);
    b.slot("bump.texture", "Texture");
    b.group = "Normal";
    b.pct("normal.strength", "Strength", 100.0, 0.0, 100000.0);
    b.en(
        "normal.method",
        "Method",
        "tangent",
        &[
            ("tangent", "Tangent"),
            ("object", "Object"),
            ("world", "World"),
        ],
    );
    b.on("normal.flip_x", "Flip X (Red)", false);
    b.on("normal.flip_y", "Flip Y (Green)", false);
    b.on("normal.flip_z", "Flip Z (Blue)", false);
    b.on("normal.swap_yz", "Swap Y & Z (Y up)", false);
    b.slot("normal.texture", "Texture");
    b.group = "Alpha";
    b.col("alpha.color", "Alpha Color", [0.0, 0.0, 0.0]);
    b.col("alpha.delta", "Alpha Delta", [0.5, 0.5, 0.5]);
    b.on("alpha.invert", "Invert", false);
    b.on("alpha.soft", "Soft", true);
    b.on("alpha.image_alpha", "Image Alpha", true);
    b.on("alpha.premultiplied", "Pre-multiplied", false);
    b.slot("alpha.texture", "Texture");
    b.group = "Glow";
    b.col("glow.color", "Glow Color", [1.0, 1.0, 1.0]);
    b.pct("glow.brightness", "Glow Brightness", 100.0, 0.0, 100.0);
    b.pct("glow.inner_strength", "Inner Strength", 100.0, 0.0, 10000.0);
    b.pct("glow.outer_strength", "Outer Strength", 500.0, 0.0, 10000.0);
    b.dist("glow.radius", "Radius", 10.0, 0.0, 999999999.0);
    b.pct("glow.random", "Random", 0.0, 0.0, 100.0);
    b.num("glow.frequency", "Frequency", 1.0, 0.0, 999999999.0);
    b.on("glow.use_material_color", "Use Material Color", true);
    b.group = "Displacement";
    b.pct("displacement.strength", "Strength", 100.0, -100.0, 100.0);
    b.dist(
        "displacement.height",
        "Height",
        10.0,
        -999999999.0,
        999999999.0,
    );
    b.en(
        "displacement.type",
        "Type",
        "intensity_centered",
        &[
            ("intensity", "Intensity"),
            ("intensity_centered", "Intensity (Centered)"),
            ("red_green", "Red / Green"),
            ("rgb_tangent", "RGB (XYZ Tangent)"),
            ("rgb_object", "RGB (XYZ Object)"),
            ("rgb_world", "RGB (XYZ World)"),
        ],
    );
    b.slot("displacement.texture", "Texture");
    b.on("displacement.subpoly", "Sub Polygon Displacement", false);
    b.int(
        "displacement.subdivision_level",
        "Subdivision Level",
        4,
        1,
        12,
    );
    b.on("displacement.round_geometry", "Round Geometry", false);
    b.on("displacement.round_contour", "Round Contour", true);
    b.on("displacement.map_rounded", "Map Rounded Geometry", true);
    b.on(
        "displacement.map_resulting",
        "Map Resulting Geometry",
        false,
    );
    b.on("displacement.keep_edges", "Keep Original Edges", true);
    b.on("displacement.best_distribution", "Best Distribution", true);
    b.group = "Illumination";
    b.on("illumination.polygon_light", "Polygon Light", false);
    b.on("illumination.portal", "Portal", false);
    b.on("illumination.gi_generate", "GI Generate", true);
    b.pct(
        "illumination.gi_generate_strength",
        "GI Generate Strength",
        100.0,
        0.0,
        10000.0,
    );
    b.pct(
        "illumination.gi_generate_saturation",
        "GI Generate Saturation",
        100.0,
        0.0,
        1000.0,
    );
    b.on("illumination.gi_receive", "GI Receive", true);
    b.pct(
        "illumination.gi_receive_strength",
        "GI Receive Strength",
        100.0,
        0.0,
        10000.0,
    );
    b.pct(
        "illumination.gi_receive_saturation",
        "GI Receive Saturation",
        100.0,
        0.0,
        1000.0,
    );
    b.on("illumination.caustics_generate", "Caustics Generate", true);
    b.pct(
        "illumination.caustics_generate_strength",
        "Caustics Generate Strength",
        100.0,
        0.0,
        10000.0,
    );
    b.on("illumination.caustics_receive", "Caustics Receive", true);
    b.pct(
        "illumination.caustics_receive_strength",
        "Caustics Receive Strength",
        100.0,
        0.0,
        10000.0,
    );
    b.dist(
        "illumination.caustics_radius",
        "Caustics Sample Radius",
        10.0,
        0.0,
        999999999.0,
    );
    b.num(
        "illumination.caustics_samples",
        "Caustics Samples",
        100.0,
        1.0,
        10000.0,
    );
    b.group = "Viewport";
    b.on("viewport.animate_preview", "Animate Preview", false);
    b.en(
        "viewport.texture_size",
        "Texture Preview Size",
        "default",
        &[
            ("default", "Default"),
            ("none", "No Scaling"),
            ("s64", "64x64"),
            ("s128", "128x128"),
            ("s256", "256x256"),
            ("s512", "512x512"),
            ("s1024", "1024x1024"),
            ("s2048", "2048x2048"),
            ("s4096", "4096x4096"),
            ("s8192", "8192x8192"),
            ("s16384", "16384x16384"),
        ],
    );
    b.en(
        "viewport.display",
        "Viewport Display",
        "combined",
        &[
            ("combined", "Combined"),
            ("color", "Color"),
            ("diffusion", "Diffusion"),
            ("luminance", "Luminance"),
            ("transparency", "Transparency"),
            ("reflectance", "Reflectance"),
            ("environment", "Environment"),
            ("bump", "Bump"),
            ("alpha", "Alpha"),
            ("displacement", "Displacement"),
            ("normal", "Normal"),
        ],
    );
    for c in [
        "color",
        "diffusion",
        "luminance",
        "transparency",
        "environment",
        "bump",
        "normal",
        "alpha",
        "reflectance",
        "displacement",
    ] {
        b.on(&format!("viewport.show.{c}"), &format!("Display {c}"), true);
    }
    b.ty
}

/// The texture tag (M4).
pub fn texture_tag_type() -> NodeType {
    let mut b = B {
        ty: NodeType::new(TEXTURE_TAG, "Texture", Category::Tag),
        group: "Tag",
    };
    b.add(ParamDesc::link(
        "material",
        "Material",
        &[Category::Material],
    ));
    b.add(ParamDesc::string("selection", "Selection", ""));
    b.en(
        "projection",
        "Projection",
        "spherical",
        &[
            ("spherical", "Spherical"),
            ("cylindrical", "Cylindrical"),
            ("flat", "Flat"),
            ("cubic", "Cubic"),
            ("frontal", "Frontal"),
            ("spatial", "Spatial"),
            ("uvw", "UVW Mapping"),
            ("shrink_wrap", "Shrink Wrapping"),
            ("camera", "Camera Mapping"),
        ],
    );
    b.en(
        "side",
        "Side",
        "both",
        &[
            ("both", "Front and Back"),
            ("front", "Front"),
            ("back", "Back"),
        ],
    );
    b.on("mix_textures", "Mix Textures", false);
    b.pct("offset_u", "Offset U", 0.0, -1e9, 1e9);
    b.pct("offset_v", "Offset V", 0.0, -1e9, 1e9);
    b.pct("length_u", "Length U", 100.0, -1e9, 1e9);
    b.pct("length_v", "Length V", 100.0, -1e9, 1e9);
    b.num("tiles_u", "Tiles U", 1.0, 0.0, 1e9);
    b.num("tiles_v", "Tiles V", 1.0, 0.0, 1e9);
    b.num("repeat_u", "Repeat U", 1.0, 0.0, 1e9);
    b.num("repeat_v", "Repeat V", 1.0, 0.0, 1e9);
    b.on("seamless", "Seamless", false);
    b.add(ParamDesc::vec3("position", "Position", [0.0; 3]));
    b.add(ParamDesc::vec3("rotation", "Rotation", [0.0; 3]).unit(Unit::Angle));
    b.add(ParamDesc::vec3("size", "Size", [100.0; 3]).range(1e-6, 1e12));
    b.ty
}

/// The UVW tag (UV.md U5): `uvw` holds `u, v` per polygon corner.
pub fn uvw_tag_type() -> NodeType {
    NodeType::new(UVW_TAG, "UVW", Category::Tag)
        .param(ParamDesc::float_list("uvw", "UVW").range(-1e12, 1e12))
        .param(ParamDesc::index_list("seams", "UV Seams").hidden())
}

/// Registers the material node types.
pub struct MaterialPlugin;

impl Plugin for MaterialPlugin {
    fn id(&self) -> &str {
        "forge.material"
    }
    fn register(&self, registry: &mut Registry) -> Result<(), RegistryError> {
        registry.register(standard_type())?;
        registry.register(texture_tag_type())?;
        registry.register(uvw_tag_type())?;
        registry.register(crate::node_material::node_material_type())
    }
}
