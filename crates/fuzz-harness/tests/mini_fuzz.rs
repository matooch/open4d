//! docs/specs/PLATFORM.md §9.3: the stable-toolchain mini-fuzz. Every cargo-fuzz target's entry
//! point runs on the committed seeds and a few thousand seeded mutations of them, under the
//! counting allocator. Deterministic: a failure reproduces from the printed iteration and input.

use std::panic::{AssertUnwindSafe, catch_unwind};

use forge_fuzz_harness::alloc::{CountingAlloc, DEFAULT_ALLOC_LIMIT, assert_bounded, reset};
use forge_fuzz_harness::targets::MAX_INPUT;
use forge_fuzz_harness::{
    FS_COMMANDS, XorShift64, fuzz_command_args, fuzz_exr, fuzz_image, fuzz_obj, fuzz_rpc_line,
    fuzz_scene_file, fuzz_script, mutate, seed_corpus,
};

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

fn hex(data: &[u8]) -> String {
    data.iter().take(512).map(|b| format!("{b:02x}")).collect()
}

/// Runs `entry` on the seeds, then on `iterations` mutations; fails with the offending input.
fn mini_fuzz(target: &str, entry: fn(&[u8]), iterations: usize, seed: u64) {
    let seeds = seed_corpus(target);
    assert!(!seeds.is_empty(), "no seed corpus for {target}");
    let offset: u64 = std::env::var("FORGE_MINI_FUZZ_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let seed = seed + offset;
    let mut rng = XorShift64::new(seed);
    let total = seeds.len() + iterations;
    for i in 0..total {
        let input = if i < seeds.len() {
            seeds[i].clone()
        } else {
            mutate(&mut rng, &seeds, MAX_INPUT)
        };
        reset();
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            entry(&input);
            assert_bounded(DEFAULT_ALLOC_LIMIT);
        }));
        if let Err(p) = outcome {
            let msg = p
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            let path = std::env::temp_dir().join(format!("forge-fuzz-crash-{target}.bin"));
            let _ = std::fs::write(&path, &input);
            panic!(
                "mini-fuzz {target}: iteration {i} (seed {seed}) panicked: {msg}\n\
                 input ({} bytes, saved to {}): {}",
                input.len(),
                path.display(),
                hex(&input)
            );
        }
    }
}

fn iterations(default: usize) -> usize {
    std::env::var("FORGE_MINI_FUZZ_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[test]
fn scene_file() {
    mini_fuzz("scene_file", fuzz_scene_file, iterations(4000), 11);
}

#[test]
fn obj() {
    mini_fuzz("obj", fuzz_obj, iterations(4000), 12);
}

#[test]
fn image() {
    mini_fuzz("image", fuzz_image, iterations(4000), 13);
}

#[test]
fn exr() {
    mini_fuzz("exr", fuzz_exr, iterations(3000), 14);
}

#[test]
fn command_args() {
    mini_fuzz("command_args", fuzz_command_args, iterations(2500), 15);
}

#[test]
fn rpc_line() {
    mini_fuzz("rpc_line", fuzz_rpc_line, iterations(3000), 16);
}

#[test]
fn script() {
    mini_fuzz("script", fuzz_script, iterations(1500), 17);
}

#[test]
fn counting_allocator_sees_big_allocations() {
    reset();
    let big = vec![1u8; 8 << 20];
    assert!(forge_fuzz_harness::alloc::peak() >= 8 << 20);
    let r = catch_unwind(|| assert_bounded(1 << 20));
    assert!(r.is_err(), "a limit below the peak must fail");
    drop(big);
}

#[test]
fn file_commands_are_excluded_and_exist() {
    let app = forge_app::App::new_default();
    for id in FS_COMMANDS {
        assert!(app.commands.get(id).is_some(), "{id} is not a command");
    }
    // Every command that names a file path in its schema must be excluded.
    for c in app.commands.iter() {
        let schema = c.args_schema().to_string();
        if schema.contains("\"path\"") {
            assert!(
                FS_COMMANDS.contains(&c.id()),
                "{} takes a path but is not in FS_COMMANDS",
                c.id()
            );
        }
    }
}

/// Every command id, each with schema-valid, corrupted and hostile arguments, on a small scene.
#[test]
fn every_command_survives_hostile_arguments() {
    use forge_fuzz_harness::schema_gen::{GenCtx, corrupt, generate};
    use forge_fuzz_harness::targets::{fuzzable_command_ids, seed_scene};
    use serde_json::{Value, json};

    let hostile: Vec<Value> = vec![
        Value::Null,
        json!([]),
        json!({}),
        json!("x"),
        json!(-1),
        json!(i64::MAX),
        json!(u64::MAX),
        json!(1e308),
        json!({"nodes": [u64::MAX, 0, 1, 1], "node": u64::MAX, "id": u64::MAX, "count": i64::MAX}),
        json!({"copies": i64::MAX, "segments": i64::MAX, "levels": i64::MAX, "size": 1e300}),
        json!({"a": {"a": {"a": {"a": {"a": {"a": {"a": null}}}}}}}),
        json!({"frame": i64::MIN, "start": i64::MIN, "end": i64::MAX, "fps": 0}),
    ];
    let probe = forge_app::App::new_default();
    let ids = fuzzable_command_ids(&probe);
    let mut rng = XorShift64::new(77);
    let mut ran = 0usize;
    for id in &ids {
        let schema = probe.commands.get(id).expect("listed").args_schema();
        let mut cases = hostile.clone();
        for k in 0..10 {
            let nodes = [1u64, 2, 3];
            let mut cx = GenCtx {
                rng: &mut rng,
                nodes: &nodes,
            };
            let mut v = generate(&mut cx, &schema, &schema, 0);
            if k >= 6 {
                corrupt(&mut rng, &mut v);
            }
            cases.push(v);
        }
        for args in cases {
            let mut app = forge_app::App::new_default();
            seed_scene(&mut app);
            reset();
            let shown = args.to_string();
            let r = catch_unwind(AssertUnwindSafe(|| {
                let _ = app.run(id, args);
                assert_bounded(DEFAULT_ALLOC_LIMIT);
            }));
            assert!(
                r.is_ok(),
                "command {id} panicked on args {}",
                shown.chars().take(400).collect::<String>()
            );
            ran += 1;
        }
    }
    assert!(ran > 2000, "ran only {ran} cases");
}

#[test]
fn resealed_chunks_reach_the_decoders() {
    use forge_fuzz_harness::targets::reseal_chunks;
    let seeds = seed_corpus("scene_file");
    let chunked = seeds
        .iter()
        .find(|s| s.windows(8).any(|w| w == b"%%FORGE-"))
        .expect("a chunked seed");
    // Unchanged data reseals to itself.
    assert_eq!(reseal_chunks(chunked).as_deref(), Some(&chunked[..]));
    // A flipped payload byte fails the hash; resealed, it passes the hash check.
    let mut bad = chunked.clone();
    let last = bad.len() - 1;
    bad[last] ^= 0x55;
    let registry = std::sync::Arc::new(forge_app::default_registry().unwrap());
    let raw = forge_core::file::load_bytes(registry.clone(), &bad);
    assert!(
        matches!(raw, Err(ref e) if e.to_string().contains("hash")),
        "{raw:?}"
    );
    let sealed = reseal_chunks(&bad).expect("frame intact");
    let after = forge_core::file::load_bytes(registry, &sealed);
    assert!(
        !matches!(after, Err(ref e) if e.to_string().contains("hash")),
        "{after:?}"
    );
}
