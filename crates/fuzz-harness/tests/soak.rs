//! docs/specs/PLATFORM.md §9.5: the random-command soak. The smoke run is part of `cargo test`;
//! `soak_long` is the nightly job (`FORGE_SOAK_SECS=600 cargo test --release -p
//! forge-fuzz-harness --test soak -- --ignored --nocapture`).

use std::time::Duration;

use forge_fuzz_harness::soak::{SoakConfig, run};

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[test]
fn soak_smoke() {
    let stats = run(&SoakConfig {
        seed: env_u64("FORGE_SOAK_SEED", 1),
        max_steps: Some(1_500),
        max_time: None,
        checkpoint_every: 250,
    });
    assert_eq!(stats.steps, 1_500);
    assert!(stats.ok > 100, "too few commands succeeded: {stats:?}");
    assert!(stats.commands_used > 50);
}

#[test]
#[ignore = "long: set FORGE_SOAK_SECS (default 600)"]
fn soak_long() {
    let secs = env_u64("FORGE_SOAK_SECS", 600);
    let seed = env_u64("FORGE_SOAK_SEED", 20260303);
    let stats = run(&SoakConfig {
        seed,
        max_steps: None,
        max_time: Some(Duration::from_secs(secs)),
        checkpoint_every: 500,
    });
    println!("soak seed {seed}: {stats:?}");
}
