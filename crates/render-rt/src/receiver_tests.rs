use super::*;
use crate::path_trace::attributes;
use crate::shader::receiver_geometry_wgsl;
use forge_render_api::{Camera, Instance, MaterialData, MeshData, Projection};
use forge_render_gpu::{await_mapped, map_read};
use glam::{DMat4, DQuat};
use wgpu::util::DeviceExt;

// Deliberately not a visibility shader: expose the geometry before any material or ray work.
const PROBE: &str = "
@group(0) @binding(1) var<storage, read> input: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> output: array<vec4<f32>>;
@compute @workgroup_size(1)
fn probe(@builtin(global_invocation_id) id: vec3<u32>) {
    let b = id.x * 6u;
    let dst = id.x * 5u;
    let p = input[b];
    let frame = input[b + 2u];
    let ids = receiver_triangle(u32(p.w), u32(frame.w));
    output[dst] = vec4<f32>(vec2<f32>(ids), 0.0, 0.0);
    if ids.x == u32(frame.w) { return; }
    let g = receiver_geometry(p.xyz, ids);
    let pixel = input[b + 1u];
    let view = receiver_view(pixel.xy, pixel.zw, frame.xyz, mat3x3<f32>(
        input[b + 3u].xyz, input[b + 4u].xyz, input[b + 5u].xyz));
    output[dst + 1u] = vec4<f32>(g.p, view.x);
    output[dst + 2u] = vec4<f32>(g.ng, view.y);
    output[dst + 3u] = vec4<f32>(g.ns, view.z);
    output[dst + 4u] = vec4<f32>(g.uvw, 0.0);
}";

#[test]
fn gpu_raster_receiver_matches_cpu_primary_geometry() {
    let (device, queue) = match crate::request_default_device() {
        Ok(gpu) => gpu,
        Err(err) if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") => {
            panic!("FORGE_REQUIRE_GPU=1 but no GPU: {err}");
        }
        Err(err) => {
            eprintln!("skipping GPU test: {err}");
            return;
        }
    };
    let mut mesh = MeshData::new(
        vec![[0., 0., 0.], [2., 0., 1.], [0., 3., 1.], [2., 2., 2.]],
        vec![[0, 1, 2], [2, 1, 3]],
    );
    mesh.normals = vec![
        [0.1, 0.2, 1.],
        [0.6, -0.3, 1.],
        [-0.1, 0.7, 1.],
        [0.3, -0.1, 1.],
        [0.5, 0.3, 1.],
        [-0.4, 0.6, 1.],
    ];
    mesh.uvs = vec![
        [0., 2., 1.],
        [1., 0., 2.],
        [2., 1., 3.],
        [1., 0., 3.],
        [0., 1., 2.],
        [1., 1., 1.],
    ];
    let transformed = DMat4::from_scale_rotation_translation(
        DVec3::new(-2.5, 0.6, 1.3),
        DQuat::from_rotation_y(0.3),
        DVec3::new(4., -1., 6.),
    );
    let mut snapshot = Snapshot {
        meshes: vec![mesh, MeshData::new(vec![], vec![])],
        instances: vec![
            Instance::new(1, DMat4::IDENTITY, 0, 10),
            Instance::new(0, DMat4::IDENTITY, 0, 11),
            Instance::new(1, DMat4::IDENTITY, 0, 12),
            Instance::new(0, transformed, 0, 13),
            Instance::new(1, DMat4::IDENTITY, 0, 14),
        ],
        materials: vec![MaterialData::new("receiver", Default::default())],
        ..Snapshot::default()
    };
    let settings = RenderSettings {
        width: 9,
        height: 7,
        region: Some([2, 1, 7, 6]),
        ..Default::default()
    };
    let primary = PrimaryGeometry::new(&snapshot).unwrap();
    let attrs = attributes(&device, &snapshot, &primary.scene).unwrap();
    let count = snapshot.instances.len();
    let mut rows = Vec::<[f32; 4]>::new();
    let mut expected = Vec::new();
    for projection in [
        Projection::Perspective {
            focal_mm: 42.,
            sensor_mm: 36.,
            film_offset: [0.12, -0.18],
        },
        Projection::Orthographic { width: 8. },
    ] {
        snapshot.camera =
            Camera::from_hpb(DVec3::new(2., -3., -10.), [0.15, -0.2, 0.3], projection);
        let frame = snapshot
            .camera
            .pixel_frame(settings.width, settings.height)
            .map_or([0.; 3], |(f, [x, y])| [f as f32, x as f32, y as f32]);
        // Synthetic G-buffer values at absolute pixels in an offset crop;
        // clear samples include a pixel outside that crop.
        for (key, inst, tri, x, y) in [
            (0, count, 0, 1, 0),
            (0, count, 0, 2, 1),
            (1, 1, 0, 3, 2),
            (2, 1, 1, 5, 2),
            (3, 3, 0, 2, 4),
            (4, 3, 1, 6, 5),
            (5, count, 0, 6, 1),
        ] {
            let mut positions = [[0.; 4]; 63];
            let mut normals = [[0.; 4]; 63];
            let jitter = [-0.375, 0.25];
            let i = y * settings.width as usize + x;
            let point = if inst == count {
                DVec3::ZERO
            } else {
                let mesh = &snapshot.meshes[0];
                let [a, b, c] =
                    mesh.triangles[tri].map(|j| DVec3::from(mesh.positions[j as usize]));
                let p = a * 0.23 + b * 0.31 + c * 0.46 + (b - a).cross(c - a).normalize() * 0.002;
                DMat4::from_cols_array(&snapshot.instances[inst].transform).transform_point3(p)
            };
            let [px, py, pz] = point.as_vec3().to_array();
            positions[i] = [px, py, pz, f32::from(key != 0)];
            normals[i][3] = key as f32;
            rows.extend([
                [px, py, pz, key as f32],
                [x as f32, y as f32, jitter[0], jitter[1]],
                [frame[0], frame[1], frame[2], count as f32],
            ]);
            rows.extend(
                snapshot.camera.matrix().as_mat4().to_cols_array_2d()[..3]
                    .iter()
                    .copied(),
            );
            let result = (inst != count).then(|| {
                let geom = primary
                    .scene
                    .geometry_at(&snapshot, xyz(positions[i]), inst, tri);
                let surface = primary.surface(
                    &snapshot,
                    &settings,
                    &PrimaryInput {
                        positions: &positions,
                        normals: &normals,
                        jitter,
                        epsilon: 1e-6,
                    },
                    i,
                );
                [surface.position, geom.ng, geom.ns, geom.uvw, surface.view]
            });
            expected.push(((inst, tri), result));
        }
    }
    let source = format!("{}{PROBE}", receiver_geometry_wgsl());
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("raster receiver probe"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("probe"),
        compilation_options: Default::default(),
        cache: None,
    });
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&rows),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = expected.len() as u64 * 5 * 16;
    let buffer = |usage| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage,
            mapped_at_creation: false,
        })
    };
    let output = buffer(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
    let read = buffer(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST);
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[&attrs, &input, &output]
            .map(|buffer| buffer.as_entire_binding())
            .into_iter()
            .enumerate()
            .map(|(i, resource)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource,
            })
            .collect::<Vec<_>>(),
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(expected.len() as u32, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &read, 0, size);
    queue.submit([encoder.finish()]);
    let slice = read.slice(..);
    await_mapped(&device, &map_read(&slice)).unwrap();
    let mapped = slice.get_mapped_range().unwrap();
    let actual: &[[f32; 4]] = bytemuck::cast_slice(&mapped);
    for (i, (((inst, tri), reference), g)) in expected
        .into_iter()
        .zip(actual.as_chunks::<5>().0)
        .enumerate()
    {
        assert_eq!(g[0], [inst as f32, tri as f32, 0., 0.], "probe {i}");
        if let Some(reference) = reference {
            let view = DVec3::new(g[1][3] as f64, g[2][3] as f64, g[3][3] as f64);
            for (label, (gpu, cpu)) in ["position", "Ng", "Ns", "UVW", "view"].into_iter().zip(
                [xyz(g[1]), xyz(g[2]), xyz(g[3]), xyz(g[4]), view]
                    .into_iter()
                    .zip(reference),
            ) {
                assert!(
                    (gpu - cpu).abs().max_element() < 2e-6,
                    "probe {i} {label}: {gpu} != {cpu}"
                );
            }
        }
    }
}
