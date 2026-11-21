//! Scene objects to render snapshot data (docs/specs/SCENE-OBJECTS.md S5, RENDER.md R1).
//!
//! The first camera of the scene (tree order) becomes the snapshot camera, every light with
//! illumination on becomes a [`Light`], the first Environment (else the first Sky) becomes the
//! environment, and each Floor becomes a large two-triangle instance. Without a camera the
//! caller's framing camera stays; without lights the engine's default light applies.

use forge_core::{RenderConfig, RenderFilter, Scene};
use forge_generators::scene_objects::{
    AreaKind, CameraData, FalloffKind, FloorData, Lens, LightData, LightShape, ShadowKind,
    scene_objects,
};
use forge_material::{Material, compile};
use forge_render_api::{
    Aperture, AreaShape, Background, Camera, Environment, Falloff, Instance, Light, LightKind,
    Lighting, MaterialData, MeshData, PixelFilter, Projection, RenderSettings, ShadowMode,
    Snapshot,
};

/// What [`apply_scene_objects`] did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Applied {
    /// A scene camera replaced the framing camera.
    pub camera: bool,
    /// Lights added to the snapshot.
    pub lights: usize,
    /// The scene has light objects (even switched off): the default light must stay off.
    pub scene_has_lights: bool,
    /// Options that are not rendered as the object asks (never silent).
    pub warnings: Vec<String>,
}

/// The snapshot camera of a camera object.
pub fn camera_from_data(c: &CameraData) -> Camera {
    let projection = match c.lens {
        Lens::Perspective {
            focal_mm,
            sensor_mm,
            film_offset,
        } => Projection::Perspective {
            focal_mm,
            sensor_mm,
            film_offset,
        },
        Lens::Orthographic { width } => Projection::Orthographic { width },
    };
    let camera = Camera::new(c.world, projection);
    if c.lens_radius > 0.0 {
        camera.with_aperture(Aperture {
            radius: c.lens_radius,
            focus_distance: c.focus_distance.max(1e-6),
        })
    } else {
        camera
    }
}

/// The snapshot light of a light object; `Err` names why it cannot be rendered.
pub fn light_from_data(l: &LightData) -> Result<Light, String> {
    let kind = match l.shape {
        LightShape::Omni => LightKind::Omni,
        LightShape::Spot => LightKind::Spot {
            inner: l.inner,
            outer: l.outer,
        },
        LightShape::Infinite => LightKind::Infinite {
            angle: l.infinite_angle,
        },
        LightShape::Area => LightKind::Area {
            shape: match l.area {
                AreaKind::Disc { radius } => AreaShape::Disc { radius },
                AreaKind::Rectangle { width, height } => AreaShape::Rectangle { width, height },
                AreaKind::Sphere { radius } => AreaShape::Sphere { radius },
                AreaKind::Unsupported(s) => {
                    return Err(format!("area shape {s} is not rendered yet"));
                }
            },
            two_sided: l.two_sided,
        },
    };
    let mut out = Light::new(kind, l.world);
    out.color = l.color;
    out.intensity = l.intensity;
    out.falloff = match l.falloff {
        FalloffKind::None | FalloffKind::Step => Falloff::None,
        FalloffKind::Linear => Falloff::Linear,
        FalloffKind::InverseSquare => Falloff::InverseSquare,
    };
    out.falloff_radius = l.falloff_radius;
    out.shadow = match l.shadow {
        ShadowKind::None => ShadowMode::None,
        ShadowKind::Hard => ShadowMode::Hard,
        ShadowKind::Soft => ShadowMode::Soft,
    };
    out.shadow_density = l.shadow_density;
    out.shadow_color = l.shadow_color;
    out.diffuse = l.diffuse;
    out.specular = l.specular;
    out.visible = l.visible;
    Ok(out)
}

fn floor_mesh(f: &FloorData) -> MeshData {
    let e = f.extent;
    let mut m = MeshData::new(
        vec![[-e, 0.0, -e], [e, 0.0, -e], [e, 0.0, e], [-e, 0.0, e]],
        // wound so that the geometric normal is +Y
        vec![[0, 2, 1], [0, 3, 2]],
    );
    m.normals = vec![[0.0, 1.0, 0.0]; 6];
    m
}

fn flat_material(name: &str, color: [f64; 3]) -> MaterialData {
    let mut m = Material::new();
    m.set("color.color", forge_core::Value::Color(color));
    MaterialData::new(name, compile(&m))
}

/// Adds the scene's camera, lights, environment, sky and floors to `snap`.
pub fn apply_scene_objects(snap: &mut Snapshot, scene: &Scene) -> Applied {
    let mut objs = scene_objects(scene);
    // hidden objects and layers switched off for rendering are not part of the render
    let shown = |n| forge_core::visibility::render_visible(scene, n);
    objs.cameras.retain(|c| shown(c.node));
    objs.lights.retain(|l| shown(l.node));
    objs.environments.retain(|e| shown(e.node));
    objs.skies.retain(|e| shown(e.node));
    objs.floors.retain(|e| shown(e.node));
    let mut out = Applied::default();
    if let Some(c) = objs.cameras.first() {
        snap.camera = camera_from_data(c);
        out.camera = true;
        for n in &c.notes {
            out.warnings.push(format!("camera {:?}: {n}", c.name));
        }
        if objs.cameras.len() > 1 {
            out.warnings.push(format!(
                "{} cameras in the scene: rendering through {:?}, the first",
                objs.cameras.len(),
                c.name
            ));
        }
    }
    out.scene_has_lights = !objs.lights.is_empty();
    for l in &objs.lights {
        for n in &l.notes {
            out.warnings.push(format!("light {:?}: {n}", l.name));
        }
        if l.off {
            continue;
        }
        if l.falloff == FalloffKind::Step {
            out.warnings.push(format!(
                "light {:?}: falloff Step is rendered as None",
                l.name
            ));
        }
        match light_from_data(l) {
            Ok(light) => {
                snap.lights.push(light);
                out.lights += 1;
            }
            Err(e) => out
                .warnings
                .push(format!("light {:?} skipped: {e}", l.name)),
        }
    }
    let mut env = Environment::default();
    if let Some(e) = objs.environments.first() {
        env.lighting = Lighting::Constant {
            color: e.color,
            intensity: e.brightness,
        };
        if e.visible_in_background {
            env.background = Background::Environment;
        }
        if !e.image.is_empty() {
            out.warnings.push(format!(
                "environment image {:?} is not loaded yet: using the colour",
                e.image
            ));
        }
        if objs.environments.len() > 1 || !objs.skies.is_empty() {
            out.warnings
                .push("more than one Environment or Sky: the first Environment is used".into());
        }
    } else if let Some(s) = objs.skies.first() {
        env.lighting = Lighting::Sky {
            zenith: s.zenith,
            horizon: s.horizon,
            ground: s.ground,
            intensity: s.brightness,
        };
        if s.visible_in_background {
            env.background = Background::Environment;
        }
        if objs.skies.len() > 1 {
            out.warnings
                .push("more than one Sky: the first is used".into());
        }
    }
    snap.environment = env;
    for f in &objs.floors {
        snap.meshes.push(floor_mesh(f));
        snap.materials.push(flat_material("Floor", f.color));
        let (mesh, material) = (snap.meshes.len() - 1, snap.materials.len() - 1);
        // object ids continue after the geometry instances
        let id = snap.instances.len() as u32;
        snap.instances
            .push(Instance::new(mesh, f.world, material, id));
    }
    out
}

/// Render settings of the scene's stored configuration, or `base` when it has none. Command
/// line overrides are applied by the caller afterwards.
pub fn settings_from_scene(scene: &Scene, base: RenderSettings) -> RenderSettings {
    let Some(c) = scene.render_config() else {
        return base;
    };
    settings_from_config(c, base)
}

pub fn settings_from_config(c: &RenderConfig, base: RenderSettings) -> RenderSettings {
    RenderSettings {
        width: c.width,
        height: c.height,
        samples: c.samples,
        max_depth: c.max_depth,
        seed: c.seed,
        default_light: c.default_light,
        filter: match c.filter {
            RenderFilter::Box => PixelFilter::Box,
            RenderFilter::Triangle => PixelFilter::Triangle,
            RenderFilter::Gaussian => PixelFilter::Gaussian,
            RenderFilter::Cubic => PixelFilter::Cubic,
        },
        filter_width: c.filter_width,
        depth_of_field: c.depth_of_field,
        ..base
    }
}
