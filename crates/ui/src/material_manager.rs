//! The Materials panel: the document's material list with the commands that create, assign and
//! delete (docs/specs/MATERIAL.md M6). Every edit is a `material.*` command.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use egui::{Color32, ColorImage, Label, Sense, TextureHandle, Ui, vec2};
use forge_core::{NodeId, Scene, Value};
use forge_geometry::triangulate::triangulate;
use forge_material::{MATERIAL, Material, compile};
use forge_render_api::{
    AreaShape, Background, Control, Environment, Instance, Light, LightKind, Lighting,
    MaterialData, MeshData, RenderEngine, RenderSettings, Snapshot,
};
use forge_render_cpu::CpuEngine;
use forge_shaders::ShaderSet;
use glam::{DMat4, DVec3};
use serde_json::json;

use crate::object_manager::Cmd;
use crate::state::EditorState;

const SIZE: usize = 64;

#[derive(Clone, Copy)]
pub(crate) struct MaterialDrag(pub NodeId);

pub(crate) fn assign_to(MaterialDrag(material): MaterialDrag, node: NodeId) -> Cmd {
    (
        "material.assign",
        json!({"material": material.0, "nodes": [node.0]}),
    )
}

/// Fixed studio snapshot: one smooth UV sphere with an area key and environment fill.
pub fn render_preview(scene: &Scene, id: NodeId) -> Option<ColorImage> {
    let material = scene.node(id).filter(|m| m.type_id == MATERIAL)?;
    let (shaders, _) = ShaderSet::from_material(scene, id, &crate::material_preview::load_image);
    let graph = compile(&Material::from_node(material));
    let values = BTreeMap::from([("segments".to_owned(), Value::Int(24))]);
    let (mesh, uv) = forge_generators::uv::build_with_uvs("forge.sphere", &values).ok()?;
    let uv = uv?;
    let mut data = MeshData {
        positions: mesh.points.iter().map(|p| p.to_array()).collect(),
        ..MeshData::default()
    };
    for (f, tri) in triangulate(&mesh) {
        data.triangles.push(tri);
        let poly = &mesh.polys[f as usize];
        for v in tri {
            data.normals
                .push(mesh.points[v as usize].normalize_or_zero().to_array());
            let corner = poly.iter().position(|&p| p == v)?;
            let [u, v] = uv.corners[f as usize][corner];
            data.uvs.push([u, v, 0.0]);
        }
    }
    let mut key = Light::new(
        LightKind::Area {
            shape: AreaShape::Sphere { radius: 120.0 },
            two_sided: false,
        },
        DMat4::from_translation(DVec3::new(140.0, 210.0, -360.0)),
    );
    key.intensity = 3.0;
    let scene = Snapshot {
        meshes: vec![data],
        materials: vec![MaterialData::new(&material.name, graph).with_sampler(Arc::new(shaders))],
        instances: vec![Instance::new(
            0,
            DMat4::from_scale(DVec3::splat(1.75)),
            0,
            1,
        )],
        lights: vec![key],
        environment: Environment {
            background: Background::Color([0.045, 0.049, 0.057]),
            lighting: Lighting::Constant {
                color: [1.0; 3],
                intensity: 0.25,
            },
        },
        ..Snapshot::default()
    };
    let settings = RenderSettings {
        threads: Some(1),
        max_depth: 3,
        ..RenderSettings::new(SIZE as u32, SIZE as u32, 4)
    };
    let out = CpuEngine::new()
        .render(&scene, &settings, &Control::new())
        .ok()?;
    let pixels = out
        .beauty
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, _]| {
            egui::Rgba::from_rgb(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)).into()
        })
        .collect();
    Some(ColorImage::new([SIZE, SIZE], pixels))
}

struct Job {
    id: NodeId,
    rev: u64,
    result: Receiver<Option<ColorImage>>,
}

/// A single in-flight render, with GPU images retained only for current materials.
#[derive(Default)]
pub struct MaterialPreviews {
    pending: Option<Job>,
    complete: BTreeMap<NodeId, (u64, Option<TextureHandle>)>,
}

impl MaterialPreviews {
    fn refresh(&mut self, ctx: &egui::Context, scene: &Scene) {
        let live: BTreeSet<_> = scene.materials().iter().copied().collect();
        self.complete
            .retain(|id, (rev, _)| live.contains(id) && scene.deep_revision(*id) == Some(*rev));
        if let Some(job) = self.pending.take() {
            match job.result.try_recv() {
                Err(TryRecvError::Empty) => self.pending = Some(job),
                result
                    if live.contains(&job.id) && scene.deep_revision(job.id) == Some(job.rev) =>
                {
                    let tex = result.ok().flatten().map(|image| {
                        ctx.load_texture("material thumbnail", image, egui::TextureOptions::LINEAR)
                    });
                    self.complete.insert(job.id, (job.rev, tex));
                }
                _ => {}
            }
        }
        if self.pending.is_none()
            && let Some((id, rev)) = scene.materials().iter().find_map(|&id| {
                (!self.complete.contains_key(&id))
                    .then(|| scene.deep_revision(id).map(|rev| (id, rev)))
                    .flatten()
            })
        {
            let (tx, result) = channel();
            let scene = scene.clone();
            let ctx = ctx.clone();
            if std::thread::Builder::new()
                .name("material-thumbnail".into())
                .spawn(move || {
                    let _ = tx.send(render_preview(&scene, id));
                    ctx.request_repaint();
                })
                .is_ok()
            {
                self.pending = Some(Job { id, rev, result });
            } else {
                self.complete.insert(id, (rev, None));
            }
        }
    }
}

/// The materials as (id, name, icon colour as 8-bit sRGB), in list order.
pub fn rows(s: &EditorState) -> Vec<(NodeId, String, [u8; 3])> {
    let scene = s.doc.scene();
    scene
        .materials()
        .iter()
        .filter_map(|m| {
            let n = scene.node(*m)?;
            let c = match n.params.get("icon_color") {
                Some(Value::Color(c)) => c.map(|x| (x.clamp(0.0, 1.0) * 255.0).round() as u8),
                _ => [200; 3],
            };
            Some((*m, n.name.clone(), c))
        })
        .collect()
}

/// Draws the panel and returns the commands the buttons ask for.
pub fn material_manager_ui(
    ui: &mut Ui,
    s: &EditorState,
    previews: &mut MaterialPreviews,
) -> Vec<Cmd> {
    let mut cmds: Vec<Cmd> = Vec::new();
    let has_selection = !s.selection.nodes.is_empty();
    ui.horizontal(|ui| {
        if ui.button("New").clicked() {
            cmds.push(("material.create", json!({})));
        }
    });
    ui.separator();
    let rows = rows(s);
    if rows.is_empty() {
        ui.weak("No materials. New creates one.");
    }
    previews.refresh(ui.ctx(), s.doc.scene());
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (id, name, c) in rows {
                ui.push_id(id, |ui| {
                    ui.vertical(|ui| {
                        let size = vec2(SIZE as f32, SIZE as f32);
                        let image = if let Some((_, Some(texture))) = previews.complete.get(&id) {
                            ui.add(egui::Image::new((texture.id(), size)).sense(Sense::drag()))
                        } else {
                            let (rect, response) = ui.allocate_exact_size(size, Sense::drag());
                            ui.painter().rect_filled(
                                rect,
                                3.0,
                                Color32::from_rgb(c[0], c[1], c[2]),
                            );
                            response
                        };
                        image.dnd_set_drag_payload(MaterialDrag(id));
                        ui.add_sized(
                            [100.0, 18.0],
                            Label::new(name).truncate().sense(Sense::drag()),
                        )
                        .dnd_set_drag_payload(MaterialDrag(id));
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(has_selection, egui::Button::new("Assign"))
                                .clicked()
                            {
                                cmds.push(("material.assign", json!({"material": id.0})));
                            }
                            if ui.button("Delete").clicked() {
                                cmds.push(("material.delete", json!({"materials": [id.0]})));
                            }
                        });
                    });
                });
            }
        });
    });
    cmds
}
