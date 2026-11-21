//! Render API: snapshot, settings, camera, frames, paths, files (docs/specs/RENDER.md).

use std::collections::BTreeMap;

use forge_image::{AlphaMode, Image, Limits, decode_exr_layers};
use forge_render_api::*;
use glam::{DMat4, DVec3};

#[test]
fn settings_validation() {
    assert!(RenderSettings::new(8, 8, 1).validate().is_ok());
    assert!(RenderSettings::new(0, 8, 1).validate().is_err());
    assert!(RenderSettings::new(8, 8, 0).validate().is_err());
    let mut s = RenderSettings::new(8, 8, 1);
    s.region = Some([0, 0, 9, 8]);
    assert!(s.validate().is_err());
    s.region = Some([2, 2, 2, 8]);
    assert!(s.validate().is_err());
    s.region = Some([2, 2, 8, 8]);
    assert!(s.validate().is_ok());
    s.filter_width = 0.0;
    assert!(s.validate().is_err());
}

#[test]
fn snapshot_validation() {
    let mut s = Snapshot::default();
    assert!(s.validate().is_ok());
    s.meshes.push(MeshData::new(
        vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0, 1, 3]],
    ));
    assert!(matches!(s.validate(), Err(RenderError::Snapshot(_))));
    s.meshes[0].triangles[0] = [0, 1, 2];
    assert!(s.validate().is_ok());
    s.instances.push(Instance::new(0, DMat4::IDENTITY, 0, 1));
    assert!(s.validate().is_err(), "no material 0");
    s.materials.push(MaterialData::new(
        "m",
        forge_material::compile(&forge_material::Material::new()),
    ));
    assert!(s.validate().is_ok());
    s.camera.projection = Projection::Perspective {
        focal_mm: 0.0,
        sensor_mm: 36.0,
        film_offset: [0.0; 2],
    };
    assert!(s.validate().is_err());
}

#[test]
fn camera_centre_ray_and_pitch_sign() {
    let proj = Projection::Perspective {
        focal_mm: 36.0,
        sensor_mm: 36.0,
        film_offset: [0.0; 2],
    };
    let cam = Camera::from_hpb(
        DVec3::new(0.0, 200.0, -600.0),
        [0.0, (-15.0f64).to_radians(), 0.0],
        proj,
    );
    let (o, d) = cam.ray(160.0, 120.0, 320, 240);
    assert_eq!(o, DVec3::new(0.0, 200.0, -600.0));
    assert!(d.y < 0.0, "negative pitch looks down: {d:?}");
    assert!((d.y - (-15.0f64).to_radians().sin()).abs() < 1e-12);
    // Top of the image looks higher than the bottom.
    let (_, up) = cam.ray(160.0, 0.0, 320, 240);
    let (_, dn) = cam.ray(160.0, 240.0, 320, 240);
    assert!(up.y > dn.y);
    // Pixel focal length = width * focal / sensor = 320: one image width to the right is 45 deg.
    let flat = Camera::from_hpb(DVec3::ZERO, [0.0; 3], proj);
    let (_, r) = flat.ray(160.0 + 320.0, 120.0, 320, 240);
    assert!((r.x / r.z - 1.0).abs() < 1e-12);
}

#[test]
fn camera_film_offset_moves_the_principal_point() {
    let cam = Camera::from_hpb(
        DVec3::ZERO,
        [0.0; 3],
        Projection::Perspective {
            focal_mm: 36.0,
            sensor_mm: 36.0,
            film_offset: [0.25, 0.1],
        },
    );
    let (f, c) = cam.pixel_frame(320, 240).unwrap();
    assert_eq!(f, 320.0);
    // x is a fraction of the width (0.25 * 320), y of the height (0.1 * 240): `film-offset-y`.
    assert_eq!(c, [240.0, 120.0 - 24.0]);
    let (_, d) = cam.ray(240.0, 96.0, 320, 240);
    assert!((d - DVec3::Z).length() < 1e-12);
}

#[test]
fn heading_turns_toward_minus_x() {
    let cam = Camera::from_hpb(
        DVec3::ZERO,
        [90.0f64.to_radians(), 0.0, 0.0],
        Projection::Orthographic { width: 100.0 },
    );
    let (_, d) = cam.ray(0.0, 0.0, 10, 10);
    assert!((d + DVec3::X).length() < 1e-12, "{d:?}");
}

#[test]
fn frame_lists() {
    assert_eq!(frames(FrameRange::Current, 1, 7, (0, 90)).unwrap(), vec![7]);
    assert_eq!(
        frames(FrameRange::All, 30, 0, (0, 90)).unwrap(),
        vec![0, 30, 60, 90]
    );
    assert_eq!(
        frames(FrameRange::Manual { start: 2, end: 5 }, 2, 0, (0, 90)).unwrap(),
        vec![2, 4]
    );
    assert!(frames(FrameRange::Manual { start: 5, end: 2 }, 1, 0, (0, 90)).is_err());
    assert!(frames(FrameRange::Current, 0, 0, (0, 90)).is_err());
}

#[test]
fn path_tokens() {
    let f = OutputFormat::Png;
    assert_eq!(
        expand_path("out/{name}_{frame:4}", 12, "beauty", "shot", true, f),
        "out/shot_0012.png"
    );
    assert_eq!(
        expand_path("out/img.png", 3, "beauty", "x", true, f),
        "out/img0003.png"
    );
    assert_eq!(
        expand_path("out/img", 3, "beauty", "x", false, f),
        "out/img.png"
    );
    assert_eq!(
        expand_path("out/img{frame}", 3, "depth", "x", true, f),
        "out/img3_depth.png"
    );
    assert_eq!(
        expand_path("{pass}/f{frame:3}", 5, "normal", "x", true, f),
        "normal/f005.png"
    );
    // The default of the reference: Name0000.TIF
    let d = OutputSettings::default();
    assert_eq!(
        expand_path(&d.path, 0, "beauty", &d.name, false, d.format),
        "Name0000.tif"
    );
}

fn sample_output(w: u32, h: u32) -> RenderOutput {
    let img = |v: f32| {
        let mut i = Image::new(w, h, AlphaMode::Straight);
        for p in i.data.as_chunks_mut::<4>().0.iter_mut() {
            p.copy_from_slice(&[v, v * 0.5, v * 0.25, 1.0]);
        }
        i
    };
    let mut aovs = BTreeMap::new();
    aovs.insert(Aov::Depth, img(3.0));
    aovs.insert(Aov::Normal, img(0.5));
    RenderOutput {
        width: w,
        height: h,
        beauty: img(0.8),
        aovs,
        stats: RenderStats::default(),
    }
}

#[test]
fn separate_files_per_pass() {
    let dir = tempfile::tempdir().unwrap();
    let out = sample_output(4, 3);
    let s = OutputSettings {
        format: OutputFormat::Exr,
        depth_bits: 32,
        alpha: true,
        path: "seq/shot".into(),
        ..OutputSettings::default()
    };
    let files = write_output(&out, &s, 7, true, dir.path()).unwrap();
    let names: Vec<String> = files
        .iter()
        .map(|p| {
            p.strip_prefix(dir.path())
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(
        names,
        [
            "seq/shot0007.exr",
            "seq/shot0007_depth.exr",
            "seq/shot0007_normal.exr"
        ]
    );
    for f in &files {
        assert!(f.exists());
    }
}

#[test]
fn multipart_exr_has_a_layer_per_pass() {
    let dir = tempfile::tempdir().unwrap();
    let out = sample_output(4, 3);
    let s = OutputSettings {
        format: OutputFormat::Exr,
        depth_bits: 32,
        alpha: true,
        straight_alpha: true,
        multipass: Multipass::Multipart,
        path: "m".into(),
        ..OutputSettings::default()
    };
    let files = write_output(&out, &s, 0, false, dir.path()).unwrap();
    assert_eq!(files.len(), 1);
    let bytes = std::fs::read(&files[0]).unwrap();
    let layers = decode_exr_layers(&bytes, &Limits::default()).unwrap();
    let names: Vec<_> = layers.iter().filter_map(|l| l.name.clone()).collect();
    assert_eq!(names, ["beauty", "depth", "normal"]);
    let depth = layers
        .iter()
        .find(|l| l.name.as_deref() == Some("depth"))
        .unwrap();
    assert_eq!(depth.channels.len(), 1);
    assert_eq!(depth.channels[0].name, "Z");
    assert!((depth.channels[0].samples[0] - 3.0).abs() < 1e-6);
}

#[test]
fn bad_pairings_are_errors() {
    let dir = tempfile::tempdir().unwrap();
    let out = sample_output(2, 2);
    let mut s = OutputSettings {
        format: OutputFormat::Jpeg,
        depth_bits: 16,
        ..OutputSettings::default()
    };
    assert!(write_output(&out, &s, 0, false, dir.path()).is_err());
    s.format = OutputFormat::Png;
    s.depth_bits = 8;
    s.multipass = Multipass::Multipart;
    assert!(write_output(&out, &s, 0, false, dir.path()).is_err());
    assert!(!dir.path().join("Name0000.png").exists());
}

#[test]
fn png_writes_alpha_only_when_asked() {
    let dir = tempfile::tempdir().unwrap();
    let mut out = sample_output(2, 2);
    for p in out.beauty.data.as_chunks_mut::<4>().0.iter_mut() {
        p[3] = 0.5;
    }
    let mut s = OutputSettings {
        format: OutputFormat::Png,
        path: "a".into(),
        straight_alpha: true,
        ..OutputSettings::default()
    };
    let f = write_output(&out, &s, 0, false, dir.path()).unwrap();
    let d = forge_image::decode(&std::fs::read(&f[0]).unwrap(), &Limits::default()).unwrap();
    assert_eq!(d.image.pixel(0, 0)[3], 1.0);
    s.alpha = true;
    let f = write_output(&out, &s, 0, false, dir.path()).unwrap();
    let d = forge_image::decode(&std::fs::read(&f[0]).unwrap(), &Limits::default()).unwrap();
    assert!((d.image.pixel(0, 0)[3] - 0.5).abs() < 0.01);
}

#[test]
fn events_and_cancel() {
    use std::sync::{Arc, Mutex};
    let seen = Arc::new(Mutex::new(Vec::new()));
    let s2 = seen.clone();
    let c = Control::with_events(move |e| s2.lock().unwrap().push(format!("{e:?}")));
    c.emit(RenderEvent::Finished);
    assert_eq!(seen.lock().unwrap().len(), 1);
    assert!(!c.is_cancelled());
    let c2 = c.clone();
    c2.cancel();
    assert!(c.is_cancelled());
}

#[test]
fn aov_names_round_trip() {
    for a in Aov::ALL {
        assert_eq!(Aov::parse(a.name()), Some(a));
    }
    assert_eq!(Aov::parse("nope"), None);
}
