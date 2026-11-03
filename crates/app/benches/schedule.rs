//! Evaluation scheduler benchmarks (docs/specs/CORE.md section 5b): one animated cube among
//! 1000 static ones, and one 10 000-clone cloner. Run: `cargo bench -p forge-app`.

use criterion::{Criterion, criterion_group, criterion_main};
use forge_app::animate::Evaluated;
use forge_app::default_registry;
use forge_app::schedule::{Scheduler, Threads, drawn_nodes, node_output};
use forge_core::{Document, EvalCtx, Key, Value};
use std::sync::Arc;

fn many_cubes() -> Document {
    let mut doc = Document::new(Arc::new(default_registry().unwrap()));
    doc.transact("bench", |tx| {
        let moving = tx.create("forge.cube", None, None)?;
        tx.set_key(moving, "transform.pos.x", Key::new(0.0, 0.0))?;
        tx.set_key(moving, "transform.pos.x", Key::new(1000.0, 500.0))?;
        for i in 0..1000 {
            let c = tx.create("forge.cube", None, None)?;
            tx.set_param(
                c,
                "transform.pos",
                Value::Vec3([(i % 40) as f64 * 150.0, (i / 40) as f64 * 150.0, 0.0]),
            )?;
        }
        Ok(())
    })
    .unwrap();
    doc
}

fn big_cloner() -> Document {
    let mut doc = Document::new(Arc::new(default_registry().unwrap()));
    doc.transact("bench", |tx| {
        let c = tx.create("forge.cloner", None, None)?;
        tx.set_param(c, "mode", Value::Enum("linear".into()))?;
        tx.set_param(c, "linear_count", Value::Int(10_000))?;
        tx.create("forge.cube", Some(c), None)?;
        Ok(())
    })
    .unwrap();
    doc
}

fn benches(c: &mut Criterion) {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    println!("threads available: {threads}");

    let doc = many_cubes();
    let scene = doc.scene();
    let mut g = c.benchmark_group("animated_cube_plus_1000_static");
    g.sample_size(10);
    // The old path: animate, then evaluate every drawn node.
    let mut frame = 0i64;
    g.bench_function("full_serial_every_frame", |b| {
        b.iter(|| {
            frame = (frame + 1) % 900;
            let ev = Evaluated::default().get(scene, frame);
            let ctx = EvalCtx::at_frame(frame, 30.0);
            drawn_nodes(&ev)
                .into_iter()
                .map(|id| node_output(&ev, id, &ctx).mesh.is_some())
                .filter(|m| *m)
                .count()
        })
    });
    for (name, t) in [
        ("dirty_only_serial", Threads::Serial),
        ("dirty_only_parallel", Threads::Auto),
    ] {
        let mut s = Scheduler::new(t);
        s.evaluate(scene, 0);
        let mut frame = 0i64;
        g.bench_function(name, |b| {
            b.iter(|| {
                frame = (frame + 1) % 900;
                s.evaluate(scene, frame).stats.evaluated.len()
            })
        });
    }
    for (name, t) in [
        ("full_scheduler_serial", Threads::Serial),
        ("full_scheduler_parallel", Threads::Auto),
    ] {
        let mut frame = 0i64;
        g.bench_function(name, |b| {
            b.iter(|| {
                frame = (frame + 1) % 900;
                Scheduler::new(t).evaluate(scene, frame).outputs.len()
            })
        });
    }
    g.finish();

    let doc = big_cloner();
    let scene = doc.scene();
    let mut g = c.benchmark_group("cloner_10k_clones");
    g.sample_size(10);
    for (name, t) in [("serial", Threads::Serial), ("parallel", Threads::Auto)] {
        // one pool for all iterations; the first pass inside each is a full evaluation
        let mut s = Scheduler::new(t);
        g.bench_function(name, |b| {
            b.iter(|| {
                s.clear();
                s.evaluate(scene, 0).outputs.len()
            })
        });
    }
    g.finish();
}

criterion_group!(schedule, benches);
criterion_main!(schedule);
