//! Scene-building helpers shared by the render tests.
#![allow(dead_code)]

pub mod golden;

use std::collections::BTreeMap;

use forge_core::Value;
use forge_material::{Material, compile};
use forge_render_api::*;
use glam::{DMat4, DVec3};

pub fn material(over: &[(&str, Value)]) -> MaterialData {
    let map: BTreeMap<String, Value> = over
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    MaterialData::new("m", compile(&Material::from_values(&map)))
}

/// A white (or `color`) Lambert material with the specular layer off.
pub fn diffuse(color: [f64; 3]) -> MaterialData {
    material(&[
        ("color.color", Value::Color(color)),
        ("enable.reflectance", Value::Bool(false)),
    ])
}

/// UV sphere with smooth corner normals, outward winding for +Y up.
pub fn sphere(radius: f64, seg: u32) -> MeshData {
    let mut pos = Vec::new();
    for j in 0..=seg {
        let th = std::f64::consts::PI * j as f64 / seg as f64;
        for i in 0..seg {
            let ph = std::f64::consts::TAU * i as f64 / seg as f64;
            pos.push([
                radius * th.sin() * ph.cos(),
                radius * th.cos(),
                radius * th.sin() * ph.sin(),
            ]);
        }
    }
    let mut tris = Vec::new();
    let idx = |j: u32, i: u32| j * seg + (i % seg);
    for j in 0..seg {
        for i in 0..seg {
            let (a, b, c, d) = (idx(j, i), idx(j, i + 1), idx(j + 1, i), idx(j + 1, i + 1));
            if j > 0 {
                tris.push([a, c, b]);
            }
            if j + 1 < seg {
                tris.push([b, c, d]);
            }
        }
    }
    let mut m = MeshData::new(pos, tris);
    m.normals = m
        .triangles
        .iter()
        .flatten()
        .map(|&v| DVec3::from(m.positions[v as usize]).normalize().to_array())
        .collect();
    m
}

/// A quad in the XZ plane (y = 0) of half size `h`, facing +Y.
pub fn plane(h: f64) -> MeshData {
    MeshData::new(
        vec![[-h, 0.0, -h], [h, 0.0, -h], [h, 0.0, h], [-h, 0.0, h]],
        vec![[0, 2, 1], [0, 3, 2]],
    )
}

/// A unit-ish cube mesh of half size `h`.
pub fn cube(h: f64) -> MeshData {
    let p = [
        [-h, -h, -h],
        [h, -h, -h],
        [h, h, -h],
        [-h, h, -h],
        [-h, -h, h],
        [h, -h, h],
        [h, h, h],
        [-h, h, h],
    ];
    let q = [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 4, 7, 3],
        [1, 2, 6, 5],
        [3, 7, 6, 2],
        [0, 1, 5, 4],
    ];
    let mut t = Vec::new();
    for f in q {
        t.push([f[0], f[1], f[2]]);
        t.push([f[0], f[2], f[3]]);
    }
    MeshData::new(p.to_vec(), t)
}

pub fn looking_at_origin(dist: f64) -> Camera {
    Camera::from_hpb(
        DVec3::new(0.0, 0.0, -dist),
        [0.0; 3],
        Projection::Perspective {
            focal_mm: 36.0,
            sensor_mm: 36.0,
            film_offset: [0.0; 2],
        },
    )
}

pub fn snapshot_with(
    mesh: MeshData,
    mat: MaterialData,
    transform: DMat4,
    camera: Camera,
) -> Snapshot {
    Snapshot {
        meshes: vec![mesh],
        instances: vec![Instance::new(0, transform, 0, 0)],
        materials: vec![mat],
        lights: vec![],
        camera,
        environment: Environment::default(),
    }
}

pub fn px(out: &RenderOutput, x: u32, y: u32) -> [f32; 4] {
    out.beauty.pixel(x, y)
}

/// Mean RGB of the pixels inside a centred box of half size `r`.
pub fn centre_mean(out: &RenderOutput, r: u32) -> [f64; 3] {
    let (cx, cy) = (out.width / 2, out.height / 2);
    let mut s = [0.0; 3];
    let mut n = 0.0;
    for y in cy - r..cy + r {
        for x in cx - r..cx + r {
            let p = out.beauty.pixel(x, y);
            for k in 0..3 {
                s[k] += p[k] as f64;
            }
            n += 1.0;
        }
    }
    s.map(|v| v / n)
}
