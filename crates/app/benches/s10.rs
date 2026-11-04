//! S10 performance scenario at scale (docs/specs/PERFORMANCE.md): a scene with 1 M polygons
//! and 10 k clones. Headless and CPU only, so it runs on any CI runner. Run:
//! `cargo bench -p forge-app --bench s10`; the budgets live in `bench/budgets.toml` and are
//! checked by `cargo run -p forge-bench-check`.
//!
//! Group and bench ids are the keys of the budget file: keep them in step.

use std::hint::black_box;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use forge_app::schedule::{Scheduler, Threads};
use forge_app::{App, default_registry};
use forge_core::{Document, EvalCtx, NodeId, Value, file};
use forge_generators::evaluate;
use forge_io::obj::{WriteOptions, write};
use forge_io::{export_scene, import_into};

/// 1000 x 1000 quads: 1 000 000 polygons.
const PLANE_SEGMENTS: i64 = 1000;
const CLONES: i64 = 10_000;

fn registry() -> Arc<forge_core::Registry> {
    Arc::new(default_registry().expect("built-in registry"))
}

/// A plane of 1 M quads; optionally with a bend deformer child.
fn plane_scene(bend: bool) -> (Document, NodeId) {
    let mut doc = Document::new(registry());
    let plane = doc
        .transact("plane", |tx| {
            let p = tx.create("forge.plane", None, None)?;
            tx.set_param(p, "segments_w", Value::Int(PLANE_SEGMENTS))?;
            tx.set_param(p, "segments_h", Value::Int(PLANE_SEGMENTS))?;
            if bend {
                tx.create("forge.bend", Some(p), None)?;
            }
            Ok(p)
        })
        .expect("plane scene");
    (doc, plane)
}

/// 10 k clones driven by a Plain effector and a Random effector (noise), both limited by a
/// spherical field: clone, effector and field evaluation at scale.
fn field_cloner_scene() -> Document {
    let mut doc = Document::new(registry());
    doc.transact("cloner", |tx| {
        let cloner = tx.create("forge.cloner", None, None)?;
        tx.set_param(cloner, "mode", Value::Enum("linear".into()))?;
        tx.set_param(cloner, "linear_count", Value::Int(CLONES))?;
        tx.create("forge.cube", Some(cloner), None)?;
        let field = tx.create("forge.field.spherical", None, None)?;
        let plain = tx.create("forge.effector.plain", None, None)?;
        tx.set_param(plain, "p", Value::Vec3([0.0, 40.0, 0.0]))?;
        tx.set_param(plain, "fields", Value::LinkList(vec![field]))?;
        let random = tx.create("forge.effector.random", None, None)?;
        tx.set_param(random, "random_mode", Value::Enum("noise".into()))?;
        tx.set_param(cloner, "effectors", Value::LinkList(vec![plain, random]))?;
        Ok(())
    })
    .expect("cloner scene");
    doc
}

/// The 1 M polygon plane as an editable polygon object (a mesh value, so it is saved as a
/// binary chunk), built once outside the timed loops.
fn editable_scene() -> Document {
    let (doc, _) = plane_scene(false);
    let objects = export_scene(doc.scene(), None);
    let mut out = Document::new(registry());
    out.transact("editable", |tx| import_into(tx, &objects))
        .expect("import");
    out
}

fn generators(c: &mut Criterion) {
    let (doc, plane) = plane_scene(false);
    let (bent, bent_plane) = plane_scene(true);
    let mut g = c.benchmark_group("s10_generate");
    g.sample_size(10);
    g.bench_function("plane_1M_polys", |b| {
        b.iter(|| black_box(evaluate(doc.scene(), plane, &EvalCtx::default()).unwrap()))
    });
    g.bench_function("plane_1M_polys_bend", |b| {
        b.iter(|| black_box(evaluate(bent.scene(), bent_plane, &EvalCtx::default()).unwrap()))
    });
    g.finish();

    let doc = field_cloner_scene();
    let mut g = c.benchmark_group("s10_clones");
    g.sample_size(10);
    for (name, threads) in [
        ("fields_10k_serial", Threads::Serial),
        ("fields_10k_parallel", Threads::Auto),
    ] {
        let mut s = Scheduler::new(threads);
        g.bench_function(name, |b| {
            b.iter(|| {
                s.clear();
                black_box(s.evaluate(doc.scene(), 0).outputs.len())
            })
        });
    }
    g.finish();
}

fn files(c: &mut Criterion) {
    let doc = editable_scene();
    let scene = doc.scene();
    let opts = file::SaveOptions::default();
    let bytes = file::save_with(scene, &opts);
    let reg = scene.registry().clone();
    let mut g = c.benchmark_group("s10_file");
    g.sample_size(10);
    g.bench_function("save_1M_polys_chunked", |b| {
        b.iter(|| black_box(file::save_with(scene, &opts).len()))
    });
    g.bench_function("load_1M_polys_chunked", |b| {
        b.iter(|| black_box(file::load_bytes(reg.clone(), &bytes).unwrap().0.revision()))
    });
    g.finish();

    let (doc, _) = plane_scene(false);
    let objects = export_scene(doc.scene(), None);
    let mut g = c.benchmark_group("s10_export");
    g.sample_size(10);
    g.bench_function("obj_write_1M_polys", |b| {
        b.iter(|| black_box(write(&objects, &WriteOptions::default()).len()))
    });
    g.finish();
}

/// Open a large scene and produce the first frame's draw data, without a window: the
/// headless part of "startup to first frame" (window and GPU setup are not measured).
fn startup(c: &mut Criterion) {
    let doc = editable_scene();
    let bytes = file::save_with(doc.scene(), &file::SaveOptions::default());
    let dir = std::env::temp_dir().join(format!("forge-s10-bench-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("big.forge");
    std::fs::write(&path, &bytes).expect("write scene");
    let path_str = path.to_string_lossy().into_owned();

    let mut g = c.benchmark_group("s10_startup");
    g.sample_size(10);
    g.bench_function("empty_app_to_first_frame", |b| {
        b.iter(|| {
            let mut app = App::new_default();
            black_box(app.scheduled_draw(0).outputs.len())
        })
    });
    g.bench_function("open_1M_polys_to_first_frame", |b| {
        b.iter(|| {
            let mut app = App::new_default();
            app.open(&path_str).expect("open");
            black_box(app.scheduled_draw(0).outputs.len())
        })
    });
    g.finish();
    let _ = std::fs::remove_dir_all(&dir);
}

criterion_group!(s10, generators, files, startup);
criterion_main!(s10);
