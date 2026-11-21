//! Scene objects in the snapshot (docs/specs/SCENE-OBJECTS.md S5).

use std::sync::Arc;

use forge_core::{Document, NodeId, Registry, Value};
use forge_generators::Primitives;
use forge_render_api::{Background, LightKind, Lighting, Projection, Snapshot};
use forge_render_bridge::*;
use glam::DVec3;

fn doc() -> Document {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    Document::new(Arc::new(r))
}

fn create(d: &mut Document, t: &str) -> NodeId {
    let mut id = None;
    d.transact("create", |tx| {
        id = Some(tx.create(t, None, None)?);
        Ok(())
    })
    .unwrap();
    id.unwrap()
}

fn set(d: &mut Document, n: NodeId, p: &str, v: Value) {
    d.transact("set", |tx| tx.set_param(n, p, v)).unwrap();
}

#[test]
fn an_empty_scene_changes_nothing() {
    let d = doc();
    let mut snap = Snapshot::default();
    let before = snap.camera;
    let a = apply_scene_objects(&mut snap, d.scene());
    assert_eq!(a, Applied::default());
    assert_eq!(snap.camera, before);
    assert!(snap.lights.is_empty() && snap.instances.is_empty());
}

#[test]
fn the_camera_object_gives_the_pixel_mapping_of_the_formula() {
    let mut d = doc();
    let c = create(&mut d, "forge.camera");
    set(&mut d, c, "transform.pos", Value::Vec3([0.0, 0.0, -600.0]));
    // O57 focal-18: horizontal FOV 90 degrees; film offset X 20 %
    set(
        &mut d,
        c,
        "fov_horizontal",
        Value::Float(std::f64::consts::FRAC_PI_2),
    );
    set(&mut d, c, "film_offset_x", Value::Float(0.2));
    let mut snap = Snapshot::default();
    let a = apply_scene_objects(&mut snap, d.scene());
    assert!(a.camera && a.warnings.is_empty());
    let Projection::Perspective {
        focal_mm,
        sensor_mm,
        film_offset,
    } = snap.camera.projection
    else {
        panic!()
    };
    assert!((focal_mm - 18.0).abs() < 1e-9 && sensor_mm == 36.0 && film_offset == [-0.2, 0.0]);
    // 320 x 240: f_px = 320 * 18 / 36 = 160; Film Offset X +20 % moves the picture left (O57
    // film-offset): the principal point sits 0.2 * 320 = 64 px left of the centre
    let (f, p) = snap.camera.pixel_frame(320, 240).unwrap();
    assert!((f - 160.0).abs() < 1e-9 && (p[0] - 96.0).abs() < 1e-9 && (p[1] - 120.0).abs() < 1e-9);
    // the centre pixel looks along +Z from (0, 0, -600)
    let (o, dir) = snap.camera.ray(p[0], p[1], 320, 240);
    assert_eq!(o, DVec3::new(0.0, 0.0, -600.0));
    assert!((dir - DVec3::Z).length() < 1e-12);
}

#[test]
fn heading_sign_is_the_references_not_the_right_handed_one() {
    let mut d = doc();
    let c = create(&mut d, "forge.camera");
    set(
        &mut d,
        c,
        "transform.rot",
        Value::Vec3([(-15f64).to_radians(), 0.0, 0.0]),
    );
    let mut snap = Snapshot::default();
    apply_scene_objects(&mut snap, d.scene());
    // the same matrix as the render API's hpb convention (RENDER.md R10): heading -15 looks right
    let expect = forge_render_api::Camera::from_hpb(
        DVec3::ZERO,
        [(-15f64).to_radians(), 0.0, 0.0],
        snap.camera.projection,
    );
    assert!((snap.camera.matrix() - expect.matrix()).abs_diff_eq(glam::DMat4::ZERO, 1e-12));
    assert!(snap.camera.matrix().transform_vector3(DVec3::Z).x > 0.0);
}

#[test]
fn axis_projections_ignore_the_camera_rotation() {
    let mut d = doc();
    let c = create(&mut d, "forge.camera");
    set(&mut d, c, "projection", Value::Enum("top".into()));
    set(&mut d, c, "transform.pos", Value::Vec3([0.0, 600.0, 0.0]));
    set(&mut d, c, "transform.rot", Value::Vec3([0.7, -0.3, 0.1]));
    let mut snap = Snapshot::default();
    apply_scene_objects(&mut snap, d.scene());
    let m = snap.camera.matrix();
    assert!((m.transform_vector3(DVec3::Z) - DVec3::NEG_Y).length() < 1e-12);
    assert!((m.transform_vector3(DVec3::Y) - DVec3::Z).length() < 1e-12);
    assert!(
        matches!(snap.camera.projection, Projection::Orthographic { width } if width == 1000.0)
    );
}

#[test]
fn lights_environment_sky_and_floor() {
    let mut d = doc();
    let l = create(&mut d, "forge.light");
    set(&mut d, l, "type", Value::Enum("spot".into()));
    set(&mut d, l, "shadow", Value::Enum("raytraced".into()));
    set(&mut d, l, "falloff", Value::Enum("inverse_square".into()));
    let off = create(&mut d, "forge.light");
    set(&mut d, off, "no_illumination", Value::Bool(true));
    let bad = create(&mut d, "forge.light");
    set(&mut d, bad, "type", Value::Enum("area".into()));
    set(&mut d, bad, "area_shape", Value::Enum("cube".into()));
    let s = create(&mut d, "forge.sky");
    create(&mut d, "forge.floor");
    let mut snap = Snapshot::default();
    let a = apply_scene_objects(&mut snap, d.scene());
    assert_eq!((a.lights, a.scene_has_lights), (1, true));
    assert_eq!(a.warnings.len(), 1, "{:?}", a.warnings);
    assert!(a.warnings[0].contains("cube"));
    assert!(matches!(snap.lights[0].kind, LightKind::Spot { .. }));
    assert_eq!(snap.lights[0].shadow, forge_render_api::ShadowMode::Hard);
    assert!(matches!(snap.environment.lighting, Lighting::Sky { .. }));
    assert_eq!(snap.environment.background, Background::Environment);
    assert_eq!(snap.instances.len(), 1, "the floor");
    assert!(snap.validate().is_ok());
    // a sky hidden for rendering is not used
    set(&mut d, s, "object.render_vis", Value::Enum("off".into()));
    let mut snap = Snapshot::default();
    apply_scene_objects(&mut snap, d.scene());
    assert!(matches!(snap.environment.lighting, Lighting::None));
}

#[test]
fn stored_settings_become_render_settings() {
    let mut d = doc();
    let base = forge_render_api::RenderSettings::default();
    assert_eq!(
        settings_from_scene(d.scene(), base.clone()).width,
        base.width
    );
    d.transact("r", |tx| {
        tx.set_render_config(Some(forge_core::RenderConfig {
            width: 100,
            height: 50,
            samples: 3,
            ..Default::default()
        }))
    })
    .unwrap();
    let s = settings_from_scene(d.scene(), base);
    assert_eq!((s.width, s.height, s.samples), (100, 50, 3));
    assert_eq!(s.filter, forge_render_api::PixelFilter::Cubic);
    assert!(s.validate().is_ok());
}

#[test]
fn an_environment_image_becomes_image_lighting() {
    use forge_image::{AlphaMode, BitDepth, EncodeOptions, Format, Image};
    let dir = std::env::temp_dir().join(format!("forge-bridge-hdri-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sky.hdr");
    let mut data = Vec::new();
    for _ in 0..8 * 4 {
        data.extend_from_slice(&[2.0, 2.0, 2.0, 1.0]);
    }
    let img = Image {
        width: 8,
        height: 4,
        data,
        alpha: AlphaMode::Straight,
    };
    let opts = EncodeOptions {
        depth: BitDepth::F32,
        ..EncodeOptions::default()
    };
    std::fs::write(
        &path,
        forge_image::encode(&img, Format::Hdr, &opts).unwrap(),
    )
    .unwrap();

    let mut d = doc();
    let e = create(&mut d, "forge.environment");
    set(
        &mut d,
        e,
        "image",
        Value::String(path.to_string_lossy().into_owned()),
    );
    set(&mut d, e, "brightness", Value::Float(0.5));
    set(&mut d, e, "visible_in_background", Value::Bool(true));
    let mut snap = Snapshot::default();
    let a = apply_scene_objects(&mut snap, d.scene());
    assert!(a.warnings.is_empty(), "{:?}", a.warnings);
    let Lighting::Image {
        image, intensity, ..
    } = &snap.environment.lighting
    else {
        panic!("{:?}", snap.environment.lighting)
    };
    assert_eq!((image.width, image.height, *intensity), (8, 4, 0.5));
    assert!(
        (image.pixel(3, 2)[0] - 2.0).abs() < 0.05,
        "{:?}",
        image.pixel(3, 2)
    );
    assert_eq!(snap.environment.background, Background::Environment);

    // a missing file warns and keeps the uniform colour
    set(
        &mut d,
        e,
        "image",
        Value::String(dir.join("nope.hdr").to_string_lossy().into_owned()),
    );
    let mut snap = Snapshot::default();
    let a = apply_scene_objects(&mut snap, d.scene());
    assert_eq!(a.warnings.len(), 1, "{:?}", a.warnings);
    assert!(matches!(
        snap.environment.lighting,
        Lighting::Constant { .. }
    ));
    std::fs::remove_dir_all(&dir).ok();
}
