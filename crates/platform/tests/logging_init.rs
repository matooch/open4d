//! `logging::init_with` writes events to the file and the in-memory tail, honouring the
//! level (docs/specs/PLATFORM.md §8.2). One global subscriber per process: a single test.

use forge_platform::logging::{LogConfig, init_with, recent_lines};

#[test]
fn events_reach_the_file_and_the_tail_filtered_by_level() {
    let t = tempfile::tempdir().unwrap();
    let config = LogConfig {
        level: "warn,forge_special=debug".into(),
        ..LogConfig::default()
    };
    let info = init_with(Some(t.path().to_path_buf()), &config, false).expect("first init");
    assert_eq!(info.level, "warn,forge_special=debug");
    assert!(
        init_with(None, &config, false).is_none(),
        "second init is a no-op"
    );

    tracing::info!("hidden info line");
    tracing::warn!("visible warning");
    tracing::debug!(target: "forge_special", "special debug");

    let file = std::fs::read_to_string(t.path().join("forge.log")).unwrap();
    assert!(file.contains("visible warning"), "{file}");
    assert!(file.contains("special debug"));
    assert!(!file.contains("hidden info line"));
    assert!(file.contains("WARN"), "level is printed: {file}");
    assert!(!file.contains('\u{1b}'), "no ANSI colours");

    let tail = recent_lines();
    assert!(
        tail.iter().any(|l| l.contains("visible warning")),
        "{tail:?}"
    );
}
