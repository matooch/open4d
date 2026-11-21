//! Golden render cases (tests/golden/render): OBJ and case.toml to a snapshot.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use forge_core::Value;
use forge_material::{Material, compile};
use forge_parity_harness::ImageBuf;
use forge_render_api::*;
use glam::{DMat4, DVec3};

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/render")
}

pub fn case_names() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(root())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("case.toml").exists())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

pub struct ObjGroup {
    pub name: String,
    pub mesh: MeshData,
}

/// Reads `o` groups with `v`, `vt`, `vn` and (fan-triangulated) `f` statements.
pub fn parse_obj(src: &str) -> Vec<ObjGroup> {
    let (mut v, mut vt, mut vn) = (Vec::new(), Vec::new(), Vec::new());
    let mut groups: Vec<(String, Vec<Vec<[usize; 3]>>)> = Vec::new();
    for line in src.lines() {
        let mut it = line.split_whitespace();
        let nums = |it: std::str::SplitWhitespace| -> Vec<f64> {
            it.filter_map(|t| t.parse().ok()).collect()
        };
        match it.next() {
            Some("v") => {
                let c = nums(it);
                v.push([c[0], c[1], c[2]]);
            }
            Some("vt") => {
                let c = nums(it);
                vt.push([c[0], c.get(1).copied().unwrap_or(0.0), 0.0]);
            }
            Some("vn") => {
                let c = nums(it);
                vn.push([c[0], c[1], c[2]]);
            }
            Some("o") => groups.push((it.next().unwrap_or("").to_string(), Vec::new())),
            Some("f") => {
                let face = it
                    .map(|t| {
                        let mut p = t.split('/').map(|s| s.parse::<usize>().unwrap_or(0));
                        [
                            p.next().unwrap_or(0),
                            p.next().unwrap_or(0),
                            p.next().unwrap_or(0),
                        ]
                    })
                    .collect();
                if groups.is_empty() {
                    groups.push((String::new(), Vec::new()));
                }
                groups.last_mut().unwrap().1.push(face);
            }
            _ => {}
        }
    }
    groups
        .into_iter()
        .map(|(name, faces)| {
            let mut m = MeshData::new(v.clone(), Vec::new());
            for f in faces {
                for k in 1..f.len() - 1 {
                    let c = [f[0], f[k], f[k + 1]];
                    m.triangles.push([
                        (c[0][0] - 1) as u32,
                        (c[1][0] - 1) as u32,
                        (c[2][0] - 1) as u32,
                    ]);
                    for corner in c {
                        if corner[2] > 0 {
                            m.normals.push(vn[corner[2] - 1]);
                        }
                        if corner[1] > 0 {
                            m.uvs.push(vt[corner[1] - 1]);
                        }
                    }
                }
            }
            if m.normals.len() != m.triangles.len() * 3 {
                m.normals.clear();
            }
            if m.uvs.len() != m.triangles.len() * 3 {
                m.uvs.clear();
            }
            ObjGroup { name, mesh: m }
        })
        .collect()
}

/// A fine sphere standing for a sphere the reference renders as a perfect one
/// (`render_perfect`): the 16-segment OBJ is only its editor tessellation. UVs come from the
/// primitive layout (UV.md U2), or from the texture tag's projection when it is not UVW mapping.
pub fn perfect_sphere(
    center: [f64; 3],
    radius: f64,
    seg: i64,
    projection: Option<&str>,
) -> MeshData {
    let mut reg = forge_core::Registry::with_builtins();
    reg.install(&forge_generators::Primitives).unwrap();
    let ty = reg.get("forge.sphere").unwrap();
    let mut values: BTreeMap<String, Value> = ty
        .params
        .iter()
        .map(|p| (p.id.clone(), p.default.clone()))
        .collect();
    values.insert("radius".into(), Value::Float(radius));
    values.insert("segments".into(), Value::Int(seg));
    let (poly, uvs) = forge_generators::uv::build_with_uvs("forge.sphere", &values).unwrap();
    let id = projection.and_then(|p| match p {
        "Spherical" => Some("spherical"),
        "Cubic" => Some("cubic"),
        "Cylindrical" => Some("cylindrical"),
        "Flat" => Some("flat"),
        _ => None,
    });
    let uvs = match id {
        Some(id) => forge_material::generate_uvw(
            &poly,
            &forge_material::TagSpace::default(),
            forge_material::Projection::from_id(id, None).unwrap(),
            DMat4::from_translation(DVec3::from(center)),
        )
        .unwrap(),
        None => uvs.expect("sphere UV layout"),
    };
    let c = DVec3::from(center);
    let positions: Vec<[f64; 3]> = poly.points.iter().map(|p| (*p + c).to_array()).collect();
    let mut m = MeshData::new(positions, Vec::new());
    for (pi, face) in poly.polys.iter().enumerate() {
        for k in 1..face.len() - 1 {
            let corners = [0, k, k + 1];
            m.triangles.push([face[0], face[k], face[k + 1]]);
            for ci in corners {
                m.normals
                    .push(poly.points[face[ci] as usize].normalize().to_array());
                let uv = uvs.corners[pi][ci];
                m.uvs.push([uv[0], uv[1], 0.0]);
            }
        }
    }
    m
}

pub fn f3(v: &toml::Value) -> [f64; 3] {
    let a = v.as_array().unwrap();
    [
        a[0].as_float().unwrap(),
        a[1].as_float().unwrap(),
        a[2].as_float().unwrap(),
    ]
}

/// Case file rotations are heading, pitch, bank in degrees.
pub fn hpb_of(rot_deg: [f64; 3]) -> [f64; 3] {
    rot_deg.map(f64::to_radians)
}

pub fn camera_of(params: &toml::Table) -> Camera {
    let pos = f3(&params["camera_position"]);
    let rot = f3(&params["camera_rotation_deg"]);
    // "36 mm": the 35 mm equivalent focal length, i.e. against a 36 mm wide film gate.
    let focal: f64 = params
        .get("camera_35mm_equiv")
        .and_then(|v| v.as_str())
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(36.0);
    Camera::from_hpb(
        DVec3::from(pos),
        hpb_of(rot),
        Projection::Perspective {
            focal_mm: focal,
            sensor_mm: 36.0,
            film_offset: [0.0, 0.0],
        },
    )
}

pub fn light_of(t: &toml::Table) -> Option<Light> {
    if t.get("type")?.as_str()? != "light" {
        return None;
    }
    let pos = f3(&t["position"]);
    let rot = f3(&t["rotation_deg"]);
    let m = forge_render_api::camera::hpb_matrix(DVec3::from(pos), hpb_of(rot));
    let kind = match t
        .get("light_type")
        .and_then(|v| v.as_str())
        .unwrap_or("Omni")
    {
        "Infinite" => LightKind::Infinite {
            angle: t["infinite_angle_deg"]
                .as_float()
                .unwrap_or(0.5)
                .to_radians(),
        },
        "Spot" => LightKind::Spot {
            inner: t["inner_angle_deg"].as_float().unwrap_or(0.0).to_radians(),
            outer: t["outer_angle_deg"].as_float().unwrap_or(50.0).to_radians(),
        },
        // The reference's Area light has a size; a default-sized rectangle stands in for it.
        "Area" => LightKind::Area {
            shape: AreaShape::Rectangle {
                width: 100.0,
                height: 100.0,
            },
            two_sided: false,
        },
        _ => LightKind::Omni,
    };
    let mut l = Light::new(kind, m);
    l.intensity = t["intensity_percent"].as_float().unwrap_or(100.0) / 100.0;
    l.color = f3(&t["color"]);
    l.shadow = match t.get("shadow").and_then(|v| v.as_str()).unwrap_or("None") {
        "None" => ShadowMode::None,
        s if s.contains("Hard") => ShadowMode::Hard,
        _ => ShadowMode::Soft,
    };
    Some(l)
}

pub fn material_of(t: Option<&toml::Table>, name: &str) -> MaterialData {
    let mut o: BTreeMap<String, Value> = BTreeMap::new();
    let Some(t) = t else {
        // Cases without a material table: the observation rows say what the sphere carries.
        if name == "material-red" {
            o.insert("color.color".into(), Value::Color([1.0, 0.0, 0.0]));
            o.insert("enable.reflectance".into(), Value::Bool(false));
        } else if name == "material-matte" {
            o.insert("enable.reflectance".into(), Value::Bool(false));
        }
        return MaterialData::new("m", compile(&Material::from_values(&o)));
    };
    let b = |k: &str| t.get(k).and_then(|v| v.as_bool());
    for ch in ["color", "luminance", "transparency", "reflectance", "bump"] {
        if let Some(v) = b(&format!("enable_{ch}")) {
            o.insert(format!("enable.{ch}"), Value::Bool(v));
        }
    }
    if let Some(c) = t.get("color") {
        o.insert("color.color".into(), Value::Color(f3(c)));
    }
    if let Some(c) = t.get("luminance_color") {
        o.insert("luminance.color".into(), Value::Color(f3(c)));
    }
    if let Some(v) = t.get("transparency_refraction").and_then(|v| v.as_float()) {
        o.insert("transparency.refraction".into(), Value::Float(v));
    }
    if let Some(v) = t
        .get("transparency_brightness_percent")
        .and_then(|v| v.as_float())
    {
        o.insert("transparency.brightness".into(), Value::Float(v / 100.0));
    }
    if t.get("reflection_type").and_then(|v| v.as_str()) == Some("GGX") {
        o.insert("reflectance.spec.type".into(), Value::Enum("ggx".into()));
        if let Some(w) = t.get("reflection_width_percent").and_then(|v| v.as_float()) {
            o.insert("reflectance.spec.width".into(), Value::Float(w / 100.0));
        }
        if let Some(c) = t.get("default_specular_layer_color_color") {
            o.insert("reflectance.spec.color".into(), Value::Color(f3(c)));
        }
    }
    let mut shaders = forge_shaders::ShaderSet::default();
    let mut add = |name: &str, type_id: &str| {
        let sh = forge_shaders::build(
            type_id,
            &forge_shaders::Values::new(),
            &forge_shaders::Inputs::default(),
        )
        .unwrap();
        shaders.shaders.insert(name.to_string(), sh);
    };
    match t.get("texture").and_then(|v| v.as_str()) {
        Some("Checkerboard") => {
            add("tex", "forge.shader.checkerboard");
            o.insert("color.texture".into(), Value::String("tex".into()));
        }
        Some("Fresnel") => {
            add("tex", "forge.shader.fresnel");
            o.insert("color.texture".into(), Value::String("tex".into()));
        }
        _ => {}
    }
    if t.get("texture_4").and_then(|v| v.as_str()) == Some("Noise") {
        add("bump", "forge.shader.noise");
        o.insert("bump.texture".into(), Value::String("bump".into()));
        if let Some(v) = t.get("bump_strength_percent").and_then(|v| v.as_float()) {
            o.insert("bump.strength".into(), Value::Float(v / 100.0));
        }
    }
    MaterialData::new("m", compile(&Material::from_values(&o)))
        .with_sampler(std::sync::Arc::new(shaders))
}

pub struct Case {
    pub name: String,
    pub params: toml::Table,
    pub snapshot: Snapshot,
    pub expected: ImageBuf,
}

/// Builds the snapshot of a case: the OBJ groups that the case file lists as objects, its
/// lights, its material and its camera.
pub fn load(name: &str) -> Case {
    let dir = root().join(name);
    let doc: toml::Table = std::fs::read_to_string(dir.join("case.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let params = doc["params"].as_table().unwrap().clone();
    let objects: Vec<&toml::Table> = params
        .iter()
        .filter(|(k, _)| k.starts_with("object"))
        .filter_map(|(_, v)| v.as_table())
        .collect();
    let listed: Vec<String> = objects
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(String::from))
        .collect();
    let obj = std::fs::read_to_string(dir.join("mesh.obj")).unwrap();
    let mut snap = Snapshot {
        camera: camera_of(&params),
        ..Snapshot::default()
    };
    snap.materials.push(material_of(
        params.get("material").and_then(|v| v.as_table()),
        name,
    ));
    snap.materials.push(material_of(None, "default"));
    for g in parse_obj(&obj) {
        if !listed.contains(&g.name) {
            continue;
        }
        let mat = if g.name == "Plane" { 1 } else { 0 };
        let listed_obj = objects
            .iter()
            .find(|t| t.get("name").and_then(|n| n.as_str()) == Some(g.name.as_str()))
            .unwrap();
        let mesh = if listed_obj.get("type").and_then(|t| t.as_str()) == Some("sphere")
            && listed_obj.get("render_perfect").and_then(|t| t.as_bool()) == Some(true)
        {
            perfect_sphere(
                f3(&listed_obj["position"]),
                listed_obj["radius"].as_float().unwrap(),
                96,
                params
                    .get("texture_tag")
                    .and_then(|t| t.get("projection"))
                    .and_then(|p| p.as_str()),
            )
        } else {
            g.mesh
        };
        snap.meshes.push(mesh);
        let ix = snap.meshes.len() - 1;
        snap.instances
            .push(Instance::new(ix, DMat4::IDENTITY, mat, ix as u32));
    }
    snap.lights = objects.iter().filter_map(|t| light_of(t)).collect();
    let png = std::fs::read(dir.join("render.png")).unwrap();
    let img = forge_image::decode(&png, &forge_image::Limits::default())
        .unwrap()
        .image;
    let expected = ImageBuf::from_fn(img.width, img.height, |x, y| {
        let p = img.pixel(x, y);
        [p[0], p[1], p[2]]
    });
    Case {
        name: name.to_string(),
        params,
        snapshot: snap,
        expected,
    }
}
